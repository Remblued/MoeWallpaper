/**
 * 前端入口：图源选择 → 搜索 → 缩略图网格 → 详情页（大图 + 设壁纸/下载）。
 *
 * 交互铁律：点缩略图 → 打开详情页看大图，绝不直接换桌面。
 * 三种状态（加载中 / 空 / 出错）都有可见文案，不会白屏。
 */

import { listSources, fetchImages, setWallpaper, saveImage } from "./lib/api";
import type { FetchResult, ImageDto, WallpaperSource } from "./lib/api";
import {
  gridEntries,
  parseRatio,
  parseResTier,
  NO_FILTER,
} from "./lib/filters";
import type { FilterState } from "./lib/filters";
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
const ratioSelect  = document.getElementById("ratio-select") as HTMLSelectElement;
const themeBtn     = document.getElementById("theme-toggle") as HTMLButtonElement;
const sourceNoteEl = document.getElementById("source-note") as HTMLParagraphElement;

// ── 状态 ────────────────────────────────────────────────────
let currentSourceId = "";
let currentQuery = "";               // 当前搜索词，空串 = 浏览模式
let loadSeq = 0;                     // 请求序号，过期响应直接丢弃
let lastImages: ImageDto[] = [];     // 最近一次拉到的**完整**一页，供点击处理器按下标取图
let lastNotice: string | null = null;// 后端"为了让结果不为空做了什么"的提示
let allSources: WallpaperSource[] = []; // 缓存图源列表，用于判断 search 能力
let currentPage = 1;                 // 当前页码，从 1 开始
let hasMorePages = false;            // 上一页拿满了才认为有下一页

/** 当前筛选条件。规则在 `lib/filters.ts`，那边有测试。 */
let filter: FilterState = { ...NO_FILTER };

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

type Theme = "light" | "dark" | "glass";

const THEME_KEY = "moewallpaper.theme";

/** 点击顺序。玻璃排在最后：它是锦上添花，不该挡住浅色/深色的切换。 */
const THEMES: Theme[] = ["light", "dark", "glass"];

/**
 * 图标显示的是**当前**主题，不是下一个。
 *
 * 两个主题时显示"目标"还算直观，三个就会让人猜不透点下去会变成什么。
 * 名字放在 title 里，鼠标一悬停就知道现在是什么。
 */
const THEME_META: Record<Theme, { icon: string; name: string }> = {
  light: { icon: "☀️", name: "浅色" },
  dark: { icon: "🌙", name: "深色" },
  glass: { icon: "🫧", name: "玻璃" },
};

function isTheme(value: string | null | undefined): value is Theme {
  return value === "light" || value === "dark" || value === "glass";
}

