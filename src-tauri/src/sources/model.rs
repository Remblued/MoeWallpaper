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

/// 过滤规则。
///
/// 默认值是 `sfwOnly: true`，而且是**故意的**：
/// 配置里没写这个字段时，必须按最保守的规则处理。
/// 想改成"默认不过滤"，等于改产品底线，不是改代码风格。
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SourceFilter {
    pub sfw_only: bool,
}

impl Default for SourceFilter {
    fn default() -> Self {
        Self { sfw_only: true }
    }
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
}

/// `sources/builtin/*.json` 的顶层结构。
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourcesFile {
    pub schema_version: u32,
    pub sources: Vec<WallpaperSource>,
}
