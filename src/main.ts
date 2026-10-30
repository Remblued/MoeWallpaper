/**
 * 前端入口：图源选择 → 搜索 → 缩略图网格 → 详情页（大图 + 设壁纸/下载）。
 *
 * 交互铁律：点缩略图 → 打开详情页看大图，绝不直接换桌面。
 * 三种状态（加载中 / 空 / 出错）都有可见文案，不会白屏。
 */

import { listSources, fetchImages, setWallpaper, saveImage } from "./lib/api";
import type { ImageDto, WallpaperSource } from "./lib/api";
import { USE_MOCK, mockImages, mockDelay } from "./lib/mock";
import { openUrl } from "@tauri-apps/plugin-opener";

// ── DOM 引用 ────────────────────────────────────────────────
const sourceSelect = document.getElementById("source-select") as HTMLSelectElement;
const searchInput  = document.getElementById("search-input") as HTMLInputElement;
const statusEl     = document.getElementById("status") as HTMLParagraphElement;
const gridEl       = document.getElementById("grid") as HTMLDivElement;
const toastEl      = document.getElementById("toast") as HTMLDivElement;
const prevBtn      = document.getElementById("prev-page") as HTMLButtonElement;
const nextBtn      = document.getElementById("next-page") as HTMLButtonElement;
const pageLabel    = document.getElementById("page-label") as HTMLSpanElement;
const resSelect    = document.getElementById("res-select") as HTMLSelectElement;
const themeBtn     = document.getElementById("theme-toggle") as HTMLButtonElement;

// ── 状态 ────────────────────────────────────────────────────
let currentSourceId = "";
let currentQuery = "";               // 当前搜索词，空串 = 浏览模式
let loadSeq = 0;                     // 请求序号，过期响应直接丢弃
let lastImages: ImageDto[] = [];     // 最近一次渲染的图片列表，供点击处理器读取
let allSources: WallpaperSource[] = []; // 缓存图源列表，用于判断 search 能力
let currentPage = 1;                 // 当前页码，从 1 开始
let hasMorePages = false;            // 上一页拿满了才认为有下一页

/**
 * 分辨率档位 → 最低像素总数。
 *
 * 为什么比像素总数而不是"宽 ≥ x 且 高 ≥ y"：竖屏壁纸（1080×1920）和横屏
 * （1920×1080）像素数一样，都该算 1080p。逐边比较会把竖图全判掉。
 */
const RES_TIERS: Record<string, number> = {
  any: 0,
  fhd: 1920 * 1080,
  qhd: 2560 * 1440,
  uhd: 3840 * 2160,
};
let minPixels = 0;                   // 0 = 不筛选

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

// ── 主题 ────────────────────────────────────────────────────

type Theme = "light" | "dark";

const THEME_KEY = "moewallpaper.theme";

/** 读写 localStorage 都包一层：存储被禁时不该让整个界面崩掉。 */
function loadStoredTheme(): Theme | null {
  try {
    const value = localStorage.getItem(THEME_KEY);
    return value === "light" || value === "dark" ? value : null;
  } catch {
    return null;
  }
}

function storeTheme(theme: Theme) {
  try {
    localStorage.setItem(THEME_KEY, theme);
  } catch {
    // 存不下就算了，本次会话内仍然生效
  }
}

function systemTheme(): Theme {
  return window.matchMedia?.("(prefers-color-scheme: dark)").matches ? "dark" : "light";
}

function applyTheme(theme: Theme) {
  document.documentElement.dataset.theme = theme;
  // 显示的是"点下去会切到哪"，不是当前状态。反着放用户每次都要先点错一次。
  themeBtn.textContent = theme === "dark" ? "☀️" : "🌙";
  const label = theme === "dark" ? "切换到浅色主题" : "切换到深色主题";
  themeBtn.title = label;
  themeBtn.setAttribute("aria-label", label);
}

function initTheme() {
  // 没有存过就跟随系统一次，然后固定下来——不在运行中跟着系统变，
  // 否则用户手动选完，系统一变又被覆盖，会以为按钮坏了。
  applyTheme(loadStoredTheme() ?? systemTheme());

  themeBtn.onclick = () => {
    const next: Theme = document.documentElement.dataset.theme === "dark" ? "light" : "dark";
    applyTheme(next);
    storeTheme(next);
  };
}

// ── 详情页覆盖层 ────────────────────────────────────────────

