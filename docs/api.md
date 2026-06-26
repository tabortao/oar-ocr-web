# API 文档

## 认证

当服务端配置了 `TOKEN` 环境变量时，除 `/api/health` 外的所有端点都需要在请求头中携带 Bearer Token：

```
Authorization: Bearer <your-token>
```

> 未配置 `TOKEN` 时，所有接口均可匿名访问。

---

## 通用说明

### 支持的图片来源

| 输入方式 | 端点 | 字段 | 说明 |
|---------|------|------|------|
| 文件上传 (multipart) | `/api/ocr`, `/api/structure` | `file` / `image` / `image_data` | 通过 HTTP multipart 表单上传本地图片字节 |
| Base64 (JSON) | `/api/ocr/json`, `/api/structure/json` | `images[]` / `image` | 直接传 `data:image/...;base64,...` 或裸 base64 字符串 |
| 图床链接 (JSON) | `/api/ocr/json`, `/api/structure/json` | `images[]` / `image` | 服务端下载 `http(s)://` 图片后再识别，限 20MB |

### 支持的图片格式

JPEG、PNG、BMP、WebP、TIFF

### 错误响应格式

```json
{
  "status": "error",
  "message": "错误描述"
}
```

| HTTP 状态码 | 说明 |
|-------------|------|
| 400 | 请求参数错误（无文件、格式不支持、图片下载失败、图片超过 20MB 等） |
| 401 | 缺少或无效的 Token |
| 500 | 服务内部错误（OCR 推理失败等） |

---

## 端点

### 1. GET /api/health

服务健康检查，无需认证。`structure.status` 反映结构引擎实时状态：`loaded`（已加载）、`not_loaded`（未加载）、`busy`（正在使用中）。

**响应示例：**

```json
{
  "status": "ok",
  "version": "0.1.0",
  "uptime_secs": 3600,
  "engines": {
    "ocr": {
      "status": "ready",
      "model": "PP-OCRv6 small"
    },
    "structure": {
      "status": "not_loaded",
      "model": "PP-DocLayout_plus-L + SLANet_plus + PP-FormulaNet_plus-S"
    },
    "auth_enabled": true
  }
}
```

---

### 2. GET /api/auth/verify

验证 Token 是否有效。

**请求头：** `Authorization: Bearer <token>`

**响应：**

```json
{
  "status": "ok",
  "message": "Token 有效"
}
```

**错误响应：** 401 Unauthorized

---

### 3. POST /api/ocr

文本 OCR 识别，支持 multipart/form-data 上传图片。适用于**本地文件**直接上传场景。

**请求：**
- Content-Type: `multipart/form-data`
- 字段名: `file`、`image` 或 `image_data`（三者任选其一）
- 支持格式: JPEG、PNG、BMP、WebP、TIFF

**响应字段：**

| 字段 | 类型 | 说明 |
|------|------|------|
| `status` | string | `success` 或 `error` |
| `results` | array | 识别结果列表 |
| `results[].text` | string | 识别到的文字 |
| `results[].confidence` | float | 置信度 0.0~1.0 |
| `results[].text_region` | `[[x,y],[x,y],[x,y],[x,y]]` | 文本框四顶点坐标（左上→右上→右下→左下） |
| `total` | int | 识别结果总数 |

**响应示例：**

```json
{
  "status": "success",
  "results": [
    {
      "text": "识别到的文字",
      "confidence": 0.98,
      "text_region": [[10.0, 20.0], [100.0, 20.0], [100.0, 40.0], [10.0, 40.0]]
    }
  ],
  "total": 1
}
```

**cURL 示例（Windows PowerShell 上传本地文件）：**

```powershell
# Windows PowerShell（推荐）
curl.exe -X POST http://localhost:3000/api/ocr `
  -H "Authorization: Bearer your-token" `
  -F "file=@D:\images\invoice.png"

# Windows CMD
curl.exe -X POST http://localhost:3000/api/ocr ^
  -H "Authorization: Bearer your-token" ^
  -F "file=@D:\images\invoice.png"

# Linux / macOS
curl -X POST http://localhost:3000/api/ocr \
  -H "Authorization: Bearer your-token" \
  -F "file=@/path/to/document.jpg"
# Git Bash
curl -X POST http://10.0.8.4:3045/api/ocr \
  -F "file=@D:\Code\Rust\oar-ocr-web\docs\demo.png"

```

