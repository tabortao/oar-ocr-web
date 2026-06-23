// ===== Token 管理 =====
(function () {
    // 优先 URL query
    var params = new URLSearchParams(location.search);
    var token = params.get("token");

    if (!token) {
        token = sessionStorage.getItem("ocr_token");
    }

    // 先探测服务器是否需要认证
    fetch("/api/auth/verify")
        .then(function (r) {
            if (r.status === 200) {
                // 服务器无需认证，直接进入主界面
                showMain();
                return;
            }
            // 需要认证，检查是否有缓存的 token
            if (token) {
                sessionStorage.setItem("ocr_token", token);
                fetch("/api/auth/verify", {
                    headers: { Authorization: "Bearer " + token },
                })
                    .then(function (r2) {
                        if (r2.status === 200) {
                            showMain();
                        } else {
                            showLogin();
                        }
                    })
                    .catch(function () { showLogin(); });
            } else {
                showLogin();
            }
        })
        .catch(function () {
            // 服务器不可达，尝试用缓存 token
            if (token) {
                sessionStorage.setItem("ocr_token", token);
                fetch("/api/auth/verify", {
                    headers: { Authorization: "Bearer " + token },
                })
                    .then(function (r2) {
                        if (r2.status === 200) {
                            showMain();
                        } else {
                            showLogin();
                        }
                    })
                    .catch(function () { showLogin(); });
            } else {
                showLogin();
            }
        });

    function showLogin() {
        sessionStorage.removeItem("ocr_token");
        document.getElementById("login-overlay").style.display = "flex";
        document.getElementById("main-app").style.display = "none";
    }

    function showMain() {
        document.getElementById("login-overlay").style.display = "none";
        document.getElementById("main-app").style.display = "block";
    }

    document.getElementById("login-btn").addEventListener("click", function () {
        const val = document.getElementById("login-input").value.trim();
        if (!val) return;

        fetch("/api/auth/verify", {
            headers: { Authorization: "Bearer " + val },
        })
            .then((r) => {
                if (r.status === 200) {
                    sessionStorage.setItem("ocr_token", val);
                    showMain();
                } else {
                    document.getElementById("login-error").textContent =
                        "Token 无效";
                }
            })
            .catch(function () {
                document.getElementById("login-error").textContent =
                    "验证失败，请检查服务状态";
            });
    });

    document
        .getElementById("login-input")
        .addEventListener("keydown", function (e) {
            if (e.key === "Enter") document.getElementById("login-btn").click();
        });
})();

// ===== 全局 =====
function getToken() {
    return sessionStorage.getItem("ocr_token") || "";
}

function authHeaders() {
    const t = getToken();
    return t ? { Authorization: "Bearer " + t } : {};
}

function showToast(msg) {
    var el = document.querySelector(".toast");
    if (el) el.remove();
    el = document.createElement("div");
    el.className = "toast";
    el.textContent = msg;
    document.body.appendChild(el);
    setTimeout(function () {
        el.remove();
    }, 2200);
}

function escapeHtml(t) {
    var d = document.createElement("div");
    d.textContent = t;
    return d.innerHTML;
}

async function copyText(t) {
    try {
        await navigator.clipboard.writeText(t);
        showToast("已复制到剪贴板");
    } catch {
        var ta = document.createElement("textarea");
        ta.value = t;
        ta.style.cssText = "position:fixed;opacity:0";
        document.body.appendChild(ta);
        ta.select();
        document.execCommand("copy");
        document.body.removeChild(ta);
        showToast("已复制到剪贴板");
    }
}

// ===== Tab 切换 =====
document.querySelectorAll(".tab-btn").forEach(function (btn) {
    btn.addEventListener("click", function () {
        var tab = this.dataset.tab;
        document.querySelectorAll(".tab-btn").forEach(function (b) {
            b.classList.remove("active");
        });
        this.classList.add("active");
        document.querySelectorAll(".panel").forEach(function (p) {
            p.classList.remove("active");
        });
        document.getElementById("panel-" + tab).classList.add("active");
    });
});

