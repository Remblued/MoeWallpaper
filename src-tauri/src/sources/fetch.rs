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

/// 把配置里写的 `{page}` / `{limit}` / `{offset}` / `{pageIndex}` 换成真实数字。
///
/// 配置是静态 JSON，不能在 JSON 里做运算，所以占位符由代码替换。
/// 不做这一步，请求会带着字面量 `{page}` 发出去，服务器直接 400。
pub fn substitute_placeholders(url: &str, page: u32, limit: u32) -> String {
    // 偏移量分页的接口（360 壁纸）要的是 `start=0/20/40`，不是页码。
    // 让配置写 `{offset}`、由代码算，配置本身就不用做算术。
    let offset = page.saturating_sub(1).saturating_mul(limit);

    substitute_with_offset(url, page, limit, offset, page.saturating_sub(1))
}

/// 同上，但偏移量和页码都由调用方给。
///
/// 分开的理由：顺序固定的图源要的是**随机**起点（见 `SourceFilter::random_offset_max`
/// 和 `random_page_index_max`），而随机值不能藏在"替换函数"里——那样这个函数就没法测了。
///
/// 为什么除了 `{page}` 还要有 `{pageIndex}`：有整类图源（Gelbooru 的 `dapi`，
/// safebooru 走的就是它）的翻页参数 `pid` 是**从 0 开始的页码**，不是"第几页"，
/// 也不是字节偏移。实测 `pid=0/1/2` 对应连续的三段结果，每段挪动 `limit` 条——
/// 也就是说 `pid = page - 1`。少了这个占位符，配置里只能写 `pid={page}`，
/// 结果是第 1 页就跳到第二批，永远看不见开头那 20 张。
fn substitute_with_offset(
    url: &str,
    page: u32,
    limit: u32,
    offset: u32,
    page_index: u32,
) -> String {
    url.replace("{page}", &page.to_string())
        .replace("{pageIndex}", &page_index.to_string())
        .replace("{limit}", &limit.to_string())
        .replace("{offset}", &offset.to_string())
}

