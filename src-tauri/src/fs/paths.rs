use std::path::PathBuf;

//%LOCALAPPDATA%\com.remblued.moewallpaper
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

//目前使用的壁纸
pub fn current_file() -> Result<PathBuf, String> {
    Ok(cache_dir()?.join("current.jpg"))
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
