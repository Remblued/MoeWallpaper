use std::fs;
use std::path::Path;

/// 创建目录（含父目录）。已存在时视为成功——幂等。
pub fn ensure_dir(path: &Path) -> Result<(), String> {
    fs::create_dir_all(path).map_err(|e| format!("创建目录失败 {:?} : {}", path, e))
}

pub fn ensure_base_layout() -> Result<(), String> {
    use crate::fs::paths::*;

    ensure_dir(&cache_dir()?)?;
    ensure_dir(&thumbnails_dir()?)?;
    ensure_dir(&wallpapers_dir()?)?;

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
    }
}
