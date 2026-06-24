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
ls -lh "$MODEL_DIR/" 2>/dev/null | awk '{print "  " $NF " (" $5 ")"}'

echo "=== 启动 oar-ocr-web ==="
exec "$@"