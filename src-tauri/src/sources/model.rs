//! 图源配置的数据结构。
//!
//! 对应 `sources/builtin/*.json`。修改这里的字段 = 修改图源格式，
//! 属于破坏性变更：需要同步改 `sources/builtin/` 下的文件、
//! 前端 `src/lib/sources/schema.ts`，以及 README 的示例。

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// HTTP 请求描述。
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpRequestSpec {
    /// 请求地址。占位符由代码替换，JSON 本身不做替换：
    ///
    /// - `{page}` / `{limit}`：页码和每页数量
    /// - `{pageIndex}`：`page - 1`，给"页码从 0 开始"的接口用（Gelbooru 的 `pid`）
    /// - `{offset}`：`(page - 1) * limit`，给偏移量分页的接口用（360 壁纸的 `start`）
    pub url: String,
    pub method: Option<String>,
    #[serde(default)]
    pub headers: HashMap<String, String>,
}

/// 响应映射：告诉代码"列表在哪、每张图的字段叫什么"。
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResponseMapping {
    /// 图片数组在响应 JSON 里的位置；省略表示响应本身就是数组。
    pub items_path: Option<String>,
    /// 原图 URL（必填）。
    pub image_url: String,
    pub thumbnail_url: Option<String>,
    pub page_url: Option<String>,
    pub tags: Option<String>,
    /// 原图宽度的字段路径。不配 = 这个图源不提供尺寸，分辨率筛选对它无效。
    ///
    /// 数字和数字字符串都认（有的 API 返回 `1920`，有的返回 `"1920"`）——
    /// 只认数字的话，字符串型的图源会静默地永远筛不出东西，而且没有任何报错。
    pub width: Option<String>,
    /// 原图高度的字段路径。规则同 `width`。
    pub height: Option<String>,
    /// **合在一起**的尺寸字段路径，形如 `"2560x1440"`。
    ///
    /// 有些接口不分开给宽高，只给一个 `resolution` 字符串（360 壁纸就是这样）。
    /// 配了它，代码会拆成宽高；同时配了 `width`/`height` 时以那两个为准。
    pub resolution: Option<String>,
}

/// 图源的附加过滤。
///
/// **为什么不是 `sfwOnly: true` 这种布尔开关**：
/// 布尔字段看起来像"安全保护"，但只要没有代码消费它，它就只是一句口号，
/// 比没有更糟——读配置的人会以为有保护。
///
/// 真正能保证安全的是**请求参数**（wallhaven 的 `purity=100`、lolicon 的 `r18=0`），
/// 所以这里直接存参数，由代码注入。字段因此是被消费的，不是摆设。
#[derive(Debug, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct SourceFilter {
    /// 发请求时由代码拼到地址末尾的安全参数，例如 `purity=100`、`r18=0`。
    ///
    /// 只写裸参数，不要带 `?` 或 `&`——由代码判断用哪个连接。
    /// 需要参数限制的图源必须写；本身只提供安全内容的图源留空。
    ///
    /// 写在这里而不是直接塞进 `request.url`，是为了让"安全"这件事在配置里
    /// 一眼可见，而且加图源的人不会忘。
    pub sfw_query: Option<String>,
    /// 把图片地址从 `http://` 升级成 `https://`。
    ///
    /// **为什么必须有这个开关**：有的接口（360 壁纸）只支持 http，但它返回的图片
    /// 在 https 下同样能取到。而界面跑在安全上下文里，WebView 不会加载 http 图片——
    /// 表现出来就是"列表拿到了，图全是裂的"，且控制台里只有一行 mixed content 警告。
    ///
    /// 只作用于图片和缩略图；页面链接不改，它是要交给浏览器打开的。
    #[serde(default)]
    pub https_upgrade: bool,
    /// 把 `{offset}` 换成**随机起点**，用于"顺序固定、但支持偏移量"的图源。
    ///
    /// 360 壁纸就是这样：第 1 页永远是那 20 张，翻到第 2 页也是固定的。
    /// 配上这个值后，每次请求从 `[0, random_offset_max)` 里随机取一个 `limit` 的倍数，
    /// 于是"翻页"变成"换一批"——效果对齐 wallhaven 的 `sorting=random`。
    ///
    /// 填"想覆盖的总量"：接口 `total` 是 10000、每页 20，就填 8000。
    #[serde(default)]
    pub random_offset_max: Option<u32>,
    /// 把 `{pageIndex}` 换成**随机页码**，用于"用 0 起页码分页"的图源。
    ///
    /// 和 `random_offset_max` 是同一个病、两个药：safebooru（Gelbooru `dapi`）的 `pid`
    /// 是页码，`pid=0` 永远是"最新那 20 张"。不配这个值，用户每次打开、每次切回这个
    /// 图源看到的都是同一批图——**界面上这和"缓存坏了"完全无法区分**。
    ///
    /// 所以新接图源时必须问自己一句：第 1 页是不是永远同一批？
    /// 是就配这个（或 `random_offset_max`），别等用户来报"图怎么不换"。
    ///
    /// 值 = 想覆盖的页数。**不要贪大**：safebooru 实测 `pid=10000`（第 20 万条）
    /// 还正常、`pid=15000` 起它返回 XML 的 `API limited due to abuse`，
    /// 所以这里填 8000 而不是几万。
    #[serde(default)]
    pub random_page_index_max: Option<u32>,
}

