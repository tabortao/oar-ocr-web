use serde::{Deserialize, Serialize};

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
