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

# 收集 ONNX Runtime 共享库 (ort-sys 在构建时下载，运行时需要)
# ort 默认动态链接 libonnxruntime.so，必须复制到运行时镜像
RUN mkdir -p /app/ort-lib && \
    find /app/target -name "libonnxruntime.so*" -exec cp -L {} /app/ort-lib/ \; && \
    ls -la /app/ort-lib/

# ─── Runtime Stage ───
# 必须与 builder 使用相同或更高 glibc 版本
FROM ubuntu:24.04

ENV DEBIAN_FRONTEND=noninteractive

# ONNX Runtime 运行时依赖 + TLS 支持（ureq/oar-ocr auto-download 需要）
# libstdc++6: ONNX Runtime C++ 运行时依赖
# libgomp1: OpenMP (ONNX Runtime 多线程)
RUN apt-get update && apt-get install -y --no-install-recommends \
    libgomp1 \
    libssl3 \
    libstdc++6 \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

# 复制构建产物
COPY --from=builder /app/target/release/oar-ocr-web /usr/local/bin/oar-ocr-web

# 复制 ONNX Runtime 共享库并注册到动态链接器缓存
COPY --from=builder /app/ort-lib/ /usr/local/lib/
RUN ldconfig

# 复制静态文件
COPY static/ /app/static/

# 复制启动脚本
COPY docker-entrypoint.sh /usr/local/bin/
RUN chmod +x /usr/local/bin/docker-entrypoint.sh

# 模型缓存目录（卷挂载点，持久化，首次启动自动下载）
ENV OAR_HOME=/app/models

# OCR 请求日志目录
ENV LOG_DIR=/app/logs
ENV LOG_RETENTION_DAYS=30

# ONNX Runtime 动态库路径（ort load-dynamic 模式下通过 dlopen 加载）
ENV ORT_DYLIB_PATH=/usr/local/lib/libonnxruntime.so
ENV LD_LIBRARY_PATH=/usr/local/lib

WORKDIR /app
EXPOSE 3000

# 确保目录存在
RUN mkdir -p /app/logs /app/models

HEALTHCHECK --interval=30s --timeout=5s --start-period=120s --retries=3 \
    CMD curl -f http://localhost:3000/api/health || exit 1

ENTRYPOINT ["docker-entrypoint.sh"]
CMD ["oar-ocr-web"]
