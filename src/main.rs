mod auth;
mod error;
mod logger;
mod ocr_engine;
mod routes;
mod types;

use axum::{routing::get, routing::post, Router};
use std::sync::Arc;
use std::time::Instant;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::{ServeDir, ServeFile};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    // 加载 .env 文件（不存在不报错）
    let _ = dotenvy::dotenv();

    // 如果 OAR_HOME 未设置，默认使用 exe 所在目录的 models/ 文件夹
    let oar_home = std::env::var("OAR_HOME").unwrap_or_default();
    if oar_home.is_empty() {
        let exe_dir = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.to_path_buf()))
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
        let models_dir = exe_dir.join("models");
        std::env::set_var("OAR_HOME", models_dir.to_string_lossy().to_string());
    }

    // 初始化日志
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    // 读取 TOKEN
    let token = std::env::var("TOKEN")
        .map(|t| {
            let t = t.trim().to_string();
            if t.is_empty() {
                None
            } else {
                Some(t)
            }
        })
        .unwrap_or(None);

    if token.is_some() {
        tracing::info!("Token 认证已启用");
    } else {
        tracing::info!("Token 未配置，跳过认证");
    }

    tracing::info!("正在启动 oar-ocr-web 服务...");
    tracing::info!(
        "模型缓存目录: {}",
        oar_ocr::download::cache_dir().display()
    );

    // 创建 HTTP 客户端（用于图床链接下载）
    let http_client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .user_agent("oar-ocr-web/0.1.0")
        .build()
        .expect("创建 HTTP 客户端失败");

    // 初始化 OCR 引擎 (首次运行会从 ModelScope 下载模型)
    tracing::info!("正在初始化文本 OCR 引擎 (PP-OCRv6 small)...");
    let ocr = ocr_engine::build_ocr_engine().expect("OCR 引擎初始化失败");

    // 结构 OCR 引擎按需加载，不在启动时初始化（节省 ~160MB 内存）
    tracing::info!("结构 OCR 引擎设为按需加载模式");

    // 初始化 OCR 请求日志系统
    let ocr_logger = logger::OcrLogger::from_env();
    // 启动时清理过期日志
    ocr_logger.cleanup();

    let start_time = Instant::now();

    let state = routes::AppState {
        ocr,
        structure_cache: Arc::new(tokio::sync::Mutex::new(None)),
        start_time,
        token,
        http_client: Arc::new(http_client),
        ocr_logger: Arc::new(ocr_logger),
    };

    // CORS 配置 (开发环境宽松)
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // 路由
    let app = Router::new()
        .route("/api/auth/verify", get(routes::auth_verify_handler))
        .route("/api/ocr", post(routes::ocr_handler))
        .route("/api/ocr/json", post(routes::ocr_json_handler))
        .route("/api/structure", post(routes::structure_handler))
        .route("/api/structure/json", post(routes::structure_json_handler))
        .route("/api/health", get(routes::health_handler))
        // 静态文件: /
        .route_service("/", ServeFile::new("static/index.html"))
        // PWA 资源（根路径，便于浏览器直接访问）
        .route_service("/site.webmanifest", ServeFile::new("static/site.webmanifest"))
        .route_service("/favicon.ico", ServeFile::new("static/favicon.ico"))
        .route_service("/favicon-16x16.png", ServeFile::new("static/favicon-16x16.png"))
        .route_service("/favicon-32x32.png", ServeFile::new("static/favicon-32x32.png"))
        .route_service("/apple-touch-icon.png", ServeFile::new("static/apple-touch-icon.png"))
        .route_service("/android-chrome-192x192.png", ServeFile::new("static/android-chrome-192x192.png"))
        .route_service("/android-chrome-512x512.png", ServeFile::new("static/android-chrome-512x512.png"))
        // 静态资源目录
        .nest_service("/static", ServeDir::new("static"))
        .layer(cors)
        .with_state(state);

    // 监听端口
    let port: u16 = std::env::var("PORT")
        .unwrap_or_else(|_| "3000".to_string())
        .parse()
        .expect("无效的端口号");

    let addr = format!("0.0.0.0:{port}");
    tracing::info!("服务已启动: http://{addr}");

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("绑定地址失败");

    axum::serve(listener, app).await.expect("服务运行出错");
}
