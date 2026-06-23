# ─── Build Stage ───
FROM rust:1.95-bookworm AS builder

WORKDIR /app

# 安装构建依赖
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    cmake \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# 先复制依赖清单，利用 Docker 缓存层
COPY Cargo.toml Cargo.lock* ./
COPY .cargo/ .cargo/
# 创建空的 src 用于依赖预下载
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo fetch || true

# 复制源码
COPY src/ src/
COPY static/ static/

# Release 构建 (auto-download 在 Cargo.toml 中已声明)
RUN cargo build --release

# ─── Runtime Stage ───
FROM debian:bookworm-slim

# ONNX Runtime 运行时依赖
RUN apt-get update && apt-get install -y --no-install-recommends \
    libgomp1 \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# 复制构建产物
COPY --from=builder /app/target/release/oar-ocr-web /usr/local/bin/oar-ocr-web

# 复制静态文件
COPY static/ /app/static/

# 复制预下载的模型文件（构建镜像时需先执行 download_models.ps1）
COPY models/ /app/models/

# 模型缓存目录（内置模型，无需持久化卷）
ENV OAR_HOME=/app/models

# OCR 请求日志目录
ENV LOG_DIR=/app/logs
ENV LOG_RETENTION_DAYS=30

WORKDIR /app
EXPOSE 3000

CMD ["oar-ocr-web"]
