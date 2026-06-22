use oar_ocr::domain::tasks::TextDetectionConfig;
use oar_ocr::oarocr::{OAROCR, OAROCRBuilder};
use oar_ocr::prelude::OCRError;
use std::sync::Arc;

/// 构建 PP-OCRv6 small OCR 引擎
///
/// 使用 auto-download 特性自动从 ModelScope 下载模型文件到 `~/.oar/`。
/// 首次运行需联网下载，后续使用缓存。
pub fn build_ocr_engine() -> Result<Arc<OAROCR>, OCRError> {
    let det_config = TextDetectionConfig {
        // PP-OCRv6 官方推荐的检测阈值
        score_threshold: 0.2,
        box_threshold: 0.45,
        unclip_ratio: 1.4,
        ..Default::default()
    };

    let ocr = OAROCRBuilder::new(
        // auto-download feature: 裸文件名自动从 ModelScope 注册表下载
        "pp-ocrv6_small_det.onnx",
        "pp-ocrv6_small_rec.onnx",
        "ppocrv6_dict.txt",
    )
    .text_detection_config(det_config)
    .return_word_box(false) // 不返回单字框，提高性能
    .build()?;

    tracing::info!("OCR 引擎初始化完成 (PP-OCRv6 small)");
    Ok(Arc::new(ocr))
}
