//! 把图片存到用户自己的目录（**不是缓存**）。

use std::path::{Path, PathBuf};

use crate::fs::{dirs, files};

/// 把文件复制到 `dir` 下，命名为 `stem.<原扩展名>`，返回最终路径。
///
/// 同名文件已存在时依次尝试 `stem (1).ext`、`stem (2).ext`……
/// **绝不覆盖已有文件**——用户可能已经存过同名文件并改过内容，
/// 应用无权替他决定"覆盖"还是"再存一份"。
///
/// 目标目录不存在时会自动创建（用户第一次点下载时目录还不存在）。
pub fn save_unique(src: &Path, dir: &Path, stem: &str) -> Result<PathBuf, String> {
    // 1. 目录不存在就建——用户第一次点下载时目录还不存在
    dirs::ensure_dir(dir)?;

    // 2. 取扩展名，取不到就当成 "png"
    let ext = src.extension().and_then(|e| e.to_str()).unwrap_or("png");

    // 3. 先把字节读出来，避免在循环里反复读盘
    let bytes = std::fs::read(src).map_err(|e| format!("读取源文件 {:?} 失败: {e}", src))?;

    // 4. 循环上限：目录状态异常时不要转一辈子
    const MAX_ATTEMPTS: usize = 1000;

    // 5. 依次尝试 `stem.ext`、`stem (1).ext`、`stem (2).ext`……
    for n in 0..MAX_ATTEMPTS {
        let candidate: PathBuf = if n == 0 {
            dir.join(format!("{stem}.{ext}"))
        } else {
            dir.join(format!("{stem} ({n}).{ext}"))
        };

        if candidate.exists() {
            continue;
        }

        // 6. 原子写入，成功即返回
        files::write_atomic(&candidate, &bytes)?;
        return Ok(candidate);
    }

    Err(format!(
        "在 {dir:?} 下尝试了 {MAX_ATTEMPTS} 个候选文件名，均不可用"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 每个测试用自己的目录：`cargo test` 默认并行，共用目录会互相删文件。
    fn scratch_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("moe-wp-save-{}-{tag}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// A5 的验收标准：连存三次同名，得到三个不同的文件，谁都不被覆盖。
    ///
    /// 注意这里**故意不预先建目标目录**——第 1 步就该由 `save_unique` 自己建。
    #[test]
    fn save_unique_never_overwrites() {
        let dir = scratch_dir("dedup");
        let src = dir.join("source.png");
        std::fs::write(&src, b"image-bytes").unwrap();

        let out_dir = dir.join("Pictures").join("MoeWallpaper");

        let first = save_unique(&src, &out_dir, "cat").unwrap();
        let second = save_unique(&src, &out_dir, "cat").unwrap();
        let third = save_unique(&src, &out_dir, "cat").unwrap();

        assert_eq!(first.file_name().unwrap().to_str().unwrap(), "cat.png");
        assert_eq!(second.file_name().unwrap().to_str().unwrap(), "cat (1).png");
        assert_eq!(third.file_name().unwrap().to_str().unwrap(), "cat (2).png");

        // 三个文件都在，内容都对——没有被覆盖成同一个
        for p in [&first, &second, &third] {
            assert_eq!(std::fs::read(p).unwrap(), b"image-bytes", "{p:?}");
        }

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn save_unique_keeps_the_extension() {
        let dir = scratch_dir("ext");
        let src = dir.join("source.webp");
        std::fs::write(&src, b"x").unwrap();

        let out = save_unique(&src, &dir.join("out"), "pic").unwrap();

        assert_eq!(out.extension().and_then(|e| e.to_str()), Some("webp"));

        std::fs::remove_dir_all(&dir).unwrap();
    }
}