/// 搜索能力。
///
/// **不配这个块的图源 = 不支持搜索**，界面上搜索框对它禁用。
/// 这比"配了但静默无效"好：用户能看见边界在哪。
///
/// 为什么单独给一个 `url` 而不是复用 `request.url`：
/// 有些站点的搜索是**另一个端点**。nekos.best 浏览走 `/api/v2/neko`，
/// 搜索走 `/api/v2/search` 且必须带 `type` 参数——一个地址模板表达不了两件事。
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchSpec {
    /// 搜索用的完整地址模板，必须包含 `{query}`。
    ///
    /// `{page}` / `{limit}` 同样可用。关键词会被百分号编码后填进去。
    pub url: String,
    /// 关键词要不要先翻成英文。
    ///
    /// - `true`：走 `search/terms.json` 词表（wallhaven、nekos.best 只吃英文）
    /// - `false`：原样传（lolicon 走 pixiv 标签，中文直接可用）
    #[serde(default)]
    pub english: bool,
    /// 关键词里的空白要不要换成下划线。
    ///
    /// **booru 类站点必须开这个**：它们的标签是 `hatsune_miku` 这种下划线形态，
    /// 多个词之间是 **AND**。发 `hatsune miku` 会被理解成"同时有 `hatsune` 和 `miku`
    /// 两个标签"，实测 safebooru 返回 0 条；而 `hatsune_miku` 返回 20 条。
    ///
    /// 为什么一定要转：`search/terms.json` 里有 43/149 条译名带空格
    /// （初音 → `hatsune miku`、原神 → `genshin impact`）。
    /// 不转的话，中文搜索**逢多词必 0 条**，表现是"这个图源不能搜索"。
    #[serde(default)]
    pub underscore_tags: bool,
}

/// 单个图源。
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WallpaperSource {
    /// 稳定标识，用于缓存目录名；发布后不要改。
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub enabled: bool,
    pub homepage: Option<String>,
    /// 内容提示，比如"尺度较大"。有值就会显示在图源名旁边。
    ///
    /// **为什么要这个字段**：`lolicon` 即使固定 `r18=0`，pixiv 的"一般向"里仍然有
    /// 暴露程度较高的图。用户有权在点开之前就知道，而不是打开大图才发现。
    ///
    /// 它是**提示**不是过滤器：本项目没有任何机器判定的能力分级，
    /// 所以这里只写人能看懂的一句话，不假装能拦住什么。
    #[serde(default)]
    pub content_note: Option<String>,
    pub request: HttpRequestSpec,
    pub response: ResponseMapping,
    #[serde(default)]
    pub filter: SourceFilter,
    /// 不配 = 不支持搜索。
    #[serde(default)]
    pub search: Option<SearchSpec>,
}

/// `sources/builtin/*.json` 的顶层结构。
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourcesFile {
    pub schema_version: u32,
    pub sources: Vec<WallpaperSource>,
}
