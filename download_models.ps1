# Download OCR model files from ModelScope
# Usage: .\download_models.ps1
# Models are saved to ./models/ directory

$ErrorActionPreference = "Stop"

$MODELSCOPE_BASE = "https://www.modelscope.cn/api/v1/models/greatv/oar-ocr/repo"
$REVISION = "master"
$OUTPUT_DIR = Join-Path $PSScriptRoot "models"

$MODELS = @(
    # OCR (PP-OCRv6 small)
    "pp-ocrv6_small_det.onnx",
    "pp-ocrv6_small_rec.onnx",
    "ppocrv6_dict.txt",
    # Structure (PP-DocLayout_plus-L + SLANet_plus)
    "pp-doclayout_plus-l.onnx",
    "pp-lcnet_x1_0_table_cls.onnx",
    "slanet_plus.onnx",
    "table_structure_dict_ch.txt",
    # Formula (PP-FormulaNet_plus-S)
    "pp-formulanet_plus-s.onnx",
    "pp-formulanet-tokenizer.json"
)

if (-not (Test-Path $OUTPUT_DIR)) {
    New-Item -ItemType Directory -Path $OUTPUT_DIR | Out-Null
}

Write-Host "Downloading $($MODELS.Count) model files to $OUTPUT_DIR..." -ForegroundColor Cyan

foreach ($model in $MODELS) {
    $url = "$MODELSCOPE_BASE`?Revision=$REVISION&FilePath=$model"
    $output = Join-Path $OUTPUT_DIR $model
    $totalSize = "unknown"

    if (Test-Path $output) {
        Write-Host "  [SKIP] $model (already exists)" -ForegroundColor Yellow
        continue
    }

    Write-Host "  [DOWNLOAD] $model ..." -ForegroundColor Green
    try {
        Invoke-WebRequest -Uri $url -OutFile $output -UseBasicParsing
        $size = (Get-Item $output).Length
        $sizeMB = [math]::Round($size / 1MB, 2)
        Write-Host "    -> $sizeMB MB" -ForegroundColor Gray
    } catch {
        Write-Host "  [FAIL] $model : $_" -ForegroundColor Red
        if (Test-Path $output) { Remove-Item $output }
        exit 1
    }
}

Write-Host "`nAll models downloaded successfully!" -ForegroundColor Cyan