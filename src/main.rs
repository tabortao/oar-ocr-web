mod error;
mod ocr_engine;
mod routes;
mod types;

use axum::{routing::get, routing::post, Router};
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::{ServeDir, ServeFile};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    tracing::info!("正在启动 oar-ocr-web 服务...");
    tracing::info!(
        "模型缓存目录: {}",
        oar_ocr::download::cache_dir().display()
    );

    // 初始化 OCR 引擎 (首次运行会从 ModelScope 下载模型)
    tracing::info!("正在初始化 OCR 引擎 (PP-OCRv6 small)...");
    let ocr = ocr_engine::build_ocr_engine().expect("OCR 引擎初始化失败");

    let state = routes::AppState { ocr };

    // CORS 配置 (开发环境宽松)
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // 路由
    let app = Router::new()
        // API 路由
        .route("/api/ocr", post(routes::ocr_handler))
        .route("/api/health", get(routes::health_handler))
        // 静态文件: /
        .route_service("/", ServeFile::new("static/index.html"))
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
