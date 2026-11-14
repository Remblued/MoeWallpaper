//! 图源配置的**加载期**校验。
//!
//! `v0.1.x` 的图源只有内置那 5 份，格式错了由写代码的人当场发现。
//! `v0.2.0` 起用户会把自己的 JSON 丢进配置目录，于是"没有报错的那种错"
//! 全都会变成用户来报的 bug：字段名大小写写错 → 那一项**静默失效**；
//! 忘了写 `sfwQuery` → 内容限制**静默消失**；地址里占位符打错一个字母 →
//! 请求带着字面量 `{pages}` 发出去。
//!
//! 所以这里的规矩是：
//!
//! - **错误（`Error`）让这条图源作废**，但绝不让程序启动失败——一个坏文件
//!   不该把另外四个能用的图源一起拖下水；
//! - **警告（`Warning`）只提示**，图源照常加载；
//! - 每条问题都必须带"哪个文件、哪个图源、怎么改"，因为看的人不知道我们的代码。
//!
//! 校验分三层，顺序有讲究：
//!
//! 1. **语法**（`serde_json::from_str`）：连 JSON 都不是，只能报一行。
//! 2. **结构**（扫 `Value`）：未知字段、`schemaVersion`。这一步要用 `Value`，
//!    因为 `*Source` 结构体上没写 `deny_unknown_fields`（写了的话，用户多留一个
//!    `_comment` 键就整份配置报销，太脆），只能在解析后自己比一遍字段表。
//! 3. **语义**（`check_source`）：id 形态、占位符、安全声明这些"能解析但会出错"的。

use std::collections::BTreeSet;

use serde::Serialize;
use serde_json::Value;

use crate::sources::model::{SourceFilter, SourcesFile, WallpaperSource};

/// 一个配置文件最多几条图源。
///
/// 不是为了防恶意（配置是用户自己的），是为了防"一次导入 3000 条"把下拉框
/// 变成一个没法用的列表——那时候用户只会觉得程序卡了。
pub const MAX_SOURCES_PER_FILE: usize = 50;

/// 一个配置文件最大多少字节。超过就不再读，直接报错。
///
/// 正常的图源配置是 1–5 KB。定 256 KB 是给"带一大堆注释的自嗨版"留足余量，
/// 同时挡住"不小心把图片拖进文件夹"这种手滑。
pub const MAX_FILE_BYTES: usize = 256 * 1024;

const MAX_ID_LEN: usize = 64;
const MAX_NAME_LEN: usize = 48;

/// 随机上限超过这个值就警告：服务端通常把它当滥用。
///
/// 依据是实测：safebooru 的 `pid=10000` 正常、`pid=15000` 起返回
/// `API limited due to abuse`（`docs/SOURCES-CN.md` §0.1 第 6 条）。
/// 不拦，只提醒——别人的接口不一定有这个脾气。
const RANDOM_MAX_WARN: u32 = 20_000;

/// 当前程序认识的图源格式版本。
pub const SCHEMA_VERSION: u32 = 1;

/// 问题严重程度。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// 这条图源不能加载。
    Error,
    /// 能加载，但多半不是你要的。
    Warning,
}

/// 一条能显示给用户的问题。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Problem {
    pub severity: Severity,
    /// 出问题的文件。内置配置写 `内置图源`，用户文件写文件名（不带目录）。
    pub file: String,
    /// 涉及哪个图源。文件级问题（语法、版本号）为 `None`。
    pub source_id: Option<String>,
    pub message: String,
}

impl Problem {
    pub fn error(file: &str, source_id: Option<&str>, message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Error,
            file: file.to_string(),
            source_id: source_id.map(str::to_string),
            message: message.into(),
        }
    }

    pub fn warning(file: &str, source_id: Option<&str>, message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Warning,
            file: file.to_string(),
            source_id: source_id.map(str::to_string),
            message: message.into(),
        }
    }

    pub fn is_error(&self) -> bool {
        self.severity == Severity::Error
    }
}

/// 有人给这个 id 判过 `Error` 吗？——有就必须把这条图源丢掉。
pub fn has_error_for(problems: &[Problem], source_id: &str) -> bool {
    problems
        .iter()
        .any(|p| p.is_error() && p.source_id.as_deref() == Some(source_id))
}