> ⚠️ Windows PowerShell 中请使用 `curl.exe` 而非 `curl`，后者是 `Invoke-WebRequest` 的别名，不支持 `-F` 参数。

**Python 示例（上传本地路径文件）：**

```python
import requests

url = "http://localhost:3000/api/ocr"
headers = {"Authorization": "Bearer your-token"}

# Windows 本地路径
with open(r"D:\images\invoice.png", "rb") as f:
    files = {"file": ("invoice.png", f, "image/png")}
    resp = requests.post(url, headers=headers, files=files)

print(resp.json())
```

**JavaScript (浏览器 / Node.js 18+) 示例：**

```javascript
// 浏览器: 通过 <input type="file"> 获取 File 对象
const file = fileInput.files[0];
const formData = new FormData();
formData.append("file", file);

const resp = await fetch("http://localhost:3000/api/ocr", {
  method: "POST",
  headers: { Authorization: "Bearer your-token" }, // 注意: 不要手动设置 Content-Type
  body: formData,
});
const data = await resp.json();
console.log(data);
```

---

### 4. POST /api/ocr/json

文本 OCR 识别，JSON 格式请求体，支持 **base64 编码图片** 或 **图床链接**。

支持三种图片字符串格式：
1. **完整 Data URL**: `data:image/jpeg;base64,/9j/4AAQ...`
2. **裸 Base64**: `/9j/4AAQ...`（自动解码）
3. **图床 URL**: `https://example.com/image.jpg`（服务端下载，限 20MB）

`images` 数组可同时传入多张图片，结果按顺序合并返回。

**请求：**

```json
{
  "images": [
    "data:image/jpeg;base64,/9j/4AAQ...",
    "https://example.com/image.jpg"
  ]
}
```

**响应：** 与 `/api/ocr` 相同；若首张为图床链接，额外返回 `image_base64` 字段（用于前端预览绕过 CORS）。

**cURL 示例：**

```bash
# 1) 图床链接（最简单）
curl -X POST http://localhost:3000/api/ocr/json \
  -H "Authorization: Bearer your-token" \
  -H "Content-Type: application/json" \
  -d '{"images": ["https://example.com/document.jpg"]}'

# 2) 多张图床链接批量识别
curl -X POST http://localhost:3000/api/ocr/json \
  -H "Authorization: Bearer your-token" \
  -H "Content-Type: application/json" \
  -d '{"images": ["https://a.com/1.jpg", "https://b.com/2.png"]}'

# 3) Base64（Linux）
curl -X POST http://localhost:3000/api/ocr/json \
  -H "Authorization: Bearer your-token" \
  -H "Content-Type: application/json" \
  -d '{"images": ["data:image/jpeg;base64,'$(base64 -w 0 document.jpg)'"]}'
```

**Windows PowerShell 示例（读取本地文件转 Base64 后调用）：**

```powershell
# 读取本地图片 → Base64 → 调用 /api/ocr/json
$imgPath = "D:\images\invoice.png"
$bytes   = [System.IO.File]::ReadAllBytes($imgPath)
$b64     = [Convert]::ToBase64String($bytes)
$body    = @{ images = @("data:image/png;base64,$b64") } | ConvertTo-Json

$resp = Invoke-RestMethod -Uri "http://localhost:3000/api/ocr/json" `
    -Method Post `
    -Headers @{ Authorization = "Bearer your-token"; "Content-Type" = "application/json" } `
    -Body $body

$resp.results | ForEach-Object { "{0} (conf={1:P0})" -f $_.text, $_.confidence }
```

**Python 示例（本地文件 Base64 + 图床链接混合）：**

```python
import base64
import requests

url = "http://localhost:3000/api/ocr/json"
headers = {
    "Authorization": "Bearer your-token",
    "Content-Type": "application/json",
}

# 1) 本地文件 → base64
with open(r"D:\images\invoice.png", "rb") as f:
    b64 = base64.b64encode(f.read()).decode("ascii")
local_image = f"data:image/png;base64,{b64}"

# 2) 图床链接
remote_image = "https://example.com/banner.jpg"

