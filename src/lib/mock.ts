/**
 * Mock 数据层。
 *
 * USE_MOCK = true  → 前端用假数据，不碰后端
 * USE_MOCK = false → 走真实 IPC（等 A 的 fetch_images 就绪后切过来）
 *
 * 切换只需改这一个常量。
 */

import type { ImageDto } from "./api";

/** ★ 开关：true = mock，false = 真实后端 ★ */
export const USE_MOCK = true;

const COLORS = [
  "e8a0bf", "a0c4e8", "bfe8a0", "e8d4a0", "c4a0e8",
  "a0e8d4", "e8a0a0", "a0a0e8", "d4e8a0", "e8a0d4",
  "a0e8a0", "e8c4a0",
];

function placeholderUrl(idx: number, size: string): string {
  const bg = COLORS[idx % COLORS.length];
  return `https://placehold.co/${size}/${bg}/ffffff?text=${idx + 1}`;
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
