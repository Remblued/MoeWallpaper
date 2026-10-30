//! 把远端图片下载进本地缓存。

use std::path::PathBuf;

use crate::fs::{dirs, files, naming, paths};
use crate::sources::fetch::client;

/// 这张图在缓存里的固定位置：`wallpapers/<URL哈希><扩展名>`。
///
/// "固定"是这里最关键的性质：同一个 URL，无论点多少次，都落到同一个路径。
/// 有它才能判断"是不是已经下载过了"。
pub fn cached_path(url: &str) -> Result<PathBuf, String> {
    let name = format!(
        "{}{}",
        naming::url_hash(url),
        naming::extension_from_url(url)
    );
    Ok(paths::wallpapers_dir()?.join(name))
}

/// 保证这张图已经在本地。已缓存时直接返回路径，不发任何请求。
pub async fn ensure_cached(url: &str) -> Result<PathBuf, String> {
    // 1. 算目标路径
    let dest = cached_path(url)?;

    // 2. 先查缓存：命中就完全不碰网络
    if dest.is_file() {
        return Ok(dest);
    }

    // 3. 确保父目录存在
    let parent = dest
        .parent()
        .ok_or_else(|| format!("缓存路径没有父目录: {}（URL: {}）", dest.display(), url))?;
    dirs::ensure_dir(parent).map_err(|e| {
        format!(
            "创建缓存目录 {} 失败（URL: {}）: {}",
            parent.display(),
            url,
            e
        )
    })?;

    // 4. 发请求
    let resp = client()
        .get(url)
        .send()
        .await
        .map_err(|e| format!("下载 {} 失败: {}", url, e))?;

    // 5. 查状态码
    let status = resp.status();
    if !status.is_success() {
        return Err(format!("下载 {} 返回状态码 {}", url, status));
    }

    // 6. 取字节
    let bytes = resp
        .bytes()
        .await
        .map_err(|e| format!("读取 {} 的响应体失败: {}", url, e))?;

    // 7. 原子写入
    files::write_atomic(&dest, &bytes)
        .map_err(|e| format!("写入 {} 的缓存文件 {} 失败: {}", url, dest.display(), e))?;

    // 8. 返回
    Ok(dest)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_url_gives_same_path() {
        let a = cached_path("https://a.com/x.png").unwrap();
        let b = cached_path("https://a.com/x.png").unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn path_ends_with_extension() {
        let p = cached_path("https://a.com/x.png").unwrap();
        assert_eq!(p.extension().and_then(|e| e.to_str()), Some("png"));
    }

    #[test]
    fn path_lives_under_wallpapers_dir() {
        let p = cached_path("https://a.com/x.png").unwrap();
        assert!(p.starts_with(paths::wallpapers_dir().unwrap()));
    }

    /// A4 的验收标准。
    ///
    /// 手法：先把"缓存文件"手工造出来，再拿一个**永远连不上**的地址去调用。
    /// 如果缓存判断生效 → 直接返回，压根不碰网络，测试通过；
    /// 如果没有判断 → 它会去连 example.invalid，失败返回 Err，测试失败。
    ///
    /// 这样就不用真的下载任何东西，测试既是确定的、又不依赖网络。
    ///
    /// 注意：这条测试会在真实的用户缓存目录里建一个文件，跑完会删掉。
    /// （更好的做法是让缓存目录可注入，但那是一次重构，先记在技术债里。）
    #[tokio::test]
    async fn existing_file_skips_download() {
        let url = "https://example.invalid/never-fetched.jpg";
        let dest = cached_path(url).unwrap();

        crate::fs::dirs::ensure_dir(dest.parent().unwrap()).unwrap();
        crate::fs::files::write_atomic(&dest, b"already here").unwrap();

        let got = ensure_cached(url)
            .await
            .expect("文件已经在缓存里，不该再去联网");

        assert_eq!(got, dest);
        assert_eq!(std::fs::read(&dest).unwrap(), b"already here");

        std::fs::remove_file(&dest).unwrap();
    }
}
