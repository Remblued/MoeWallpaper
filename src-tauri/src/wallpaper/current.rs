//! 维护"当前壁纸"那个固定路径。

use std::path::{Path, PathBuf};

use crate::fs::{files, paths};
use crate::wallpaper::normalize;

/// 把图片规整成 JPEG 后落到固定的当前壁纸位置，返回该路径。
///
/// 复制而不是直接设缓存文件：见 `fs::paths::current_file` 的说明。
/// 用 `write_atomic` 而不是 `fs::copy`，是为了"要么是旧图，要么是新图，
/// 不会出现写了一半的文件"——系统随时可能来读这个路径。
///
/// **落盘前过一次 `normalize`**：`SystemParametersInfoW` 只认 BMP/JPEG，
/// 而图源里会出现 webp / PNG。缓存保持原样（用户下载要原始格式），
/// 这个固定路径则必须是 Windows 一定认得的东西。
pub fn publish(src: &Path) -> Result<PathBuf, String> {
    let dest = paths::current_file()?;
    publish_to(src, &dest)?;
    Ok(dest)
}

/// 实际干活的版本，目标路径由调用方给，方便测试。
fn publish_to(src: &Path, dest: &Path) -> Result<(), String> {
    let bytes = std::fs::read(src).map_err(|e| format!("读取图片 {:?} 失败: {}", src, e))?;

    let wallpaper = normalize::to_wallpaper_bytes(&bytes)
        .map_err(|e| format!("{} 不能设为壁纸（缓存文件 {:?}）", e, src))?;

    files::write_atomic(dest, &wallpaper)
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
    fn publish_writes_a_jpeg_even_when_the_cache_holds_png() {
        let dir = scratch_dir("png");
        let src = dir.join("cached.png");
        let dest = dir.join("current.jpg");

        // 造一张真 PNG 当缓存（仓库里不放图片文件，夹具现造）
        let mut png = Vec::new();
        image::DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(
            3,
            2,
            image::Rgba([255, 0, 0, 255]),
        ))
        .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
        .unwrap();
        std::fs::write(&src, &png).unwrap();

        publish_to(&src, &dest).unwrap();

        let dest_bytes = std::fs::read(&dest).unwrap();
        assert!(
            normalize::is_jpeg(&dest_bytes),
            "current.jpg 里必须是 JPEG，不能是 PNG 数据"
        );
        assert!(!dir.join("current.tmp").exists()); // 临时文件必须被消费掉

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn publish_rejects_a_cache_that_is_not_an_image() {
        let dir = scratch_dir("html");
        let src = dir.join("cached.jpg");
        let dest = dir.join("current.jpg");
        std::fs::write(&src, b"<html>403 Forbidden</html>").unwrap();

        let err = publish_to(&src, &dest).unwrap_err();

        // 这就是"缓存记住了错误页"那条债：现在会报错，而不是把错误页设成壁纸
        assert!(err.contains("不能设为壁纸"), "错误要带上下文，实际: {err}");
        assert!(!dest.exists(), "失败时不该产生目标文件");

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