/// 一个文件的校验结果。
pub struct Checked {
    /// 解析成功时的结构。文件级 `Error` 时是 `None`。
    pub parsed: Option<SourcesFile>,
    /// 文件级 + 每条图源级的问题，全都带 `file` / `source_id`。
    pub problems: Vec<Problem>,
}

impl Checked {
    /// 文件级问题里有 `Error` 吗（=整份文件不能生效）。
    pub fn file_is_broken(&self) -> bool {
        self.problems
            .iter()
            .any(|p| p.is_error() && p.source_id.is_none())
    }

    /// 这条图源的问题。
    pub fn problems_for<'a>(&'a self, source_id: &'a str) -> impl Iterator<Item = &'a Problem> {
        self.problems
            .iter()
            .filter(move |p| p.source_id.as_deref() == Some(source_id))
    }
}

/// 校验一份**用户**配置文件的文本。
///
/// 不会返回 `Err`：所有失败都是 `Checked.problems` 里的 `Error`。
/// 调用方只需要看 `file_is_broken()` 和 `has_error_for()`。
pub fn check_file_text(text: &str, file: &str) -> Checked {
    let mut problems = Vec::new();

    if text.len() > MAX_FILE_BYTES {
        problems.push(Problem::error(
            file,
            None,
            format!(
                "文件 {} KB，超过 {} KB 上限——这多半不是一份图源配置",
                text.len() / 1024,
                MAX_FILE_BYTES / 1024
            ),
        ));
        return Checked {
            parsed: None,
            problems,
        };
    }

    let value: Value = match serde_json::from_str(text) {
        Ok(v) => v,
        Err(e) => {
            problems.push(Problem::error(file, None, format!("JSON 语法错误：{e}")));
            return Checked {
                parsed: None,
                problems,
            };
        }
    };

    scan_file(&value, file, &mut problems);

    // 结构扫描过一遍再交给 serde：类型错（`"enabled": "yes"`）只有它能报。
    let parsed = match serde_json::from_value::<SourcesFile>(value) {
        Ok(f) => Some(f),
        Err(e) => {
            problems.push(Problem::error(
                file,
                None,
                format!("字段类型或必填项不对：{e}"),
            ));
            None
        }
    };

    if let Some(file_spec) = &parsed {
        problems.extend(check_sources_file(file_spec, file, false));
    }

    Checked { parsed, problems }
}

/// 校验一份已经解析好的配置。内置图源走这条（`builtin = true`）。
///
/// `builtin` 只影响一件事：用户配置里出现 `http://` 要警告（用户的图多半也是
/// http，界面里会全是裂图），而内置 360 的 http 是已知且已经用
/// `filter.httpsUpgrade` 处理过的——对用户反复提一件他改不了的事，只会让
/// 问题列表变成噪音。
pub fn check_sources_file(file_spec: &SourcesFile, file: &str, builtin: bool) -> Vec<Problem> {
    let mut problems = Vec::new();

    if file_spec.schema_version != SCHEMA_VERSION {
        problems.push(Problem::error(
            file,
            None,
            format!(
                "schemaVersion 是 {}，本程序只认 {}",
                file_spec.schema_version, SCHEMA_VERSION
            ),
        ));
    }

    if file_spec.sources.len() > MAX_SOURCES_PER_FILE {
        problems.push(Problem::error(
            file,
            None,
            format!(
                "一个文件最多 {} 个图源，这里有 {} 个",
                MAX_SOURCES_PER_FILE,
                file_spec.sources.len()
            ),
        ));
    }

    for source in &file_spec.sources {
        problems.extend(check_source(source, file, builtin));
    }

    problems
}

