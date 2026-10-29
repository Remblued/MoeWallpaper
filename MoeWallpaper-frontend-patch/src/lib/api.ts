/**
 * Tauri IPC 薄封装。
 *
 * 整个前端只有这一个文件出现 `invoke(`，其余地方只调这里的函数。
 * 好处：改命令名 / 加参数 / 换 mock，只动这一处。
 */

import { invoke } from "@tauri-apps/api/core";

/** 与后端 WallpaperSource 序列化结果对应（camelCase）。 */
export interface WallpaperSource {
  id: string;
  name: string;
  enabled?: boolean;
  homepage?: string | null;
}

/** 与后端 ImageDto 序列化结果对应（camelCase）。 */
export interface ImageDto {
  imageUrl: string;
  thumbnailUrl: string;
  pageUrl: string | null;
}

/** 返回内置图源列表。 */
export async function listSources(): Promise<WallpaperSource[]> {
  return invoke<WallpaperSource[]>("list_sources");
}

/** 从指定图源拉一页图片。page 从 1 开始。 */
export async function fetchImages(
  sourceId: string,
  page: number,
): Promise<ImageDto[]> {
  return invoke<ImageDto[]>("fetch_images", {
    sourceId,
    page,
  });
}

/** 把一张远程图片设为桌面壁纸。 */
export async function setWallpaper(imageUrl: string): Promise<void> {
  await invoke("set_wallpaper", { imageUrl });
}
