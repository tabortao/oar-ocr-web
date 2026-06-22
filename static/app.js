// ===== DOM Elements =====
const dropZone = document.getElementById("drop-zone");
const fileInput = document.getElementById("file-input");
const loadingSection = document.getElementById("loading-section");
const resultSection = document.getElementById("result-section");
const errorSection = document.getElementById("error-section");
const errorMessage = document.getElementById("error-message");
const resultCanvas = document.getElementById("result-canvas");
const resultCount = document.getElementById("result-count");
const textList = document.getElementById("text-list");
const copyAllBtn = document.getElementById("copy-all-btn");

let currentResults = [];
let currentImage = null;

// ===== Event Listeners =====

// 点击上传
dropZone.addEventListener("click", () => fileInput.click());

// 文件选择
fileInput.addEventListener("change", (e) => {
    if (e.target.files.length > 0) {
        handleFile(e.target.files[0]);
        fileInput.value = "";
    }
});

// 拖拽上传
dropZone.addEventListener("dragover", (e) => {
    e.preventDefault();
    dropZone.classList.add("drag-over");
});

dropZone.addEventListener("dragleave", () => {
    dropZone.classList.remove("drag-over");
});

dropZone.addEventListener("drop", (e) => {
    e.preventDefault();
    dropZone.classList.remove("drag-over");
    const files = e.dataTransfer.files;
    if (files.length > 0) {
        handleFile(files[0]);
    }
});

// 一键复制
copyAllBtn.addEventListener("click", copyAllText);

// ===== File Handling =====

async function handleFile(file) {
    // 隐藏之前的错误/结果
    hideError();
    hideResult();

    // 显示加载中
    showLoading();

    try {
        const results = await uploadAndOCR(file);

        // 保存结果用于复制
        currentResults = results;
        currentImage = file;

        // 渲染结果
        renderResults(file, results);

        hideLoading();
        showResult();
    } catch (err) {
        hideLoading();
        showError(err.message || "识别失败，请重试");
    }
}

// ===== API Call =====

async function uploadAndOCR(file) {
    const formData = new FormData();
    formData.append("file", file);

    const response = await fetch("/api/ocr", {
        method: "POST",
        body: formData,
    });

    const data = await response.json();

    if (!response.ok || data.status === "error") {
        throw new Error(data.message || `请求失败 (${response.status})`);
    }

    return data.results || [];
}

// ===== Rendering =====

async function renderResults(file, results) {
    // 1. 渲染图片 + 检测框
    await renderCanvas(file, results);

    // 2. 渲染文字列表
    renderTextList(results);
}

async function renderCanvas(file, results) {
    const img = await loadImage(file);
    const canvas = resultCanvas;
    const ctx = canvas.getContext("2d");

    // 设置 canvas 尺寸
    const maxWidth = 880;
    let w = img.naturalWidth;
    let h = img.naturalHeight;

    if (w > maxWidth) {
        const scale = maxWidth / w;
        w = maxWidth;
        h = Math.round(h * scale);
    }

    canvas.width = w;
    canvas.height = h;
    ctx.clearRect(0, 0, w, h);

    // 绘制图片
    const scaleX = w / img.naturalWidth;
    const scaleY = h / img.naturalHeight;
    ctx.drawImage(img, 0, 0, w, h);

    // 绘制检测框
    results.forEach((result, idx) => {
        const region = result.text_region;
        if (!region || region.length < 4) return;

        // 坐标缩放
        const pts = region.map(([x, y]) => [x * scaleX, y * scaleY]);

        // 绘制半透明填充
        ctx.beginPath();
        ctx.moveTo(pts[0][0], pts[0][1]);
        for (let i = 1; i < pts.length; i++) {
            ctx.lineTo(pts[i][0], pts[i][1]);
        }
        ctx.closePath();
        ctx.fillStyle = "rgba(79, 70, 229, 0.12)";
        ctx.fill();

        // 绘制边框
        ctx.strokeStyle = "#4f46e5";
        ctx.lineWidth = 2;
        ctx.stroke();

        // 绘制序号标签
        const labelX = pts[0][0];
        const labelY = pts[0][1] - 6;
        ctx.fillStyle = "#4f46e5";
        ctx.font = "bold 12px -apple-system, sans-serif";
        ctx.fillText(`${idx + 1}`, labelX, labelY > 12 ? labelY : labelY + 18);
    });
}

