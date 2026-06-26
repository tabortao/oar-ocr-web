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

## Windows User Guide

Windows builds are ideal for local testing and development — no Docker required.

### Prerequisites

- **OS**: Windows 10/11 64-bit
- **No Rust, Docker, or dev tools required** for the pre-built binary — everything works with PowerShell out of the box
- **CPU note**: The Windows ONNX Runtime prebuilts use AVX2 and require a modern desktop/laptop CPU.
  Older low-power CPUs (e.g. Celeron J4125/N5105, common in NAS devices) will crash — in that case use the Docker section below to build from source with AVX disabled

### Option 1: Use the pre-built Release binary (recommended)

GitHub Releases ships a pre-compiled Windows executable — no Rust installation needed.

1. **Download**: grab `oar-ocr-web-v0.1.x-windows-amd64.zip` from [GitHub Releases](https://github.com/tabortao/oar-ocr-web/releases)
2. **Extract**: unzip to any folder (e.g. `D:\oar-ocr-web`)
3. **Download models**: open PowerShell in the extracted folder and run:

   ```powershell
   powershell -ExecutionPolicy Bypass -File .\download_models.ps1
   ```

   Or just double-click `download_models.ps1` in Explorer (~30 seconds)

4. **Start**: double-click `oar-ocr-web.exe`. A black console window will show logs.
5. **Open browser**: visit http://localhost:3000

See Configuration below for token auth, logging, etc.

### Option 2: Build from source (developers)

1. **Install Rust**: go to https://rustup.rs/, download and run rustup (all defaults), then restart PowerShell
2. **Clone**:

   ```powershell
   git clone https://github.com/tabortao/oar-ocr-web.git
   cd oar-ocr-web
   ```

3. **Download models**:

   ```powershell
   .\download_models.ps1
   ```

4. **Configure (optional)**:

   ```powershell
   copy .env.example .env
   notepad .env
   ```

5. **Build & run**:

   ```powershell
   cargo run --release
   ```

   First build takes ~5–10 minutes (downloads deps + compiles). Incremental builds are fast.

### Configuration

All settings are environment variables or entries in `.env`:

| Variable | Default | Description |
|----------|---------|-------------|
| `TOKEN` | empty (auth disabled) | API Bearer token — leave empty to skip auth |
| `PORT` | 3000 | Web service port |
| `OAR_HOME` | `models/` next to the exe | Model file directory |
| `LOG_DIR` | `./logs` | OCR request log directory |
| `LOG_RETENTION_DAYS` | 30 | Auto-delete logs older than N days |

Temporary (current PowerShell window only):

```powershell
$env:TOKEN = "my-secret-token"
$env:PORT = "8080"
.\oar-ocr-web.exe
```

Persistent (works for both Option 1 and Option 2, put in `.env`):

```env
TOKEN=my-secret-token
PORT=8080
OAR_HOME=D:\oar-ocr-web\models
LOG_DIR=D:\oar-ocr-web\logs
LOG_RETENTION_DAYS=30
```

### FAQ

**Q: Browser can't open http://localhost:3000 after startup?**
A: Check the console window for logs. If it says `listening on port 3000` but the browser times out, Windows Firewall is blocking it — click "Allow" when prompted on first launch.

**Q: Error `model file pp-ocrv6_small_det.onnx not found`?**
A: Model download failed. Re-run `.\download_models.ps1` and confirm your machine can reach modelscope.cn. You can also manually place all 9 `.onnx` / `.txt` / `.json` files in `./models/`.

**Q: OCR feels slow?**
A: First use loads the model (~200 MB) into RAM — expect a few seconds of delay. Subsequent recognitions are fast. Minimum recommended RAM: 4 GB.

**Q: How to auto-start on boot?**
A: Put a shortcut to `oar-ocr-web.exe` in `C:\Users\<your-name>\AppData\Roaming\Microsoft\Windows\Start Menu\Programs\Startup\`.

---

## Docker Deployment (NAS / non-AVX CPUs)

### When to use this

- **NAS** (fnOS / Synology / ZSpace, etc.)
- CPUs without AVX (e.g. Celeron N5105, J4125, N100 — common in low-power NAS)
- Background always-on service

### On AVX-capable machines (most PCs / servers)

Use the pre-built Docker Hub image directly:

```powershell
docker run -d --name oar-ocr-web ^
  -p 3000:3000 ^
  -v ${PWD}/models:/app/models ^
  -v ${PWD}/logs:/app/logs ^
  -e TOKEN=your-secret-token ^
  -e OAR_HOME=/app/models ^
  --restart unless-stopped ^
  ghcr.io/tabortao/oar-ocr-web:latest
```

Or with docker-compose:

```powershell
.\download_models.ps1
docker compose up -d
```

### On non-AVX CPUs (NAS / low-power devices)

⚠️ **Using the pre-built image will crash** (SIGILL / exit code 132). You must build ONNX Runtime from source with AVX disabled:

```bash
# After cloning the repo, from the Dockerfile directory:
docker build -t oar-ocr-web-noavx .
docker run -d --name oar-ocr-web \
  -p 3000:3000 \
  -v $(pwd)/models:/app/models \
  -v $(pwd)/logs:/app/logs \
  -e TOKEN=your-secret-token \
  -e OAR_HOME=/app/models \
  --restart unless-stopped \
  oar-ocr-web-noavx
```

The ONNX Runtime source build takes ~30–60 minutes (the `ort-builder` stage of the Dockerfile), but layer caching makes subsequent builds very fast. For technical details and troubleshooting notes, see [docs/ChangeLog.md](docs/ChangeLog.md).

### NAS tips

- **fnOS (飞牛NAS)**: open the "Containers" app → New container → use image `ghcr.io/tabortao/oar-ocr-web:latest` (non-AVX CPUs must build the noavx image first as shown above) → map port 3000, create persistent volumes `/app/models` and `/app/logs`
- **Synology DSM**: open "Container Manager" → New container → same as above
- **ZSpace / 极空间**: open Docker from the app store → use the docker-compose approach above

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