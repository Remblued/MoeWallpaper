use std::path::PathBuf;

/// 缓存根目录：`%LOCALAPPDATA%\com.remblued.moewallpaper`。
///
/// 用 LOCALAPPDATA 而不是安装目录，因为安装目录可能没有写权限（Program Files）。
pub fn cache_dir() -> Result<PathBuf, String> {
    let base =
        std::env::var("LOCALAPPDATA").map_err(|_| "找不到环境变量 LOCALAPPDATA".to_string())?;
    Ok(PathBuf::from(base).join("com.remblued.moewallpaper"))
}

pub fn thumbnails_dir() -> Result<PathBuf, String> {
    Ok(cache_dir()?.join("thumbnails"))
}

pub fn wallpapers_dir() -> Result<PathBuf, String> {
    Ok(cache_dir()?.join("wallpapers"))
}

/// 用户配置根目录：`%APPDATA%\com.remblued.moewallpaper`。
///
/// **故意不放在 `cache_dir()` 下面。** 缓存是可以随时清空的东西，而这里存的是
/// 用户自己写的图源配置——将来的「清空缓存」按钮只要顺手写成删 `cache_dir()`，
/// 用户的配置就没了。两个生命周期不同的数据不该共用一棵树（这条在
/// `current_file` 上面也写过一次，同一种错不值得犯两遍）。
///
/// 用 `APPDATA`（Roaming）而不是 `LOCALAPPDATA` 也是同一个理由：配置跟着用户走，
/// 缓存不用。
pub fn config_dir() -> Result<PathBuf, String> {
    let base = std::env::var("APPDATA").map_err(|_| "找不到环境变量 APPDATA".to_string())?;
    Ok(PathBuf::from(base).join("com.remblued.moewallpaper"))
}

/// 用户图源配置目录：把 `*.json` 丢进这里，点「重新加载」就会出现在下拉框里。
///
/// 格式见 `docs/CUSTOM-SOURCES.md`；加载逻辑见 `sources::loader`。
pub fn sources_dir() -> Result<PathBuf, String> {
    Ok(config_dir()?.join("sources"))
}

/// 正在使用的壁纸的固定落点。
///
/// 为什么要单独存一份，而不是直接把缓存文件设成壁纸：
/// 缓存和"当前壁纸"是两种**生命周期不同**的数据。缓存可以随时清空，
/// 正在用的壁纸不能。混在一起，将来加"清空缓存"按钮就会把用户的桌面弄挂。
///
/// 扩展名固定写 `.jpg` 是故意的：Windows 的壁纸加载器按**文件内容**判断格式，
/// 不看扩展名。路径永远不变，系统层面没有歧义。
pub fn current_file() -> Result<PathBuf, String> {
    Ok(cache_dir()?.join("current.jpg"))
}

/// 用户下载的图片放在这里：`%USERPROFILE%\Pictures\MoeWallpaper`。
///
/// **这和缓存是两个完全不同的地方。**
/// - 缓存是我们的：随时可以清，丢了用户也不该心疼
/// - 这里是用户的：他自己下的东西，删不删由他决定
///
/// 混在一起，早晚出现"清缓存把用户存了半年的图删了"这种事。
pub fn pictures_dir() -> Result<PathBuf, String> {
    let base =
        std::env::var("USERPROFILE").map_err(|_| "找不到环境变量 USERPROFILE".to_string())?;
    Ok(PathBuf::from(base).join("Pictures").join("MoeWallpaper"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_are_wired() {
        println!("cache      = {:?}", cache_dir().unwrap());
        println!("thumbnails = {:?}", thumbnails_dir().unwrap());
        println!("wallpapers = {:?}", wallpapers_dir().unwrap());
        println!("current    = {:?}", current_file().unwrap());
        assert!(current_file().unwrap().to_string_lossy().ends_with(".jpg"));
    }
}
