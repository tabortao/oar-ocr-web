# API 文档

## 认证

当服务端配置了 `TOKEN` 环境变量时，除 `/api/health` 外的所有端点都需要在请求头中携带 Bearer Token：

```
Authorization: Bearer <your-token>
```

---

## 端点

### 1. GET /api/health

服务健康检查，无需认证。

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
      "status": "ready",
      "model": "PP-DocLayout_plus-L"
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

文本 OCR 识别，支持 multipart/form-data 上传图片。

**请求：**
- Content-Type: `multipart/form-data`
- 字段名: `file`、`image` 或 `image_data`
- 支持格式: JPEG、PNG、BMP、WebP、TIFF

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

**cURL 示例：**

```bash
curl -X POST http://localhost:3000/api/ocr \
  -H "Authorization: Bearer your-token" \
  -F "file=@document.jpg"
```

---

### 4. POST /api/ocr/json

文本 OCR 识别，JSON 格式请求体，支持 base64 编码图片或图床链接。

**请求：**

```json
{
  "images": [
    "data:image/jpeg;base64,/9j/4AAQ...",
    "https://example.com/image.jpg"
  ]
}
```

**响应：** 与 `/api/ocr` 相同

**cURL 示例：**

```bash
# 图床链接
curl -X POST http://localhost:3000/api/ocr/json \
  -H "Authorization: Bearer your-token" \
  -H "Content-Type: application/json" \
  -d '{"images": ["https://example.com/document.jpg"]}'

# Base64
curl -X POST http://localhost:3000/api/ocr/json \
  -H "Authorization: Bearer your-token" \
  -H "Content-Type: application/json" \
  -d '{"images": ["data:image/jpeg;base64,'$(base64 -w 0 document.jpg)'"]}'
```

---

### 5. POST /api/structure

文档结构分析，支持 multipart/form-data 上传图片。返回布局元素、表格、公式、图表和 Markdown/HTML 输出。

**请求：**
- Content-Type: `multipart/form-data`
- 字段名: `file`

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

---

### 6. POST /api/structure/json

文档结构分析，JSON 格式请求体，支持 base64 编码图片或图床链接。

**请求：**

```json
{
  "image": "https://example.com/document.jpg"
}
```

**响应：** 与 `/api/structure` 相同

**cURL 示例：**

```bash
curl -X POST http://localhost:3000/api/structure/json \
  -H "Authorization: Bearer your-token" \
  -H "Content-Type: application/json" \
  -d '{"image": "https://example.com/document.jpg"}'
```

---

## 错误响应

所有错误响应遵循统一格式：

```json
{
  "status": "error",
  "message": "错误描述"
}
```

常见错误码：

| HTTP 状态码 | 说明 |
|-------------|------|
| 400 | 请求参数错误（无文件、格式不支持、图片下载失败等） |
| 401 | 缺少或无效的 Token |
| 500 | 服务内部错误（OCR 推理失败等） |