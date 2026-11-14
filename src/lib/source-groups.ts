/**
 * 图源列表的"显示"逻辑：分组、筛选、重新加载后选谁。
 *
 * 抽出来单独放，是因为这三件事都能**错得很安静**：
 * 关掉的图源还出现在下拉框里、自定义图源混进内置那一组、重新加载后选中的
 * 是一个已经不存在的 id（接着每一次拉图都报"找不到图源"）——界面看着都正常。
 * 纯函数才能进 `tests/` 用 `node --test` 跑，见 `tests/sources.test.ts`。
 */

import type { WallpaperSource } from "./api";

export interface SourceGroup {
  label: string;
  sources: WallpaperSource[];
}

/**
 * 只留下要显示的图源，并按"内置 / 自定义"分组。
 *
 * `enabled !== false` 是故意的宽松写法：后端默认就是 true（`model.rs` 里改过），
 * 老版本前端拿不到这个字段时也不该把图源全部藏起来。
 */
export function groupSources(sources: WallpaperSource[]): SourceGroup[] {
  const visible = sources.filter((s) => s.enabled !== false);

  const builtin = visible.filter((s) => s.custom !== true);
  const custom = visible.filter((s) => s.custom === true);

  const groups: SourceGroup[] = [];
  if (builtin.length > 0) groups.push({ label: "内置图源", sources: builtin });
  if (custom.length > 0) groups.push({ label: "我的图源", sources: custom });
  return groups;
}

/** 下拉框里显示的文字：内容提示跟在名字后面（用户选之前就该知道）。 */
export function sourceLabel(source: WallpaperSource): string {
  return source.contentNote ? `${source.name}（${source.contentNote}）` : source.name;
}

/**
 * 重新加载之后该选中谁。
 *
 * 原来那个还在就继续用它（用户点「重新加载」不是为了被换到别的图源），
 * 不在了就退回第一个——**绝不能留一个指向已删配置的选中值**，
 * 否则下一次拉图只会得到"找不到图源"。
 */
export function pickSourceId(sources: WallpaperSource[], previous: string): string {
  const visible = sources.filter((s) => s.enabled !== false);
  if (visible.some((s) => s.id === previous)) return previous;
  return visible[0]?.id ?? "";
}
