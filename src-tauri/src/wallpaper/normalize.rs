//! 把任意可解码的图片规整成 Windows 认得的 JPEG 字节。
//!
//! **为什么需要这一步**：`SystemParametersInfoW` 只认 BMP / JPEG（Windows 10 之后的
//! 部分场景也吃 PNG），**webp 一律要系统里有解码器**才认。而 webp 正好是我们会遇到的
//! 格式：safebooru 的部分条目、nekos.best 与 lolicon 的图都有 webp，夜轻那个源则是
//! 100% webp（`docs/SOURCES-CN.md` §7.1）。
//!
//! 之前的做法是把缓存文件原样复制成 `current.jpg`，指望 Windows 按内容嗅探。这条路
//! 只在"用户机器上碰巧装了 webp 扩展"时成立——`Microsoft.WebpImageExtension` 是商店
//! 扩展，不能当成假定。**2026-10-31 的产品决定：宁可安装包变大，也不让"设为壁纸"
//! 在别人机器上失效。** 代价是一次解码 + 一次编码，按下按钮时多花几百毫秒。
//!
//! 顺带修掉的技术债：`current.jpg` 里再也不会装着 PNG / WebP 数据。

/// 重新编码时用的 JPEG 质量。
///
/// 92 是"肉眼与无损难分辨"的常用档；再高只是让文件变大，壁纸最终由系统缩放显示。
const JPEG_QUALITY: u8 = 92;

/// 这段字节是不是 JPEG（`FF D8 FF`）。
///
/// 只看魔数，不解码。要的是"最常见的情况零成本"：图源里绝大多数就是 JPEG。
pub fn is_jpeg(bytes: &[u8]) -> bool {
    bytes.len() >= 3 && bytes[0] == 0xFF && bytes[1] == 0xD8 && bytes[2] == 0xFF
}

