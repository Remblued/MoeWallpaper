/**
 * 前端入口：图源选择 → 缩略图网格 → 一键设壁纸。
 *
 * 三种状态（加载中 / 空 / 出错）都有可见文案，不会白屏。
 */

import { listSources, fetchImages, setWallpaper } from "./lib/api";
import type { ImageDto, WallpaperSource } from "./lib/api";
import { USE_MOCK, mockImages, mockDelay } from "./lib/mock";

// ── DOM 引用 ────────────────────────────────────────────────
const sourceSelect = document.getElementById("source-select") as HTMLSelectElement;
const statusEl     = document.getElementById("status") as HTMLParagraphElement;
const gridEl       = document.getElementById("grid") as HTMLDivElement;
const toastEl      = document.getElementById("toast") as HTMLDivElement;

// ── 状态 ────────────────────────────────────────────────────
let currentSourceId = "";
let settingLock = false;           // 防止重复点击
let loadSeq = 0;                   // 请求序号，过期响应直接丢弃

// ── 工具 ────────────────────────────────────────────────────

/** 转义 HTML 特殊字符，防 XSS。innerHTML 拼接时必须用。 */
function esc(s: string): string {
  return s
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
}

// ── Toast ───────────────────────────────────────────────────
let toastTimer: ReturnType<typeof setTimeout> | null = null;

function showToast(msg: string, isError = false) {
  toastEl.textContent = msg;
  toastEl.classList.toggle("toast--error", isError);
  toastEl.hidden = false;

  if (toastTimer) clearTimeout(toastTimer);
  toastTimer = setTimeout(() => {
    toastEl.hidden = true;
  }, 2500);
}

// ── 渲染函数 ────────────────────────────────────────────────

function renderLoading() {
  statusEl.textContent = "加载中…";
  statusEl.className = "status status--loading";
  // Skeleton：占位卡片，比空白体验好
  gridEl.innerHTML = Array.from({ length: 8 }, () => `
    <div class="card card--skeleton" aria-hidden="true">
      <div class="skeleton-pulse"></div>
    </div>`).join("");
}

function renderEmpty() {
  statusEl.textContent = "当前图源没有图片";
  statusEl.className = "status status--empty";
  gridEl.innerHTML = "";
}

function renderError(err: unknown) {
  statusEl.textContent = `加载失败：${String(err)}`;
  statusEl.className = "status status--error";
  gridEl.innerHTML = "";
}

function renderGrid(images: ImageDto[]) {
  statusEl.textContent = `共 ${images.length} 张图片`;
  statusEl.className = "status";

  gridEl.innerHTML = images
    .map((img, idx) => {
      const title = esc(img.pageUrl ?? img.imageUrl);
      const thumb = esc(img.thumbnailUrl);
      return `
    <button class="card" data-idx="${idx}" title="${title}">
      <img
        src="${thumb}"
        alt="壁纸 ${idx + 1}"
        loading="lazy"
        decoding="async"
      />
    </button>`;
    })
    .join("");

  // 图片加载失败 → 显示占位背景 + 文字
  gridEl.querySelectorAll<HTMLImageElement>(".card img").forEach((img) => {
    img.onerror = () => {
      img.classList.add("img--broken");
      img.alt = "加载失败";
    };
  });
}

// ── 事件委托：点缩略图 → 设壁纸 ─────────────────────────────

gridEl.onclick = async (e) => {
  if (settingLock) return; // 防重复

  const btn = (e.target as HTMLElement).closest(".card") as HTMLButtonElement | null;
  if (!btn || btn.classList.contains("card--skeleton")) return;

  const idx = Number(btn.dataset.idx);
  // images 通过闭包拿不到，改成从 DOM 重建不划算；
  // 这里用一个模块级缓存解决（见下方 lastImages）
  const img = lastImages[idx];
  if (!img) return;

  settingLock = true;
  btn.classList.add("card--setting");
  showToast("设置中…");

  try {
    if (USE_MOCK) {
      await mockDelay(800);
      showToast("✅ 已设为壁纸（mock）");
    } else {
      await setWallpaper(img.imageUrl);
      showToast("✅ 壁纸已更换");
    }
  } catch (err) {
    showToast(`❌ 设置失败：${String(err)}`, true);
  } finally {
    btn.classList.remove("card--setting");
    settingLock = false;
  }
};

/** 最近一次渲染的图片列表，供点击处理器读取。 */
let lastImages: ImageDto[] = [];

// ── 加载图片 ────────────────────────────────────────────────

async function loadImages(sourceId: string) {
  const seq = ++loadSeq;       // 新请求序号
  renderLoading();

  try {
    let images: ImageDto[];

    if (USE_MOCK) {
      await mockDelay();
      images = mockImages();
    } else {
      images = await fetchImages(sourceId, 1);
    }

    // 过期响应：用户已经切到别的图源了，丢弃
    if (seq !== loadSeq) return;

    if (images.length === 0) {
      renderEmpty();
    } else {
      lastImages = images;
      renderGrid(images);
    }
  } catch (err) {
    if (seq !== loadSeq) return;
    renderError(err);
  }
}

// ── 初始化 ──────────────────────────────────────────────────

async function init() {
  try {
    let sources: WallpaperSource[];

    if (USE_MOCK) {
      sources = [
        { id: "mock-1", name: "Mock 图源 A", enabled: true },
        { id: "mock-2", name: "Mock 图源 B", enabled: true },
      ];
    } else {
      sources = await listSources();
    }

    // 填充下拉框（escape 防注入）
    sourceSelect.innerHTML = sources
      .map((s) => `<option value="${esc(s.id)}">${esc(s.name)}</option>`)
      .join("");
    sourceSelect.disabled = false;

    // 选第一个可用图源
    currentSourceId = sources[0]?.id ?? "";
    sourceSelect.value = currentSourceId;

    // 切换图源
    sourceSelect.onchange = () => {
      currentSourceId = sourceSelect.value;
      loadImages(currentSourceId);
    };

    // 首次加载
    if (currentSourceId) {
      loadImages(currentSourceId);
    } else {
      renderEmpty();
    }
  } catch (err) {
    renderError(err);
  }
}

window.addEventListener("DOMContentLoaded", init);
