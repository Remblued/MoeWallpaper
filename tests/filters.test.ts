/**
 * 筛选逻辑的测试。零依赖：Node 自带的 `node:test` + 类型剥离。
 *
 * 跑法：`npm run test:web`（= `node --test tests/`）
 *
 * 为什么这些规则值得测：它们错了以后界面**看起来完全正常**——
 * 筛掉几张、下标错位、未知尺寸被当成很小，全都是"没有报错的那种错"。
 * 下面第一条就是用户报上来的 bug（点缩略图打开的是隔壁那张大图）。
 */

import assert from "node:assert/strict";
import { test } from "node:test";

import {
  NO_FILTER,
  gridEntries,
  isFiltering,
  matchFilters,
  parseRatio,
  parseResTier,
  ratioMatches,
} from "../src/lib/filters.ts";
import type { FilterState } from "../src/lib/filters.ts";
import type { ImageDto } from "../src/lib/api.ts";

const FHD = 1920 * 1080;

/** 造一张只有尺寸有意义的图。 */
function img(width: number | null, height: number | null, tag = ""): ImageDto {
  return {
    imageUrl: `https://example.invalid/${tag || "x"}.jpg`,
    thumbnailUrl: `https://example.invalid/${tag || "x"}-thumb.jpg`,
    pageUrl: null,
    width,
    height,
  };
}

test("gridEntries 保留原始下标——点缩略图只能打开它自己那张", () => {
  // 4 张图里只有第 0 和第 3 张够大（中间两张会被筛掉）
  const images = [
    img(3840, 2160, "a"),
    img(800, 600, "b"),
    img(1024, 768, "c"),
    img(2560, 1440, "d"),
  ];

  const entries = gridEntries(images, { minPixels: FHD, ratio: "any" });

  assert.deepEqual(
    entries.map((e) => e.index),
    [0, 3],
    "下标应该是 lastImages 里的位置，不是筛选后的位置",
  );
  // 关键断言：拿卡片上的下标回原数组取图，必须取到同一张
  for (const entry of entries) {
    assert.equal(images[entry.index]?.imageUrl, entry.img.imageUrl);
  }
});

test("没开筛选时，每一张都在，且下标等于原位置", () => {
  const images = [img(100, 100, "a"), img(null, null, "b"), img(4000, 3000, "c")];

  const entries = gridEntries(images, NO_FILTER);

  assert.deepEqual(
    entries.map((e) => e.index),
    [0, 1, 2],
  );
});

test("尺寸未知的图：不筛选时保留，一旦筛选就筛掉", () => {
  const unknown = img(null, null, "unknown");

  assert.equal(matchFilters(NO_FILTER, unknown), true, "没有证据 ≠ 不合格");

  // 0 也不能当成"很小"以外的东西——这里两个条件都必须真的生效
  assert.equal(matchFilters({ minPixels: 1, ratio: "any" }, unknown), false);
  assert.equal(matchFilters({ minPixels: 0, ratio: "phone" }, unknown), false);
});

test("分辨率按像素总数算，竖屏也能过 1080p", () => {
  const portrait = img(1080, 1920);
  const landscape = img(1920, 1080);
  const small = img(1280, 720);

  assert.equal(matchFilters({ minPixels: FHD, ratio: "any" }, portrait), true);
  assert.equal(matchFilters({ minPixels: FHD, ratio: "any" }, landscape), true);
  assert.equal(matchFilters({ minPixels: FHD, ratio: "any" }, small), false);
});

test("比例分档覆盖真实壁纸分辨率", () => {
  // 显示器
  for (const [w, h] of [
    [1920, 1080],
    [2560, 1440],
    [3840, 2160],
    [1920, 1200], // 16:10
    [3440, 1440], // 21:9
  ]) {
    assert.equal(ratioMatches("desktop", w!, h!), true, `${w}×${h} 应该算显示器`);
  }

  // 平板（横屏与竖屏都算）
  for (const [w, h] of [
    [2048, 1536], // 4:3
    [3000, 2000], // 3:2
    [2736, 1824], // Surface
    [1536, 2048], // 竖着的平板
    [1640, 2360], // iPad 竖屏
  ]) {
    assert.equal(ratioMatches("tablet", w!, h!), true, `${w}×${h} 应该算平板`);
  }

  // 手机
  for (const [w, h] of [
    [1080, 1920],
    [1080, 2400],
    [1170, 2532],
    [1440, 3200],
  ]) {
    assert.equal(ratioMatches("phone", w!, h!), true, `${w}×${h} 应该算手机`);
  }
});

test("比例档位之间不重叠，方形图不属于任何一档", () => {
  const square = [1_000, 1_000] as const;
  const wide = [1_600, 1_000] as const; // 16:10
  const portrait = [1_080, 1_920] as const;

  // 每一张"像屏幕"的图只能落进一档，不能既是平板又是手机
  for (const [w, h] of [wide, portrait]) {
    const hits = (["desktop", "tablet", "phone"] as const).filter((r) =>
      ratioMatches(r, w, h),
    );
    assert.equal(hits.length, 1, `${w}×${h} 只该落进一档，实际 ${hits.join("/")}`);
  }

  // 1:1 谁都不是——硬塞进哪一档都会让用户看到形状不对的图
  assert.equal(ratioMatches("desktop", ...square), false);
  assert.equal(ratioMatches("tablet", ...square), false);
  assert.equal(ratioMatches("phone", ...square), false);
});

test("两个筛选可以叠加：4K 横屏", () => {
  const filters: FilterState = { minPixels: 3840 * 2160, ratio: "desktop" };

  assert.equal(matchFilters(filters, img(3840, 2160)), true);
  assert.equal(matchFilters(filters, img(1920, 1080)), false, "分辨率不够");
  assert.equal(matchFilters(filters, img(2160, 3840)), false, "竖屏不是显示器");
  assert.equal(matchFilters(filters, img(5120, 2880)), true, "更大也算");
});

test("下拉框的值怎么读都不该崩，认不出来就是不限", () => {
  assert.equal(parseResTier("fhd"), FHD);
  assert.equal(parseResTier("nope"), 0);
  assert.equal(parseResTier(""), 0);

  assert.equal(parseRatio("phone"), "phone");
  assert.equal(parseRatio("nope"), "any");
  assert.equal(parseRatio(""), "any");
});

test("isFiltering 只在真的有条件时为真", () => {
  assert.equal(isFiltering(NO_FILTER), false);
  assert.equal(isFiltering({ minPixels: 1, ratio: "any" }), true);
  assert.equal(isFiltering({ minPixels: 0, ratio: "phone" }), true);
});