/// 把图片字节规整成能直接交给 `SystemParametersInfoW` 的 JPEG 字节。
///
/// - 已经是 JPEG：**原样返回**，不解码也不重编码（省时间、不掉质量）
/// - 其他能解码的（PNG / WebP / GIF 第一帧）：解码后编码成 JPEG
/// - 解不出来：报错，并带上字节数——最常见的成因是"缓存里存了一张 HTML 错误页"
pub fn to_wallpaper_bytes(bytes: &[u8]) -> Result<Vec<u8>, String> {
    if is_jpeg(bytes) {
        return Ok(bytes.to_vec());
    }

    let format = image::guess_format(bytes).map_err(|e| {
        format!(
            "缓存里的 {} 字节不是能识别的图片（可能是错误页或下载不完整）: {}",
            bytes.len(),
            e
        )
    })?;

    let decoded = image::load_from_memory_with_format(bytes, format)
        .map_err(|e| format!("按 {:?} 解码失败（{} 字节）: {}", format, bytes.len(), e))?;

    // GIF 只取第一帧（`load_from_memory` 的默认行为），动图当壁纸没有意义。
    // 透明通道不能直接丢：转 RGB 之前先压在白底上，否则透明区域会变成黑块。
    let rgba = decoded.to_rgba8();
    let (width, height) = rgba.dimensions();
    let flat: image::RgbImage = image::RgbImage::from_fn(width, height, |x, y| {
        let px = rgba.get_pixel(x, y);
        let alpha = u16::from(px[3]);
        let over_white = |c: u8| -> u8 {
            let blended = (u16::from(c) * alpha + 255 * (255 - alpha)) / 255;
            blended.min(255) as u8
        };
        image::Rgb([over_white(px[0]), over_white(px[1]), over_white(px[2])])
    });

    let mut out = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, JPEG_QUALITY)
        .encode(flat.as_raw(), width, height, image::ExtendedColorType::Rgb8)
        .map_err(|e| format!("把 {:?} 重新编码成 JPEG 失败: {}", format, e))?;

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 造一张 4×2 的 PNG：左半边不透明白，右半边全透明。
    ///
    /// 不用图片文件当夹具：仓库里不放图片（含截图/壁纸），而这里用编码器现造
    /// 既省一个文件，也不会让夹具和测试一起过期。
    fn half_transparent_png() -> Vec<u8> {
        let mut rgba = image::RgbaImage::new(4, 2);
        for (x, _y, px) in rgba.enumerate_pixels_mut() {
            *px = if x < 2 {
                image::Rgba([0, 0, 255, 255])
            } else {
                image::Rgba([0, 0, 0, 0])
            };
        }
        let mut png = Vec::new();
        image::DynamicImage::ImageRgba8(rgba)
            .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
            .unwrap();
        png
    }

    #[test]
    fn jpeg_is_passed_through_untouched() {
        let jpeg = [0xFFu8, 0xD8, 0xFF, 0xE0, 0x00, 0x10, b'J', b'F', b'I', b'F'];
        let out = to_wallpaper_bytes(&jpeg).unwrap();
        assert_eq!(out, jpeg, "已经是 JPEG 时不该重编码");
    }

    #[test]
    fn png_becomes_a_real_jpeg() {
        let png = half_transparent_png();
        let out = to_wallpaper_bytes(&png).unwrap();

        assert!(
            is_jpeg(&out),
            "输出必须是 JPEG，否则 SystemParametersInfoW 不认"
        );
        assert_ne!(out, png);

        let decoded = image::load_from_memory(&out).expect("输出的 JPEG 要能解回来");
        assert_eq!((decoded.width(), decoded.height()), (4, 2), "尺寸不能变");

        // 透明的那半边应该被压成白色，而不是黑色
        let rgb = decoded.to_rgb8();
        let transparent_side = rgb.get_pixel(3, 0);
        assert!(
            transparent_side[0] > 240 && transparent_side[1] > 240 && transparent_side[2] > 240,
            "透明像素该压在白底上，实际是 {:?}",
            transparent_side
        );
    }

    #[test]
    fn error_page_is_reported_with_size() {
        let html = b"<!DOCTYPE html><html><body>403</body></html>";
        let err = to_wallpaper_bytes(html).unwrap_err();
        assert!(
            err.contains("不是能识别的图片"),
            "错误要说清楚是格式问题，实际: {err}"
        );
        assert!(
            err.contains(&html.len().to_string()),
            "错误要带上字节数，方便判断是不是下载不完整，实际: {err}"
        );
    }

    /// 手动跑：`cargo test -- --ignored --nocapture webp_is_transcoded`
    ///
    /// webp 的解码器只能读不能写，测试里造不出 webp 夹具，所以这条必须联网。
    /// 用的是夜轻的真实图片（`cdn.yppp.net`），因为那正是我们要支持的场景。
    #[tokio::test]
    #[ignore = "要联网，且依赖第三方 CDN 可用"]
    async fn webp_is_transcoded_from_a_real_file() {
        let url = "https://cdn.yppp.net/acg/pc/1.webp";
        let bytes = crate::sources::fetch::client()
            .get(url)
            .send()
            .await
            .expect("下载失败（网络问题，不是代码问题）")
            .bytes()
            .await
            .expect("读不到正文")
            .to_vec();

        assert!(!is_jpeg(&bytes), "夹具本身就应该是 webp，而不是 JPEG");

        let out = to_wallpaper_bytes(&bytes).expect("webp 应该能被转成 JPEG");
        assert!(is_jpeg(&out));
        let decoded = image::load_from_memory(&out).unwrap();
        println!(
            "webp {} 字节 → JPEG {} 字节，{}x{}",
            bytes.len(),
            out.len(),
            decoded.width(),
            decoded.height()
        );
        assert_eq!(
            (decoded.width(), decoded.height()),
            (2560, 1446),
            "夜轻 pc 的图是 2560×1446，尺寸变了说明解码有问题"
        );
    }
}
