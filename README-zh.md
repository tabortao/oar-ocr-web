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

## Windows 版使用指南

Windows 版适合本地快速体验和开发调试，无需 Docker 环境。以下步骤以最傻瓜化的方式带您跑通。

### 前置条件

- **操作系统**：Windows 10 64 位 / Windows 11 64 位
- **不需要**安装 Rust、Docker 或任何开发工具——所有步骤都用 PowerShell 内置功能
- **模型要求**：Windows 版 ONNX Runtime 预构建包使用 AVX2，**仅支持现代桌面/笔记本 CPU**。
  如果 CPU 较老（如赛扬 J4125/N5105 等低功耗 NAS CPU），请改用下方「Docker 部署」部分的说明，从源码构建无 AVX 版本

### 方式一：使用已编译的 Release 二进制包（推荐）

GitHub Releases 提供预编译的 Windows 可执行文件，无需安装 Rust 工具链。

1. **下载**：访问 [GitHub Releases](https://github.com/tabortao/oar-ocr-web/releases)，下载 `oar-ocr-web-v0.1.x-windows-amd64.zip`
2. **解压**：解压到任意目录（如 `D:\oar-ocr-web`）
3. **下载模型**：在解压目录中右键打开 PowerShell，执行：

   ```powershell
   powershell -ExecutionPolicy Bypass -File .\download_models.ps1
   ```

   或直接在资源管理器中双击 `download_models.ps1`（约 30 秒）

4. **启动**：双击 `oar-ocr-web.exe`，会弹出黑色控制台窗口显示日志
5. **打开浏览器**：访问 http://localhost:3000

如需配置 Token 认证或日志目录，见下文「配置」。

### 方式二：从源码编译（适合开发者）

1. **安装 Rust**：打开 https://rustup.rs/ 下载安装 rustup（一路默认即可），安装完成后重启 PowerShell
2. **克隆项目**：

   ```powershell
   git clone https://github.com/tabortao/oar-ocr-web.git
   cd oar-ocr-web
   ```

3. **下载模型**：

   ```powershell
   .\download_models.ps1
   ```

4. **配置环境变量（可选）**：

   ```powershell
   copy .env.example .env
   notepad .env
   ```

5. **编译并运行**：

   ```powershell
   cargo run --release
   ```

   首次编译约 5~10 分钟（下载依赖 + 编译），后续增量编译仅需几秒。

### 配置

所有配置通过环境变量或 `.env` 文件设置：

| 变量 | 默认值 | 说明 |
|------|--------|------|
| `TOKEN` | 空（不启用认证） | API Bearer Token，留空则不需要认证 |
| `PORT` | 3000 | Web 服务监听端口 |
| `OAR_HOME` | 可执行文件旁的 `models/` | 模型文件目录 |
| `LOG_DIR` | `./logs` | OCR 请求日志目录 |
| `LOG_RETENTION_DAYS` | 30 | 日志保留天数，到期自动删除 |

PowerShell 临时设置（只对当前窗口有效）：

```powershell
$env:TOKEN = "my-secret-token"
$env:PORT = "8080"
.\oar-ocr-web.exe
```

持久设置（方式一和方式二通用，放到 `.env` 文件中）：

```env
TOKEN=my-secret-token
PORT=8080
OAR_HOME=D:\oar-ocr-web\models
LOG_DIR=D:\oar-ocr-web\logs
LOG_RETENTION_DAYS=30
```

### 常见问题

**Q: 启动后浏览器访问 http://localhost:3000 打不开？**
A: 检查控制台窗口是否有日志输出。如果显示 `listening on port 3000` 但浏览器访问超时，可能是 Windows 防火墙拦截——第一次启动时 Windows 会弹出防火墙提示，点「允许」即可。

**Q: 启动报错 `找不到模型文件 pp-ocrv6_small_det.onnx`？**
A: 说明模型没下载成功。重新运行 `.\download_models.ps1`，确认网络可访问 modelscope.cn。也可以手动将 9 个模型文件放到 `./models/` 目录。

**Q: OCR 速度慢？**
A: 首次使用会延迟几秒加载模型到内存（约 200 MB），后续识别会非常快。内存建议至少 4 GB。

**Q: 需要开机自启？**
A: 把 `oar-ocr-web.exe` 的快捷方式放到 `C:\Users\<你的用户名>\AppData\Roaming\Microsoft\Windows\Start Menu\Programs\Startup\` 即可。

---

## Docker 部署（适合 NAS / 非 AVX CPU）

### 适用场景

- **NAS 部署**（飞牛NAS / 群晖 / 极空间等）
- CPU 不支持 AVX 指令集（如 Intel Celeron N5105、J4125、N100 等低功耗 CPU）
- 需要后台常驻运行

### 在支持 AVX 的机器上（大多数 PC / 服务器）

直接使用 Docker Hub 镜像：

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

或使用 docker-compose：

```powershell
.\download_models.ps1
docker compose up -d
```

### 在非 AVX CPU 上（NAS / 低功耗设备）

⚠️ **直接使用预构建镜像会崩溃**（SIGILL / exit code 132），必须从源码构建 ONNX Runtime（禁用 AVX）：

```bash
# 克隆项目后，在 Dockerfile 所在目录执行
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

ONNX Runtime 源码构建约需 30~60 分钟（Dockerfile 的 `ort-builder` 阶段），但层缓存会让后续构建极快。关于技术细节和排障记录，见 [docs/ChangeLog.md](docs/ChangeLog.md)。

### NAS 部署小贴士

- **飞牛NAS / fnOS**：打开「容器」应用 → 新建容器 → 选择镜像 `ghcr.io/tabortao/oar-ocr-web:latest`（非 AVX CPU 需先按上面步骤构建无 AVX 镜像）→ 映射端口 3000、创建 `/app/models` 和 `/app/logs` 持久化卷
- **群晖 DSM**：打开「容器管理器」→ 新建容器 → 同上
- **极空间 ZSpace**：应用市场搜索 Docker → 同 docker-compose 方式

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