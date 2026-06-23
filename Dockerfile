# ─── Build Stage ───
# ONNX Runtime 2.38+ 需要 glibc 2.38+，Ubuntu 24.04 提供 glibc 2.39
FROM ubuntu:24.04 AS builder

ENV DEBIAN_FRONTEND=noninteractive
WORKDIR /app

# 安装构建依赖和 Rust
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    curl \
    ca-certificates \
    libssl-dev \
    pkg-config \
    cmake \
    && rm -rf /var/lib/apt/lists/*

# 安装 Rust stable
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
ENV PATH="/root/.cargo/bin:${PATH}"

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
# 必须与 builder 使用相同或更高 glibc 版本
FROM ubuntu:24.04

ENV DEBIAN_FRONTEND=noninteractive

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