// ===== 文本 OCR =====
var dropZone = document.getElementById("drop-zone");
var fileInput = document.getElementById("file-input");
var urlInput = document.getElementById("url-input");
var urlBtn = document.getElementById("url-btn");
var loadingSection = document.getElementById("loading-section");
var resultSection = document.getElementById("result-section");
var errorSection = document.getElementById("error-section");
var errorMessage = document.getElementById("error-message");
var resultCanvas = document.getElementById("result-canvas");
var resultCount = document.getElementById("result-count");
var textList = document.getElementById("text-list");
var copyAllBtn = document.getElementById("copy-all-btn");

var currentResults = [];
var currentImage = null;

dropZone.addEventListener("click", function () {
    fileInput.click();
});
fileInput.addEventListener("change", function (e) {
    if (e.target.files.length > 0) {
        handleFile(e.target.files[0]);
        fileInput.value = "";
    }
});
dropZone.addEventListener("dragover", function (e) {
    e.preventDefault();
    dropZone.classList.add("drag-over");
});
dropZone.addEventListener("dragleave", function () {
    dropZone.classList.remove("drag-over");
});
dropZone.addEventListener("drop", function (e) {
    e.preventDefault();
    dropZone.classList.remove("drag-over");
    if (e.dataTransfer.files.length > 0) {
        handleFile(e.dataTransfer.files[0]);
    }
});

urlBtn.addEventListener("click", function () {
    var url = urlInput.value.trim();
    if (!url) return;
    handleUrl(url);
});

urlInput.addEventListener("keydown", function (e) {
    if (e.key === "Enter") urlBtn.click();
});

copyAllBtn.addEventListener("click", function () {
    var t = currentResults.map(function (r) {
        return r.text;
    }).join("\n");
    if (!t) { showToast("没有可复制的内容"); return; }
    copyText(t);
});

async function handleFile(file) {
    hideError();
    hideResult();
    showLoading();

    try {
        var fd = new FormData();
        fd.append("file", file);

        var resp = await fetch("/api/ocr", {
            method: "POST",
            headers: authHeaders(),
            body: fd,
        });
        var data = await resp.json();

        if (!resp.ok || data.status === "error") {
            throw new Error(data.message || "OCR 失败");
        }

        currentResults = data.results || [];
        currentImage = file;
        await renderResults(file, currentResults);
        hideLoading();
        showResult();
    } catch (err) {
        hideLoading();
        showError(err.message);
    }
}

async function handleUrl(url) {
    hideError();
    hideResult();
    showLoading();

    try {
        var resp = await fetch("/api/ocr/json", {
            method: "POST",
            headers: Object.assign({ "Content-Type": "application/json" }, authHeaders()),
            body: JSON.stringify({ images: [url] }),
        });
        var data = await resp.json();

        if (!resp.ok || data.status === "error") {
            throw new Error(data.message || "OCR 失败");
        }

        currentResults = data.results || [];
        currentImage = url;

        // 尝试下载图片用于 Canvas 渲染，失败不影响结果显示
        try {
            var img = await loadImageUrl(url);
            await renderResultsImg(img, currentResults);
        } catch (e) {
            // 图片加载失败（如 CORS 限制），仅显示文字结果
            console.warn("图片加载失败，仅显示文字结果:", e.message);
            renderTextList(currentResults);
        }
        hideLoading();
        showResult();
    } catch (err) {
        hideLoading();
        showError(err.message);
    }
}

async function renderResults(file, results) {
    var img = await loadImageFile(file);
    renderCanvas(img, results);
    renderTextList(results);
}

async function renderResultsImg(img, results) {
    renderCanvas(img, results);
    renderTextList(results);
}

