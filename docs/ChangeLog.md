# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.3] - 2026-06-25
### Fixed
- **Docker SIGILL crash on non-AVX CPUs (e.g. Intel Celeron N5105 / fnNAS)**: The prebuilt ONNX Runtime downloaded by `ort-sys` is compiled with `-mavx2`, which triggers `SIGILL` (exit code 132) on CPUs without AVX/AVX2/FMA support. Now ONNX Runtime v1.25.0 is built from source in a dedicated Docker stage with `-march=x86-64 -msse4.2 -mtune=generic` (SSE4.2 baseline, no AVX), and `ort-sys` is configured via `ORT_LIB_LOCATION` + `ORT_PREFER_DYNAMIC_LINK=1` to link against this custom build instead of the prebuilt artifact
- Docker container crash loop: `.so` files are now copied from the custom ONNX Runtime build stage to `/usr/local/lib/` with `ldconfig` registration
- Added `libstdc++6` and `libgomp1` to runtime image (ONNX Runtime C++ and OpenMP dependencies)

### Changed
- Bumped version to 0.1.3
- Dockerfile refactored to 3-stage build: `ort-builder` (ONNX Runtime from source), `builder` (Rust app), `runtime` (minimal image)
- Docker entrypoint simplified: removed diagnostic output (ldd/CPU/memory checks, exit-code signal decoding), restored `exec "$@"` for proper PID 1 signal handling

## [0.1.2] - 2026-06-24
### Added
- Favicon and logo support; PWA manifest, Service Worker, and responsive mobile layout
- Footer with copyright and GitHub link
- Docker model volume persistence: container downloads models from ModelScope on first run to mounted volume
- Docker HEALTHCHECK for container health monitoring
- Startup model file listing for easier debugging

### Changed
- Bumped version to 0.1.2
- Docker models now persist across container updates via volume mount (`./models:/app/models`), downloaded on first run from ModelScope
- Docker entrypoint script auto-downloads models from ModelScope on first run
- Replaced `expect()` panics with graceful error handling and detailed logging
- Release binaries now include version number in filename (e.g. `oar-ocr-web-v0.1.2-windows-amd64.zip`)
- Windows build artifact packaging now produces single-layer zip (no nested folder)

### Fixed
- Docker container crash loop: added `libssl3` runtime dependency for `oar-ocr` auto-download TLS support
- Fixed entrypoint model file listing producing empty filename

## [0.1.1] - 2026-06-23

### Added
- GitHub Actions now builds Windows executable on every push/PR and uploads artifact
- Web UI now displays total elapsed time from click to result for OCR and structure analysis
- `/api/ocr/json` returns `image_base64` for hotlinked images so the browser can render the annotated preview without CORS issues


### Changed
- Bumped version to 0.1.1

## [0.1.0] - 2026-06-23

### Added
- OCR API endpoint (`/api/ocr`, `/api/ocr/json`) compatible with PaddleOCR Serving protocol
- Document structure recognition API (`/api/structure`, `/api/structure/json`) with PP-DocLayout_plus-L layout detection and SLANet_plus table recognition
- Formula recognition (LaTeX) via PP-FormulaNet_plus-S in structure analysis
- Chart element detection in document structure analysis
- Health check endpoint (`/api/health`) with detailed engine status
- Token verification endpoint (`/api/auth/verify`) for secure login validation
- Web UI with drag-and-drop image upload, URL input, and result visualization
- Token-based authentication for API endpoints and web UI
- Image URL (hotlink) support for OCR and structure analysis
- `download_models.ps1` script to pre-download model files from ModelScope
- Docker support with bundled model files and persistent volume via `docker-entrypoint.sh`
- README.md, README-zh.md, and API documentation (docs/api.md)
- OCR request logging system (JSONL format) with configurable log directory and retention period
- Automatic log cleanup for expired log files based on `LOG_RETENTION_DAYS`
- Auto-download models to `models/` directory next to the executable on first startup
- Linux/macOS model download script (`download_models.sh`)
- GitHub Actions CI/CD workflow for automated Docker build, binary release, and testing

### Changed
- Structure engine now lazy-loaded on demand and released after each request (~160MB memory saved at idle)
- PP-OCRv6 text OCR engine stays resident for fast text OCR response
- `/api/health` now shows actual structure engine status (`loaded` / `not_loaded` / `busy`)
- Docker configuration added log volume mount and environment variables (`LOG_DIR`, `LOG_RETENTION_DAYS`)
- Docker models now bundled directly in image (no persistent volume needed); removed `docker-entrypoint.sh`
- Default `OAR_HOME` changed to `models/` next to the executable (was `~/.oar`)
- `docker-compose.yml` now uses prebuilt `ghcr.io/tabortao/oar-ocr-web:latest` image for NAS deployment

### Fixed
- Suppressed `dead_code` warning for `StructureResponse::error` with `#[allow(dead_code)]`
- Fixed token login validation using `/api/auth/verify` instead of `/api/health`
- Fixed WebP image URL OCR failure by adding `webp` feature to the `image` crate
- Fixed `$` character in token being interpreted as variable expansion by dotenvy (use `$$` to escape)
- Fixed frontend: now auto-detects whether auth is required and skips login when no token is configured
- Fixed frontend: image URL OCR results now display correctly even when canvas rendering fails due to CORS
- Fixed `civil_from_days` test using wrong Unix epoch day count for 2026-06-23
- Fixed Docker Linux build linker errors by switching base image to `ubuntu:24.04` (glibc 2.39 required by ONNX Runtime)
