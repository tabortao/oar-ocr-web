use axum::{
    extract::{Multipart, State},
    http::header,
    Json,
};
use image::ImageReader;
use oar_ocr::oarocr::{OAROCR, OARStructure};
use std::io::Cursor;
use std::sync::Arc;
use std::time::Instant;

use crate::error::AppError;
use crate::logger::{OcrLogEntry, OcrLogger};
use crate::ocr_engine;
use crate::types::*;

/// 共享应用状态
#[derive(Clone)]
pub struct AppState {
    /// 文本 OCR 引擎 — 常驻内存，保证快速响应
    pub ocr: Arc<OAROCR>,
    /// 结构引擎缓存 — 按需加载，用后释放（~160MB，不常驻）
    pub structure_cache: Arc<tokio::sync::Mutex<Option<Arc<OARStructure>>>>,
    pub start_time: Instant,
    pub token: Option<String>,
    pub http_client: Arc<reqwest::Client>,
    /// OCR 请求日志记录器
    pub ocr_logger: Arc<OcrLogger>,
}

/// 支持的图片 MIME 类型
const SUPPORTED_MIME_TYPES: &[&str] = &[
    "image/jpeg",
    "image/png",
    "image/bmp",
    "image/webp",
    "image/tiff",
];

fn is_supported_mime(mime: &str) -> bool {
    SUPPORTED_MIME_TYPES.contains(&mime)
}

// ===== Token 验证 =====

/// 检查 API Token（除 /api/health 外均需验证）
fn check_token(
    state: &AppState,
    headers: &axum::http::HeaderMap,
    uri: &axum::http::Uri,
) -> Result<(), AppError> {
    let Some(ref expected) = state.token else {
        return Ok(());
    };

    // health 端点免验证
    if uri.path() == "/api/health" {
        return Ok(());
    }

    let provided = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(|v| v.trim().to_string())
        .unwrap_or_default();

    if provided.is_empty() {
        Err(AppError::MissingToken)
    } else if provided != *expected {
        Err(AppError::InvalidToken)
    } else {
        Ok(())
    }
}

// ===== Token 验证 =====

/// GET /api/auth/verify — 验证 Token 是否有效
pub async fn auth_verify_handler(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    uri: axum::http::Uri,
) -> Result<Json<serde_json::Value>, AppError> {
    check_token(&state, &headers, &uri)?;
    Ok(Json(serde_json::json!({
        "status": "ok",
        "message": "Token 有效"
    })))
}

// ===== 健康检查 =====

/// GET /api/health — 详细服务状态（无需认证）
pub async fn health_handler(
    State(state): State<AppState>,
) -> Json<HealthResponse> {
    let structure_status = {
        let guard = state.structure_cache.try_lock();
        match guard {
            Ok(g) => {
                if g.is_some() {
                    "loaded".to_string()
                } else {
                    "not_loaded".to_string()
                }
            }
            Err(_) => "busy".to_string(),
        }
    };

    Json(HealthResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_secs: state.start_time.elapsed().as_secs(),
        engines: HealthEngines {
            ocr: EngineStatus {
                status: "ready".to_string(),
                model: "PP-OCRv6 small".to_string(),
            },
            structure: EngineStatus {
                status: structure_status,
                model: "PP-DocLayout_plus-L + SLANet_plus + PP-FormulaNet_plus-S".to_string(),
            },
            auth_enabled: Some(state.token.is_some()),
        },
    })
}

// ===== 日志辅助 =====

/// 记录 OCR 请求日志
fn log_ocr_request(
    logger: &OcrLogger,
    request_type: &str,
    image_source: &str,
    image_url: Option<&str>,
    image_size: u64,
    result_count: usize,
    total_text_length: usize,
    duration_ms: u64,
    status: &str,
    error_message: Option<&str>,
) {
    let entry = OcrLogEntry {
        timestamp: chrono_now(),
        request_type: request_type.to_string(),
        image_source: image_source.to_string(),
        image_url: image_url.map(|s| s.to_string()),
        image_size_bytes: image_size,
        result_count,
        total_text_length,
        duration_ms,
        status: status.to_string(),
        error_message: error_message.map(|s| s.to_string()),
    };
    logger.write(&entry);
}

