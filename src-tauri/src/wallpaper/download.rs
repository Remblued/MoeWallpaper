//! 把远端图片下载进本地缓存。

use std::path::PathBuf;

use crate::fs::{naming, paths};

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

// ============================================================================
//  下面这个函数由你实现（A4 的核心）
// ============================================================================

/// 保证这张图已经在本地磁盘上，返回它的路径。
///
/// # 实现步骤
///
/// 1. 算目标路径：`cached_path(url)?`
/// 2. **先查缓存**：`dest.exists()` 为真就直接 `Ok(dest)` 返回。
///    这一步就是"不重复下载"的全部实现——每次设壁纸前都走一遍，省一次网络往返。
/// 3. 确保目录存在：`crate::fs::dirs::ensure_dir(父目录)`。
///    第一次运行 `wallpapers/` 还不存在，直接写文件会失败。
/// 4. 下载：`client().get(url).send().await`
///    —— 但 `client()` 在 `sources::fetch` 里是私有的，这里有两个选择：
///    (a) 把那个 `fn client()` 改成 `pub(crate)`，两处共用（推荐，省一次 TLS 握手）
///    (b) 在这里自己 `reqwest::Client::new()`（简单但浪费）
/// 5. 查状态码：非 2xx 返回 `Err`，错误信息带状态码和 URL
/// 6. 取字节：`.bytes().await` → 得到 `reqwest::Response` 的 `Bytes`
/// 7. 原子写入：`crate::fs::files::write_atomic(&dest, &bytes)`
///    —— 不要用 `std::fs::write`，那就是上次讲的"写到一半断电"问题
/// 8. 返回 `Ok(dest)`
///
/// # 错误处理约定
///
/// - 任何一步失败 → `Err(String)`，消息里带 URL
/// - 注意：目录可以写、但文件写不进去（磁盘满、杀软拦截）也算失败，别吞掉
pub async fn ensure_cached(url: &str) -> Result<PathBuf, String> {
    // TODO(你写)：按上面 8 步实现。
    // 现在这行只是为了让变量被"用到"，你写完后删掉它。
    let _ = url;
    todo!("ensure_cached 由你实现")
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
}
