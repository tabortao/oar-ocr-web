use axum::{
    extract::{Multipart, State},
    Json,
};
use image::ImageReader;
use oar_ocr::oarocr::OAROCR;
use std::io::Cursor;
use std::sync::Arc;

use crate::error::AppError;
use crate::types::{OcrResponse, OcrResult};

/// 共享应用状态
#[derive(Clone)]
pub struct AppState {
    pub ocr: Arc<OAROCR>,
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

/// POST /api/ocr
///
/// 接收 multipart/form-data 图片上传，返回 PaddleOCR Serving 兼容的 JSON 响应。
pub async fn ocr_handler(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<Json<OcrResponse>, AppError> {
    // 1. 提取上传的图片文件
    let (bytes, content_type) = extract_image_file(&mut multipart).await?;

    // 2. 校验文件类型
    if let Some(ref ct) = content_type {
        if !is_supported_mime(ct) {
            return Err(AppError::UnsupportedFormat(ct.clone()));
        }
    }

    tracing::info!(
        "收到 OCR 请求, 文件大小: {} bytes, 类型: {:?}",
        bytes.len(),
        content_type
    );

    // 3. 从内存加载图片 → RgbImage
    let img = ImageReader::new(Cursor::new(&bytes))
        .with_guessed_format()
        .map_err(|e| AppError::ImageLoad(e.to_string()))?
        .decode()
        .map_err(|e| AppError::ImageLoad(e.to_string()))?
        .to_rgb8();

    // 4. 执行 OCR 推理
    let ocr_results = state.ocr.predict(vec![img]).map_err(AppError::from)?;

    // 5. 转换为 PaddleOCR 兼容格式
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

    if results.is_empty() {
        return Ok(Json(OcrResponse {
            status: "success".to_string(),
            results: Some(vec![]),
            total: Some(0),
            message: Some("未识别到文字".to_string()),
        }));
    }

    Ok(Json(OcrResponse::success(results)))
}

/// 从 multipart 中提取图片文件
async fn extract_image_file(
    multipart: &mut Multipart,
) -> Result<(Vec<u8>, Option<String>), AppError> {
    while let Ok(Some(field)) = multipart.next_field().await {
        let content_type = field.content_type().map(|s| s.to_string());
        let name = field.name().unwrap_or("").to_string();

        // 接受 file / image 字段名
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

/// 健康检查端点
pub async fn health_handler() -> &'static str {
    "ok"
}
