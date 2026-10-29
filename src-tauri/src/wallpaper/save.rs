//! 把图片存到用户自己的目录（**不是缓存**）。

use std::path::{Path, PathBuf};

// 提示：写到一半嫌 `crate::fs::dirs::...` 太长，可以在顶部加
// `use crate::fs::{dirs, files};`，然后把下面步骤里的全路径换短。
// 现在不加是因为：还没人用它们，clippy 会报 `unused imports`。

// ============================================================================
//  下面这个函数由你实现（A5 的核心）
// ============================================================================

/// 把一个本地文件复制到 `dir` 下，命名成 `stem.ext`，返回最终路径。
///
/// 目录里已有同名文件时，依次尝试 `stem (1).ext`、`stem (2).ext`……
/// **绝不覆盖用户已有的文件。**
///
/// # 实现步骤
///
/// 1. 目录不存在就建：`crate::fs::dirs::ensure_dir(dir)?`——用户第一次点下载时目录还不存在
/// 2. 取扩展名：`src.extension()`。取不到就当成 `"png"`（宁可后缀不准，也别存成无后缀文件）
/// 3. 拼第一个候选：`dir.join(format!("{stem}.{ext}"))`
/// 4. 循环判断：候选**不存在**就写进去并返回；存在就把名字换成 `stem (n).ext` 再试
/// 5. 写入用 `crate::fs::files::write_atomic(&candidate, &bytes)`，字节先 `std::fs::read(src)`
/// 6. **循环必须设上限**（比如 1000 次）。没有上限的话，一旦目录状态异常
///    （比如候选路径永远判断为"存在"），这个函数会转到天荒地老，
///    而且是在后台线程里转，用户只能看到界面卡住
///
/// # 为什么不能直接覆盖
///
/// 用户可能已经把上一张存成同名文件、还改了内容。应用无权替他决定
/// "覆盖掉那张"还是"再存一张"。**默认不覆盖，是唯一安全的默认值。**
pub fn save_unique(src: &Path, dir: &Path, stem: &str) -> Result<PathBuf, String> {
    let _ = (src, dir, stem);
    todo!("save_unique 由你实现")
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