/// 校验一条图源。`file` 只用来填进 `Problem`。
pub fn check_source(source: &WallpaperSource, file: &str, builtin: bool) -> Vec<Problem> {
    let mut problems = Vec::new();
    let id = Some(source.id.as_str());

    // ── id ────────────────────────────────────────────────
    // id 是下拉框的 value、localStorage 的键、报错文案里的名字。
    // 限制成 ASCII 小写是**为了让它在任何地方都不会被误解**（大小写、全角、
    // 空格、路径分隔符全都不进这个字段）。
    if source.id.trim().is_empty() {
        problems.push(Problem::error(file, id, "id 不能为空"));
    } else if source.id.len() > MAX_ID_LEN {
        problems.push(Problem::error(
            file,
            id,
            format!("id 太长（{} 字符，最多 {MAX_ID_LEN}）", source.id.len()),
        ));
    } else if !id_looks_sane(&source.id) {
        problems.push(Problem::error(
            file,
            id,
            format!(
                "id \"{}\" 不合法：只能用 a-z、0-9、`.`、`_`、`-`，并以字母或数字开头",
                source.id
            ),
        ));
    }

    // ── name ──────────────────────────────────────────────
    let name = source.name.trim();
    if name.is_empty() {
        problems.push(Problem::error(
            file,
            id,
            "name 不能为空（下拉框里会是一行空白）",
        ));
    } else if source.name.chars().count() > MAX_NAME_LEN {
        problems.push(Problem::warning(
            file,
            id,
            format!(
                "name 有 {} 个字，超过 {MAX_NAME_LEN} 会被下拉框截断",
                source.name.chars().count()
            ),
        ));
    }

    // ── request ───────────────────────────────────────────
    if let Some(method) = &source.request.method
        && !method.eq_ignore_ascii_case("GET")
    {
        problems.push(Problem::error(
            file,
            id,
            format!("只支持 GET，配置里写的是 {method}"),
        ));
    }

    check_url(
        &source.request.url,
        "request.url",
        file,
        id,
        &mut problems,
        true,
        builtin,
    );

    // `{query}` 只会在 `search.url` 里被替换。写在 `request.url`（浏览地址）里
    // 永远不会生效，请求会带着字面量 `{query}` 发出去——服务端多半返回 400 或空列表，
    // 而界面上只显示"没搜到"。
    if source.request.url.contains("{query}") {
        problems.push(Problem::error(
            file,
            id,
            "request.url 里有 {query}，但浏览时它不会被替换（只有 search.url 里的才会）——\
             会带着字面量发出去。浏览地址删掉它，搜索地址写到 search.url 里",
        ));
    }

    // ── response ──────────────────────────────────────────
    if source.response.image_url.trim().is_empty() {
        problems.push(Problem::error(
            file,
            id,
            "response.imageUrl 不能为空（拉到数据也取不出图片）",
        ));
    }

    match (&source.response.width, &source.response.height) {
        (Some(_), None) | (None, Some(_)) => problems.push(Problem::warning(
            file,
            id,
            "width 和 height 只配了一个：尺寸是成对的，少一个的话分辨率/比例筛选对它无效",
        )),
        _ => {}
    }

    if source.response.resolution.is_some()
        && source.response.width.is_some()
        && source.response.height.is_some()
    {
        problems.push(Problem::warning(
            file,
            id,
            "width/height 和 resolution 都配了：以 width/height 为准，resolution 不会生效",
        ));
    }

    // ── filter ────────────────────────────────────────────
    check_sfw(&source.filter, file, id, &mut problems);

    if let Some(max) = source.filter.random_page_index_max
        && max > RANDOM_MAX_WARN
    {
        problems.push(Problem::warning(
            file,
            id,
            format!(
                "randomPageIndexMax = {max} 偏大：有的接口把它当滥用（safebooru 实测 15000 起拒绝），\
                 偶发报错最难查"
            ),
        ));
    }
    if let Some(max) = source.filter.random_offset_max
        && max > RANDOM_MAX_WARN
    {
        problems.push(Problem::warning(
            file,
            id,
            format!("randomOffsetMax = {max} 偏大：接口的 total 可能就到不了那么深"),
        ));
    }

    // ── search ────────────────────────────────────────────
    if let Some(spec) = &source.search {
        check_url(
            &spec.url,
            "search.url",
            file,
            id,
            &mut problems,
            false,
            builtin,
        );
        if !spec.url.contains("{query}") {
            problems.push(Problem::error(
                file,
                id,
                "search.url 里没有 {query}：关键词会被静默丢掉，用户搜什么都拿到随机图",
            ));
        }
    }

    if let Some(homepage) = &source.homepage
        && !is_http_url(homepage)
    {
        problems.push(Problem::warning(
            file,
            id,
            format!("homepage 不是 http(s) 地址：{homepage}"),
        ));
    }

    problems
}

