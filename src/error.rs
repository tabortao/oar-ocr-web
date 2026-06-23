use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use crate::types::OcrResponse;

/// 统一的 Web 层错误类型
#[derive(Debug)]
pub enum AppError {
    /// 没有上传文件
    NoFile,
    /// 缺少认证 Token
    MissingToken,
    /// Token 无效
    InvalidToken,
    /// 文件格式不支持
    UnsupportedFormat(String),
    /// 图片加载失败
    ImageLoad(String),
    /// OCR 推理错误
    OcrError(String),
    /// 结构识别错误
    StructureError(String),
    /// Base64 解码失败
    Base64Decode(String),
    /// 图床链接下载失败
    ImageDownload(String),
    /// 内部服务错误
    Internal(String),
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppError::NoFile => write!(f, "no file uploaded"),
            AppError::MissingToken => write!(f, "missing auth token"),
            AppError::InvalidToken => write!(f, "invalid auth token"),
            AppError::UnsupportedFormat(fmt) => write!(f, "unsupported format: {fmt}"),
            AppError::ImageLoad(msg) => write!(f, "failed to load image: {msg}"),
            AppError::OcrError(msg) => write!(f, "OCR error: {msg}"),
            AppError::StructureError(msg) => write!(f, "structure error: {msg}"),
            AppError::Base64Decode(msg) => write!(f, "base64 decode error: {msg}"),
            AppError::ImageDownload(msg) => write!(f, "image download error: {msg}"),
            AppError::Internal(msg) => write!(f, "internal error: {msg}"),
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            AppError::NoFile => (StatusCode::BAD_REQUEST, "请上传图片文件".to_string()),
            AppError::MissingToken => (StatusCode::UNAUTHORIZED, "缺少认证 Token".to_string()),
            AppError::InvalidToken => (StatusCode::UNAUTHORIZED, "Token 无效".to_string()),
            AppError::UnsupportedFormat(fmt) => {
                (StatusCode::BAD_REQUEST, format!("不支持的文件格式: {fmt}"))
            }
            AppError::ImageLoad(msg) => {
                (StatusCode::BAD_REQUEST, format!("图片加载失败: {msg}"))
            }
            AppError::OcrError(msg) => {
                tracing::error!("OCR error: {msg}");
                (StatusCode::INTERNAL_SERVER_ERROR, format!("OCR 识别失败: {msg}"))
            }
            AppError::StructureError(msg) => {
                tracing::error!("Structure error: {msg}");
                (StatusCode::INTERNAL_SERVER_ERROR, format!("结构识别失败: {msg}"))
            }
            AppError::Base64Decode(msg) => {
                (StatusCode::BAD_REQUEST, format!("Base64 解码失败: {msg}"))
            }
            AppError::ImageDownload(msg) => {
                (StatusCode::BAD_REQUEST, format!("图片下载失败: {msg}"))
            }
            AppError::Internal(msg) => {
                tracing::error!("Internal error: {msg}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "服务内部错误".to_string(),
                )
            }
        };

        let body = OcrResponse::error(message);
        (status, Json(body)).into_response()
    }
}

impl From<anyhow::Error> for AppError {
    fn from(e: anyhow::Error) -> Self {
        AppError::Internal(e.to_string())
    }
}

impl From<oar_ocr::core::OCRError> for AppError {
    fn from(e: oar_ocr::core::OCRError) -> Self {
        AppError::OcrError(e.to_string())
    }
}