/// 获取当前时间字符串（ISO 8601）
fn chrono_now() -> String {
    use std::time::SystemTime;
    let secs = SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    // 简单格式化: 2026-06-23T12:00:00Z
    let days = secs / 86400;
    let time_of_day = secs % 86400;
    let h = time_of_day / 3600;
    let m = (time_of_day % 3600) / 60;
    let s = time_of_day % 60;
    let (y, mo, d) = crate::logger::civil_from_days(days as i64);
    format!("{y:04}-{mo:02}-{d:02}T{h:02}:{m:02}:{s:02}Z")
}

// ===== 文本 OCR (multipart) =====

/// POST /api/ocr — multipart/form-data 上传
pub async fn ocr_handler(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    uri: axum::http::Uri,
    mut multipart: Multipart,
) -> Result<Json<OcrResponse>, AppError> {
    check_token(&state, &headers, &uri)?;

    let start = Instant::now();
    let (bytes, content_type) = extract_image_file(&mut multipart).await?;
    validate_mime(&content_type)?;
    let image_size = bytes.len() as u64;

    tracing::info!(
        "收到 OCR 请求 (multipart), 文件大小: {} bytes",
        bytes.len()
    );

    let img = img_from_bytes(&bytes)?;
    let results = do_ocr(&state.ocr, img)?;
    let duration_ms = start.elapsed().as_millis() as u64;

    let total_text_len: usize = results.iter().map(|r| r.text.len()).sum();
    log_ocr_request(
        &state.ocr_logger,
        "ocr",
        "upload",
        None,
        image_size,
        results.len(),
        total_text_len,
        duration_ms,
        "success",
        None,
    );

    Ok(Json(OcrResponse::success(results)))
}

// ===== 文本 OCR (JSON / RESTful + 图床链接) =====

/// POST /api/ocr/json — JSON body
/// `{ "images": ["base64...", "https://example.com/img.jpg"] }`
pub async fn ocr_json_handler(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    uri: axum::http::Uri,
    Json(body): Json<OcrRequest>,
) -> Result<Json<OcrResponse>, AppError> {
    check_token(&state, &headers, &uri)?;

    if body.images.is_empty() {
        return Err(AppError::NoFile);
    }

    let start = Instant::now();
    let mut all_results = Vec::new();
    let mut total_image_size: u64 = 0;
    let mut image_source = "base64";
    let mut image_url: Option<&str> = None;

    for source in &body.images {
        let trimmed = source.trim();
        if trimmed.is_empty() {
            continue;
        }

        if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
            image_source = "url";
            image_url = Some(trimmed);
        }

        let img = resolve_image(&state.http_client, trimmed).await?;
        // 估算图片大小（RGB 像素 × 3）
        total_image_size += (img.width() as u64) * (img.height() as u64) * 3;
        let page_results = do_ocr(&state.ocr, img)?;
        all_results.extend(page_results);
    }

    let duration_ms = start.elapsed().as_millis() as u64;
    let total_text_len: usize = all_results.iter().map(|r| r.text.len()).sum();

    log_ocr_request(
        &state.ocr_logger,
        "ocr",
        image_source,
        image_url,
        total_image_size,
        all_results.len(),
        total_text_len,
        duration_ms,
        "success",
        None,
    );

    Ok(Json(OcrResponse::success(all_results)))
}

// ===== 结构引擎懒加载 =====

/// 按需获取结构引擎（首次加载，后续复用缓存）
async fn get_or_load_structure(
    cache: &tokio::sync::Mutex<Option<Arc<OARStructure>>>,
) -> Result<Arc<OARStructure>, AppError> {
    let mut guard = cache.lock().await;
    if let Some(ref s) = *guard {
        tracing::debug!("复用已加载的结构引擎");
        return Ok(s.clone());
    }

    tracing::info!("结构引擎未加载，开始初始化...");
    let structure = tokio::task::spawn_blocking(ocr_engine::build_structure_engine)
        .await
        .map_err(|e| AppError::Internal(format!("spawn_blocking 失败: {e}")))?
        .map_err(|e| AppError::StructureError(e.to_string()))?;

    *guard = Some(structure.clone());
    Ok(structure)
}

/// 释放结构引擎，回收内存
async fn release_structure(cache: &tokio::sync::Mutex<Option<Arc<OARStructure>>>) {
    let mut guard = cache.lock().await;
    if guard.take().is_some() {
        tracing::info!("结构引擎已释放，内存已回收");
    }
}

// ===== 结构 OCR (multipart) =====