# 混合传入
payload = {"images": [local_image, remote_image]}
resp = requests.post(url, headers=headers, json=payload)
print(resp.json())
```

**JavaScript 示例（图床链接）：**

```javascript
const resp = await fetch("http://localhost:3000/api/ocr/json", {
  method: "POST",
  headers: {
    "Authorization": "Bearer your-token",
    "Content-Type": "application/json",
  },
  body: JSON.stringify({
    images: ["https://example.com/document.jpg"],
  }),
});
const data = await resp.json();
console.log(data);
```

---

### 5. POST /api/structure

文档结构分析，支持 multipart/form-data 上传图片。返回布局元素、表格、公式、图表和 Markdown/HTML 输出。

**请求：**
- Content-Type: `multipart/form-data`
- 字段名: `file`、`image` 或 `image_data`（三者任选其一）

**响应字段：**

| 字段 | 类型 | 说明 |
|------|------|------|
| `status` | string | `success` 或 `error` |
| `layout_elements` | array | 布局元素列表 |
| `tables` | array | 表格识别结果（含 HTML 结构与单元格） |
| `formulas` | array | 公式识别结果（LaTeX） |
| `chart_elements` | array | 图表元素列表 |
| `markdown` | string | 整页 Markdown 输出 |
| `html` | string | 整页 HTML 输出 |
| `total_layout` / `total_tables` / `total_formulas` / `total_charts` | int | 各类元素总数 |

**响应示例：**

```json
{
  "status": "success",
  "layout_elements": [
    {
      "element_type": "paragraph_title",
      "confidence": 0.95,
      "bbox": [[10.0, 20.0], [200.0, 20.0], [200.0, 40.0], [10.0, 40.0]],
      "text": "第一章 引言",
      "order_index": 1
    }
  ],
  "tables": [
    {
      "table_type": "Wired",
      "classification_confidence": 0.98,
      "structure_confidence": 0.95,
      "html_structure": "<table>...</table>",
      "cells": [
        {
          "row": 0,
          "col": 0,
          "row_span": 1,
          "col_span": 1,
          "confidence": 0.99,
          "text": "姓名"
        }
      ]
    }
  ],
  "formulas": [
    {
      "bbox": [[50.0, 100.0], [200.0, 100.0], [200.0, 130.0], [50.0, 130.0]],
      "latex": "E = mc^2",
      "confidence": 0.97
    }
  ],
  "chart_elements": [
    {
      "element_type": "chart",
      "confidence": 0.92,
      "bbox": [[300.0, 200.0], [600.0, 200.0], [600.0, 500.0], [300.0, 500.0]],
      "text": null,
      "order_index": 5
    }
  ],
  "markdown": "# 第一章 引言\n\n...",
  "html": "<h1>第一章 引言</h1>...",
  "total_layout": 10,
  "total_tables": 2,
  "total_formulas": 3,
  "total_charts": 1
}
```

**布局元素类型 (`element_type`)：**

| 类型 | 说明 |
|------|------|
| `doc_title` | 文档标题 |
| `paragraph_title` | 段落标题 |
| `text` | 正文文本 |
| `content` | 内容块 |
| `abstract` | 摘要 |
| `image` | 图片 |
| `table` | 表格 |
| `chart` | 图表 |
| `formula` | 公式 |
| `figure_title` | 图片标题 |
| `table_title` | 表格标题 |
| `chart_title` | 图表标题 |
| `header` | 页眉 |
| `footer` | 页脚 |
| `footnote` | 脚注 |
| `seal` | 印章 |
| `number` | 编号 |
| `reference` | 参考文献 |
| `list` | 列表 |
| `algorithm` | 算法 |
| `aside_text` | 侧边栏文字 |

**cURL 示例（Windows PowerShell 上传本地文件）：**

```powershell
# Windows PowerShell
curl.exe -X POST http://localhost:3000/api/structure `
  -H "Authorization: Bearer your-token" `
  -F "file=@D:\docs\paper.pdf.png"

# Linux / macOS
curl -X POST http://localhost:3000/api/structure \
  -H "Authorization: Bearer your-token" \
  -F "file=@/path/to/document.png"
```

**Python 示例（上传本地文件并保存 Markdown）：**

```python
import requests

url = "http://localhost:3000/api/structure"
headers = {"Authorization": "Bearer your-token"}

with open(r"D:\docs\contract.png", "rb") as f:
    files = {"file": ("contract.png", f, "image/png")}
    resp = requests.post(url, headers=headers, files=files)

data = resp.json()
print(f"布局元素: {data['total_layout']}, 表格: {data['total_tables']}")
print(f"公式: {data['total_formulas']}, 图表: {data['total_charts']}")

# 保存 Markdown 输出
with open("contract.md", "w", encoding="utf-8") as f:
    f.write(data["markdown"])
```

