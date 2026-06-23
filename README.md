# oar-ocr-web

A high-performance OCR web API service powered by [oar-ocr](https://github.com/GreatV/oar-ocr), compatible with the PaddleOCR Serving protocol.

## Features

- **Text OCR** — PP-OCRv6 small with word-level detection and confidence scoring
- **Document Structure Analysis** — Layout detection (PP-DocLayout_plus-L), table recognition (SLANet_plus), and formula recognition (PP-FormulaNet_plus-S)
- **Chart Detection** — Automatic identification of charts, flowcharts, and figure titles in documents
- **Image URL Support** — Accept image URLs (hotlinks) for OCR processing
- **Token Authentication** — Optional Bearer token authentication for API and web UI
- **PaddleOCR Compatible** — Drop-in replacement for PaddleOCR Serving API
- **Web UI** — Built-in drag-and-drop web interface with progress visualization
- **Docker Support** — Pre-bundled models with persistent volume, no download on first run

## Quick Start

### 1. Download Models

```powershell
.\download_models.ps1
```

This downloads all required model files (~167 MB) to the `./models/` directory.

### 2. Configure Environment

```powershell
copy .env.example .env
# Edit .env to set your TOKEN (optional)
```

### 3. Run

```powershell
$env:OAR_HOME = "./models"; cargo run --release 2>&1

# Enable logging    
$env:OAR_HOME = "./models"; $env:LOG_DIR = "./logs"; $env:LOG_RETENTION_DAYS = "30"; cargo run --release 2>&1
```

The service starts at `http://localhost:3000`.

### 4. Docker Deployment

```powershell
.\download_models.ps1
docker compose build
docker compose up -d
```

Models are bundled into the image and copied to the persistent volume on first run.

## API Endpoints

| Method | Endpoint | Auth | Description |
|--------|----------|------|-------------|
| `GET` | `/api/health` | No | Service health and status |
| `GET` | `/api/auth/verify` | Yes | Verify token validity |
| `POST` | `/api/ocr` | Yes | Text OCR (multipart upload) |
| `POST` | `/api/ocr/json` | Yes | Text OCR (JSON: base64 or URL) |
| `POST` | `/api/structure` | Yes | Structure analysis (multipart) |
| `POST` | `/api/structure/json` | Yes | Structure analysis (JSON: base64 or URL) |

For detailed API documentation, see [docs/api.md](docs/api.md).

## Authentication

Set `TOKEN` in `.env` to enable authentication:

```env
TOKEN=your-secret-token
```

All API endpoints (except `/api/health`) and the web UI require the token in the `Authorization` header:

```
Authorization: Bearer your-secret-token
```

## Models

| Model | Size | Purpose |
|-------|------|---------|
| `pp-ocrv6_small_det.onnx` | 9.4 MB | Text detection |
| `pp-ocrv6_small_rec.onnx` | 20.2 MB | Text recognition |
| `ppocrv6_dict.txt` | 73 KB | Character dictionary |
| `pp-doclayout_plus-l.onnx` | 123.7 MB | Layout detection |
| `pp-lcnet_x1_0_table_cls.onnx` | 6.5 MB | Table classification |
| `slanet_plus.onnx` | 7.4 MB | Table structure recognition |
| `table_structure_dict_ch.txt` | 0.6 KB | Table structure dictionary |
| `pp-formulanet_plus-s.onnx` | ~20 MB | Formula recognition (LaTeX) |
| `pp-formulanet-tokenizer.json` | ~1 MB | Formula tokenizer |

## License

MIT