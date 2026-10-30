//! 按图源配置发一次 HTTP 请求，把图片列表拿回来。

use crate::sources::extract::ImageRef;
use crate::sources::model::{SourceFilter, WallpaperSource};
use crate::sources::search;

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

    CLIENT.get_or_init(|| build_client(true))
}

/// 不走代理的备用客户端。
///
/// 系统里 `ProxyEnable = 1` 但代理软件已经退出，是很常见的情况（关了 Clash 却忘了
/// 取消系统代理）。这时每个请求都会先撞代理超时，用户看到的是"图源全挂了"。
/// 所以代理请求失败后要能用直连再试一次，而不是直接把错误甩给用户。
fn direct_client() -> &'static reqwest::Client {
    static CLIENT: std::sync::OnceLock<reqwest::Client> = std::sync::OnceLock::new();

    CLIENT.get_or_init(|| build_client(false))
}

fn build_client(use_proxy: bool) -> reqwest::Client {
    let mut builder = reqwest::Client::builder()
        // 有些图源会拒绝没有 User-Agent 的请求（当成爬虫）
        .user_agent(concat!("MoeWallpaper/", env!("CARGO_PKG_VERSION")))
        // 连接超时单独设短一点：代理没开时能更快失败，早点切到直连
        .connect_timeout(std::time::Duration::from_secs(10))
        // 整体超时：不设的话网络卡住时界面永远转圈。
        // 30 秒是给"慢网下一张 4K 原图"留的余量。
        .timeout(std::time::Duration::from_secs(30));

    // 只有 reqwest 不读系统代理，这一步补上它的短板：见 `crate::net` 的模块注释。
    if use_proxy
        && let Some(proxy) = crate::net::system_proxy()
        && let Ok(parsed) = reqwest::Proxy::all(&proxy)
    {
        builder = builder.proxy(parsed);
    }

    builder.build().unwrap_or_else(|_| reqwest::Client::new())
}

/// 建一个 GET 请求并带上图源配置里的请求头。
fn build_request(
    client: &reqwest::Client,
    source: &WallpaperSource,
    url: &reqwest::Url,
) -> reqwest::RequestBuilder {
    let mut req = client.get(url.clone());
    for (k, v) in &source.request.headers {
        req = req.header(k, v);
    }
    req
}

/// 把配置里写的 `{page}` / `{limit}` 换成真实数字。
///
/// 配置是静态 JSON，不能在 JSON 里做运算，所以占位符由代码替换。
/// 不做这一步，请求会带着字面量 `{page}` 发出去，服务器直接 400。
pub fn substitute_placeholders(url: &str, page: u32, limit: u32) -> String {
    url.replace("{page}", &page.to_string())
        .replace("{limit}", &limit.to_string())
}

/// 把图源声明的安全参数拼到地址末尾。
///
/// 配置里 `sfwQuery` 写的是 `purity=100` 这样的裸参数，不带问号。
/// 这里负责判断该用 `?` 还是 `&`——让写配置的人少操一件心。
///
/// 没配就不动地址。**这意味着"忘配 = 没有保护"**，
/// 所以内置图源到底配了什么，是 review 的必查项，不是可选项。
fn apply_sfw_query(url: &str, filter: &SourceFilter) -> String {
    let Some(query) = filter.sfw_query.as_deref() else {
        return url.to_string();
    };

    if query.is_empty() {
        return url.to_string();
    }

    let separator = if url.contains('?') { '&' } else { '?' };

    format!("{url}{separator}{query}")
}

/// 拼出最终要请求的地址。
///
/// 三件事按顺序做：
/// 1. **选模板**：有关键词且图源支持搜索 → 用 `search.url`，否则用 `request.url`
/// 2. **替换占位符**：`{page}` / `{limit}` / `{query}`
/// 3. **交给 `Url::parse`** 做百分号编码和合法性校验——中文关键词、空格、`&`
///    全在这一步被正确处理，不用自己写编码
///
/// 为什么第二步不做编码、第三步才做：`Url::parse` 是唯一权威。
/// 手工拼一个"看起来对"的地址，遇到中文就会发出一个坏请求，
/// 而且错误信息会是"400 Bad Request"，完全看不出是编码问题。
fn build_request_url(
    source: &WallpaperSource,
    page: u32,
    query: Option<&str>,
) -> Result<reqwest::Url, String> {
    let keyword = query.map(str::trim).filter(|q| !q.is_empty());

    let raw = match (keyword, &source.search) {
        (Some(kw), Some(spec)) => {
            let mapped = search::to_source_query(kw, spec.english);
            substitute_placeholders(&spec.url, page, DEFAULT_LIMIT).replace("{query}", &mapped)
        }
        _ => substitute_placeholders(&source.request.url, page, DEFAULT_LIMIT),
    };

    let raw = apply_sfw_query(&raw, &source.filter);

    reqwest::Url::parse(&raw).map_err(|e| format!("图源 {} 的请求地址不合法 {raw}: {e}", source.id))
}