/// 内容安全声明必须被满足。
///
/// **这是 `v0.2.0` 最重要的一条校验**：以前"图源没写 `sfwQuery`"只有 code review
/// 能发现，现在配置是用户写的，没人 review。要么给参数，要么声明接口本身安全，
/// 不能什么都不说。
fn check_sfw(filter: &SourceFilter, file: &str, id: Option<&str>, out: &mut Vec<Problem>) {
    let query = filter.sfw_query.as_deref().unwrap_or("").trim();

    if query.is_empty() && !filter.sfw_safe_endpoint {
        out.push(Problem::error(
            file,
            id,
            "没有声明内容安全条件：有分级参数的接口写 filter.sfwQuery（例如 purity=100、r18=0）；\
             接口本身只给安全内容的写 filter.sfwSafeEndpoint: true。两者都没有的图源一律不加载",
        ));
    }

    if !query.is_empty() {
        if query.starts_with('?') || query.starts_with('&') {
            out.push(Problem::warning(
                file,
                id,
                format!("sfwQuery 只写裸参数，不要带 ? 或 &：现在是 \"{query}\""),
            ));
        }
        if query.contains('{') {
            out.push(Problem::warning(
                file,
                id,
                "sfwQuery 里不做占位符替换，{...} 会原样发出去",
            ));
        }
    }
}

/// 地址模板的通用检查：能不能解析、是不是 http(s)、占位符有没有打错。
fn check_url(
    url: &str,
    field: &str,
    file: &str,
    id: Option<&str>,
    out: &mut Vec<Problem>,
    required: bool,
    builtin: bool,
) {
    let url = url.trim();

    if url.is_empty() {
        if required {
            out.push(Problem::error(file, id, format!("{field} 不能为空")));
        }
        return;
    }

    // 占位符先查：`Url::parse` 会把 `{pages}` 百分号编码成一个合法地址，
    // 于是拼错一个字母的后果是"请求 200，但服务端看不懂"——最难查的一种。
    for name in placeholder_blocks(url) {
        if !PLACEHOLDERS.contains(&name.as_str()) {
            out.push(Problem::error(
                file,
                id,
                format!(
                    "{field} 里有不认识的占位符 {{{name}}}；可用的只有 {}",
                    PLACEHOLDERS.join(" / ")
                ),
            ));
        }
    }

    if !is_http_url(url) {
        out.push(Problem::error(
            file,
            id,
            format!("{field} 必须是 http(s) 地址：{url}"),
        ));
        return;
    }

    // 内置图源的 http 是已知的（360，图片已经用 httpsUpgrade 处理），
    // 对用户反复提一件他改不了的事只会让问题列表变成噪音。
    if url.starts_with("http://") && !builtin {
        out.push(Problem::warning(
            file,
            id,
            format!(
                "{field} 用的是 http：接口能连，但如果图片地址也是 http，界面里加载不出来\
                 （WebView 不加载 http 图片）。图片能走 https 的话加 filter.httpsUpgrade"
            ),
        ));
    }
}

/// 配置里能用的占位符。和 `fetch::substitute_placeholders` 一一对应。
pub const PLACEHOLDERS: &[&str] = &["page", "pageIndex", "offset", "limit", "query"];

/// 把 `{xxx}` 里的 `xxx` 都取出来（不含花括号）。
fn placeholder_blocks(url: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = url;
    while let Some(start) = rest.find('{') {
        let after = &rest[start + 1..];
        let Some(end) = after.find('}') else {
            break;
        };
        out.push(after[..end].to_string());
        rest = &after[end + 1..];
    }
    out
}

fn is_http_url(url: &str) -> bool {
    let lower = url.to_ascii_lowercase();
    (lower.starts_with("http://") || lower.starts_with("https://")) && lower.len() > 8
}

fn id_looks_sane(id: &str) -> bool {
    let mut chars = id.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !first.is_ascii_lowercase() && !first.is_ascii_digit() {
        return false;
    }
    chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '.' | '_' | '-'))
}

