# ─── Stage 1: Build ONNX Runtime from source (no AVX, SSE4.2 baseline) ───
# 飞牛NAS 使用 Intel Celeron N5105 (Jasper Lake)，仅支持 SSE4.2，不支持 AVX/AVX2/FMA
# ort-sys 默认下载的预构建 ONNX Runtime 使用 -mavx2 编译，在 N5105 上触发 SIGILL (exit 132)
# 因此从源码构建 ONNX Runtime，使用 -march=x86-64 -msse4.2 禁用 AVX 指令集
FROM ubuntu:24.04 AS ort-builder

ENV DEBIAN_FRONTEND=noninteractive
WORKDIR /tmp/onnxruntime

# 安装 ONNX Runtime 构建依赖
# python3-pip: build.py 需要 pip 安装依赖
# ninja-build: 比 make 更快的构建工具
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    cmake \
    git \
    curl \
    ca-certificates \
    python3 \
    python3-pip \
    ninja-build \
    && rm -rf /var/lib/apt/lists/*

# 安装 build.py 所需的 Python 依赖
RUN pip3 install --no-cache-dir --break-system-packages \
    pyyaml \
    coloredlogs \
    packaging

# 克隆 ONNX Runtime v1.25.0（完整克隆含子模块，确保子模块正确初始化）
RUN git clone --branch v1.25.0 --recursive https://github.com/microsoft/onnxruntime.git .

# 设置编译器标志：禁用 AVX，仅使用 SSE4.2（Intel Celeron N5105 兼容）
ENV CFLAGS="-march=x86-64 -msse4.2"
ENV CXXFLAGS="-march=x86-64 -msse4.2"

# 验证构建环境
RUN python3 --version && \
    python3 -c "import sys; sys.path.insert(0, 'tools/python'); from build_args import parse_arguments; print('build_args import OK')" && \
    echo "Submodule count: $(git submodule status | wc -l)"

# 构建共享库，捕获完整日志以便诊断错误
# --cmake_generator Ninja: 使用 Ninja 构建工具（比 make 更快）
# --skip_submodule_sync: 子模块已在 clone --recursive 时初始化
# --skip_tests: 跳过测试编译以加速构建
# --compile_no_warning_as_error: 避免警告导致构建失败
# 同时通过 cmake_extra_defines 传递编译器标志（双引号确保空格不被分割）
RUN ./build.sh --config Release \
    --build_shared_lib \
    --parallel $(nproc) \
    --compile_no_warning_as_error \
    --skip_tests \
    --skip_submodule_sync \
    --cmake_generator Ninja \
    --cmake_extra_defines CMAKE_C_FLAGS="-march=x86-64 -msse4.2" \
    --cmake_extra_defines CMAKE_CXX_FLAGS="-march=x86-64 -msse4.2" \
    2>&1 | tee /tmp/build.log; \
    EXIT_CODE=${PIPESTATUS[0]}; \
    if [ $EXIT_CODE -ne 0 ]; then \
        echo ""; \
        echo "========================================"; \
        echo "BUILD FAILED (exit code: $EXIT_CODE)"; \
        echo "=== Last 150 lines of build log ==="; \
        tail -150 /tmp/build.log; \
        echo "========================================"; \
        exit $EXIT_CODE; \
    fi

# 验证构建产物
RUN ls -la build/Linux/Release/libonnxruntime.so*

# ─── Stage 2: Build Rust application ───
# Ubuntu 24.04 提供 glibc 2.39，满足 ONNX Runtime 要求
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

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
ENV PATH="/root/.cargo/bin:${PATH}"

# 复制自定义构建的 ONNX Runtime 共享库（禁用 AVX）
COPY --from=ort-builder /tmp/onnxruntime/build/Linux/Release/libonnxruntime.so* /opt/onnxruntime/lib/
RUN ldconfig

# 设置 ort-sys 使用自定义 ONNX Runtime，跳过预构建下载，强制动态链接
# ORT_LIB_LOCATION: 指定自定义 .so 所在目录，ort-sys 不再下载预构建包
# ORT_PREFER_DYNAMIC_LINK=1: 使用动态链接 (cargo:rustc-link-lib=onnxruntime) 而非静态链接
ENV ORT_LIB_LOCATION=/opt/onnxruntime/lib
ENV ORT_PREFER_DYNAMIC_LINK=1

# 先复制依赖清单，利用 Docker 缓存层
COPY Cargo.toml Cargo.lock* ./
COPY .cargo/ .cargo/
# 创建空的 src 用于依赖预下载
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo fetch || true

# 复制源码
COPY src/ src/
COPY static/ static/

# Release 构建
RUN cargo build --release

# ─── Stage 3: Runtime ───
# 必须与 builder 使用相同或更高 glibc 版本
FROM ubuntu:24.04

ENV DEBIAN_FRONTEND=noninteractive

# 运行时依赖
# libgomp1: OpenMP (ONNX Runtime 多线程)
# libstdc++6: ONNX Runtime C++ 运行时依赖
# libssl3: TLS 支持（ureq/oar-ocr auto-download 需要）
RUN apt-get update && apt-get install -y --no-install-recommends \
    libgomp1 \
    libssl3 \
    libstdc++6 \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

# 复制二进制
COPY --from=builder /app/target/release/oar-ocr-web /usr/local/bin/oar-ocr-web

# 复制 ONNX Runtime 共享库并注册到动态链接器缓存
COPY --from=builder /opt/onnxruntime/lib/libonnxruntime.so* /usr/local/lib/
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

# 动态库搜索路径
ENV LD_LIBRARY_PATH=/usr/local/lib

WORKDIR /app
EXPOSE 3000

# 确保目录存在
RUN mkdir -p /app/logs /app/models

HEALTHCHECK --interval=30s --timeout=5s --start-period=120s --retries=3 \
    CMD curl -f http://localhost:3000/api/health || exit 1

ENTRYPOINT ["docker-entrypoint.sh"]
CMD ["oar-ocr-web"]
