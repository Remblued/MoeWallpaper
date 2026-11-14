use std::fs;
use std::path::Path;

/// 创建目录（含父目录）。已存在时视为成功——幂等。
pub fn ensure_dir(path: &Path) -> Result<(), String> {
    fs::create_dir_all(path).map_err(|e| format!("创建目录失败 {:?}: {}", path, e))
}

pub fn ensure_base_layout() -> Result<(), String> {
    use crate::fs::paths::*;

    ensure_dir(&cache_dir()?)?;
    ensure_dir(&thumbnails_dir()?)?;
    ensure_dir(&wallpapers_dir()?)?;

    // 用户图源目录也一起建：界面上「打开文件夹」要在用户还没放过任何文件时
    // 就能打开一个真的存在的文件夹，而不是一句"路径不存在"。
    ensure_dir(&sources_dir()?)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_creates_dirs() {
        ensure_base_layout().unwrap();

        assert!(crate::fs::paths::cache_dir().unwrap().is_dir());
        assert!(crate::fs::paths::thumbnails_dir().unwrap().is_dir());
        assert!(crate::fs::paths::wallpapers_dir().unwrap().is_dir());
        assert!(crate::fs::paths::sources_dir().unwrap().is_dir());
    }
}