// ── 结构扫描：未知字段 ──────────────────────────────────────

/// 顶层允许的键。
///
/// **加字段时必须同步改这里**，否则用户写了新字段会被当成"未知字段"警告。
/// `fields_match_the_tables` 这条测试会盯着这张表别落后于结构体。
const FILE_KEYS: &[&str] = &["schemaVersion", "sources"];
const SOURCE_KEYS: &[&str] = &[
    "id",
    "name",
    "enabled",
    "homepage",
    "contentNote",
    "request",
    "response",
    "filter",
    "search",
    "custom",
];
const REQUEST_KEYS: &[&str] = &["url", "method", "headers"];
const RESPONSE_KEYS: &[&str] = &[
    "itemsPath",
    "imageUrl",
    "thumbnailUrl",
    "pageUrl",
    "tags",
    "width",
    "height",
    "resolution",
];
const FILTER_KEYS: &[&str] = &[
    "sfwQuery",
    "sfwSafeEndpoint",
    "httpsUpgrade",
    "randomOffsetMax",
    "randomPageIndexMax",
];
const SEARCH_KEYS: &[&str] = &["url", "english", "underscoreTags", "wildcardFallback"];

fn scan_file(value: &Value, file: &str, out: &mut Vec<Problem>) {
    let Some(root) = value.as_object() else {
        out.push(Problem::error(
            file,
            None,
            "配置文件最外层必须是对象 {\"schemaVersion\": 1, \"sources\": [...]}",
        ));
        return;
    };

    scan_keys(root, FILE_KEYS, file, None, "", out);

    // schemaVersion 单独判断，因为"版本比程序新"和"版本写错了"要说两句话。
    match root.get("schemaVersion") {
        None => out.push(Problem::error(file, None, "缺少 schemaVersion，写 1")),
        Some(Value::Number(n)) if n.as_u64() == Some(SCHEMA_VERSION as u64) => {}
        Some(Value::Number(n)) if n.as_u64().is_some_and(|v| v > SCHEMA_VERSION as u64) => {
            out.push(Problem::error(
                file,
                None,
                format!(
                    "schemaVersion = {}，比本程序认识的 {SCHEMA_VERSION} 新：\
                     这份配置是更新版本的程序写的，请升级 MoeWallpaper",
                    n
                ),
            ));
        }
        Some(other) => out.push(Problem::error(
            file,
            None,
            format!("schemaVersion 必须是 {SCHEMA_VERSION}，现在是 {other}"),
        )),
    }

    let Some(list) = root.get("sources") else {
        out.push(Problem::error(file, None, "缺少 sources 数组"));
        return;
    };
    let Some(list) = list.as_array() else {
        out.push(Problem::error(file, None, "sources 必须是数组"));
        return;
    };

    if list.is_empty() {
        out.push(Problem::warning(file, None, "这个文件里一个图源都没有"));
    }

    for (index, item) in list.iter().enumerate() {
        let Some(object) = item.as_object() else {
            out.push(Problem::error(
                file,
                None,
                format!("sources[{index}] 不是对象"),
            ));
            continue;
        };

        // 先拿 id 只是为了把问题挂到具体图源上；拿不到就用"第 N 个"。
        let id = object.get("id").and_then(Value::as_str).unwrap_or("");
        let label = if id.is_empty() { None } else { Some(id) };

        scan_keys(object, SOURCE_KEYS, file, label, "", out);

        for (key, allowed) in [
            ("request", REQUEST_KEYS),
            ("response", RESPONSE_KEYS),
            ("filter", FILTER_KEYS),
            ("search", SEARCH_KEYS),
        ] {
            if let Some(nested) = object.get(key).and_then(Value::as_object) {
                scan_keys(nested, allowed, file, label, key, out);
            }
        }
    }
}