/**
 * 动态创建详情页 DOM（只建一次）。
 * 结构：overlay > backdrop + content > img + actions + close-btn + error-msg
 */
function createDetailOverlay(): HTMLElement {
  const overlay = document.createElement("div");
  overlay.id = "detail-overlay";
  overlay.className = "detail-overlay";
  overlay.hidden = true;
  overlay.innerHTML = `
    <div class="detail-backdrop" data-action="close"></div>
    <div class="detail-content" role="dialog" aria-modal="true" aria-label="图片详情">
      <button class="detail-close" data-action="close" aria-label="关闭" title="关闭 (Esc)">×</button>
      <div class="detail-img-wrap">
        <img class="detail-img" alt="大图预览" />
        <div class="detail-loading">加载中…</div>
        <div class="detail-error" hidden>图片加载失败</div>
      </div>
      <div class="detail-actions">
        <button class="detail-btn detail-btn--primary" data-action="set-wallpaper">设为壁纸</button>
        <button class="detail-btn" data-action="download">下载</button>
        <a class="detail-btn detail-btn--link" data-action="open-page" hidden target="_blank" rel="noopener">打开原网页</a>
      </div>
    </div>`;
  document.body.appendChild(overlay);
  return overlay;
}

let detailOverlay: HTMLElement;
let detailImg: HTMLImageElement;
let detailLoading: HTMLDivElement;
let detailError: HTMLDivElement;
let detailBtnSet: HTMLButtonElement;
let detailBtnDownload: HTMLButtonElement;
let detailBtnPage: HTMLAnchorElement;
let currentDetailImage: ImageDto | null = null;

function initDetailOverlay() {
  detailOverlay = createDetailOverlay();
  detailImg = detailOverlay.querySelector(".detail-img") as HTMLImageElement;
  detailLoading = detailOverlay.querySelector(".detail-loading") as HTMLDivElement;
  detailError = detailOverlay.querySelector(".detail-error") as HTMLDivElement;
  detailBtnSet = detailOverlay.querySelector('[data-action="set-wallpaper"]') as HTMLButtonElement;
  detailBtnDownload = detailOverlay.querySelector('[data-action="download"]') as HTMLButtonElement;
  detailBtnPage = detailOverlay.querySelector('[data-action="open-page"]') as HTMLAnchorElement;

  // 事件委托
  detailOverlay.addEventListener("click", async (e) => {
    const target = e.target as HTMLElement;
    const action = target.dataset.action;

    if (action === "close") {
      closeDetail();
      return;
    }

    if (action === "set-wallpaper" && currentDetailImage) {
      await handleSetWallpaper();
      return;
    }

    if (action === "download" && currentDetailImage) {
      await handleDownload();
      return;
    }
  });

  // open-page 是 <a> 标签，用单独事件处理
  detailBtnPage.addEventListener("click", (e) => {
    if (currentDetailImage?.pageUrl) {
      e.preventDefault();
      openUrl(currentDetailImage.pageUrl).catch((err: unknown) => {
        showToast(`❌ 打开失败：${String(err)}`, true);
      });
    }
  });

  // Esc 关闭
  document.addEventListener("keydown", (e) => {
    if (e.key === "Escape" && !detailOverlay.hidden) {
      closeDetail();
    }
  });
}

function openDetail(img: ImageDto) {
  currentDetailImage = img;

  // 重置状态
  detailImg.src = "";
  detailImg.hidden = true;
  detailLoading.hidden = false;
  detailLoading.textContent = "加载中…";
  detailError.hidden = true;
  detailBtnSet.disabled = false;
  detailBtnSet.textContent = "设为壁纸";
  detailBtnDownload.disabled = false;
  detailBtnDownload.textContent = "下载";

  // 原网页链接
  if (img.pageUrl) {
    detailBtnPage.href = img.pageUrl;
    detailBtnPage.hidden = false;
  } else {
    detailBtnPage.hidden = true;
  }

  // 加载大图
  detailImg.onload = () => {
    detailLoading.hidden = true;
    detailImg.hidden = false;
  };
  detailImg.onerror = () => {
    detailLoading.hidden = true;
    detailError.hidden = false;
    detailError.textContent = "图片加载失败，请检查网络后重试";
  };
  detailImg.src = img.imageUrl;

  detailOverlay.hidden = false;
  document.body.classList.add("detail-open");

  // 焦点管理：聚焦到关闭按钮
  requestAnimationFrame(() => {
    (detailOverlay.querySelector(".detail-close") as HTMLElement)?.focus();
  });
}

