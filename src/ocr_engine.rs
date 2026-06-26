use oar_ocr::core::config::{OrtExecutionProvider, OrtGraphOptimizationLevel, OrtSessionConfig};
use oar_ocr::domain::tasks::TextDetectionConfig;
use oar_ocr::oarocr::{OAROCR, OAROCRBuilder, OARStructure, OARStructureBuilder};
use oar_ocr::prelude::OCRError;
use std::sync::Arc;

/// 构建 ONNX Runtime session 配置，关闭内存模式缓存和 arena 分配器。
///
/// 三项关键配置解决常驻 OCR 引擎的 RSS 无限增长问题：
///
/// 1. `enable_mem_pattern=false` — 关闭 ORT 内存模式缓存。OCR 识别模型输入宽度随文本行
///    长度变化，默认的 mem_pattern 会为每个新形状缓存 arena 块且永不释放。
///
/// 2. 显式注册 `CPU` EP — `oar-ocr-core` 在 `execution_providers=None` 时不注册任何 EP，
///    ORT 默认 CPU EP 的 arena 是**启用**的。显式注册 `OrtExecutionProvider::CPU` 会触发
///    `DisableCpuMemArena`（ort 2.0.0-rc.12 中 `CPU::default()` 的 `use_arena=false`），
///    使所有分配走 malloc/free 而非 arena 池。
///
/// 3. `malloc_trim(0)` / `HeapCompact` — 在 routes 层每次请求后调用，把 glibc/Windows
///    heap 的 free-list 归还给 OS。
///
/// 保留 Level3 图优化与 intra-op 多线程以兼顾速度。
fn build_ort_session_config() -> OrtSessionConfig {
    OrtSessionConfig::new()
        .with_memory_pattern(false)
        .with_optimization_level(OrtGraphOptimizationLevel::Level3)
        // 显式注册 CPU EP，触发 DisableCpuMemArena（禁用 arena 分配器）
        .with_execution_providers(vec![OrtExecutionProvider::CPU])
        // intra-op 线程数：使用全部逻辑核心加速单图推理
        .with_intra_threads(
            std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(4),
        )
}

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
    .ort_session(build_ort_session_config())
    .build()?;

    tracing::info!("OCR 引擎初始化完成 (PP-OCRv6 small, mem_pattern=off, arena=off)");
    Ok(Arc::new(ocr))
}

/// 构建 PP-StructureV3 结构 OCR 引擎（含公式识别）
///
/// 模型:
/// - 布局检测: pp-doclayout_plus-l.onnx (~124MB)
/// - 表格分类: pp-lcnet_x1_0_table_cls.onnx (~6.5MB)
/// - 表格结构: slanet_plus.onnx (~7.4MB)
/// - 表格字典: table_structure_dict_ch.txt
/// - 公式识别: pp-formulanet_plus-s.onnx + pp-formulanet-tokenizer.json
/// - 文本 OCR: PP-OCRv6 small (与 text OCR 共用模型文件)
pub fn build_structure_engine() -> Result<Arc<OARStructure>, OCRError> {
    let det_config = TextDetectionConfig {
        // 结构分析中降低检测阈值，避免表格碎片化
        score_threshold: 0.2,
        box_threshold: 0.4,
        unclip_ratio: 1.4,
        ..Default::default()
    };

    let structure = OARStructureBuilder::new("pp-doclayout_plus-l.onnx")
        .layout_model_name("PP-DocLayout_plus-L")
        .with_table_classification("pp-lcnet_x1_0_table_cls.onnx")
        .with_table_structure_recognition("slanet_plus.onnx", "wireless")
        .table_structure_dict_path("table_structure_dict_ch.txt")
        .with_formula_recognition(
            "pp-formulanet_plus-s.onnx",
            "pp-formulanet-tokenizer.json",
            "pp_formulanet",
        )
        .with_ocr(
            "pp-ocrv6_small_det.onnx",
            "pp-ocrv6_small_rec.onnx",
            "ppocrv6_dict.txt",
        )
        .text_detection_config(det_config)
        .ort_session(build_ort_session_config())
        .build()?;

    tracing::info!("结构 OCR 引擎初始化完成 (PP-DocLayout_plus-L + SLANet_plus + PP-FormulaNet_plus-S, mem_pattern=off, arena=off)");
    Ok(Arc::new(structure))
}