/** 读写 localStorage 都包一层：存储被禁时不该让整个界面崩掉。 */
function loadStoredTheme(): Theme | null {
  try {
    const value = localStorage.getItem(THEME_KEY);
    return isTheme(value) ? value : null;
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
  const meta = THEME_META[theme];
  themeBtn.textContent = meta.icon;
  const label = `当前：${meta.name}主题，点击切换`;
  themeBtn.title = label;
  themeBtn.setAttribute("aria-label", label);
}

function initTheme() {
  // 没有存过就跟随系统一次，然后固定下来——不在运行中跟着系统变，
  // 否则用户手动选完，系统一变又被覆盖，会以为按钮坏了。
  applyTheme(loadStoredTheme() ?? systemTheme());

  themeBtn.onclick = () => {
    const current = document.documentElement.dataset.theme;
    const index = isTheme(current) ? THEMES.indexOf(current) : 0;
    const next = THEMES[(index + 1) % THEMES.length] ?? "light";
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

// ── 筛选与网格 ──────────────────────────────────────────────

/** 当前生效的筛选条件（中文说法），用于状态栏文案。 */
function activeFilterLabel(): string {
  const parts: string[] = [];
  if (filter.minPixels > 0) parts.push("分辨率");
  if (filter.ratio !== "any") parts.push("比例");
  return parts.join("和");
}

/**
 * 只重画，不联网。
 *
 * 参数一个都不收：**永远从 `lastImages` + 当前 `filter` 出发**。
 * 收"某个列表"当参数的话，调用方就有可能传进来一个和 `lastImages`
 * 不同源的数组，下标再次失配——那正是这次修掉的 bug。
 */
function renderGrid() {
  const entries = gridEntries(lastImages, filter);
  const total = lastImages.length;
  const hidden = total - entries.length;
  const pageText = currentPage > 1 ? `第 ${currentPage} 页 · ` : "";
  const noticeText = lastNotice ? `（${lastNotice}）` : "";

  if (total > 0 && entries.length === 0) {
    // 整页被筛空有两种完全不同的原因，说法必须分开：
    // 图源不报尺寸（如 nekos.best）时筛选对它根本无效，说"没有够大的图"是误导。
    const unknown = lastImages.filter((img) => !img.width || !img.height).length;
    // 比例档还多一种情况：有些图源的列表本身就偏横图（360 全是横屏壁纸，
    // safebooru 的浏览模板还带着 width:>=1920）。这时"换一页"多半还是空，
    // 换图源才是正解，所以要单独说。
    const hint =
      filter.ratio !== "any"
        ? "试试下一页，或换个图源（有些图源的列表以横图为主）"
        : "试试下一页或放宽筛选";

    statusEl.textContent =
      unknown === total
        ? `${pageText}这个图源不提供图片尺寸，无法按${activeFilterLabel()}筛选（本页 ${total} 张）`
        : `${pageText}本页没有符合${activeFilterLabel()}的图片（共 ${total} 张）——${hint}`;
  } else {
    statusEl.textContent =
      hidden > 0
        ? `${pageText}共 ${entries.length} 张（按${activeFilterLabel()}筛掉 ${hidden} 张）${noticeText}`
        : `${pageText}共 ${entries.length} 张图片${noticeText}`;
  }
  statusEl.className = "status";

  gridEl.innerHTML = entries
    .map(({ img, index }, position) => {
      const dims = img.width && img.height ? `${img.width}×${img.height} · ` : "";
      const title = esc(dims + (img.pageUrl ?? img.imageUrl));
      const thumb = esc(img.thumbnailUrl);
      return `
    <button class="card" data-idx="${index}" title="${title}">
      <img
        src="${thumb}"
        alt="壁纸 ${position + 1}"
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

// ── 事件委托：点缩略图 → 打开详情页 ─────────────────────────

gridEl.onclick = (e) => {
  const btn = (e.target as HTMLElement).closest(".card") as HTMLButtonElement | null;
  if (!btn || btn.classList.contains("card--skeleton")) return;

  // data-idx 是 lastImages 里的下标（见 gridEntries 的注释），不是网格里的位置
  const idx = Number(btn.dataset.idx);
  const img = lastImages[idx];
  if (!img) return;

  openDetail(img);
};

// ── 图源相关的界面同步 ──────────────────────────────────────

/**
 * 把界面同步到"当前选中的图源"。
 *
 * 两件事：搜索框能不能用、有没有内容提示。
 *
 * 搜索框为什么是禁用而不是隐藏：**不配 `search` 的图源 = 不支持搜索**，
 * 让它保持可见但禁用、placeholder 写明原因，用户才知道这是图源的边界，
 * 而不是以为程序坏了。
 */
function syncSourceUi() {
  const source = allSources.find((s) => s.id === currentSourceId);
  const supportsSearch = source?.search != null;

  searchInput.disabled = !supportsSearch;
  searchInput.placeholder = supportsSearch
    ? "搜索关键词…"
    : "该图源不支持搜索";

  // 切换图源时清空搜索
  searchInput.value = "";
  currentQuery = "";

  // 内容提示（如"尺度较大"）。图源没声明就整块隐藏，不留空行。
  const note = source?.contentNote;
  if (source && note) {
    sourceNoteEl.textContent = `⚠ ${source.name}：${note}`;
    sourceNoteEl.hidden = false;
  } else {
    sourceNoteEl.textContent = "";
    sourceNoteEl.hidden = true;
  }
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
    let result: FetchResult;

    if (USE_MOCK) {
      await mockDelay();
      result = { images: mockImages(), notice: null };
    } else {
      result = await fetchImages(sourceId, page, query || undefined);
    }

    // 过期响应：用户已经切到别的图源了，丢弃
    if (seq !== loadSeq) return;

    const images = result.images;

    if (images.length === 0) {
      hasMorePages = false;
      lastNotice = null;
      if (page > 1) {
        // 翻过头了。退回上一页，并把上一页的图放回去——
        // 留一个空白网格会让人以为图源坏了。
        currentPage = page - 1;
        updatePager();
        if (lastImages.length > 0) renderGrid();
        showToast("没有更多了");
      } else {
        updatePager();
        // 空结果不需要 notice：后端只在"放宽后拿到了图"时才给提示，
        // 放宽也空的时候它返回的就是精确的 0 条，这里照实说"没搜到"。
        renderEmpty(!!query);
      }
    } else {
      lastImages = images;
      lastNotice = result.notice;
      hasMorePages = images.length >= FULL_PAGE_HINT;
      renderGrid();
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

    // 填充下拉框（escape 防注入）。
    // 有 contentNote 的图源在名字后面标出来——用户在选之前就该知道。
    sourceSelect.innerHTML = sources
      .map((s) => {
        const label = s.contentNote ? `${s.name}（${s.contentNote}）` : s.name;
        return `<option value="${esc(s.id)}">${esc(label)}</option>`;
      })
      .join("");
    sourceSelect.disabled = false;

    // 选第一个可用图源
    currentSourceId = sources[0]?.id ?? "";
    sourceSelect.value = currentSourceId;

    // 切换图源
    sourceSelect.onchange = () => {
      currentSourceId = sourceSelect.value;
      syncSourceUi();
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

    // 筛选：只重画当前页，不重新联网。
    // 两个筛选互不影响，可以叠加（"4K 横屏壁纸"就是一个合法组合）。
    resSelect.onchange = () => {
      filter = { ...filter, minPixels: parseResTier(resSelect.value) };
      if (lastImages.length > 0) renderGrid();
    };
    ratioSelect.onchange = () => {
      filter = { ...filter, ratio: parseRatio(ratioSelect.value) };
      if (lastImages.length > 0) renderGrid();
    };

    // 初始化搜索框 / 内容提示
    syncSourceUi();

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
