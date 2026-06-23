#!/bin/bash
# Download OCR model files from ModelScope (Linux/macOS)
# Usage: bash download_models.sh
# Models are saved to ./models/ directory

set -e

MODELSCOPE_BASE="https://www.modelscope.cn/api/v1/models/greatv/oar-ocr/repo"
REVISION="master"
OUTPUT_DIR="$(cd "$(dirname "$0")" && pwd)/models"

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

mkdir -p "$OUTPUT_DIR"

echo "Downloading ${#MODELS[@]} model files to $OUTPUT_DIR..."

for model in "${MODELS[@]}"; do
    output="$OUTPUT_DIR/$model"
    if [ -f "$output" ]; then
        echo "  [SKIP] $model (already exists)"
        continue
    fi

    echo "  [DOWNLOAD] $model ..."
    curl -fSL --progress-bar -o "$output" "${MODELSCOPE_BASE}?Revision=${REVISION}&FilePath=${model}"
    size=$(du -h "$output" | cut -f1)
    echo "    -> $size"
done

echo ""
echo "All models downloaded successfully!"