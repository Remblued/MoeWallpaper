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
pub(crate) fn client() -> &'static reqwest::Client {
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

/// 从指定图源拉一页图片。
pub async fn fetch_source_images(
    source: &WallpaperSource,
    page: u32,
) -> Result<Vec<ImageRef>, String> {
    // 拼地址
    let url = substitute_placeholders(&source.request.url, page, DEFAULT_LIMIT);

    // 只放行 GET；没配 method 视为默认 GET
    if let Some(m) = &source.request.method
        && !m.eq_ignore_ascii_case("GET")
    {
        return Err(format!("图源 {} 暂不支持 {} 方法", url, m));
    }

    // 建请求 + 请求头
    let mut req = client().get(&url);
    for (k, v) in &source.request.headers {
        req = req.header(k, v);
    }

    // 发出去
    let resp = req
        .send()
        .await
        .map_err(|e| format!("请求 {} 失败: {}", url, e))?;

    // 查状态码
    let status = resp.status();
    if !status.is_success() {
        return Err(format!("请求 {} 返回状态码 {}", url, status));
    }

    // 读 JSON
    let value = resp
        .json::<serde_json::Value>()
        .await
        .map_err(|e| format!("解析 {} 的 JSON 失败: {}", url, e))?;

    // 提取
    crate::sources::extract::extract_images(&source.response, &value)
        .map_err(|e| format!("从 {} 提取图片失败: {}", url, e))
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

    /// A1 的验收标准。
    ///
    /// `example.invalid` 这个域名永远不会解析成功（`.invalid` 是 RFC 2606
    /// 保留给"保证不存在"的域名），所以这条测试**不依赖真实网络**，也不会被墙。
    ///
    /// 它验证两件事：坏地址必须返回 `Err`，且错误信息里带请求地址。
    #[tokio::test]
    async fn example_source_fails_with_readable_error() {
        let source = crate::sources::loader::load_builtin()
            .unwrap()
            .sources
            .into_iter()
            .find(|s| s.id == "example")
            .unwrap();

        let result = fetch_source_images(&source, 1).await;

        assert!(result.is_err(), "example.invalid 不可能成功，必须返回 Err");

        // 错误信息必须带上地址，否则用户只知道"失败了"，没法排查是哪个图源坏的
        let message = result.unwrap_err();
        assert!(
            message.contains("example.invalid"),
            "错误信息里必须包含请求地址，实际是：{message:?}"
        );
    }
}
