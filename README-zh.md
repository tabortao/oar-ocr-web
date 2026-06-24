# oar-ocr-web

基于 [oar-ocr](https://github.com/GreatV/oar-ocr) 的高性能 OCR Web API 服务，兼容 PaddleOCR Serving 协议。

## 功能特性

- **文本 OCR** — PP-OCRv6 small 引擎，支持字级检测和置信度评分
- **文档结构分析** — 布局检测 (PP-DocLayout_plus-L)、表格识别 (SLANet_plus)、公式识别 (PP-FormulaNet_plus-S)
- **图表检测** — 自动识别文档中的图表、流程图和图片标题
- **图床链接支持** — 支持输入图片 URL 直接进行 OCR 识别
- **Token 认证** — 可选的 Bearer Token 认证，保护 API 和 Web UI
- **PaddleOCR 兼容** — 可直接替换 PaddleOCR Serving API
- **Web 界面** — 内置拖拽上传界面，支持进度可视化
- **Docker 部署** — 模型文件预打包进镜像，首次运行无需下载

## 快速开始

### 1. 下载模型

```powershell
.\download_models.ps1
```

下载所有模型文件（约 167 MB）到 `./models/` 目录。

### 2. 配置环境

```powershell
copy .env.example .env
# 编辑 .env 设置 TOKEN（可选）
```

### 3. 运行

```powershell
$env:OAR_HOME = "./models"; cargo run --release
# 启用日志记录
$env:OAR_HOME = "./models"; $env:LOG_DIR = "./logs"; $env:LOG_RETENTION_DAYS = "30"; cargo run --release
```

服务启动在 `http://localhost:3000`。

### 4. Docker 部署

```powershell
.\download_models.ps1
docker compose build
docker compose up -d
```

模型文件打包在镜像中，首次启动时自动释放到持久化卷。

## API 端点

| 方法 | 端点 | 认证 | 说明 |
|------|------|------|------|
| `GET` | `/api/health` | 否 | 服务健康状态 |
| `GET` | `/api/auth/verify` | 是 | 验证 Token 有效性 |
| `POST` | `/api/ocr` | 是 | 文本 OCR（multipart 上传） |
| `POST` | `/api/ocr/json` | 是 | 文本 OCR（JSON: base64 或 URL） |
| `POST` | `/api/structure` | 是 | 结构分析（multipart 上传） |
| `POST` | `/api/structure/json` | 是 | 结构分析（JSON: base64 或 URL） |

详细 API 文档见 [docs/api.md](docs/api.md)。

## 认证

在 `.env` 中设置 `TOKEN` 启用认证：

```env
TOKEN=your-secret-token
```

除 `/api/health` 外，所有 API 端点和 Web UI 需要在 `Authorization` 头中携带 Token：

```
Authorization: Bearer your-secret-token
```

## 模型列表

| 模型 | 大小 | 用途 |
|------|------|------|
| `pp-ocrv6_small_det.onnx` | 9.4 MB | 文本检测 |
| `pp-ocrv6_small_rec.onnx` | 20.2 MB | 文本识别 |
| `ppocrv6_dict.txt` | 73 KB | 字符词典 |
| `pp-doclayout_plus-l.onnx` | 123.7 MB | 布局检测 |
| `pp-lcnet_x1_0_table_cls.onnx` | 6.5 MB | 表格分类 |
| `slanet_plus.onnx` | 7.4 MB | 表格结构识别 |
| `table_structure_dict_ch.txt` | 0.6 KB | 表格结构词典 |
| `pp-formulanet_plus-s.onnx` | ~20 MB | 公式识别 (LaTeX) |
| `pp-formulanet-tokenizer.json` | ~1 MB | 公式分词器 |

## License

MIT