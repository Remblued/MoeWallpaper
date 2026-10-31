/**
 * 尺寸 / 比例的筛选逻辑：纯函数，不碰 DOM、不联网。
 *
 * 为什么要单独一个文件：这些规则最容易写错，而它们错了以后界面上的表现
 * 全都是"看起来正常"——筛选筛掉了几张、下标错位、未知尺寸被当成很小。
 * 抽成纯函数以后 `npm run test:web` 能直接跑（tests/filters.test.ts）。
 */

import type { ImageDto } from "./api";

/**
 * 分辨率档位 → 最低像素总数。
 *
 * 为什么比像素总数而不是"宽 ≥ x 且 高 ≥ y"：竖屏壁纸（1080×1920）和横屏
 * （1920×1080）像素数一样，都该算 1080p。逐边比较会把竖图全判掉。
 */
export const RES_TIERS: Record<string, number> = {
  any: 0,
  fhd: 1920 * 1080,
  qhd: 2560 * 1440,
  uhd: 3840 * 2160,
};

/** `<select>` 的值是字符串，认不出来的一律当"不限"。 */
export function parseResTier(value: string): number {
  return RES_TIERS[value] ?? 0;
}

/**
 * 比例档位。判据是"宽 / 高"，按**屏幕形状**分三档，不按品牌分。
 *
 * | 档位 | 判据（宽/高） | 覆盖的常见分辨率 |
 * | --- | --- | --- |
 * | 显示器 | ≥ 1.6 | 1920×1080(1.78) · 1920×1200(1.60) · 3440×1440(2.39) |
 * | 平板 | 1.15 ~ 1.6，或 0.62 ~ 0.87 | 2048×1536(1.33) · 3000×2000(1.50) · 1640×2360(0.69) |
 * | 手机 | < 0.62 | 1080×1920(0.56) · 1080×2400(0.45) · 1440×3200(0.45) |
 *
 * 为什么用区间而不是"精确等于 16:9"：真实的壁纸差几个像素太正常了
 * （1920×1200 是 16:10，2560×1600 也是），精确相等会把它们全判错。
 *
 * 区间之间**故意留缝**：0.87 ~ 1.15（接近正方形）不是任何一种屏幕，
 * 硬塞进哪一档都会让用户看到"方形图出现在手机壁纸里"。
 */
export type Ratio = "any" | "desktop" | "tablet" | "phone";

/** 当前筛选状态。两个条件可以叠加（"4K 横屏"是合法组合）。 */
export interface FilterState {
  /** 最低像素总数；0 = 不限。 */
  minPixels: number;
  ratio: Ratio;
}

export const NO_FILTER: FilterState = { minPixels: 0, ratio: "any" };

/** 是否有任何筛选生效。 */
export function isFiltering(filters: FilterState): boolean {
  return filters.minPixels > 0 || filters.ratio !== "any";
}

/** 把 `<select>` 的值收窄成联合类型；不认识的一律当"不筛选"。 */
export function parseRatio(value: string): Ratio {
  return value === "desktop" || value === "tablet" || value === "phone" ? value : "any";
}

/** 宽高比是否落在该档位里。 */
export function ratioMatches(ratio: Ratio, width: number, height: number): boolean {
  const r = width / height;

  switch (ratio) {
    case "desktop":
      return r >= 1.6;
    case "tablet":
      return (r >= 1.15 && r < 1.6) || (r >= 0.62 && r < 0.87);
    case "phone":
      return r < 0.62;
    default:
      return true;
  }
}

/**
 * 这一张要不要显示。
 *
 * 尺寸未知的图在**设了任何筛选**时一律筛掉：没有尺寸就是没有证据，
 * 不能因为"看着还行"就放进来——那样用户会以为筛选是坏的。
 * 界面上会显示"筛掉 N 张"，所以用户能看见这件事发生。
 */
export function matchFilters(filters: FilterState, img: ImageDto): boolean {
  if (!img.width || !img.height) return !isFiltering(filters);

  if (filters.minPixels > 0 && img.width * img.height < filters.minPixels) return false;
  return ratioMatches(filters.ratio, img.width, img.height);
}

/** 网格里的一格：图 + 它在原始数组里的下标。 */
export interface GridEntry {
  img: ImageDto;
  index: number;
}

/**
 * 把一页图按筛选条件切成"要画的格子"，**每格都带上它在原数组里的下标**。
 *
 * 下标这一步出过错：卡片上写筛选后的序号，点击时拿它去查完整的数组，
 * 于是"筛掉几张 → 后面每张都错位"，点开的大图是隔壁那张。
 * 这个 bug 只在开了筛选、且筛掉了不止一张时才出现——所以它躲过了手测，
 * 是用户报上来的。修法就是让**同一个下标**同时用于渲染和点击。
 */
export function gridEntries(images: ImageDto[], filters: FilterState): GridEntry[] {
  const entries: GridEntry[] = [];

  images.forEach((img, index) => {
    if (matchFilters(filters, img)) entries.push({ img, index });
  });

  return entries;
}
