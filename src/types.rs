use serde::{Deserialize, Serialize};

// ===== OCR 结果类型 (兼容 PaddleOCR Serving) =====

/// PaddleOCR Serving 兼容的 OCR 识别结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrResult {
    /// 识别出的文字内容
    pub text: String,
    /// 置信度 (0.0 ~ 1.0)
    pub confidence: f64,
    /// 文本框四个顶点坐标 [[x0,y0], [x1,y1], [x2,y2], [x3,y3]]
    pub text_region: Vec<[f64; 2]>,
}

/// PaddleOCR Serving 兼容的 API 响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrResponse {
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub results: Option<Vec<OcrResult>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl OcrResponse {
    pub fn success(results: Vec<OcrResult>) -> Self {
        let total = results.len();
        Self {
            status: "success".to_string(),
            results: Some(results),
            total: Some(total),
            message: None,
        }
    }

    pub fn error(msg: impl Into<String>) -> Self {
        Self {
            status: "error".to_string(),
            results: None,
            total: None,
            message: Some(msg.into()),
        }
    }
}

// ===== RESTful API 请求类型 =====

/// JSON OCR 请求
/// images 数组中的每项可以是:
/// - base64 编码的图片数据
/// - http(s) 开头的图床链接
#[derive(Debug, Clone, Deserialize)]
pub struct OcrRequest {
    #[serde(default)]
    pub images: Vec<String>,
}

// ===== 健康检查 =====

/// 引擎状态
#[derive(Debug, Clone, Serialize)]
pub struct EngineStatus {
    pub status: String,
    pub model: String,
}

/// 详细健康状态
#[derive(Debug, Clone, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub uptime_secs: u64,
    pub engines: HealthEngines,
}

#[derive(Debug, Clone, Serialize)]
pub struct HealthEngines {
    pub ocr: EngineStatus,
    pub structure: EngineStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth_enabled: Option<bool>,
}

// ===== 结构识别结果类型 =====

/// 布局元素类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutElementInfo {
    pub element_type: String,
    pub confidence: f64,
    pub bbox: Vec<[f64; 2]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order_index: Option<u32>,
}

/// 表格信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableInfo {
    pub table_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub classification_confidence: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub structure_confidence: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub html_structure: Option<String>,
    pub cells: Vec<TableCellInfo>,
}

/// 表格单元格
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableCellInfo {
    pub row: Option<usize>,
    pub col: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub row_span: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub col_span: Option<usize>,
    pub confidence: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

/// 公式识别结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormulaInfo {
    /// 公式在原图中的边界框
    pub bbox: Vec<[f64; 2]>,
    /// LaTeX 表示
    pub latex: String,
    /// 识别置信度
    pub confidence: f64,
}

/// 图表元素信息（布局检测中识别到的图表区域）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChartElementInfo {
    /// 元素类型: "chart" | "chart_title" | "figure_title"
    pub element_type: String,
    /// 置信度
    pub confidence: f64,
    /// 边界框
    pub bbox: Vec<[f64; 2]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order_index: Option<u32>,
}

/// 结构识别响应
#[derive(Debug, Clone, Serialize)]
pub struct StructureResponse {
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub layout_elements: Option<Vec<LayoutElementInfo>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tables: Option<Vec<TableInfo>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub formulas: Option<Vec<FormulaInfo>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chart_elements: Option<Vec<ChartElementInfo>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub markdown: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub html: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_layout: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_tables: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_formulas: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_charts: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl StructureResponse {
    pub fn success(
        layout: Vec<LayoutElementInfo>,
        tables: Vec<TableInfo>,
        formulas: Vec<FormulaInfo>,
        chart_elements: Vec<ChartElementInfo>,
        markdown: String,
        html: String,
    ) -> Self {
        let total_layout = layout.len();
        let total_tables = tables.len();
        let total_formulas = formulas.len();
        let total_charts = chart_elements.len();
        Self {
            status: "success".to_string(),
            layout_elements: Some(layout),
            tables: Some(tables),
            formulas: Some(formulas),
            chart_elements: Some(chart_elements),
            markdown: Some(markdown),
            html: Some(html),
            total_layout: Some(total_layout),
            total_tables: Some(total_tables),
            total_formulas: Some(total_formulas),
            total_charts: Some(total_charts),
            message: None,
        }
    }

    #[allow(dead_code)]
    pub fn error(msg: impl Into<String>) -> Self {
        Self {
            status: "error".to_string(),
            layout_elements: None,
            tables: None,
            formulas: None,
            chart_elements: None,
            markdown: None,
            html: None,
            total_layout: None,
            total_tables: None,
            total_formulas: None,
            total_charts: None,
            message: Some(msg.into()),
        }
    }
}