function renderCanvas(img, results) {
    var canvas = resultCanvas;
    var ctx = canvas.getContext("2d");
    var maxW = 880;
    var w = img.naturalWidth || img.width;
    var h = img.naturalHeight || img.height;
    if (w > maxW) {
        var scale = maxW / w;
        w = maxW;
        h = Math.round(h * scale);
    }
    canvas.width = w;
    canvas.height = h;
    ctx.clearRect(0, 0, w, h);

    var sx = w / (img.naturalWidth || img.width);
    var sy = h / (img.naturalHeight || img.height);
    ctx.drawImage(img, 0, 0, w, h);

    results.forEach(function (r, i) {
        var pts = r.text_region;
        if (!pts || pts.length < 4) return;

        ctx.beginPath();
        ctx.moveTo(pts[0][0] * sx, pts[0][1] * sy);
        for (var j = 1; j < pts.length; j++) {
            ctx.lineTo(pts[j][0] * sx, pts[j][1] * sy);
        }
        ctx.closePath();
        ctx.fillStyle = "rgba(79, 70, 229, 0.12)";
        ctx.fill();
        ctx.strokeStyle = "#4f46e5";
        ctx.lineWidth = 2;
        ctx.stroke();

        var lx = pts[0][0] * sx;
        var ly = pts[0][1] * sy - 6;
        ctx.fillStyle = "#4f46e5";
        ctx.font = "bold 12px -apple-system, sans-serif";
        ctx.fillText(
            "" + (i + 1),
            lx,
            ly > 12 ? ly : ly + 18
        );
    });
}

function renderTextList(results) {
    resultCount.textContent = results.length;
    textList.innerHTML = "";
    if (results.length === 0) {
        textList.innerHTML =
            '<p style="color:var(--color-text-muted);text-align:center;padding:24px;">未识别到文字内容</p>';
        return;
    }
    results.forEach(function (r, i) {
        var cls = "confidence-high";
        if (r.confidence < 0.7) cls = "confidence-low";
        else if (r.confidence < 0.9) cls = "confidence-mid";

        var div = document.createElement("div");
        div.className = "text-item";
        div.innerHTML =
            '<div class="text-item-index">' +
            (i + 1) +
            '</div><div class="text-item-content"><div class="text-item-text">' +
            escapeHtml(r.text) +
            '</div><div class="text-item-meta"><span class="text-item-confidence ' +
            cls +
            '">置信度: ' +
            (r.confidence * 100).toFixed(1) +
            "%</span></div></div>" +
            '<button class="text-item-copy" title="复制"><svg viewBox="0 0 24 24" width="14" height="14"><path fill="currentColor" d="M16 1H4c-1.1 0-2 .9-2 2v14h2V3h12V1zm3 4H8c-1.1 0-2 .9-2 2v14c0 1.1.9 2 2 2h11c1.1 0 2-.9 2-2V7c0-1.1-.9-2-2-2zm0 16H8V7h11v14z"/></svg></button>';

        div
            .querySelector(".text-item-copy")
            .addEventListener("click", function (e) {
                e.stopPropagation();
                copyText(r.text);
            });
        textList.appendChild(div);
    });
}

