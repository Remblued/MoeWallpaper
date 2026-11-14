/**
 * Tauri IPC 薄封装。
 *
 * 整个前端只有这一个文件出现 `invoke(`，其余地方只调这里的函数。
 * 好处：改命令名 / 加参数 / 换 mock，只动这一处。
 */

import { invoke } from "@tauri-apps/api/core";

/** 搜索能力描述（对应后端 SearchSpec）。 */
export interface SearchSpec {
  url: string;
  english?: boolean;
  underscoreTags?: boolean;
  /** 精确标签搜到 0 条时，后端会不会补前缀再试一次。 */
  wildcardFallback?: boolean;
}

/** 与后端 WallpaperSource 序列化结果对应（camelCase）。 */
export interface WallpaperSource {
  id: string;
  name: string;
  /** 后端默认 true；显式 false 表示"加载但不显示"。 */
  enabled?: boolean;
  homepage?: string | null;
  /** 内容提示（如"尺度较大"）；有值就在图源名旁标出来。 */
  contentNote?: string | null;
  /** 不配 = 该图源不支持搜索。 */
  search?: SearchSpec | null;
  /** 来自用户配置目录（不是随程序打包的内置图源）。 */
  custom?: boolean;
}

/**
 * 一条加载期问题（对应后端 `validate::Problem`）。
 *
 * `severity: "error"` 意味着**那条图源没有被加载**，`"warning"` 是"加载了，
 * 但多半不是你要的"。两者都必须显示出来：用户改完配置看不到效果时，
 * 屏幕上唯一能救他的就是他面前这份逐条说明。
 */
export interface SourceProblem {
  severity: "error" | "warning";
  /** 出问题的文件：`内置图源` / `图源目录` / 具体文件名。 */
  file: string;
  /** 涉及哪个图源。文件级问题（语法、版本号）为 null。 */
  sourceId: string | null;
  message: string;
}

/** `list_sources` 的返回（对应后端 SourcesResult）。 */
export interface SourcesResult {
  /** 内置在前，用户文件按文件名排在后面。 */
  sources: WallpaperSource[];
  problems: SourceProblem[];
  /** 用户图源配置目录的绝对路径。 */
  folder: string;
  /** 目录里有几个 .json（坏文件也算）。 */
  userFiles: number;
}

/** `import_source_file` 的返回（对应后端 ImportOutcome）。 */
export interface ImportOutcome {
  /** 写进目录后的文件名；null = 校验没通过，什么都没写。 */
  written: string | null;
  problems: SourceProblem[];
}

/** 与后端 ImageDto 序列化结果对应（camelCase）。 */
export interface ImageDto {
  imageUrl: string;
  thumbnailUrl: string;
  pageUrl: string | null;
  /** 原图宽高。图源不提供时为空——**未知不等于小**，筛分辨率时不能当成 0。 */
  width?: number | null;
  height?: number | null;
}

/**
 * 一次拉取的结果（对应后端 FetchResult）。
 *
 * 为什么不直接返回 `ImageDto[]`：搜索可能走到"前缀扩展"那条路，
 * 而放宽过的结果**必须**在界面上说清楚。悄悄放宽、让用户以为精确搜到了，
 * 比搜不到更糟。
 */
export interface FetchResult {
  images: ImageDto[];
  /** `null` = 这就是精确结果，没做任何让步。 */
  notice: string | null;
}

/** 返回内置图源 + 用户图源，附带加载期的问题。 */
export async function listSources(): Promise<SourcesResult> {
  return invoke<SourcesResult>("list_sources");
}

/** 在资源管理器里打开用户图源目录。 */
export async function openSourcesDir(): Promise<string> {
  return invoke<string>("open_sources_dir");
}

/** 生成示例配置（已存在则不覆盖）。返回一句能直接显示的话。 */
export async function createExampleSource(): Promise<string> {
  return invoke<string>("create_example_source");
}

/** 导入一份配置：后端先校验，通过了才写盘。 */
export async function importSourceFile(
  fileName: string,
  content: string,
): Promise<ImportOutcome> {
  return invoke<ImportOutcome>("import_source_file", { fileName, content });
}

/** 从指定图源拉一页图片。page 从 1 开始。query 不传 = 浏览，传了 = 搜索。 */
export async function fetchImages(
  sourceId: string,
  page: number,
  query?: string,
): Promise<FetchResult> {
  return invoke<FetchResult>("fetch_images", {
    sourceId,
    page,
    query,
  });
}

/** 把一张远程图片设为桌面壁纸。 */
export async function setWallpaper(imageUrl: string): Promise<void> {
  await invoke("set_wallpaper", { imageUrl });
}

/** 下载图片到用户的「图片」目录，返回绝对路径。 */
export async function saveImage(imageUrl: string): Promise<string> {
  return invoke<string>("save_image", { imageUrl });
}
