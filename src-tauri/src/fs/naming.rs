//! 把 URL 变成一个能当文件名用的字符串。
//!
//! 为什么不直接用 URL 当文件名：
//! - Windows 路径上限 260 字符，URL 很容易超
//! - URL 里的 `/` `:` `?` `&` 在文件名里是非法或需要转义的
//!
//! 所以策略是「哈希 + 保留扩展名」：
//! `https://a.com/x/123.png` → `a3f19c...e7.png`
//!
//! 用 FNV-1a 而不是标准库的 `DefaultHasher`，原因只有一个：
//! `DefaultHasher` 不保证跨 Rust 版本、跨平台结果一致，
//! 而这个哈希要变成**落盘的缓存文件名**，一旦变了，用户缓存就全部失效、重复下载。

/// FNV-1a 64 位，输出 16 位十六进制。
///
/// 不是密码学哈希，别拿它做安全用途；它只需要「同一个 URL 永远得到同一个值」。
pub fn url_hash(url: &str) -> String {
    // FNV-1a 的两个魔数，来自算法定义，不能改
    const OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;

    let mut hash = OFFSET_BASIS;
    for byte in url.as_bytes() {
        hash ^= *byte as u64;
        // 必须用 wrapping_mul：正常乘法溢出会 panic（debug 模式）
        hash = hash.wrapping_mul(PRIME);
    }

    format!("{hash:016x}")
}

/// 从 URL 里猜图片扩展名，猜不出就用 `.jpg`。
///
/// 只做字符串判断，不联网问服务器。猜错也没关系——Windows 看图看的是文件内容，
/// 扩展名只影响"用什么程序打开"。
pub fn extension_from_url(url: &str) -> &'static str {
    // 先砍掉 ?query 和 #fragment，只留路径部分
    let path = url.split(['?', '#']).next().unwrap_or(url);

    let ext = path.rsplit('.').next().unwrap_or("");

    match ext.to_ascii_lowercase().as_str() {
        "png" => ".png",
        "jpg" | "jpeg" => ".jpg",
        "webp" => ".webp",
        "bmp" => ".bmp",
        "gif" => ".gif",
        _ => ".jpg",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_url_same_hash() {
        let a = url_hash("https://a.com/x.png");
        let b = url_hash("https://a.com/x.png");
        assert_eq!(a, b);
        assert_eq!(a.len(), 16);
    }

    #[test]
    fn different_url_different_hash() {
        assert_ne!(
            url_hash("https://a.com/1.png"),
            url_hash("https://a.com/2.png")
        );
    }

    #[test]
    fn hash_is_hex_only() {
        // 要能直接当文件名，不能出现 / : 之类
        let h = url_hash("https://a.com:8443/p/q?w=1&e=2");
        assert!(h.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn extension_detects_common_formats() {
        assert_eq!(extension_from_url("https://a.com/x.png"), ".png");
        assert_eq!(extension_from_url("https://a.com/x.JPEG"), ".jpg");
        assert_eq!(extension_from_url("https://a.com/x.webp?a=1#b"), ".webp");
    }

    #[test]
    fn extension_falls_back_to_jpg() {
        assert_eq!(extension_from_url("https://a.com/api/posts"), ".jpg");
        assert_eq!(extension_from_url("https://a.com/x"), ".jpg");
    }
}
