#!/bin/bash
set -e

echo "=== oar-ocr-web entrypoint ==="
echo "OAR_HOME: $OAR_HOME"
echo "LOG_DIR:  $LOG_DIR"

MODELSCOPE_BASE="https://www.modelscope.cn/api/v1/models/greatv/oar-ocr/repo"
REVISION="master"
MODEL_DIR="${OAR_HOME:-/app/models}"

MODELS=(
    "pp-ocrv6_small_det.onnx"
    "pp-ocrv6_small_rec.onnx"
    "ppocrv6_dict.txt"
    "pp-doclayout_plus-l.onnx"
    "pp-lcnet_x1_0_table_cls.onnx"
    "slanet_plus.onnx"
    "table_structure_dict_ch.txt"
    "pp-formulanet_plus-s.onnx"
    "pp-formulanet-tokenizer.json"
)

mkdir -p "$MODEL_DIR"

# 检查模型是否已存在
MISSING=()
for model in "${MODELS[@]}"; do
    if [ ! -f "$MODEL_DIR/$model" ]; then
        MISSING+=("$model")
    fi
done

if [ ${#MISSING[@]} -eq 0 ]; then
    echo "所有模型文件已存在 (${#MODELS[@]} 个)，跳过下载"
else
    echo "首次启动: 需要下载 ${#MISSING[@]}/${#MODELS[@]} 个模型文件 (~167MB)..."
    for model in "${MISSING[@]}"; do
        url="${MODELSCOPE_BASE}?Revision=${REVISION}&FilePath=${model}"
        echo "  [下载] $model ..."
        curl -fSL --retry 3 --retry-delay 5 --connect-timeout 30 -o "$MODEL_DIR/$model" "$url" || {
            echo "  [失败] $model 下载失败，请检查网络连接"
            rm -f "$MODEL_DIR/$model"
            exit 1
        }
        size=$(du -h "$MODEL_DIR/$model" | cut -f1)
        echo "    -> $size"
    done
    echo "模型下载完成: $(ls -1 "$MODEL_DIR" | wc -l) 个文件"
fi

echo "模型文件列表:"
for f in "$MODEL_DIR"/*; do
    if [ -f "$f" ]; then
        name=$(basename "$f")
        size=$(du -h "$f" | cut -f1)
        echo "  $name ($size)"
    fi
done

echo "=== 启动 oar-ocr-web ==="

# ── 诊断信息（帮助定位启动失败原因）──
echo "----- 诊断信息 -----"
echo "二进制路径: $(command -v oar-ocr-web)"
echo "二进制大小: $(du -h $(command -v oar-ocr-web) | cut -f1)"

echo "ldd 依赖检查:"
ldd "$(command -v oar-ocr-web)" 2>&1 || true

echo "ONNX Runtime 库文件 (/usr/local/lib/):"
ls -la /usr/local/lib/ 2>/dev/null || echo "  目录为空或不存在"
echo "ldconfig 缓存中的 onnxruntime:"
ldconfig -p 2>/dev/null | grep -i onnx || echo "  未在 ldconfig 中找到 onnxruntime"

echo "CPU 信息:"
grep -m1 "model name" /proc/cpuinfo 2>/dev/null || echo "  无法读取 CPU 信息"
echo "CPU 指令集:"
grep -m1 "flags" /proc/cpuinfo 2>/dev/null | tr ' ' '\n' | grep -E '^(avx|avx2|sse4_2|fma)$' | tr '\n' ' ' || echo "  无法读取"
echo ""

echo "内存限制:"
cat /sys/fs/cgroup/memory.max 2>/dev/null || cat /sys/fs/cgroup/memory/memory.limit_in_bytes 2>/dev/null || echo "  无法读取"
echo "环境变量:"
echo "  OAR_HOME=${OAR_HOME}"
echo "  PORT=${PORT:-3000}"
echo "  RUST_LOG=${RUST_LOG}"
echo "  RUST_BACKTRACE=${RUST_BACKTRACE}"
echo "  LD_LIBRARY_PATH=${LD_LIBRARY_PATH:-未设置}"
echo "--------------------"

# 运行二进制并捕获退出码（不使用 exec，以便诊断崩溃原因）
# 2>&1 将 stderr 重定向到 stdout，确保 tracing 日志和 panic 信息都被捕获
echo "启动 oar-ocr-web..."
set +e
"$@" 2>&1
EXIT_CODE=$?
echo "========================================"
echo "oar-ocr-web 已退出，退出码: $EXIT_CODE"
if [ $EXIT_CODE -ne 0 ]; then
    echo "异常退出诊断:"
    if [ $EXIT_CODE -eq 132 ]; then
        echo "  -> SIGILL (非法指令): CPU 不支持 ONNX Runtime 所需的指令集 (如 AVX)"
    elif [ $EXIT_CODE -eq 137 ]; then
        echo "  -> SIGKILL: 进程被强制杀死 (可能是 OOM 内存不足)"
    elif [ $EXIT_CODE -eq 139 ]; then
        echo "  -> SIGSEGV (段错误): 二进制或依赖库存在兼容性问题"
    elif [ $EXIT_CODE -gt 128 ]; then
        SIGNAL=$((EXIT_CODE - 128))
        echo "  -> 被信号 $SIGNAL 杀死"
    else
        echo "  -> 程序主动退出 (错误码 $EXIT_CODE)"
    fi
    echo "========================================"
fi
exit $EXIT_CODE