/// POST /api/structure — multipart/form-data 上传
pub async fn structure_handler(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    uri: axum::http::Uri,
    mut multipart: Multipart,
) -> Result<Json<StructureResponse>, AppError> {
    check_token(&state, &headers, &uri)?;

    let start = Instant::now();
    let (bytes, _) = extract_image_file(&mut multipart).await?;
    let image_size = bytes.len() as u64;
    let img = img_from_bytes(&bytes)?;

    let structure = get_or_load_structure(&state.structure_cache).await?;
    let result = do_structure(&structure, img)?;
    release_structure(&state.structure_cache).await;

    let duration_ms = start.elapsed().as_millis() as u64;
    // 从响应中提取统计信息
    let result_count = result.total_layout.unwrap_or(0) + result.total_tables.unwrap_or(0);
    let markdown_len = result.markdown.as_ref().map(|s| s.len()).unwrap_or(0);

    log_ocr_request(
        &state.ocr_logger,
        "structure",
        "upload",
        None,
        image_size,
        result_count,
        markdown_len,
        duration_ms,
        "success",
        None,
    );

    Ok(result)
}

// ===== 结构 OCR (JSON / 图床链接) =====

/// POST /api/structure/json — JSON body
/// `{ "image": "base64..." | "https://..." }`
#[derive(serde::Deserialize)]
pub(crate) struct StructureRequest {
    image: String,
}

pub async fn structure_json_handler(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    uri: axum::http::Uri,
    Json(body): Json<StructureRequest>,
) -> Result<Json<StructureResponse>, AppError> {
    check_token(&state, &headers, &uri)?;

    let start = Instant::now();
    let is_url = body.image.starts_with("http://") || body.image.starts_with("https://");
    let image_url: Option<&str> = if is_url { Some(&body.image) } else { None };
    let image_source = if is_url { "url" } else { "base64" };

    let img = resolve_image(&state.http_client, &body.image).await?;
    let image_size = (img.width() as u64) * (img.height() as u64) * 3;

    let structure = get_or_load_structure(&state.structure_cache).await?;
    let result = do_structure(&structure, img)?;
    release_structure(&state.structure_cache).await;

    let duration_ms = start.elapsed().as_millis() as u64;
    let result_count = result.total_layout.unwrap_or(0) + result.total_tables.unwrap_or(0);
    let markdown_len = result.markdown.as_ref().map(|s| s.len()).unwrap_or(0);

    log_ocr_request(
        &state.ocr_logger,
        "structure",
        image_source,
        image_url,
        image_size,
        result_count,
        markdown_len,
        duration_ms,
        "success",
        None,
    );

    Ok(result)
}

// ===== 工具函数 =====

/// 从字节数据解码为 RgbImage
fn img_from_bytes(bytes: &[u8]) -> Result<image::RgbImage, AppError> {
    let img = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|e| AppError::ImageLoad(e.to_string()))?
        .decode()
        .map_err(|e| AppError::ImageLoad(e.to_string()))?;
    Ok(img.to_rgb8())
}

/// 解析图片来源 (base64 或 URL)，返回 RgbImage
async fn resolve_image(
    client: &reqwest::Client,
    source: &str,
) -> Result<image::RgbImage, AppError> {
    let trimmed = source.trim();

    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        tracing::info!("下载图床图片: {trimmed}");

        let resp = client
            .get(trimmed)
            .send()
            .await
            .map_err(|e| AppError::ImageDownload(format!("请求失败: {e}")))?;

        let status = resp.status();
        if !status.is_success() {
            return Err(AppError::ImageDownload(format!("HTTP {status}")));
        }

        let bytes = resp
            .bytes()
            .await
            .map_err(|e| AppError::ImageDownload(format!("读取响应失败: {e}")))?;

        if bytes.len() > 20 * 1024 * 1024 {
            return Err(AppError::ImageDownload("图片过大 (超过 20MB)".to_string()));
        }

        img_from_bytes(&bytes)
    } else {
        // Base64 解码
        let encoded = if let Some(idx) = trimmed.find(";base64,") {
            &trimmed[idx + 8..]
        } else {
            trimmed
        };

        use base64::Engine;
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .map_err(|e| AppError::Base64Decode(format!("解码失败: {e}")))?;

        img_from_bytes(&bytes)
    }
}