/// 在 `[0, max)` 里挑一个随机页码。
///
/// 和 `random_start_from` 是同一个病、两个药：那个给"偏移量分页"，这个给"页码分页"。
fn random_page_index_from(seed: u64, max: u32) -> u32 {
    if max == 0 {
        return 0;
    }
    (seed % max as u64) as u32
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

/// 把关键词里的空白换成下划线。
///
/// booru（safebooru / Gelbooru / Danbooru）的标签是 `hatsune_miku` 这种形态，
/// 词与词之间是 **AND**。`hatsune miku` 会被当成两个标签同时命中，实测 0 条；
/// `hatsune_miku` 20 条。用 `split_whitespace` 而不是 `replace(' ', "_")`，
/// 是为了把连续空白和制表符一起收掉。
fn to_tag_style(keyword: &str) -> String {
    keyword.split_whitespace().collect::<Vec<_>>().join("_")
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

/// 给标签补一个结尾 `*`，变成前缀匹配。
///
/// 已经带 `*` 的不重复补；空串原样返回——补成 `*` 会匹配全站任何图，
/// 那不是"放宽"，那是把筛选条件删了。
fn widen_tag(tag: &str) -> String {
    if tag.is_empty() || tag.ends_with('*') {
        return tag.to_string();
    }

    format!("{tag}*")
}

/// 拼出最终要请求的地址：精确匹配。
fn build_request_url(
    source: &WallpaperSource,
    page: u32,
    query: Option<&str>,
) -> Result<reqwest::Url, String> {
    build_url(source, page, query, false)
}

/// 拼出最终要请求的地址。
///
/// 四件事按顺序做：
/// 1. **选模板**：有关键词且图源支持搜索 → 用 `search.url`，否则用 `request.url`
/// 2. **算分页**：搜索用确定的页码；浏览用随机起点（见下面那段注释）
/// 3. **替换占位符**：`{page}` / `{limit}` / `{pageIndex}` / `{offset}` / `{query}`
/// 4. **交给 `Url::parse`** 做百分号编码和合法性校验——中文关键词、空格、`&`
///    全在这一步被正确处理，不用自己写编码
///
/// 为什么第三步不做编码、第四步才做：`Url::parse` 是唯一权威。
/// 手工拼一个"看起来对"的地址，遇到中文就会发出一个坏请求，
/// 而且错误信息会是"400 Bad Request"，完全看不出是编码问题。
///
/// `widen` 为 true 时把标签放宽成前缀匹配（见 `SearchSpec::wildcard_fallback`）。
/// 要不要放宽由**调用方**决定，不藏在这里自动重试：一次搜索最多打几次网络，
/// 必须有人能一眼数清。
fn build_url(
    source: &WallpaperSource,
    page: u32,
    query: Option<&str>,
    widen: bool,
) -> Result<reqwest::Url, String> {
    let keyword = query.map(str::trim).filter(|q| !q.is_empty());

    // 只有"真的会走搜索端点"才算搜索：图源没配 `search` 时关键词会被忽略，
    // 那种情况下仍然是浏览，仍然该有随机。
    let is_search = keyword.is_some() && source.search.is_some();

    // 顺序固定的图源要随机起点，否则"第 1 页"永远是那 20 张。
    //
    // **搜索不随机。** 随机解决的是"每次打开都看到同一批"，而搜索的结果集
    // 本来就是用户指定的那一个。safebooru 上这个 bug 很致命：`pid` 是 0 起页码、
    // 每页 20 条、`randomPageIndexMax` 是 8000，于是 `wuthering_waves`（总量几百条）
    // 在绝大多数取值下返回空数组（实测 `pid=5000` → `[]`）。用户看到"没搜到"，
    // 而真相是页码被扔到了结果集第 10 万条——**这不是没图，是我们把页码扔错了地方**。
    let offset = if is_search {
        page.saturating_sub(1).saturating_mul(DEFAULT_LIMIT)
    } else {
        match source.filter.random_offset_max {
            Some(max) => random_start_from(random_u64(), max, DEFAULT_LIMIT),
            None => page.saturating_sub(1).saturating_mul(DEFAULT_LIMIT),
        }
    };

    // 同一个病、两个药：页码分页的图源随机的是**页码**，不是偏移量。
    // 同样只作用于浏览。不配的话 `{pageIndex}` 就是 `page-1`，第 1 页永远是最新那 20 张——
    // 用户每次打开看到的都是同一批图，而界面上这和"缓存坏了"没法区分。
    let page_index = if is_search {
        page.saturating_sub(1)
    } else {
        match source.filter.random_page_index_max {
            Some(max) => random_page_index_from(random_u64(), max),
            None => page.saturating_sub(1),
        }
    };

    let raw = match (keyword, &source.search) {
        (Some(kw), Some(spec)) => {
            let mut mapped = search::to_source_query(kw, spec.english);
            // booru 的标签是下划线形态，空格会被当成"两个标签同时命中"→ 0 条
            if spec.underscore_tags {
                mapped = to_tag_style(&mapped);
            }
            if widen {
                mapped = widen_tag(&mapped);
            }
            substitute_with_offset(&spec.url, page, DEFAULT_LIMIT, offset, page_index)
                .replace("{query}", &mapped)
        }
        _ => substitute_with_offset(&source.request.url, page, DEFAULT_LIMIT, offset, page_index),
    };

    let raw = apply_sfw_query(&raw, &source.filter);

    reqwest::Url::parse(&raw).map_err(|e| format!("图源 {} 的请求地址不合法 {raw}: {e}", source.id))
}

/// 一次请求拿回来的东西。
#[derive(Debug)]
pub struct FetchPage {
    pub images: Vec<ImageRef>,
    /// 后端为了让结果不为空额外做了什么。`None` = 什么也没做，结果就是精确答案。
    ///
    /// 为什么要显式带回去：前缀扩展是**模糊**的，前端有义务说清这件事。
    /// 悄悄放宽条件、让用户以为精确搜到了，比搜不到更糟。
    pub notice: Option<String>,
}

/// 从指定图源拉一页图片。
///
/// `query` 为空或不传 = 浏览模式；非空且图源配了 `search` = 搜索模式。
///
/// 搜索有两条路：先按标签精确查；**精确查为空时**，如果图源声明了
/// `wildcardFallback`，再按前缀查一次。所以一次搜索最多两次请求。
pub async fn fetch_source_images(
    source: &WallpaperSource,
    page: u32,
    query: Option<&str>,
) -> Result<FetchPage, String> {
    let keyword = query.map(str::trim).filter(|q| !q.is_empty());

    let images = request_page(source, &build_request_url(source, page, keyword)?).await?;

    let exact = FetchPage {
        images,
        notice: None,
    };
    if !exact.images.is_empty() {
        return Ok(exact);
    }

    // 空结果才有资格放宽：
    // - 浏览模式（没有关键词）空了就是这一页空了，不该乱猜
    // - 图源没声明支持前缀匹配，说明它的接口不认 `*`，硬试只是白发一次请求
    let Some(spec) = &source.search else {
        return Ok(exact);
    };
    let Some(kw) = keyword else {
        return Ok(exact);
    };
    if !spec.wildcard_fallback {
        return Ok(exact);
    }

    // 扩展请求失败不改变结论：精确搜索确实是 0 条，这个答案已经可信。
    // 把它变成报错会让用户以为"搜索坏了"，而真实情况是"这个词没有图"。
    let widened = request_page(source, &build_url(source, page, keyword, true)?)
        .await
        .unwrap_or_default();

    if widened.is_empty() {
        return Ok(exact);
    }

    Ok(FetchPage {
        images: widened,
        notice: Some(format!(
            "没有标签恰好叫「{kw}」，以下是按前缀扩展的结果——可能包含相近标签"
        )),
    })
}

/// 真的发一次请求并解析。
///
/// 只吃一个拼好的地址：**"请求什么"和"怎么发"分开**，
/// 一次搜索最多打几次网络因此能在这两层之间数清楚。
async fn request_page(
    source: &WallpaperSource,
    url: &reqwest::Url,
) -> Result<Vec<ImageRef>, String> {
    // 只放行 GET；没配 method 视为默认 GET
    if let Some(m) = &source.request.method
        && !m.eq_ignore_ascii_case("GET")
    {
        return Err(format!("图源 {} 暂不支持 {} 方法", url, m));
    }

    // 发出去：先按系统代理，连接不通就直连再试一次
    let resp = match build_request(client(), source, url).send().await {
        Ok(resp) => resp,
        Err(proxy_err) => {
            if crate::net::system_proxy().is_some() {
                build_request(direct_client(), source, url)
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

    // 先读成文本再解析，而不是直接 `resp.json()`。两个理由，都是实测踩出来的：
    //
    // 1. 有的接口"查到没有结果"时返回 200 + **0 字节 body**（safebooru 就是这样）。
    //    直接 parse 会报 `EOF while parsing a value`，用户看到的是报错，
    //    而真相是"这个词没有图"。空 body 在这里等价于空列表。
    // 2. 有的接口出错时返回 XML 而不是 JSON（safebooru 的 `API limited due to abuse`）。
    //    把正文开头塞进错误信息，一眼能看出是服务端拒绝，不是我们的配置写错。
    let body = resp
        .text()
        .await
        .map_err(|e| format!("读取 {} 的响应失败: {}", url, e))?;

    let Some(value) = parse_body(url.as_str(), &body)? else {
        return Ok(Vec::new());
    };

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

/// 把响应正文解析成 JSON。
///
/// 返回 `Ok(None)` 表示"正文是空的"——这不是错误，是接口在说"这个查询没有结果"。
/// 调用方把它当空列表处理，用户才会看到"没有结果"而不是一句解析报错。
///
/// 为什么单独抽成函数：这段逻辑决定了"空白"和"坏数据"两种情况的区别，
/// 而那正是最容易写错、也最值得测的地方。抽出来就能不联网地测。
fn parse_body(url: &str, body: &str) -> Result<Option<serde_json::Value>, String> {
    if body.trim().is_empty() {
        return Ok(None);
    }

    serde_json::from_str(body).map(Some).map_err(|e| {
        let snippet: String = body.chars().take(160).collect();
        let snippet = snippet.replace(['\r', '\n'], " ");
        format!(
            "解析 {} 的 JSON 失败: {} — 响应开头: {}",
            url,
            e,
            snippet.trim()
        )
    })
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
            random_page_index_max: None,
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
            content_note: None,
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
            underscore_tags: false,
            wildcard_fallback: false,
        });
        source
    }

    /// 下划线转换本身：连续空白也要收掉，不能出现 `a__b`。
    #[test]
    fn tag_style_replaces_whitespace_with_underscores() {
        assert_eq!(to_tag_style("hatsune miku"), "hatsune_miku");
        assert_eq!(to_tag_style("  starry   sky "), "starry_sky");
        assert_eq!(to_tag_style("landscape"), "landscape");
        assert_eq!(to_tag_style(""), "");
    }

    /// safebooru 的搜索必须走"翻译 → 下划线"这条链。
    ///
    /// 这条挡的是真实事故：中文搜不到，因为 `初音 → "hatsune miku"` 直接进了
    /// `tags` 参数，而 booru 把空格当 AND 分隔，`hatsune` 和 `miku` 两个标签
    /// 同时命中的结果实测是 0 条。要的是 `hatsune_miku`。
    #[test]
    fn safebooru_search_turns_a_chinese_phrase_into_a_booru_tag() {
        let file = crate::sources::loader::load_builtin().unwrap();
        let source = file
            .sources
            .iter()
            .find(|s| s.id == "safebooru")
            .expect("内置图源里应该有 safebooru");

        assert!(
            source.search.as_ref().is_some_and(|s| s.underscore_tags),
            "safebooru 的 search 必须开 underscoreTags，否则中文多词一律 0 条"
        );

        let url = build_request_url(source, 1, Some("初音")).unwrap();
        let tags = url
            .query_pairs()
            .find(|(k, _)| k == "tags")
            .map(|(_, v)| v.into_owned())
            .expect("地址里应该有 tags");

        assert!(
            tags.contains("hatsune_miku"),
            "中文「初音」应该变成 hatsune_miku，实际 tags = {tags}"
        );
        assert!(
            !tags.contains("hatsune miku"),
            "带空格的形态在 booru 上是 0 条：{tags}"
        );
        // 搜索不该再套"必须 >=1920 宽"的服务端条件，否则冷门标签永远搜不到
        assert!(!tags.contains("width:"), "搜索模板不该带尺寸过滤：{tags}");
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
            .expect("请求应当成功")
            .images;
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

        // 缩略图必须真的比原图小。360 的 `url_thumb` / `url_mid` / `url` 指向同一个
        // 2560×1440 文件（2026-10-31 实测 309,650 B），网格一页 20 格就是 20 张全尺寸
        // 壁纸；`img_1280_800` 才是按尺寸产出的那张。只有量字节数能发现这种回退。
        let thumb_url = images[0].thumbnail_url.clone().expect("360 应该给出缩略图");
        assert_ne!(thumb_url, images[0].image_url, "缩略图地址和原图地址一样");
        let orig_len = client()
            .get(&images[0].image_url)
            .send()
            .await
            .expect("原图下载失败")
            .bytes()
            .await
            .expect("读不到原图正文")
            .len();
        let thumb_len = client()
            .get(&thumb_url)
            .send()
            .await
            .expect("缩略图下载失败")
            .bytes()
            .await
            .expect("读不到缩略图正文")
            .len();
        println!("  原图 {orig_len} B / 缩略图 {thumb_len} B（{thumb_url}）");
        assert!(
            thumb_len < orig_len,
            "缩略图（{thumb_len} B）不比原图（{orig_len} B）小——360 可能又改成同一张图了"
        );

        // 连拉三次，第一张不能都是同一张——否则"随机"是假的
        let mut firsts = std::collections::HashSet::new();
        firsts.insert(images[0].image_url.clone());
        for _ in 0..2 {
            let again = fetch_source_images(src, 1, None)
                .await
                .expect("重复请求应当成功")
                .images;
            firsts.insert(again[0].image_url.clone());
        }
        assert!(
            firsts.len() > 1,
            "三次请求拿到同一张图，随机起点没生效：{firsts:?}"
        );
    }

    /// lolicon 必须把图片主机**钉在镜像上**。
    ///
    /// 它返回的原图域名默认已经是 `i.pixiv.re`，但那是上游的默认值，改一次我们就完了：
    /// 实测 `i.pximg.net` 在这台机器上是**连接超时**（带不带 Referer 都 20 秒死），
    /// 而 `i.pixiv.re` 4/4 成功。所以要在配置里写死，不能赌默认值。
    ///
    /// 这条测试不联网，它只是挡住"有人顺手把这个参数删掉"。
    #[test]
    fn lolicon_pins_the_reachable_image_mirror() {
        let file = crate::sources::loader::load_builtin().unwrap();
        let source = file
            .sources
            .iter()
            .find(|s| s.id == "lolicon")
            .expect("内置图源里应该有 lolicon");

        let browse = build_request_url(source, 1, None).unwrap();
        assert!(
            browse.as_str().contains("proxy=i.pixiv.re"),
            "浏览地址没钉镜像，图片会指向被墙的 i.pximg.net：{browse}"
        );

        let searched = build_request_url(source, 1, Some("猫")).unwrap();
        assert!(
            searched.as_str().contains("proxy=i.pixiv.re"),
            "搜索地址没钉镜像：{searched}"
        );
    }

    /// lolicon 必须带内容提示。
    ///
    /// 它固定了 `r18=0`，但 pixiv 的"一般向"里仍有暴露程度较高的图。
    /// 用户有权在点开之前就知道，所以这条提示是产品要求，不是可选装饰。
    #[test]
    fn lolicon_carries_a_content_note() {
        let file = crate::sources::loader::load_builtin().unwrap();
        let source = file
            .sources
            .iter()
            .find(|s| s.id == "lolicon")
            .expect("内置图源里应该有 lolicon");

        assert!(
            source
                .content_note
                .as_deref()
                .is_some_and(|n| !n.is_empty()),
            "lolicon 必须声明 contentNote，否则界面上没有任何提示"
        );
    }

    /// nekos.best **不该**提供搜索。
    ///
    /// 它的 `/search` 是相关性模糊匹配，而且永不返回空集：实测
    /// `query=zzzznotexist&type=1&category=neko` 仍然返回 5 张。
    /// 图片类结果只带 `artist_name`（画师名），用户搜番名/角色名时拿到的是
    /// 画师名里含相近字母的图——也就是"搜到不符合标签的图"。
    ///
    /// 后端没有精确匹配能力 → 配了也只是"静默地不准"。按本项目的规矩，
    /// 不配 `search` 让界面把搜索框禁用，比配了但不准更诚实。
    #[test]
    fn nekos_best_does_not_advertise_search() {
        let file = crate::sources::loader::load_builtin().unwrap();
        let source = file
            .sources
            .iter()
            .find(|s| s.id == "nekos-best")
            .expect("内置图源里应该有 nekos-best");

        assert!(
            source.search.is_none(),
            "nekos.best 的搜索无法精确匹配标签，不该在界面上提供"
        );
        // 它的响应里其实带 dimensions，映射上以后分辨率筛选才有效
        assert!(
            source.response.width.is_some() && source.response.height.is_some(),
            "nekos.best 返回 dimensions，应该映射成宽高，否则分辨率筛选对它无效"
        );
    }

    /// 手动跑：`cargo test -- --ignored --nocapture live_safebooru`
    ///
    /// 一次覆盖四件事：`{pageIndex}` 的 0 起页码、`tags` 里 `rating:general` 和
    /// `width:>=1920` 有没有被正确编码、响应是不是**根数组**（`itemsPath` 缺省）、
    /// 缩略图和宽高有没有取到。
    #[tokio::test]
    #[ignore]
    async fn live_safebooru_source_works_end_to_end() {
        let sources = crate::sources::loader::load_builtin().unwrap();
        let src = sources
            .sources
            .iter()
            .find(|s| s.id == "safebooru")
            .expect("内置图源里应该有 safebooru");

        println!("请求地址 = {}", build_request_url(src, 1, None).unwrap());

        let images = fetch_source_images(src, 1, None)
            .await
            .expect("请求应当成功")
            .images;
        println!("拿到 {} 张", images.len());
        for img in images.iter().take(3) {
            println!(
                "  {}x{:?} thumb={}",
                img.width.unwrap_or(0),
                img.height,
                img.thumbnail_url.is_some()
            );
        }

        assert!(!images.is_empty(), "safebooru 应该返回图片");
        assert!(
            images.iter().all(|i| i.image_url.starts_with("https://")),
            "图片地址必须是 https"
        );
        assert!(
            images.iter().all(|i| i.thumbnail_url.is_some()),
            "缩略图必须取到，否则一页要拉几十 MB 原图"
        );
        assert!(
            images.iter().all(|i| i.width.is_some_and(|w| w >= 1920)),
            "width:>=1920 应该由服务端过滤生效"
        );

        // 连续拉三次第 1 页，首图不能都一样——否则"每次打开都是同一批图"，
        // 用户会以为缓存坏了。这条和 360 的那条是同一个不可退回的性质。
        let mut firsts = std::collections::HashSet::new();
        firsts.insert(images[0].image_url.clone());
        for _ in 0..2 {
            let again = fetch_source_images(src, 1, None)
                .await
                .expect("重复请求应当成功")
                .images;
            firsts.insert(again[0].image_url.clone());
        }
        assert!(
            firsts.len() > 1,
            "三次请求拿到同一张图，随机页码没生效：{firsts:?}"
        );

        // 搜索：中文关键词必须能出图。这条把"词表 → 下划线 → 服务端标签"整条链串起来，
        // 任何一环断了都是"这个图源不能搜索"这个用户可见的结论。
        println!(
            "搜索词 初音 -> {}",
            build_request_url(src, 1, Some("初音")).unwrap()
        );
        let searched = fetch_source_images(src, 1, Some("初音"))
            .await
            .expect("搜索请求应当成功");
        println!("搜「初音」拿到 {} 张", searched.images.len());
        assert!(
            !searched.images.is_empty(),
            "中文关键词搜不到东西：初音 应该被翻成 hatsune_miku"
        );
        // 精确命中就不该出现"按前缀扩展"的提示
        assert!(
            searched.notice.is_none(),
            "精确命中不该有扩展提示：{:?}",
            searched.notice
        );
    }

    /// 手动跑：`cargo test -- --ignored --nocapture live_prefix_fallback`
    ///
    /// 这条串起"搜不到 → 补星号 → 拿到图 → 界面能看到提示"整条链。
    ///
    /// 用的是 2026-11-02 的实测事实：safebooru 上 `elaina` 这个标签**不存在**
    /// （精确查返回 200 + 0 字节），`elaina_(majo_no_tabitabi)` 才是。
    /// 伊雷娜 → 词表翻成 `elaina` → 精确 0 条 → 前缀 `elaina*` → 有图。
    #[tokio::test]
    #[ignore]
    async fn live_safebooru_prefix_fallback_recovers_an_unknown_tag() {
        let sources = crate::sources::loader::load_builtin().unwrap();
        let src = sources
            .sources
            .iter()
            .find(|s| s.id == "safebooru")
            .expect("内置图源里应该有 safebooru");

        println!(
            "精确地址 = {}",
            build_request_url(src, 1, Some("伊雷娜")).unwrap()
        );
        println!(
            "扩展地址 = {}",
            build_url(src, 1, Some("伊雷娜"), true).unwrap()
        );

        let page = fetch_source_images(src, 1, Some("伊雷娜"))
            .await
            .expect("搜索请求应当成功");

        println!("拿到 {} 张，提示 = {:?}", page.images.len(), page.notice);
        assert!(
            !page.images.is_empty(),
            "伊雷娜 应该通过前缀扩展拿到 elaina_(majo_no_tabitabi) 的图"
        );
        assert!(
            page.notice.is_some(),
            "扩展来的结果必须带提示，否则前端会把它当成精确命中"
        );
    }

    /// 搜索的页码必须是**确定的**：第 1 页永远是结果集的前 20 条。
    ///
    /// 挡的是 2026-11-02 的实测事故：safebooru 的 `pid` 被随机到 `[0, 8000)`，
    /// 而 `wuthering_waves` 这类标签只有几百条（每页 20 条 → 十几页），
    /// 于是绝大多数搜索都返回 200 + `[]`。用户报的是"很多关键词都搜不到"，
    /// 而代码里一个错都没有——页码被随机到了不存在的地方，而且**没有任何报错**。
    #[test]
    fn search_uses_a_deterministic_page() {
        let file = crate::sources::loader::load_builtin().unwrap();
        let source = file
            .sources
            .iter()
            .find(|s| s.id == "safebooru")
            .expect("内置图源里应该有 safebooru");

        // 连要 8 次，pid 必须一直是 0：一次都不许随机
        for _ in 0..8 {
            let url = build_request_url(source, 1, Some("初音")).unwrap();
            let pid = url
                .query_pairs()
                .find(|(k, _)| k == "pid")
                .map(|(_, v)| v.into_owned())
                .expect("地址里应该有 pid");
            assert_eq!(pid, "0", "搜索第 1 页的 pid 必须是 0，实际 {pid}（{url}）");
        }

        // 翻页仍然要连续：第 3 页 → pid=2
        let page3 = build_request_url(source, 3, Some("初音")).unwrap();
        let pid = page3
            .query_pairs()
            .find(|(k, _)| k == "pid")
            .map(|(_, v)| v.into_owned())
            .expect("地址里应该有 pid");
        assert_eq!(pid, "2", "搜索第 3 页的 pid 应该是 2，实际 {pid}");
    }

    /// 修搜索不能把浏览的"每次换一批"一起修掉。
    ///
    /// 这是同一段代码里的两个相反要求，所以两条测试必须成对存在：
    /// 只有"随机"那条，搜索会静默返回空；只有"确定"那条，浏览会永远同一批图。
    #[test]
    fn browse_keeps_the_random_page_index() {
        let file = crate::sources::loader::load_builtin().unwrap();
        let source = file
            .sources
            .iter()
            .find(|s| s.id == "safebooru")
            .expect("内置图源里应该有 safebooru");

        let mut seen = std::collections::HashSet::new();
        for _ in 0..8 {
            let url = build_request_url(source, 1, None).unwrap();
            let pid = url
                .query_pairs()
                .find(|(k, _)| k == "pid")
                .map(|(_, v)| v.into_owned())
                .expect("地址里应该有 pid");
            seen.insert(pid);
        }

        assert!(
            seen.len() > 1,
            "八次浏览请求拿到了同一个 pid（{seen:?}），随机页码没生效"
        );
    }

    #[test]
    fn widen_tag_appends_exactly_one_star() {
        assert_eq!(widen_tag("lycoris"), "lycoris*");
        assert_eq!(widen_tag("lycoris*"), "lycoris*", "已经带了就别补第二个");
        assert_eq!(widen_tag(""), "", "空标签补成 * 会匹配全站");
    }

    /// 放宽后的地址里，星号必须落在 `tags` 上，而不是地址别的什么地方。
    ///
    /// `url` crate 可能把 `*` 编码成 `%2A`，两种写法服务端都认，
    /// 所以这里比较的是**解码后**的值，不去断言原始字符串长什么样。
    #[test]
    fn widened_search_puts_the_star_on_the_tag() {
        let file = crate::sources::loader::load_builtin().unwrap();
        let source = file
            .sources
            .iter()
            .find(|s| s.id == "safebooru")
            .expect("内置图源里应该有 safebooru");

        let url = build_url(source, 1, Some("lycoris"), true).unwrap();
        let tags = url
            .query_pairs()
            .find(|(k, _)| k == "tags")
            .map(|(_, v)| v.into_owned())
            .expect("地址里应该有 tags");

        assert!(tags.ends_with('*'), "tags 应该以星号结尾，实际 {tags:?}");
        assert!(
            tags.contains("lycoris_recoil"),
            "词表里的别名应该先翻成标签再补星号：{tags:?}"
        );
        assert!(
            tags.contains("rating:general"),
            "放宽的是关键词，不是安全过滤：{tags:?}"
        );
    }

    /// 内置图源必须真的声明了这个能力。
    ///
    /// 代码写好了但配置没开，等于这个功能不存在——而用户看不到任何差别，
    /// 只会继续报"搜不到"。
    #[test]
    fn safebooru_declares_the_prefix_fallback() {
        let file = crate::sources::loader::load_builtin().unwrap();
        let source = file
            .sources
            .iter()
            .find(|s| s.id == "safebooru")
            .expect("内置图源里应该有 safebooru");

        let search = source.search.as_ref().expect("safebooru 支持搜索");
        assert!(
            search.wildcard_fallback,
            "safebooru 的标签和口语差得远（lycoris → lycoris_recoil），必须允许前缀兜底"
        );
    }

    /// 手动跑：`cargo test -- --ignored --nocapture live_reported_keywords`
    ///
    /// 用户 2026-11-02 报的那几个"搜不到"的词，逐个走完整条链。
    ///
    /// 四个词的失败原因**不是同一个**，所以必须分开验：
    /// - `鸣潮` / `wuwa`：标签 `wuthering_waves` 存在，之前被随机 pid 扔到空页
    /// - `伊雷娜`：`elaina` 不存在，靠前缀兜底到 `elaina_(majo_no_tabitabi)`
    /// - `lycoris`：词表别名 → `lycoris_recoil`
    ///
    /// 请求之间留 500 ms：safebooru 连续快打会返回 XML 的 `API limited due to abuse`，
    /// 那是限流，不是功能坏了——测试要测的是功能。
    #[tokio::test]
    #[ignore]
    async fn live_reported_keywords_are_searchable() {
        let sources = crate::sources::loader::load_builtin().unwrap();
        let src = sources
            .sources
            .iter()
            .find(|s| s.id == "safebooru")
            .expect("内置图源里应该有 safebooru");

        for keyword in ["鸣潮", "wuwa", "伊雷娜", "lycoris"] {
            let url = build_request_url(src, 1, Some(keyword)).unwrap();
            let page = fetch_source_images(src, 1, Some(keyword))
                .await
                .unwrap_or_else(|e| panic!("「{keyword}」搜索失败：{e}"));

            println!(
                "「{keyword}」→ {} 张（notice={:?}）\n    精确地址 {url}",
                page.images.len(),
                page.notice
            );

            assert!(
                !page.images.is_empty(),
                "「{keyword}」搜不到图，用户会报同一个 bug：{url}"
            );

            std::thread::sleep(std::time::Duration::from_millis(500));
        }
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

    /// Gelbooru 系的 `pid` 是 0 起的页码：第 1 页要 0，第 3 页要 2。
    /// 写成 `pid={page}` 的话第 1 页就跳过开头那 20 张，而且**不会报错**。
    #[test]
    fn page_index_is_zero_based() {
        let u = "https://a.com/api?pid={pageIndex}&limit={limit}";
        assert!(substitute_placeholders(u, 1, 20).contains("pid=0"));
        assert!(substitute_placeholders(u, 3, 20).contains("pid=2"));
    }

    #[test]
    fn page_index_never_underflows() {
        let url = substitute_placeholders("https://a.com/api?pid={pageIndex}", 0, 20);
        assert!(url.contains("pid=0"), "{url}");
    }

    #[test]
    fn random_page_index_is_within_bounds() {
        for seed in 0..200u64 {
            let idx = random_page_index_from(seed, 8000);
            assert!(idx < 8000, "页码必须在上限内：{idx}");
        }
    }

    #[test]
    fn random_page_index_covers_many_pages() {
        // 只有几个取值就等于没随机
        let picks: std::collections::HashSet<u32> = (0..50u64)
            .map(|s| random_page_index_from(s, 8000))
            .collect();
        assert!(picks.len() > 40, "随机页码太集中：{}", picks.len());
    }

    #[test]
    fn degenerate_random_page_bounds_do_not_panic() {
        assert_eq!(random_page_index_from(123, 0), 0);
        assert_eq!(random_page_index_from(123, 1), 0);
    }

    /// 新接的图源必须是"每次换一批"，不能是"第 1 页永远同一批"。
    ///
    /// 这条测试挡的是真实发生过的事故：safebooru 的 `pid=0` 永远是最新 20 张，
    /// 用户每次打开、每次切回这个图源看到的都是同一批图，
    /// 而界面上这和"缓存坏了"完全无法区分（关了重开也还是那些图）。
    ///
    /// 判定标准就一句：**这个图源的第 1 页是不是永远相同的结果？**
    #[test]
    fn deterministic_first_page_sources_must_declare_randomisation() {
        let file = crate::sources::loader::load_builtin().unwrap();
        let source = file
            .sources
            .iter()
            .find(|s| s.id == "safebooru")
            .expect("内置图源里应该有 safebooru");

        assert!(
            source.filter.random_page_index_max.is_some(),
            "safebooru 用 pid 分页，不配 randomPageIndexMax 的话第 1 页永远是最新那 20 张"
        );

        // 配了还不够，得真的体现在请求地址里：连要两次不能总是同一个 pid
        let mut seen = std::collections::HashSet::new();
        for _ in 0..8 {
            let url = build_request_url(source, 1, None).unwrap();
            let pid = url
                .query_pairs()
                .find(|(k, _)| k == "pid")
                .map(|(_, v)| v.into_owned())
                .expect("地址里应该有 pid");
            seen.insert(pid);
        }
        assert!(
            seen.len() > 1,
            "八次请求拿到了同一个 pid（{seen:?}），随机页码没生效"
        );
    }

    /// 缩略图字段不许是"名字像缩略图、内容却是原图"的那一个。
    ///
    /// 挡的是 360 这个坑：`url_thumb` / `url_mid` / `url` 三个字段**指向同一个文件**
    /// （2560×1440、309,650 B，2026-10-31 实测），而它是默认图源、界面一页 20 格，
    /// 于是网格一口气解码 20 张全尺寸壁纸——空闲内存比"无图"多约 267 MB，流量也按原图算。
    /// 真正按尺寸产出的是 `img_<宽>_<高>` 那组字段（`img_1280_800` 等）。
    ///
    /// 判据不是"两个字段名不同"（`url` 和 `url_thumb` 也不同，照样是同一个文件），
    /// 而是"映射到按尺寸命名的字段"。这条测试挡不住 CDN 哪天偷偷改行为，那一层靠
    /// `live_360_source_works_end_to_end`（手动跑、联网）量字节数。
    #[test]
    fn builtin_thumbnail_mapping_is_not_a_field_that_returns_the_original() {
        let file = crate::sources::loader::load_builtin().unwrap();
        let source = file
            .sources
            .iter()
            .find(|s| s.id == "360")
            .expect("内置图源里应该有 360");

        let field = source
            .response
            .thumbnail_url
            .as_deref()
            .expect("360 声明了缩略图，界面要用它铺网格");

        assert_ne!(
            field, source.response.image_url,
            "缩略图和原图不能是同一个字段"
        );
        assert!(
            field.starts_with("img_"),
            "360 的缩略图要用按尺寸命名的 img_<宽>_<高> 字段，实际是 {field:?}；\
             url_thumb / url_mid 实测返回的就是原图"
        );
    }

    /// 空正文 = 没有结果，不是解析失败。
    ///
    /// safebooru 搜不到东西时就是 200 + 0 字节。要是这里返回 Err，
    /// 用户搜一个不存在的番名会看到"解析 JSON 失败"，而不是"没有结果"。
    #[test]
    fn empty_body_means_no_results() {
        assert!(parse_body("https://a.com", "").unwrap().is_none());
        assert!(parse_body("https://a.com", "  \r\n\t ").unwrap().is_none());
    }

    #[test]
    fn valid_json_body_is_parsed() {
        assert_eq!(
            parse_body("https://a.com", "[]").unwrap(),
            Some(serde_json::json!([]))
        );
    }

    /// 服务端返回 XML 拒绝时，错误信息里必须有正文开头，
    /// 否则用户看到的是一个无法归因的 "expected value"。
    #[test]
    fn non_json_body_is_reported_with_its_opening() {
        let xml = "<?xml version=\"1.0\"?><response success=\"false\" reason=\"API limited due to abuse\"/>";

        let message = parse_body("https://a.com/x", xml).unwrap_err();

        assert!(message.contains("https://a.com/x"), "要带地址：{message}");
        assert!(message.contains("API limited"), "要带正文开头：{message}");
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