function renderTextList(results) {
    resultCount.textContent = results.length;
    textList.innerHTML = "";

    if (results.length === 0) {
        textList.innerHTML =
            '<p style="color: var(--color-text-muted); text-align: center; padding: 24px;">未识别到文字内容</p>';
        return;
    }

    results.forEach((result, idx) => {
        const item = document.createElement("div");
        item.className = "text-item";

        const confidenceClass = getConfidenceClass(result.confidence);

        item.innerHTML = `
                    <div class="text-item-index">${idx + 1}</div>
                    <div class="text-item-content">
                        <div class="text-item-text">${escapeHtml(result.text)}</div>
                        <div class="text-item-meta">
                            <span class="text-item-confidence ${confidenceClass}">
                                置信度: ${(result.confidence * 100).toFixed(1)}%
                            </span>
                        </div>
                    </div>
                    <button class="text-item-copy" data-text="${escapeHtml(result.text)}" title="复制">
                        <svg viewBox="0 0 24 24" width="14" height="14"><path fill="currentColor" d="M16 1H4c-1.1 0-2 .9-2 2v14h2V3h12V1zm3 4H8c-1.1 0-2 .9-2 2v14c0 1.1.9 2 2 2h11c1.1 0 2-.9 2-2V7c0-1.1-.9-2-2-2zm0 16H8V7h11v14z"/></svg>
                    </button>
                `;

        // 单条复制
        item.querySelector(".text-item-copy").addEventListener("click", (e) => {
            e.stopPropagation();
            copyText(result.text);
        });

        textList.appendChild(item);
    });
}

function getConfidenceClass(confidence) {
    if (confidence >= 0.9) return "confidence-high";
    if (confidence >= 0.7) return "confidence-mid";
    return "confidence-low";
}

// ===== Copy =====

function copyAllText() {
    if (currentResults.length === 0) {
        showToast("没有可复制的内容");
        return;
    }

    const text = currentResults.map((r) => r.text).join("\n");
    copyText(text);
}

async function copyText(text) {
    try {
        await navigator.clipboard.writeText(text);
        showToast("已复制到剪贴板");
    } catch {
        // 降级方案
        const textarea = document.createElement("textarea");
        textarea.value = text;
        textarea.style.position = "fixed";
        textarea.style.opacity = "0";
        document.body.appendChild(textarea);
        textarea.select();
        document.execCommand("copy");
        document.body.removeChild(textarea);
        showToast("已复制到剪贴板");
    }
}

// ===== UI Helpers =====

function showLoading() {
    loadingSection.style.display = "block";
    resultSection.style.display = "none";
    errorSection.style.display = "none";
}

function hideLoading() {
    loadingSection.style.display = "none";
}

function showResult() {
    resultSection.style.display = "flex";
}

function hideResult() {
    resultSection.style.display = "none";
    resultCanvas.width = 0;
    resultCanvas.height = 0;
    currentResults = [];
    currentImage = null;
}

function showError(msg) {
    errorMessage.textContent = msg;
    errorSection.style.display = "block";
}

function hideError() {
    errorSection.style.display = "none";
}

function showToast(msg) {
    const existing = document.querySelector(".toast");
    if (existing) existing.remove();

    const toast = document.createElement("div");
    toast.className = "toast";
    toast.textContent = msg;
    document.body.appendChild(toast);

    setTimeout(() => toast.remove(), 2200);
}

// ===== Utilities =====

function loadImage(file) {
    return new Promise((resolve, reject) => {
        const img = new Image();
        const url = URL.createObjectURL(file);
        img.onload = () => {
            URL.revokeObjectURL(url);
            resolve(img);
        };
        img.onerror = () => {
            URL.revokeObjectURL(url);
            reject(new Error("图片加载失败"));
        };
        img.src = url;
    });
}

function escapeHtml(text) {
    const div = document.createElement("div");
    div.textContent = text;
    return div.innerHTML;
}