fn scan_keys(
    object: &serde_json::Map<String, Value>,
    allowed: &[&str],
    file: &str,
    source_id: Option<&str>,
    context: &str,
    out: &mut Vec<Problem>,
) {
    for key in object.keys() {
        // `_` / `$` 开头当成注释：JSON 没有注释，用户想写说明总得有个地方。
        // 这条不是宽容，是必须——不然第一份"带注释的模板"就会被自己的校验骂。
        if key.starts_with('_') || key.starts_with('$') {
            continue;
        }
        if allowed.contains(&key.as_str()) {
            continue;
        }

        let hint = match allowed
            .iter()
            .find(|known| normalize(key) == normalize(known))
        {
            Some(known) => format!("，是不是想写 \"{known}\"？"),
            None => String::new(),
        };
        let where_ = if context.is_empty() {
            String::new()
        } else {
            format!("{context} 里")
        };

        out.push(Problem::warning(
            file,
            source_id,
            format!("{where_}有个不认识的字段 \"{key}\"{hint}这一项被忽略了"),
        ));
    }
}

/// 去掉 `_` `-` 再转小写，用来猜"是不是拼错了"。
fn normalize(key: &str) -> String {
    key.chars()
        .filter(|c| *c != '_' && *c != '-')
        .flat_map(char::to_lowercase)
        .collect()
}

