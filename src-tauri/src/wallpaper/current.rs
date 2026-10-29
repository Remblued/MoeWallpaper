//! 维护"当前壁纸"那个固定路径。

use std::path::{Path, PathBuf};

use crate::fs::{files, paths};

/// 把图片复制到固定的当前壁纸位置，返回该路径。
///
/// 复制而不是直接设缓存文件：见 `fs::paths::current_file` 的说明。
/// 用 `write_atomic` 而不是 `fs::copy`，是为了"要么是旧图，要么是新图，
/// 不会出现写了一半的文件"——系统随时可能来读这个路径。
pub fn publish(src: &Path) -> Result<PathBuf, String> {
    let dest = paths::current_file()?;
    publish_to(src, &dest)?;
    Ok(dest)
}

/// 实际干活的版本，目标路径由调用方给，方便测试。
fn publish_to(src: &Path, dest: &Path) -> Result<(), String> {
    let bytes = std::fs::read(src).map_err(|e| format!("读取图片 {:?} 失败: {}", src, e))?;

    files::write_atomic(dest, &bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 每个测试用自己的目录。
    ///
    /// 只用进程 ID 是不够的：`cargo test` 默认**多线程并行**跑测试，
    /// 同一个进程里的两个测试会拿到同一个目录，然后互相删对方的文件。
    /// 症状就是"单独跑能过，一起跑就挂"——最容易被误判成随机失败的那种 flaky。
    fn scratch_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("moe-wp-current-{}-{tag}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn publish_copies_bytes_to_dest() {
        let dir = scratch_dir("copy");
        let src = dir.join("cached.png");
        let dest = dir.join("current.jpg");
        std::fs::write(&src, b"image-bytes").unwrap();

        publish_to(&src, &dest).unwrap();

        assert_eq!(std::fs::read(&dest).unwrap(), b"image-bytes");
        assert!(!dir.join("current.tmp").exists()); // 临时文件必须被消费掉

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn publish_reports_missing_source() {
        let dir = scratch_dir("missing");
        let src = dir.join("nope.png");
        let dest = dir.join("current.jpg");

        let result = publish_to(&src, &dest);

        assert!(result.is_err());
        assert!(!dest.exists(), "源文件读不到时不该产生目标文件");

        std::fs::remove_dir_all(&dir).unwrap();
    }
}
