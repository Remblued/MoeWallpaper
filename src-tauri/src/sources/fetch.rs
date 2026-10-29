//! 按图源配置发一次 HTTP 请求，把图片列表拿回来。

use crate::sources::extract::ImageRef;
use crate::sources::model::WallpaperSource;

/// 一次请求要几张图。
///
/// 20 是个折中：太小要多翻页（每次翻页都是一个新的网络往返），
/// 太大单页响应变慢、内存占用变高，而且用户看不完就滚走了。
pub const DEFAULT_LIMIT: u32 = 20;

/// 全局共享的 HTTP 客户端。
///
/// 为什么不每次 `Client::new()`：
/// 一个 Client 内部维护连接池（TCP 连接、TLS 握手结果）。
/// 每次新建 = 每次重新握手 TLS，多花几十到几百毫秒。
/// `OnceLock` 保证全进程只建一次，第一次调用时初始化。
fn client() -> &'static reqwest::Client {
    static CLIENT: std::sync::OnceLock<reqwest::Client> = std::sync::OnceLock::new();

    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            // 有些图源会拒绝没有 User-Agent 的请求（当成爬虫）
            .user_agent(concat!("MoeWallpaper/", env!("CARGO_PKG_VERSION")))
            // 不设超时 = 网络卡住时界面永远转圈。20 秒足够慢网加载一页。
            .timeout(std::time::Duration::from_secs(20))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new())
    })
}

/// 把配置里写的 `{page}` / `{limit}` 换成真实数字。
///
/// 配置是静态 JSON，不能在 JSON 里做运算，所以占位符由代码替换。
/// 不做这一步，请求会带着字面量 `{page}` 发出去，服务器直接 400。
pub fn substitute_placeholders(url: &str, page: u32, limit: u32) -> String {
    url.replace("{page}", &page.to_string())
        .replace("{limit}", &limit.to_string())
}

// ============================================================================
//  下面这个函数由你实现（A1 的核心）
// ============================================================================

/// 从指定图源拉一页图片。
///
/// # 实现步骤
///
/// 1. 拼地址：`substitute_placeholders(&source.request.url, page, DEFAULT_LIMIT)`
/// 2. 建请求：`client().get(&url)`
/// 3. 加请求头：配置里的 `source.request.headers` 是 `HashMap<String, String>`，
///    遍历它，每个用 `.header(键, 值)`
/// 4. 发出去：`.send().await` → 得到 `Result<Response, reqwest::Error>`
/// 5. 查状态码：`.status()` 不是 2xx 就返回 Err，
///    错误信息里**必须带状态码**，否则用户只知道"失败了"没法排查
/// 6. 读 JSON：`.json::<serde_json::Value>().await`
/// 7. 交给现成的提取函数：`crate::sources::extract::extract_images(&source.response, &value)`
///
/// # 错误处理约定
///
/// - 网络层失败、状态码非 2xx、JSON 解析失败 → `Err(String)`
/// - 服务器返回空列表 → `Ok(vec![])`，不是错误
/// - 每处 `.map_err(|e| ...)` 的错误信息里带上 URL，排查时才知道是哪个图源坏了
///
/// 提示：`?` 只能用在 `Result<_, E>` 上，而 `reqwest::Error` 不是 `String`，
/// 所以不能直接 `?`，要么 `.map_err(...)?`，要么 `match`。
pub async fn fetch_source_images(
    source: &WallpaperSource,
    page: u32,
) -> Result<Vec<ImageRef>, String> {
    let url = substitute_placeholders(&source.request.url, page, DEFAULT_LIMIT);

    // TODO(你写)：把上面 7 步实现出来。
    // 下面两行只是临时占位，保证脚手架能编译通过；你写完后把它们和 todo! 一起删掉。
    let _ = url;
    let _ = client();
    todo!("fetch_source_images 由你实现")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn substitutes_both_placeholders() {
        let url = substitute_placeholders("https://a.com/p?limit={limit}&page={page}", 3, 20);
        assert_eq!(url, "https://a.com/p?limit=20&page=3");
    }

    #[test]
    fn leaves_url_without_placeholders_alone() {
        let url = substitute_placeholders("https://a.com/api?x=1", 1, 20);
        assert_eq!(url, "https://a.com/api?x=1");
    }

    #[test]
    fn substitutes_repeated_placeholder() {
        let url = substitute_placeholders("https://a.com/{page}/{page}", 7, 20);
        assert_eq!(url, "https://a.com/7/7");
    }
}
