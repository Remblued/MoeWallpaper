/**
 * Mock 数据层。
 *
 * USE_MOCK = true  → 前端用假数据，不碰后端
 * USE_MOCK = false → 走真实 IPC
 *
 * 切换只需改这一个常量。
 */

import type { ImageDto } from "./api";

/**
 * ★ 开关：true = mock，false = 真实后端 ★
 *
 * **发布前必须是 false。** 打包出去的正式版本带假数据，用户装完打开看到的是
 * placehold.co 的占位图——那是个发布事故，不是功能。
 */
export const USE_MOCK = false;

const COLORS = [
  "e8a0bf", "a0c4e8", "bfe8a0", "e8d4a0", "c4a0e8",
  "a0e8d4", "e8a0a0", "a0a0e8", "d4e8a0", "e8a0d4",
  "a0e8a0", "e8c4a0",
];

/**
 * 生成本地 data-URI SVG 占位图。
 * 断网也能用，不依赖任何外部服务。
 */
function placeholderUrl(idx: number, size: string): string {
  const [w, h] = size.split("x").map(Number);
  const bg = COLORS[idx % COLORS.length];
  const label = String(idx + 1);
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="${w}" height="${h}" viewBox="0 0 ${w} ${h}">` +
    `<rect width="100%" height="100%" fill="#${bg}"/>` +
    `<text x="50%" y="50%" dominant-baseline="central" text-anchor="middle" ` +
    `font-family="sans-serif" font-size="${Math.round(h / 3)}" fill="#fff">${label}</text></svg>`;
  return `data:image/svg+xml,${encodeURIComponent(svg)}`;
}

/** 24 条假 ImageDto，覆盖"有缩略图 / 无 pageUrl"等组合。 */
export function mockImages(count = 24): ImageDto[] {
  return Array.from({ length: count }, (_, i) => ({
    imageUrl: placeholderUrl(i, "1920x1080"),
    thumbnailUrl: placeholderUrl(i, "400x300"),
    pageUrl: i % 5 === 0 ? null : `https://example.invalid/post/${i + 1}`,
  }));
}

/** 模拟网络延迟，让加载状态可见。 */
export function mockDelay(ms = 600): Promise<void> {
  return new Promise((r) => setTimeout(r, ms));
}
