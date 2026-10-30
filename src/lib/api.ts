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
}

/** 与后端 WallpaperSource 序列化结果对应（camelCase）。 */
export interface WallpaperSource {
  id: string;
  name: string;
  enabled?: boolean;
  homepage?: string | null;
  /** 不配 = 该图源不支持搜索。 */
  search?: SearchSpec | null;
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

/** 返回内置图源列表。 */
export async function listSources(): Promise<WallpaperSource[]> {
  return invoke<WallpaperSource[]>("list_sources");
}

/** 从指定图源拉一页图片。page 从 1 开始。query 不传 = 浏览，传了 = 搜索。 */
export async function fetchImages(
  sourceId: string,
  page: number,
  query?: string,
): Promise<ImageDto[]> {
  return invoke<ImageDto[]>("fetch_images", {
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
