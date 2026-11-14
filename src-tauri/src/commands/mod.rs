use serde::Serialize;
use tauri_plugin_opener::OpenerExt;

use crate::fs::{naming, paths};
use crate::sources::dto::{FetchResult, ImageDto};
use crate::sources::fetch;
use crate::sources::loader;
use crate::sources::model::WallpaperSource;
use crate::sources::validate::{self, Problem};
use crate::wallpaper::{current, download, save, set};

/// 前端拿到的图源清单。
///
/// 比 v0.1.x 多两样东西：`problems`（加载期校验的问题，**空 = 一切正常**）
/// 和 `folder`（用户配置目录，界面要显示它、还要能打开它）。
///
/// 为什么不把 `problems` 塞进日志了事：用户改了配置却"看不到效果"时，
/// 唯一能救他的就是屏幕上逐条写着"哪个文件、哪个字段、怎么改"。
/// 控制台他打不开，日志他不会找。
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourcesResult {
    pub sources: Vec<WallpaperSource>,
    pub problems: Vec<Problem>,
    /// 用户图源配置目录的绝对路径。
    pub folder: String,
    /// 目录里有几个 `.json`（坏文件也算：它确实在那里）。
    pub user_files: usize,
}

impl SourcesResult {
    fn from(loaded: loader::Loaded) -> Self {
        Self {
            sources: loaded.sources,
            problems: loaded.problems,
            folder: loaded.folder.to_string_lossy().into_owned(),
            user_files: loaded.user_files,
        }
    }
}

/// 加载图源目录，目录不在就先建出来。
///
/// 建目录这件事放在这里而不是只放在启动时：用户可能中途把文件夹删了，
/// 那时「打开文件夹」应该给他一个空文件夹，而不是一句"路径不存在"。
fn load() -> Result<SourcesResult, String> {
    let dir = paths::sources_dir()?;
    let _ = crate::fs::dirs::ensure_dir(&dir);

    Ok(SourcesResult::from(loader::load_all(&dir)))
}

/// 返回内置图源 + 用户图源，附带加载期的问题。
///
/// 唯一的错误是"找不到 APPDATA"——那是环境问题，不是配置问题。
#[tauri::command]
pub fn list_sources() -> Result<SourcesResult, String> {
    load()
}

/// 在资源管理器里打开用户图源目录。
#[tauri::command]
pub fn open_sources_dir(app: tauri::AppHandle) -> Result<String, String> {
    let dir = paths::sources_dir()?;
    crate::fs::dirs::ensure_dir(&dir)?;

    app.opener()
        .open_path(dir.to_string_lossy().into_owned(), None::<&str>)
        .map_err(|e| format!("打不开 {}：{e}", dir.display()))?;

    Ok(dir.to_string_lossy().into_owned())
}

/// 生成示例配置，返回一句能直接显示给用户的话。
///
/// **已存在就不覆盖**：用户很可能已经照它改成自己的了。
#[tauri::command]
pub fn create_example_source() -> Result<String, String> {
    let dir = paths::sources_dir()?;
    let (path, created) = loader::create_example(&dir)?;

    Ok(if created {
        format!(
            "已生成 {}，照着改就行（文件里的 _comment 是说明，不算配置）",
            path.file_name().unwrap_or_default().to_string_lossy()
        )
    } else {
        format!(
            "{} 已经存在，没有覆盖它——删掉它再点一次才会重新生成",
            path.file_name().unwrap_or_default().to_string_lossy()
        )
    })
}

/// 导入一份配置：**先校验，通过了才写盘**。
///
/// 顺序很重要。反过来（先写再校验）会让用户目录里堆一串坏文件，
/// 而坏文件下一次加载会变成一堆红字——用户刚点完导入就看到满屏报错，
/// 会以为是他自己的问题。
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportOutcome {
    /// 写到磁盘上的文件名；`None` = 校验没通过，什么都没写。
    pub written: Option<String>,
    pub problems: Vec<Problem>,
}

#[tauri::command]
pub fn import_source_file(file_name: String, content: String) -> Result<ImportOutcome, String> {
    let checked = validate::check_file_text(&content, &file_name);

    // 有任何 error 就不写。警告照写，但要把话带到界面上。
    if checked.parsed.is_none() || checked.problems.iter().any(|p| p.is_error()) {
        return Ok(ImportOutcome {
            written: None,
            problems: checked.problems,
        });
    }

    let dir = paths::sources_dir()?;
    let path = loader::write_user_file(&dir, &file_name, &content)?;

    Ok(ImportOutcome {
        written: Some(
            path.file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned(),
        ),
        problems: checked.problems,
    })
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
    let loaded = loader::load_all(&paths::sources_dir()?);
    let total = loaded.sources.len();

    // 找不到时尽量说清"为什么找不到"：刚被校验拒掉的图源要给出原因，
    // 而不是一句"找不到图源"，让用户对着一个明明写了的 id 反复改。
    let rejected = loaded
        .problems
        .iter()
        .find(|p| p.is_error() && p.source_id.as_deref() == Some(source_id.as_str()))
        .map(|p| p.message.clone());

    let source = loaded
        .sources
        .into_iter()
        .find(|s| s.id == source_id)
        .ok_or_else(|| match rejected {
            Some(reason) => format!("图源 {source_id:?} 没有加载：{reason}"),
            None => format!(
                "找不到图源 {source_id:?}（当前加载了 {total} 个；如果你刚改过配置，\
                 请打开「图源」面板点「重新加载」）"
            ),
        })?;

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