---

### 6. POST /api/structure/json

文档结构分析，JSON 格式请求体，支持 **base64 编码图片** 或 **图床链接**。

支持三种图片字符串格式（与 `/api/ocr/json` 相同）：
1. **完整 Data URL**: `data:image/jpeg;base64,/9j/4AAQ...`
2. **裸 Base64**: `/9j/4AAQ...`
3. **图床 URL**: `https://example.com/image.jpg`（限 20MB）

> ⚠️ 与 `/api/ocr/json` 不同，本接口字段名为单数 `image`（仅支持单张图片），不是 `images` 数组。

**请求：**

```json
{
  "image": "https://example.com/document.jpg"
}
```

**响应：** 与 `/api/structure` 相同

**cURL 示例：**

```bash
# 1) 图床链接
curl -X POST http://localhost:3000/api/structure/json \
  -H "Authorization: Bearer your-token" \
  -H "Content-Type: application/json" \
  -d '{"image": "https://example.com/document.jpg"}'

# 2) Base64（Linux）
curl -X POST http://localhost:3000/api/structure/json \
  -H "Authorization: Bearer your-token" \
  -H "Content-Type: application/json" \
  -d '{"image": "data:image/jpeg;base64,'$(base64 -w 0 document.jpg)'"}'
```

**Windows PowerShell 示例（本地文件转 Base64 后调用）：**

```powershell
$imgPath = "D:\docs\paper.png"
$bytes   = [System.IO.File]::ReadAllBytes($imgPath)
$b64     = [Convert]::ToBase64String($bytes)
$body    = @{ image = "data:image/png;base64,$b64" } | ConvertTo-Json

$resp = Invoke-RestMethod -Uri "http://localhost:3000/api/structure/json" `
    -Method Post `
    -Headers @{ Authorization = "Bearer your-token"; "Content-Type" = "application/json" } `
    -Body $body

# 输出 Markdown
$resp.markdown | Out-File -FilePath "paper.md" -Encoding utf8
Write-Host "布局: $($resp.total_layout), 表格: $($resp.total_tables), 公式: $($resp.total_formulas)"
```

**Python 示例（图床链接 + Base64 混合）：**

```python
import base64
import requests

url = "http://localhost:3000/api/structure/json"
headers = {
    "Authorization": "Bearer your-token",
    "Content-Type": "application/json",
}

# 1) 图床链接
resp = requests.post(url, headers=headers, json={"image": "https://example.com/paper.jpg"})
print(resp.json()["markdown"])

# 2) 本地文件 → base64
with open(r"D:\docs\paper.png", "rb") as f:
    b64 = base64.b64encode(f.read()).decode("ascii")
resp = requests.post(
    url,
    headers=headers,
    json={"image": f"data:image/png;base64,{b64}"},
)
print(resp.json()["markdown"])
```

**JavaScript 示例（图床链接）：**

```javascript
const resp = await fetch("http://localhost:3000/api/structure/json", {
  method: "POST",
  headers: {
    Authorization: "Bearer your-token",
    "Content-Type": "application/json",
  },
  body: JSON.stringify({ image: "https://example.com/document.jpg" }),
});
const data = await resp.json();
console.log(data.markdown);
```

---

## 性能与内存行为

- **文本 OCR 引擎** (`/api/ocr`, `/api/ocr/json`): PP-OCRv6 small 常驻内存（~80MB），保证快速响应。每次请求结束后自动调用 `malloc_trim(0)` 归还工作内存（图像缓冲区、ORT 中间张量）给操作系统，RSS 不随请求数增长。
- **结构 OCR 引擎** (`/api/structure`, `/api/structure/json`): 按需加载（首次请求时初始化，~160MB），**每次请求结束后自动释放引擎并调用 `malloc_trim(0)` 归还内存给操作系统**，Docker 容器 RSS 应在请求结束后回落到 OCR 引擎基线。
- 两个引擎均已关闭 ORT `enable_mem_pattern`（内存模式缓存），避免变宽输入（OCR 文本行长度不同）导致 arena 内存无限增长。保留 `Level3` 图优化和 `intra_threads=available_parallelism` 以兼顾速度。
- 结构引擎初始化需要约 1~3 秒（模型加载到内存），首次请求会略慢。