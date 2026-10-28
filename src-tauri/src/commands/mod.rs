use crate::sources::loader;
use crate::sources::model::WallpaperSource;

/// 返回内置图源列表，供前端渲染。
///
/// 错误只可能是"内置配置解析失败"——那是打包问题，用户看到这个错误就该报 issue。
#[tauri::command]
pub fn list_sources() -> Result<Vec<WallpaperSource>, String> {
    let file = loader::load_builtin()?;
    Ok(file.sources)
}
