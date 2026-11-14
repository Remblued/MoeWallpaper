/**
 * 图源列表显示逻辑的测试。零依赖（`node:test` + Node 的类型剥离）。
 *
 * 跑法：`npm run test:web`
 *
 * 为什么值得测：这三条规则错了以后界面**看起来完全正常**——
 * 关掉的图源还在、自定义图源混进内置组、重新加载后选中一个不存在的 id。
 * 最后一条最难受：用户点完「重新加载」，每翻一页都得到"找不到图源"，
 * 而配置文件本身一点问题都没有。
 */

import assert from "node:assert/strict";
import { test } from "node:test";

import {
  groupSources,
  pickSourceId,
  sourceLabel,
} from "../src/lib/source-groups.ts";
import type { WallpaperSource } from "../src/lib/api.ts";

function src(
  id: string,
  extra: Partial<WallpaperSource> = {},
): WallpaperSource {
  return { id, name: `名字 ${id}`, enabled: true, ...extra };
}

test("内置和自定义分成两组，各自保持原顺序", () => {
  const sources = [
    src("a"),
    src("b"),
    src("mine", { custom: true }),
    src("c"),
    src("theirs", { custom: true }),
  ];

  const groups = groupSources(sources);

  assert.deepEqual(
    groups.map((g) => g.label),
    ["内置图源", "我的图源"],
  );
  assert.deepEqual(
    groups[0]?.sources.map((s) => s.id),
    ["a", "b", "c"],
  );
  assert.deepEqual(
    groups[1]?.sources.map((s) => s.id),
    ["mine", "theirs"],
  );
});

test("enabled: false 的图源一个都不显示（但也没被删掉）", () => {
  const sources = [src("a"), src("off", { enabled: false }), src("b")];

  const groups = groupSources(sources);

  assert.deepEqual(
    groups[0]?.sources.map((s) => s.id),
    ["a", "b"],
  );
  // 输入没有被就地修改：分组是只读视图，不是"清洗后的数据"
  assert.equal(sources.length, 3);
});

test("enabled 缺失等于显示（后端默认 true）", () => {
  const bare: WallpaperSource = { id: "x", name: "x" };
  assert.deepEqual(groupSources([bare]), [
    { label: "内置图源", sources: [bare] },
  ]);
});

test("没有自定义图源时只有一组（界面据此决定要不要画 optgroup）", () => {
  assert.deepEqual(
    groupSources([src("a"), src("b")]).map((g) => g.label),
    ["内置图源"],
  );
  assert.deepEqual(groupSources([]), []);
});

test("全部被关掉时返回空数组，不返回一个空组", () => {
  assert.deepEqual(groupSources([src("a", { enabled: false })]), []);
});

test("重新加载后选中项还在就继续用它", () => {
  const sources = [src("a"), src("b")];
  assert.equal(pickSourceId(sources, "b"), "b");
});

test("选中项被删掉时退回第一个，不留一个悬空的 id", () => {
  const sources = [src("a"), src("b")];
  assert.equal(pickSourceId(sources, "deleted"), "a");
});

test("选中项被 enabled:false 关掉时也要退回，不能选一个看不见的", () => {
  const sources = [src("a"), src("b", { enabled: false })];
  assert.equal(pickSourceId(sources, "b"), "a");
});

test("一个图源都没有时返回空串（调用方据此显示空状态）", () => {
  assert.equal(pickSourceId([], "a"), "");
  assert.equal(pickSourceId([src("a", { enabled: false })], "a"), "");
});

test("内容提示跟在名字后面", () => {
  assert.equal(sourceLabel(src("a", { name: "Lolicon" })), "Lolicon");
  assert.equal(
    sourceLabel(src("a", { name: "Lolicon", contentNote: "尺度较大" })),
    "Lolicon（尺度较大）",
  );
});
