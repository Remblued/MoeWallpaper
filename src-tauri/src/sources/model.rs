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
    /// 请求地址。`{limit}` 和 `{page}` 是占位符，由代码替换，JSON 本身不做替换。
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