function closeDetail() {
  detailOverlay.hidden = true;
  document.body.classList.remove("detail-open");
  currentDetailImage = null;
  detailImg.src = ""; // 释放内存
}

async function handleSetWallpaper() {
  if (!currentDetailImage || detailBtnSet.disabled) return;

  detailBtnSet.disabled = true;
  detailBtnSet.textContent = "设置中…";

  try {
    if (USE_MOCK) {
      await mockDelay(800);
      showToast("✅ 已设为壁纸（mock）");
    } else {
      await setWallpaper(currentDetailImage.imageUrl);
      showToast("✅ 壁纸已更换");
    }
  } catch (err) {
    showToast(`❌ 设置失败：${String(err)}`, true);
  } finally {
    // 成功也要恢复：不恢复的话按钮会永远停在"设置中…"的禁用态，
    // 用户想再设一次只能先关掉详情页再打开，看起来像卡死。
    detailBtnSet.disabled = false;
    detailBtnSet.textContent = "设为壁纸";
  }
}

async function handleDownload() {
  if (!currentDetailImage || detailBtnDownload.disabled) return;

  detailBtnDownload.disabled = true;
  detailBtnDownload.textContent = "下载中…";

  try {
    if (USE_MOCK) {
      await mockDelay(600);
      showToast("✅ 已保存到 C:\\Users\\Mock\\Pictures\\MoeWallpaper\\wallpaper.png（mock）");
    } else {
      const savedPath = await saveImage(currentDetailImage.imageUrl);
      showToast(`✅ 已保存到 ${savedPath}`);
    }
  } catch (err) {
    showToast(`❌ 下载失败：${String(err)}`, true);
  } finally {
    detailBtnDownload.disabled = false;
    detailBtnDownload.textContent = "下载";
  }
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

function renderEmpty(isSearch = false) {
  if (isSearch) {
    statusEl.textContent = "没搜到，试试英文关键词或换个图源";
  } else {
    statusEl.textContent = "当前图源没有图片";
  }
  statusEl.className = "status status--empty";
  gridEl.innerHTML = "";
}

function renderError(err: unknown) {
  statusEl.textContent = `加载失败：${String(err)}`;
  statusEl.className = "status status--error";
  gridEl.innerHTML = "";
}

function renderGrid(images: ImageDto[], totalOnPage = images.length) {
  const pageText = currentPage > 1 ? `第 ${currentPage} 页 · ` : "";
  const hidden = totalOnPage - images.length;

  if (hidden > 0 && images.length === 0) {
    // 整页被筛空有两种完全不同的原因，说法必须分开：
    // 图源不报尺寸（如 nekos.best）时筛选对它根本无效，说"没有够大的图"是误导。
    const unknown = lastImages.filter((img) => !img.width || !img.height).length;
    statusEl.textContent =
      unknown === totalOnPage
        ? `${pageText}这个图源不提供图片尺寸，无法按分辨率筛选（本页 ${totalOnPage} 张）`
        : `${pageText}本页没有达到该分辨率的图片（共 ${totalOnPage} 张）`;
  } else {
    statusEl.textContent =
      hidden > 0
        ? `${pageText}共 ${images.length} 张（按分辨率筛掉 ${hidden} 张）`
        : `${pageText}共 ${images.length} 张图片`;
  }
  statusEl.className = "status";

  gridEl.innerHTML = images
    .map((img, idx) => {
      const dims = img.width && img.height ? `${img.width}×${img.height} · ` : "";
      const title = esc(dims + (img.pageUrl ?? img.imageUrl));
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

// ── 分辨率筛选 ──────────────────────────────────────────────

/**
 * 按当前档位过滤一页图片。
 *
 * 尺寸未知的图在设了档位时**一律筛掉**：没有尺寸就是没有证据，
 * 不能因为"看着还行"就放进来——那样用户会以为筛选是坏的。
 * 界面上会显示"筛掉 N 张"，所以用户能看见这件事发生。
 */
function visibleImages(images: ImageDto[]): ImageDto[] {
  if (minPixels === 0) return images;

  return images.filter((img) => {
    if (!img.width || !img.height) return false;
    return img.width * img.height >= minPixels;
  });
}

/** 不重新联网，只用当前这一页的数据重画。 */
function refreshGrid() {
  renderGrid(visibleImages(lastImages), lastImages.length);
}

// ── 事件委托：点缩略图 → 打开详情页 ─────────────────────────

gridEl.onclick = (e) => {
  const btn = (e.target as HTMLElement).closest(".card") as HTMLButtonElement | null;
  if (!btn || btn.classList.contains("card--skeleton")) return;

  const idx = Number(btn.dataset.idx);
  const img = lastImages[idx];
  if (!img) return;

  openDetail(img);
};

// ── 搜索框状态同步 ──────────────────────────────────────────

function updateSearchBoxState() {
  const source = allSources.find((s) => s.id === currentSourceId);
  const supportsSearch = source?.search != null;

  searchInput.disabled = !supportsSearch;
  searchInput.placeholder = supportsSearch
    ? "搜索关键词…"
    : "该图源不支持搜索";

  // 切换图源时清空搜索
  searchInput.value = "";
  currentQuery = "";
}

// ── 翻页 ────────────────────────────────────────────────────

/**
 * 一页至少要有这么多张，才认为后面还有下一页。
 *
 * 后端 `DEFAULT_LIMIT` 是 20，wallhaven 的配置是 24。取 20 当门槛：
 * 拿满 20 张以上就还有下一发的可能；不足 20 张说明已经到底了。
 * 猜一个数的坏处是可能误判，好处是省掉一次必然为空的请求——
 * 而且就算误判，下一页拿到空列表也会被下面的逻辑兜住。
 */
const FULL_PAGE_HINT = 20;

function updatePager() {
  prevBtn.disabled = currentPage <= 1;
  nextBtn.disabled = !hasMorePages;
  pageLabel.textContent = `第 ${currentPage} 页`;
}

// ── 加载图片 ────────────────────────────────────────────────

async function loadImages(sourceId: string, query?: string, page = 1) {
  const seq = ++loadSeq;       // 新请求序号
  currentPage = page;
  renderLoading();
  updatePager();

  try {
    let images: ImageDto[];

    if (USE_MOCK) {
      await mockDelay();
      images = mockImages();
    } else {
      images = await fetchImages(sourceId, page, query || undefined);
    }

    // 过期响应：用户已经切到别的图源了，丢弃
    if (seq !== loadSeq) return;

    if (images.length === 0) {
      hasMorePages = false;
      if (page > 1) {
        // 翻过头了。退回上一页，并把上一页的图放回去——
        // 留一个空白网格会让人以为图源坏了。
        currentPage = page - 1;
        updatePager();
        if (lastImages.length > 0) refreshGrid();
        showToast("没有更多了");
      } else {
        updatePager();
        renderEmpty(!!query);
      }
    } else {
      lastImages = images;
      hasMorePages = images.length >= FULL_PAGE_HINT;
      refreshGrid();
      updatePager();
    }
  } catch (err) {
    if (seq !== loadSeq) return;
    renderError(err);
  }
}

// ── 初始化 ──────────────────────────────────────────────────

async function init() {
  // 主题要在任何渲染之前定下来，否则会先闪一下另一个配色
  initTheme();

  // 初始化详情页覆盖层
  initDetailOverlay();

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

    // Fix-1: 过滤掉 enabled: false 的图源
    sources = sources.filter((s) => s.enabled !== false);
    allSources = sources;

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
      updateSearchBoxState();
      loadImages(currentSourceId, currentQuery || undefined);
    };

    // 搜索框：回车触发
    searchInput.addEventListener("keydown", (e) => {
      if (e.key === "Enter") {
        currentQuery = searchInput.value.trim();
        // 新搜索从第 1 页开始：接着上次的页码会让人以为"这个关键词只有这几张"
        loadImages(currentSourceId, currentQuery || undefined, 1);
      }
    });

    // 翻页
    prevBtn.onclick = () => {
      if (currentPage > 1) {
        loadImages(currentSourceId, currentQuery || undefined, currentPage - 1);
      }
    };
    nextBtn.onclick = () => {
      if (hasMorePages) {
        loadImages(currentSourceId, currentQuery || undefined, currentPage + 1);
      }
    };

    // 分辨率筛选：只重画当前页，不重新联网
    resSelect.onchange = () => {
      minPixels = RES_TIERS[resSelect.value] ?? 0;
      if (lastImages.length > 0) refreshGrid();
    };

    // 初始化搜索框状态
    updateSearchBoxState();

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