/// 执行 OCR，返回结果列表
fn do_ocr(ocr: &OAROCR, img: image::RgbImage) -> Result<Vec<OcrResult>, AppError> {
    let ocr_results = ocr.predict(vec![img]).map_err(AppError::from)?;

    let results: Vec<OcrResult> = ocr_results
        .iter()
        .flat_map(|page| {
            page.text_regions.iter().filter_map(|region| {
                let text = region.text.as_ref()?.to_string();
                let confidence = region.confidence.unwrap_or(0.0) as f64;
                let text_region: Vec<[f64; 2]> = region
                    .bounding_box
                    .points
                    .iter()
                    .map(|p| [p.x as f64, p.y as f64])
                    .collect();
                Some(OcrResult {
                    text,
                    confidence,
                    text_region,
                })
            })
        })
        .collect();

    tracing::info!("OCR 完成, 识别到 {} 个文本区域", results.len());
    Ok(results)
}

/// 执行结构识别，返回 JSON 响应
fn do_structure(
    structure: &OARStructure,
    img: image::RgbImage,
) -> Result<Json<StructureResponse>, AppError> {
    let result = structure
        .predict_image(img)
        .map_err(|e| AppError::StructureError(e.to_string()))?;

    let layout: Vec<LayoutElementInfo> = result
        .layout_elements
        .iter()
        .map(|el| LayoutElementInfo {
            element_type: el.label.clone().unwrap_or_else(|| "unknown".to_string()),
            confidence: el.confidence as f64,
            bbox: el
                .bbox
                .points
                .iter()
                .map(|p| [p.x as f64, p.y as f64])
                .collect(),
            text: el.text.clone(),
            order_index: el.order_index,
        })
        .collect();

    let tables: Vec<TableInfo> = result
        .tables
        .iter()
        .map(|t| {
            let cells = t
                .cells
                .iter()
                .map(|c| TableCellInfo {
                    row: c.row,
                    col: c.col,
                    row_span: c.row_span,
                    col_span: c.col_span,
                    confidence: c.confidence as f64,
                    text: c.text.clone(),
                })
                .collect();

            TableInfo {
                table_type: format!("{:?}", t.table_type),
                classification_confidence: t.classification_confidence.map(|v| v as f64),
                structure_confidence: t.structure_confidence.map(|v| v as f64),
                html_structure: t.html_structure.clone(),
                cells,
            }
        })
        .collect();

    // 公式识别结果
    let formulas: Vec<FormulaInfo> = result
        .formulas
        .iter()
        .map(|f| FormulaInfo {
            bbox: f
                .bbox
                .points
                .iter()
                .map(|p| [p.x as f64, p.y as f64])
                .collect(),
            latex: f.latex.clone(),
            confidence: f.confidence as f64,
        })
        .collect();

    // 图表元素（从布局中筛选 chart 相关类型）
    let chart_labels = ["chart", "chart_title", "figure_title", "flowchart"];
    let chart_elements: Vec<ChartElementInfo> = result
        .layout_elements
        .iter()
        .filter(|el| {
            el.label
                .as_ref()
                .map(|l| chart_labels.contains(&l.as_str()))
                .unwrap_or(false)
        })
        .map(|el| ChartElementInfo {
            element_type: el.label.clone().unwrap_or_else(|| "chart".to_string()),
            confidence: el.confidence as f64,
            bbox: el
                .bbox
                .points
                .iter()
                .map(|p| [p.x as f64, p.y as f64])
                .collect(),
            text: el.text.clone(),
            order_index: el.order_index,
        })
        .collect();

    let markdown = result.to_markdown();
    let html = result.to_html();

    tracing::info!(
        "结构识别完成: {} 布局元素, {} 表格, {} 公式, {} 图表",
        layout.len(),
        tables.len(),
        formulas.len(),
        chart_elements.len()
    );

    Ok(Json(StructureResponse::success(
        layout, tables, formulas, chart_elements, markdown, html,
    )))
}

/// 从 multipart 中提取图片文件
async fn extract_image_file(
    multipart: &mut Multipart,
) -> Result<(Vec<u8>, Option<String>), AppError> {
    while let Ok(Some(field)) = multipart.next_field().await {
        let content_type = field.content_type().map(|s| s.to_string());
        let name = field.name().unwrap_or("").to_string();

        if name == "file" || name == "image" || name == "image_data" {
            let data = field
                .bytes()
                .await
                .map_err(|e| AppError::Internal(format!("读取上传文件失败: {e}")))?;
            if !data.is_empty() {
                return Ok((data.to_vec(), content_type));
            }
        }
    }
    Err(AppError::NoFile)
}

/// 校验 MIME 类型
fn validate_mime(content_type: &Option<String>) -> Result<(), AppError> {
    if let Some(ref ct) = content_type {
        if !is_supported_mime(ct) {
            return Err(AppError::UnsupportedFormat(ct.clone()));
        }
    }
    Ok(())
}