function showLoading() {
    loadingSection.style.display = "block";
}
function hideLoading() {
    loadingSection.style.display = "none";
}
function showResult() {
    resultSection.style.display = "block";
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

function loadImageFile(file) {
    return new Promise(function (ok, fail) {
        var img = new Image();
        var url = URL.createObjectURL(file);
        img.onload = function () {
            URL.revokeObjectURL(url);
            ok(img);
        };
        img.onerror = function () {
            URL.revokeObjectURL(url);
            fail(new Error("图片加载失败"));
        };
        img.src = url;
    });
}

function loadImageUrl(url) {
    return new Promise(function (ok, fail) {
        var img = new Image();
        img.crossOrigin = "anonymous";
        img.onload = function () {
            ok(img);
        };
        img.onerror = function () {
            fail(new Error("图片下载失败"));
        };
        img.src = url;
    });
}

// ===== 结构分析 =====
var sDropZone = document.getElementById("s-drop-zone");
var sFileInput = document.getElementById("s-file-input");
var sUrlInput = document.getElementById("s-url-input");
var sUrlBtn = document.getElementById("s-url-btn");
var sLoading = document.getElementById("s-loading-section");
var sError = document.getElementById("s-error-section");
var sErrorMsg = document.getElementById("s-error-message");
var sResult = document.getElementById("s-result-section");
var sLayoutList = document.getElementById("s-layout-list");
var sLayoutCount = document.getElementById("s-layout-count");
var sTableCount = document.getElementById("s-table-count");
var sTablesList = document.getElementById("s-tables-list");
var sFormulaCount = document.getElementById("s-formula-count");
var sFormulasList = document.getElementById("s-formulas-list");
var sChartCount = document.getElementById("s-chart-count");
var sChartsList = document.getElementById("s-charts-list");
var sMdCode = document.getElementById("s-markdown-code");
var sHtmlPreview = document.getElementById("s-html-preview");
var sCopyMd = document.getElementById("s-copy-md-btn");
var sCopyHtml = document.getElementById("s-copy-html-btn");

var currentStructure = { markdown: "", html: "" };

sDropZone.addEventListener("click", function () {
    sFileInput.click();
});
sFileInput.addEventListener("change", function (e) {
    if (e.target.files.length > 0) {
        handleSFile(e.target.files[0]);
        sFileInput.value = "";
    }
});
sDropZone.addEventListener("dragover", function (e) {
    e.preventDefault();
    sDropZone.classList.add("drag-over");
});
sDropZone.addEventListener("dragleave", function () {
    sDropZone.classList.remove("drag-over");
});
sDropZone.addEventListener("drop", function (e) {
    e.preventDefault();
    sDropZone.classList.remove("drag-over");
    if (e.dataTransfer.files.length > 0) handleSFile(e.dataTransfer.files[0]);
});

sUrlBtn.addEventListener("click", function () {
    var url = sUrlInput.value.trim();
    if (!url) return;
    handleSUrl(url);
});
sUrlInput.addEventListener("keydown", function (e) {
    if (e.key === "Enter") sUrlBtn.click();
});

sCopyMd.addEventListener("click", function () {
    copyText(currentStructure.markdown || "无内容");
});
sCopyHtml.addEventListener("click", function () {
    copyText(currentStructure.html || "无内容");
});

// Structure tab switching
document.querySelectorAll(".s-tab-btn").forEach(function (btn) {
    btn.addEventListener("click", function () {
        var tab = this.dataset.stab;
        document.querySelectorAll(".s-tab-btn").forEach(function (b) {
            b.classList.remove("active");
        });
        this.classList.add("active");
        document.querySelectorAll(".s-panel").forEach(function (p) {
            p.classList.remove("active");
        });
        document.getElementById("s-panel-" + tab).classList.add("active");
    });
});

async function handleSFile(file) {
    hideSError();
    hideSResult();
    showSLoading();

    try {
        var fd = new FormData();
        fd.append("file", file);

        var resp = await fetch("/api/structure", {
            method: "POST",
            headers: authHeaders(),
            body: fd,
        });
        var data = await resp.json();
        if (!resp.ok || data.status === "error") {
            throw new Error(data.message || "结构分析失败");
        }
        renderStructure(data);
        hideSLoading();
        showSResult();
    } catch (err) {
        hideSLoading();
        showSError(err.message);
    }
}

async function handleSUrl(url) {
    hideSError();
    hideSResult();
    showSLoading();

    try {
        var resp = await fetch("/api/structure/json", {
            method: "POST",
            headers: Object.assign({ "Content-Type": "application/json" }, authHeaders()),
            body: JSON.stringify({ image: url }),
        });
        var data = await resp.json();
        if (!resp.ok || data.status === "error") {
            throw new Error(data.message || "结构分析失败");
        }
        renderStructure(data);
        hideSLoading();
        showSResult();
    } catch (err) {
        hideSLoading();
        showSError(err.message);
    }
}

function renderStructure(data) {
    var layout = data.layout_elements || [];
    var tables = data.tables || [];
    var formulas = data.formulas || [];
    var charts = data.chart_elements || [];
    currentStructure.markdown = data.markdown || "";
    currentStructure.html = data.html || "";

    sLayoutCount.textContent = layout.length;
    sTableCount.textContent = tables.length;
    sFormulaCount.textContent = formulas.length;
    sChartCount.textContent = charts.length;

    // Layout
    sLayoutList.innerHTML = "";
    layout.forEach(function (el) {
        var card = document.createElement("div");
        card.className = "layout-card";
        var conf = (el.confidence * 100).toFixed(1);
        var text = el.text ? escapeHtml(el.text) : '<span style="color:var(--color-text-muted)">(无文字)</span>';
        card.innerHTML =
            '<div class="layout-type">' +
            escapeHtml(el.element_type) +
            " · " +
            conf +
            "%</div><div class='layout-text'>" +
            text +
            "</div>";
        sLayoutList.appendChild(card);
    });

    // Tables
    sTablesList.innerHTML = "";
    tables.forEach(function (t, i) {
        var card = document.createElement("div");
        card.className = "table-card";
        var title =
            "<h4>表格 " +
            (i + 1) +
            " · " +
            escapeHtml(t.table_type) +
            "</h4>";
        card.innerHTML =
            title +
            (t.html_structure
                ? t.html_structure
                : "<p style='color:var(--color-text-muted)'>无表格结构</p>");
        sTablesList.appendChild(card);
    });

    // Formulas
    sFormulasList.innerHTML = "";
    if (formulas.length === 0) {
        sFormulasList.innerHTML =
            '<p style="color:var(--color-text-muted);text-align:center;padding:24px;">未识别到公式</p>';
    }
    formulas.forEach(function (f, i) {
        var card = document.createElement("div");
        card.className = "formula-card";
        var conf = (f.confidence * 100).toFixed(1);
        card.innerHTML =
            '<div class="formula-index">#' +
            (i + 1) +
            ' · ' + conf + '%</div>' +
            '<pre class="formula-latex">' +
            escapeHtml(f.latex) +
            '</pre>';
        sFormulasList.appendChild(card);
    });

    // Charts
    sChartsList.innerHTML = "";
    if (charts.length === 0) {
        sChartsList.innerHTML =
            '<p style="color:var(--color-text-muted);text-align:center;padding:24px;">未识别到图表</p>';
    }
    charts.forEach(function (c, i) {
        var card = document.createElement("div");
        card.className = "chart-card";
        var conf = (c.confidence * 100).toFixed(1);
        var text = c.text ? '<div class="chart-text">' + escapeHtml(c.text) + '</div>' : '';
        card.innerHTML =
            '<div class="chart-type">' +
            escapeHtml(c.element_type) +
            " · " + conf + "%</div>" +
            text;
        sChartsList.appendChild(card);
    });

    // Markdown
    sMdCode.textContent = currentStructure.markdown || "无 Markdown 输出";

    // HTML preview
    sHtmlPreview.srcdoc = currentStructure.html || "<p>无 HTML 输出</p>";
}

function showSLoading() {
    sLoading.style.display = "block";
}
function hideSLoading() {
    sLoading.style.display = "none";
}
function showSResult() {
    sResult.style.display = "block";
}
function hideSResult() {
    sResult.style.display = "none";
}
function showSError(msg) {
    sErrorMsg.textContent = msg;
    sError.style.display = "block";
}
function hideSError() {
    sError.style.display = "none";
}
