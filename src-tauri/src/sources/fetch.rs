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
    // 偏移量分页的接口（360 壁纸）要的是 `start=0/20/40`，不是页码。
    // 让配置写 `{offset}`、由代码算，配置本身就不用做算术。
    let offset = page.saturating_sub(1).saturating_mul(limit);

    substitute_with_offset(url, page, limit, offset)
}

/// 同上，但偏移量由调用方给。
///
/// 分开的理由：顺序固定的图源要的是**随机**偏移量（见 `SourceFilter::random_offset_max`），
/// 而随机值不能藏在"替换函数"里——那样这个函数就没法测了。
fn substitute_with_offset(url: &str, page: u32, limit: u32, offset: u32) -> String {
    url.replace("{page}", &page.to_string())
        .replace("{limit}", &limit.to_string())
        .replace("{offset}", &offset.to_string())
}

/// 在 `[0, max_start)` 里挑一个 `limit` 的倍数。
///
/// 纯函数，只吃种子，所以能测：随机的东西不好测，但"随机值必须合法"很好测。
fn random_start_from(seed: u64, max_start: u32, limit: u32) -> u32 {
    if limit == 0 || max_start < limit {
        return 0;
    }
    let slots = max_start / limit;
    ((seed % slots as u64) as u32) * limit
}

/// 取一个伪随机数。
///
/// 不为"换一批"这一个功能引入 `rand` 依赖：`RandomState` 的种子来自操作系统熵，
/// 再掺一个自增计数器，连续调用也不会拿到同一个值。
fn random_u64() -> u64 {
    use std::hash::{BuildHasher, Hasher};
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);

    let mut hasher = std::collections::hash_map::RandomState::new().build_hasher();
    hasher.write_u64(n);
    hasher.finish()
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

    // 顺序固定的图源要随机起点，否则"第 1 页"永远是那 20 张。
    let offset = match source.filter.random_offset_max {
        Some(max) => random_start_from(random_u64(), max, DEFAULT_LIMIT),
        None => page.saturating_sub(1).saturating_mul(DEFAULT_LIMIT),
    };

    let raw = match (keyword, &source.search) {
        (Some(kw), Some(spec)) => {
            let mapped = search::to_source_query(kw, spec.english);
            substitute_with_offset(&spec.url, page, DEFAULT_LIMIT, offset)
                .replace("{query}", &mapped)
        }
        _ => substitute_with_offset(&source.request.url, page, DEFAULT_LIMIT, offset),
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
    let mut images = crate::sources::extract::extract_images(&source.response, &value)
        .map_err(|e| format!("从 {} 提取图片失败: {}", url, e))?;

    // 图片地址的 http→https 升级。为什么需要见 `SourceFilter::https_upgrade`。
    if source.filter.https_upgrade {
        for img in &mut images {
            img.image_url = upgrade_to_https(&img.image_url);
            img.thumbnail_url = img.thumbnail_url.as_deref().map(upgrade_to_https);
        }
    }

    Ok(images)
}

/// 把 `http://` 开头的地址升级成 `https://`。
///
/// 已经是 https、或根本不是 http 地址的，原样返回——不猜、不改写别的东西。
/// 只替换协议前缀，所以 `http://` 出现在路径里（很少见）也不会被误伤。
fn upgrade_to_https(url: &str) -> String {
    match url.strip_prefix("http://") {
        Some(rest) => format!("https://{rest}"),
        None => url.to_string(),
    }
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
            https_upgrade: false,
            random_offset_max: None,
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
                resolution: None,
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

    /// 手动跑：`cargo test -- --ignored --nocapture live_360`
    ///
    /// 一次覆盖三件事：`{offset}` 替换对不对、`resolution` 能不能拆、http 图片有没有
    /// 被升级成 https。这三件任何一件错了，界面都会表现为"列表拿到了但图全是裂的"，
    /// 而单元测试各自只测其中一件。
    #[tokio::test]
    #[ignore]
    async fn live_360_source_works_end_to_end() {
        let sources = crate::sources::loader::load_builtin().unwrap();
        let src = sources
            .sources
            .iter()
            .find(|s| s.id == "360")
            .expect("内置图源里应该有 360");

        let images = fetch_source_images(src, 1, None)
            .await
            .expect("请求应当成功");
        println!("拿到 {} 张", images.len());
        for img in images.iter().take(3) {
            println!(
                "  {}  {}x{:?}",
                img.image_url,
                img.width.unwrap_or(0),
                img.height
            );
        }

        assert!(!images.is_empty(), "360 应该返回图片");
        assert!(
            images.iter().all(|i| i.image_url.starts_with("https://")),
            "http 图片必须被升级成 https，否则 WebView 不会加载"
        );
        assert!(
            images.iter().any(|i| i.width.is_some()),
            "resolution 字段应该被拆成宽高"
        );

        // 连拉三次，第一张不能都是同一张——否则"随机"是假的
        let mut firsts = std::collections::HashSet::new();
        firsts.insert(images[0].image_url.clone());
        for _ in 0..2 {
            let again = fetch_source_images(src, 1, None).await.unwrap();
            firsts.insert(again[0].image_url.clone());
        }
        assert!(
            firsts.len() > 1,
            "三次请求拿到同一张图，随机起点没生效：{firsts:?}"
        );
    }

    #[test]
    fn offset_starts_at_zero_and_steps_by_limit() {
        let u = "https://a.com/p?start={offset}&count={limit}";
        assert!(substitute_placeholders(u, 1, 20).contains("start=0"));
        assert!(substitute_placeholders(u, 3, 20).contains("start=40"));
    }

    #[test]
    fn offset_never_underflows() {
        // page=0 是非法的，但也不能因为 u32 下溢而崩
        let url = substitute_placeholders("https://a.com/p?start={offset}", 0, 20);
        assert!(url.contains("start=0"), "{url}");
    }

    #[test]
    fn random_start_is_a_valid_multiple_of_limit() {
        for seed in 0..200u64 {
            let start = random_start_from(seed, 8000, 20);
            assert_eq!(start % 20, 0, "起点必须是 limit 的倍数：{start}");
            assert!(start < 8000, "起点必须在上限内：{start}");
        }
    }

    #[test]
    fn random_start_covers_many_pages() {
        // 只有几个取值就等于没随机
        let picks: std::collections::HashSet<u32> =
            (0..50u64).map(|s| random_start_from(s, 8000, 20)).collect();
        assert!(picks.len() > 5, "随机起点太集中：{picks:?}");
    }

    #[test]
    fn degenerate_random_bounds_do_not_panic() {
        assert_eq!(random_start_from(123, 0, 20), 0);
        assert_eq!(random_start_from(123, 19, 20), 0);
        assert_eq!(random_start_from(123, 8000, 0), 0);
    }

    #[test]
    fn https_upgrade_only_touches_the_scheme() {
        assert_eq!(
            upgrade_to_https("http://p3.qhimg.com/a.jpg"),
            "https://p3.qhimg.com/a.jpg"
        );
        assert_eq!(
            upgrade_to_https("https://a.com/a.jpg"),
            "https://a.com/a.jpg"
        );
        assert_eq!(upgrade_to_https("ftp://a.com/a.jpg"), "ftp://a.com/a.jpg");
        // 协议后面的内容一个字都不能动
        assert_eq!(
            upgrade_to_https("http://a.com/http://b.jpg"),
            "https://a.com/http://b.jpg"
        );
    }
}
