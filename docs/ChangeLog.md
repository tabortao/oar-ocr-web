# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.2] - 2026-06-24
### Added
- Added favicon and logo support; PWA manifest, Service Worker, and responsive mobile layout

### Changed
- Bumped version to 0.1.2

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