/// 从指定图源拉一页图片。
///
/// `query` 为空或不传 = 浏览模式；非空且图源配了 `search` = 搜索模式。
pub async fn fetch_source_images(
    source: &WallpaperSource,
    page: u32,
    query: Option<&str>,
) -> Result<Vec<ImageRef>, String> {
    let url = build_request_url(source, page, query)?;

    // 只放行 GET；没配 method 视为默认 GET
    if let Some(m) = &source.request.method
        && !m.eq_ignore_ascii_case("GET")
    {
        return Err(format!("图源 {} 暂不支持 {} 方法", url, m));
    }

    // 发出去：先按系统代理，连接不通就直连再试一次
    let resp = match build_request(client(), source, &url).send().await {
        Ok(resp) => resp,
        Err(proxy_err) => {
            if crate::net::system_proxy().is_some() {
                build_request(direct_client(), source, &url)
                    .send()
                    .await
                    .map_err(|direct_err| {
                        format!(
                            "请求 {} 失败（代理和直连都没通）\n  走代理: {}\n  直连: {}",
                            url, proxy_err, direct_err
                        )
                    })?
            } else {
                return Err(format!("请求 {} 失败: {}", url, proxy_err));
            }
        }
    };

    // 查状态码
    let status = resp.status();
    if !status.is_success() {
        // 带上响应体开头：403 可能是图源的频率限制，也可能是校园网/安全软件的拦截页，
        // 两者正文完全不同。只报状态码等于让用户猜。
        let body = resp.text().await.unwrap_or_default();
        let snippet: String = body.chars().take(200).collect();
        let snippet = snippet.replace(['\r', '\n'], " ");
        return Err(format!("请求 {} 返回 {} — {}", url, status, snippet.trim()));
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

    fn filter(query: Option<&str>) -> SourceFilter {
        SourceFilter {
            sfw_query: query.map(str::to_string),
        }
    }

    #[test]
    fn sfw_query_appends_with_ampersand() {
        let url = apply_sfw_query("https://a.com/s?page=1", &filter(Some("purity=100")));
        assert_eq!(url, "https://a.com/s?page=1&purity=100");
    }

    #[test]
    fn sfw_query_starts_the_query_string() {
        let url = apply_sfw_query("https://a.com/s", &filter(Some("r18=0")));
        assert_eq!(url, "https://a.com/s?r18=0");
    }

    #[test]
    fn sfw_query_absent_leaves_url_alone() {
        let original = "https://a.com/s?page=1";
        assert_eq!(apply_sfw_query(original, &filter(None)), original);
        assert_eq!(apply_sfw_query(original, &filter(Some(""))), original);
    }

    /// 安全参数必须真的进到最终请求地址里——这条是那个卖点唯一的自动化保障。
    #[test]
    fn sfw_query_is_applied_to_the_real_source_config() {
        let file = crate::sources::loader::load_builtin().unwrap();
        let source = file
            .sources
            .iter()
            .find(|s| s.id == "wallhaven")
            .expect("内置图源里应该有 wallhaven");

        let url = apply_sfw_query(&source.request.url, &source.filter);

        assert!(
            url.contains("purity=100"),
            "wallhaven 必须带上 purity=100，实际地址是 {url}"
        );
    }

    /// A1 的验收标准。
    ///
    /// `example.invalid` 这个域名永远不会解析成功（`.invalid` 是 RFC 2606
    /// 保留给"保证不存在"的域名），所以这条测试**不依赖真实网络**，也不会被墙。
    ///
    /// 它验证两件事：坏地址必须返回 `Err`，且错误信息里带请求地址。
    ///
    /// 注意这里**自己拼一个图源**，不去读 `sources/builtin/sources.json`。
    /// 原因：内置图源是真实可用的 API，测试不该依赖它们的死活；
    /// 而这个测试要的是一个保证失败的东西。
    #[tokio::test]
    async fn unreachable_source_fails_with_readable_error() {
        let source = test_source("https://example.invalid/api?page={page}");

        let result = fetch_source_images(&source, 1, None).await;

        assert!(result.is_err(), "example.invalid 不可能成功，必须返回 Err");

        // 错误信息必须带上地址，否则用户只知道"失败了"，没法排查是哪个图源坏的
        let message = result.unwrap_err();
        assert!(
            message.contains("example.invalid"),
            "错误信息里必须包含请求地址，实际是：{message:?}"
        );
    }

    /// 非 GET 的图源必须在**发出请求之前**就被拦下。
    ///
    /// 这条测试不碰网络，所以永远稳定。
    #[tokio::test]
    async fn non_get_source_is_rejected_before_sending() {
        let mut source = test_source("https://example.invalid/");
        source.request.method = Some("POST".to_string());

        let message = fetch_source_images(&source, 1, None).await.unwrap_err();

        assert!(
            message.contains("POST"),
            "错误里要说明是哪个方法：{message:?}"
        );
    }

    fn test_source(url: &str) -> WallpaperSource {
        use crate::sources::model::{HttpRequestSpec, ResponseMapping};

        WallpaperSource {
            id: "test".to_string(),
            name: "test".to_string(),
            enabled: true,
            homepage: None,
            request: HttpRequestSpec {
                url: url.to_string(),
                method: None,
                headers: std::collections::HashMap::new(),
            },
            response: ResponseMapping {
                items_path: None,
                image_url: "url".to_string(),
                thumbnail_url: None,
                page_url: None,
                tags: None,
                width: None,
                height: None,
            },
            filter: Default::default(),
            search: None,
        }
    }

    fn searchable_source(english: bool) -> WallpaperSource {
        use crate::sources::model::SearchSpec;

        let mut source = test_source("https://a.com/browse?page={page}");
        source.search = Some(SearchSpec {
            url: "https://a.com/search?q={query}&page={page}".to_string(),
            english,
        });
        source
    }

    #[test]
    fn without_keyword_the_browse_url_is_used() {
        let url = build_request_url(&searchable_source(true), 2, None).unwrap();
        assert_eq!(url.as_str(), "https://a.com/browse?page=2");
    }

    #[test]
    fn blank_keyword_counts_as_browsing() {
        let url = build_request_url(&searchable_source(true), 1, Some("   ")).unwrap();
        assert_eq!(url.as_str(), "https://a.com/browse?page=1");
    }

    #[test]
    fn source_without_search_ignores_the_keyword() {
        let url = build_request_url(&test_source("https://a.com/browse"), 1, Some("猫")).unwrap();
        assert_eq!(url.as_str(), "https://a.com/browse");
    }

    #[test]
    fn keyword_is_translated_when_english_is_required() {
        let url = build_request_url(&searchable_source(true), 1, Some("风景")).unwrap();
        assert_eq!(url.as_str(), "https://a.com/search?q=landscape&page=1");
    }

    #[test]
    fn keyword_is_left_alone_when_the_source_reads_chinese() {
        let url = build_request_url(&searchable_source(false), 1, Some("猫")).unwrap();
        assert_eq!(url.path(), "/search");
        assert_eq!(url.query_pairs().find(|(k, _)| k == "q").unwrap().1, "猫");
    }

    /// 中文关键词必须被百分号编码，否则发出去就是个坏请求。
    #[test]
    fn non_ascii_keyword_is_percent_encoded() {
        let url =
            build_request_url(&searchable_source(false), 1, Some("某个不存在的番名")).unwrap();

        assert!(
            url.as_str().contains('%'),
            "地址里应该有百分号编码，实际是 {}",
            url.as_str()
        );
        assert!(
            !url.as_str().contains("某个"),
            "中文不该以原样出现在地址里：{}",
            url.as_str()
        );
    }

    #[test]
    fn badly_formed_url_is_reported_with_the_source_id() {
        let message = build_request_url(&test_source("不是个地址"), 1, None).unwrap_err();

        assert!(message.contains("test"), "错误里要带图源 id：{message}");
    }

    /// 手动跑：`cargo test -- --ignored --nocapture live_request`
    ///
    /// 验证的是"真的能联网"这件事，结果取决于这台机器的网络和代理设置，
    /// 所以不能进自动测试。它覆盖的是 `client()` 有没有正确套上系统代理——
    /// 这一段纯函数测不出来。
    #[tokio::test]
    #[ignore]
    async fn live_request_reaches_a_real_host() {
        let url = reqwest::Url::parse("https://api.lolicon.app/setu/v2?num=1&p=1&r18=0").unwrap();
        println!("系统代理 = {:?}", crate::net::system_proxy());

        match client().get(url).send().await {
            Ok(resp) => println!("状态码 = {}", resp.status()),
            Err(e) => println!("请求失败 = {e}"),
        }
    }
}
