use crate::fs::{naming, paths};
use crate::sources::dto::{FetchResult, ImageDto};
use crate::sources::fetch;
use crate::sources::loader;
use crate::sources::model::WallpaperSource;
use crate::wallpaper::{current, download, save, set};

/// 返回内置图源列表，供前端渲染。
///
/// 错误只可能是"内置配置解析失败"——那是打包问题，用户看到这个错误就该报 issue。
#[tauri::command]
pub fn list_sources() -> Result<Vec<WallpaperSource>, String> {
    let file = loader::load_builtin()?;
    Ok(file.sources)
}

/// 从某个图源拉一页图片。
///
/// 这里只是门面：找图源 → 调实现 → 换成前端能吃的形状。
/// 真正的网络请求在 `sources::fetch`，这里不做任何 HTTP。
///
/// `query` 不传或传空 = 浏览；传了关键词且该图源支持搜索 = 搜索。
#[tauri::command]
pub async fn fetch_images(
    source_id: String,
    page: u32,
    query: Option<String>,
) -> Result<FetchResult, String> {
    let file = loader::load_builtin()?;

    let source = file
        .sources
        .into_iter()
        .find(|s| s.id == source_id)
        .ok_or_else(|| format!("找不到图源 {source_id:?}"))?;

    let page = fetch::fetch_source_images(&source, page, query.as_deref()).await?;

    Ok(FetchResult {
        images: page.images.into_iter().map(ImageDto::from).collect(),
        notice: page.notice,
    })
}

/// 把一张远程图片设为桌面壁纸。
///
/// 三步：下载进缓存 → 复制到固定的当前壁纸路径 → 交给 Win32。
/// 前端只管传 URL，不需要知道缓存路径长什么样。
#[tauri::command]
pub async fn set_wallpaper(image_url: String) -> Result<(), String> {
    let cached = download::ensure_cached(&image_url).await?;
    let current = current::publish(&cached)?;
    set::set_wallpaper(&current)
}

/// 把一张远程图片存到用户的「图片」目录，返回保存后的完整路径。
///
/// 先走缓存：如果这张图刚设过壁纸，保存就是瞬间完成的，不会重新下载。
#[tauri::command]
pub async fn save_image(image_url: String) -> Result<String, String> {
    let cached = download::ensure_cached(&image_url).await?;
    let dir = paths::pictures_dir()?;
    let stem = naming::friendly_stem(&image_url);

    let dest = save::save_unique(&cached, &dir, &stem)?;

    Ok(dest.to_string_lossy().into_owned())
}