/// 一条配置文件里出现的所有 id（用于查重）。
pub fn ids_in(file_spec: &SourcesFile) -> BTreeSet<&str> {
    file_spec.sources.iter().map(|s| s.id.as_str()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 从一段 JSON 造一条图源。
    fn source(json: &str) -> WallpaperSource {
        serde_json::from_str(json).expect("测试用的 JSON 应该是合法的")
    }

    /// 按"用户写的配置"校验一条图源。
    fn check(source: &WallpaperSource) -> Vec<Problem> {
        check_source(source, "f", false)
    }

    fn messages(problems: &[Problem]) -> Vec<String> {
        problems.iter().map(|p| p.message.clone()).collect()
    }

    fn errors(problems: &[Problem]) -> Vec<&Problem> {
        problems.iter().filter(|p| p.is_error()).collect()
    }

    fn warnings(problems: &[Problem]) -> Vec<&Problem> {
        problems
            .iter()
            .filter(|p| !p.is_error())
            .collect::<Vec<_>>()
    }

    /// 一份"什么都配了"的最小合法图源。
    fn good() -> WallpaperSource {
        source(
            r#"{
                "id": "demo",
                "name": "演示",
                "request": { "url": "https://a.invalid/list?page={page}&limit={limit}" },
                "response": { "imageUrl": "url", "width": "w", "height": "h" },
                "filter": { "sfwQuery": "purity=100" }
            }"#,
        )
    }

    #[test]
    fn a_complete_source_is_clean() {
        let problems = check_source(&good(), "test.json", false);
        assert!(
            problems.is_empty(),
            "干净的图源不该有任何问题：{:?}",
            messages(&problems)
        );
    }

    #[test]
    fn enabled_defaults_to_true() {
        // 默认 false 的后果：用户写了配置、下拉框里没有、还不报错。
        assert!(good().enabled);
    }

    #[test]
    fn id_must_be_ascii_lowercase_slug() {
        for bad in ["", " ", "中文", "Demo", "../etc", "a b"] {
            let mut s = good();
            s.id = bad.to_string();
            assert!(
                !errors(&check(&s)).is_empty(),
                "id {bad:?} 应该被拒，但它过了"
            );
        }

        for ok in ["demo", "a1", "my.source-1", "x_y"] {
            let mut s = good();
            s.id = ok.to_string();
            assert!(
                errors(&check(&s)).is_empty(),
                "id {ok:?} 是合法的，却被拒了"
            );
        }
    }

    #[test]
    fn every_source_must_declare_content_safety() {
        let mut s = good();
        s.filter.sfw_query = None;
        let problems = check(&s);
        assert_eq!(errors(&problems).len(), 1, "{:?}", messages(&problems));
        assert!(messages(&problems)[0].contains("sfwQuery"));

        // 声明"接口本身只给安全内容"也算数
        s.filter.sfw_safe_endpoint = true;
        assert!(errors(&check(&s)).is_empty());

        // 空白字符串等于没写——"看起来配了、其实没配"是最糟的形态
        let mut blank = good();
        blank.filter.sfw_query = Some("  ".to_string());
        assert!(!errors(&check(&blank)).is_empty());
    }

    #[test]
    fn sfw_query_should_not_carry_a_separator() {
        let mut s = good();
        s.filter.sfw_query = Some("?purity=100".to_string());
        let problems = check(&s);
        assert!(errors(&problems).is_empty(), "只是写法难看，不该拦");
        assert_eq!(warnings(&problems).len(), 1);
    }

    #[test]
    fn misspelled_placeholder_is_an_error() {
        let mut s = good();
        s.request.url = "https://a.invalid/list?page={pages}&limit={limit}".to_string();
        let problems = check(&s);
        assert_eq!(errors(&problems).len(), 1);
        assert!(messages(&problems)[0].contains("{pages}"));
    }

    #[test]
    fn query_in_browse_url_needs_a_search_block() {
        let mut s = good();
        s.request.url = "https://a.invalid/search?q={query}".to_string();
        let problems = check(&s);
        assert!(
            errors(&problems)
                .iter()
                .any(|p| p.message.contains("{query}")),
            "{:?}",
            messages(&problems)
        );

        // 配了 search 之后，浏览地址里再出现 {query} 仍然是错的（它不会被替换）
        s.search = Some(
            serde_json::from_str(r#"{"url":"https://a.invalid/s?q={query}","english":false}"#)
                .unwrap(),
        );
        assert!(
            errors(&check(&s))
                .iter()
                .any(|p| p.message.contains("request.url 里有 {query}"))
        );
    }

    #[test]
    fn search_url_without_the_keyword_is_an_error() {
        let mut s = good();
        s.search = Some(
            serde_json::from_str(r#"{"url":"https://a.invalid/s?q=nothing","english":false}"#)
                .unwrap(),
        );
        let problems = check(&s);
        assert!(
            errors(&problems)
                .iter()
                .any(|p| p.message.contains("search.url 里没有 {query}"))
        );
    }

    #[test]
    fn only_get_is_supported() {
        let mut s = good();
        s.request.method = Some("POST".to_string());
        assert!(
            errors(&check(&s))
                .iter()
                .any(|p| p.message.contains("只支持 GET"))
        );
    }

    #[test]
    fn half_a_size_pair_is_a_warning() {
        let mut s = good();
        s.response.height = None;
        let problems = check(&s);
        assert!(errors(&problems).is_empty());
        assert!(messages(&problems)[0].contains("width 和 height"));
    }

    #[test]
    fn non_http_urls_are_rejected() {
        let mut s = good();
        s.request.url = "file:///C:/x.json".to_string();
        assert!(!errors(&check(&s)).is_empty());
    }

    #[test]
    fn unknown_key_gets_a_suggestion() {
        let text = r#"{
            "schemaVersion": 1,
            "sources": [{
                "id": "demo",
                "name": "演示",
                "request": { "url": "https://a.invalid/l?page={page}" },
                "response": { "imageUrl": "url" },
                "filter": { "sfw_query": "purity=100" }
            }]
        }"#;
        let checked = check_file_text(text, "f.json");

        let hints: Vec<&Problem> = checked
            .problems
            .iter()
            .filter(|p| p.message.contains("sfw_query"))
            .collect();

        assert_eq!(hints.len(), 1, "{:?}", messages(&checked.problems));
        // 拼错那个字段 → 安全条件真的没了 → 必须同时报 error
        assert!(hints[0].message.contains("sfwQuery"));
        assert!(
            has_error_for(&checked.problems, "demo"),
            "sfw_query 拼错等于没配安全条件，必须作废"
        );
    }

    #[test]
    fn comment_keys_are_ignored() {
        let text = r#"{
            "_comment": "随便写点说明",
            "$schema": "https://example.invalid/schema.json",
            "schemaVersion": 1,
            "sources": []
        }"#;
        let checked = check_file_text(text, "f.json");
        assert!(
            checked
                .problems
                .iter()
                .all(|p| !p.message.contains("_comment")),
            "{:?}",
            messages(&checked.problems)
        );
    }

    #[test]
    fn newer_schema_version_is_reported_as_such() {
        let text = r#"{ "schemaVersion": 2, "sources": [] }"#;
        let checked = check_file_text(text, "f.json");
        assert!(checked.file_is_broken());
        assert!(
            messages(&checked.problems)
                .iter()
                .any(|m| m.contains("请升级")),
            "{:?}",
            messages(&checked.problems)
        );
    }

    #[test]
    fn broken_json_is_one_clear_message() {
        let checked = check_file_text("{ \"schemaVersion\": 1, ", "f.json");
        assert!(checked.parsed.is_none());
        assert_eq!(checked.problems.len(), 1);
        assert!(checked.problems[0].message.contains("JSON 语法错误"));
    }

    #[test]
    fn wrong_type_is_reported_by_serde() {
        let text = r#"{
            "schemaVersion": 1,
            "sources": [{
                "id": "demo", "name": "演示", "enabled": "yes",
                "request": { "url": "https://a.invalid/l" },
                "response": { "imageUrl": "url" }
            }]
        }"#;
        let checked = check_file_text(text, "f.json");
        assert!(checked.parsed.is_none(), "类型错应该整份文件不生效");
        assert!(checked.file_is_broken());
    }

    #[test]
    fn placeholder_blocks_are_found() {
        assert_eq!(
            placeholder_blocks("https://a/{page}/{limit}?x={offset}"),
            vec!["page", "limit", "offset"]
        );
        assert!(placeholder_blocks("https://a.invalid/nothing").is_empty());
        // 没闭合的花括号不该 panic
        assert!(placeholder_blocks("https://a/{page").is_empty());
    }

    /// 字段表别落后于结构体：把一条"每个字段都填满"的图源序列化出来，
    /// 顶层键必须和 `SOURCE_KEYS` 完全一致；嵌套的也一样。
    ///
    /// 这条测试的价值：以后有人给 `SourceFilter` 加字段却忘了加到
    /// `FILTER_KEYS`，用户写这个字段就会被骂"未知字段"——那是会挨投诉的 bug。
    #[test]
    fn fields_match_the_tables() {
        let full = source(
            r#"{
                "id": "demo", "name": "演示", "enabled": true, "homepage": "https://a.invalid",
                "contentNote": "x", "custom": false,
                "request": { "url": "https://a.invalid/l?p={page}", "method": "GET", "headers": {"A":"b"} },
                "response": {
                    "itemsPath": "d", "imageUrl": "u", "thumbnailUrl": "t", "pageUrl": "p",
                    "tags": "g", "width": "w", "height": "h", "resolution": "r"
                },
                "filter": {
                    "sfwQuery": "purity=100", "sfwSafeEndpoint": false, "httpsUpgrade": true,
                    "randomOffsetMax": 100, "randomPageIndexMax": 100
                },
                "search": {
                    "url": "https://a.invalid/s?q={query}", "english": true,
                    "underscoreTags": true, "wildcardFallback": true
                }
            }"#,
        );

        let value = serde_json::to_value(&full).unwrap();
        assert_eq!(
            keys_of(&value),
            sorted(SOURCE_KEYS),
            "顶层字段表和结构体不一致"
        );

        for (field, table) in [
            ("request", REQUEST_KEYS),
            ("response", RESPONSE_KEYS),
            ("filter", FILTER_KEYS),
            ("search", SEARCH_KEYS),
        ] {
            let nested = value.get(field).expect("上面那份 JSON 里应该都有");
            assert_eq!(
                keys_of(nested),
                sorted(table),
                "{field} 的字段表和结构体不一致"
            );
        }
    }

    /// 取出一个 JSON 对象的键，排序后和静态表比较。
    fn keys_of(value: &Value) -> Vec<String> {
        let mut keys: Vec<String> = value
            .as_object()
            .expect("应该是对象")
            .keys()
            .cloned()
            .collect();
        keys.sort_unstable();
        keys
    }

    /// 静态表是照结构体声明顺序写的，比较前先排序。
    fn sorted(table: &[&str]) -> Vec<String> {
        let mut keys: Vec<String> = table.iter().map(|k| (*k).to_string()).collect();
        keys.sort_unstable();
        keys
    }

    #[test]
    fn normalize_maps_the_common_typos() {
        assert_eq!(normalize("sfw_query"), normalize("sfwQuery"));
        assert_eq!(normalize("thumbnail-url"), normalize("thumbnailUrl"));
        assert_eq!(normalize("itemsPath"), normalize("itemspath"));
    }
}
