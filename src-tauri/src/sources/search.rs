//! 关键词翻译：把用户输入的中文换成图源认得的英文标签。
//!
//! 为什么需要：实测过，wallhaven 用中文搜返回**空数组**（不是报错，是"查无此事"），
//! 用户会以为程序坏了。nekos.best 更直接，中文关键词直接 400。
//! 只有 lolicon（pixiv 标签）能吃中文。
//!
//! 为什么用本地词表而不是翻译 API：
//! - 不依赖第三方，断网也能用
//! - 不把用户的关键词发给外人
//! - 翻译 API 的免费额度不稳，失败还得降级，等于多一种失败模式
//!
//! 代价是覆盖不了长尾词——查不到就原样发出去，由 UI 提示用户改用英文。

use std::collections::HashMap;
use std::sync::OnceLock;

const TERMS: &str = include_str!("../../../search/terms.json");

#[derive(serde::Deserialize)]
struct TermsFile {
    terms: HashMap<String, String>,
}

fn table() -> Option<&'static HashMap<String, String>> {
    static TABLE: OnceLock<Option<HashMap<String, String>>> = OnceLock::new();

    TABLE
        .get_or_init(|| {
            // 词表坏了不该让搜索整体失效，退化成"不翻译"即可。
            serde_json::from_str::<TermsFile>(TERMS)
                .map(|f| f.terms)
                .ok()
        })
        .as_ref()
}

/// 把关键词转成图源需要的语言。
///
/// - `english` 为 false 的图源（如 lolicon）原样返回，不做任何处理
/// - 命中词表就换成英文；没命中就原样返回，**不猜、不拆词、不音译**
///
/// 为什么"不猜"：猜错的代价是搜出一堆不相干的图，比搜不到更让人困惑。
pub fn to_source_query(keyword: &str, english: bool) -> String {
    let keyword = keyword.trim();
    if keyword.is_empty() || !english {
        return keyword.to_string();
    }

    // 英文已经很准确，不必过词表
    if keyword.is_ascii() {
        return keyword.to_ascii_lowercase();
    }

    table()
        .and_then(|t| t.get(keyword))
        .cloned()
        .unwrap_or_else(|| keyword.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn translates_common_words() {
        assert_eq!(to_source_query("风景", true), "landscape");
        assert_eq!(to_source_query("女孩子", true), "girl");
        assert_eq!(to_source_query("和服", true), "kimono");
    }

    #[test]
    fn translates_anime_and_game_names() {
        assert_eq!(to_source_query("原神", true), "genshin impact");
        assert_eq!(to_source_query("蕾姆", true), "rem");
        assert_eq!(to_source_query("和泉纱雾", true), "sagiri izumi");
        assert_eq!(to_source_query("火影忍者", true), "naruto");
    }

    #[test]
    fn unknown_chinese_passes_through() {
        // 长尾词不猜，原样发出去
        assert_eq!(
            to_source_query("某个不存在的番名", true),
            "某个不存在的番名"
        );
    }

    #[test]
    fn english_is_lowercased_not_translated() {
        assert_eq!(to_source_query("Sakura", true), "sakura");
    }

    #[test]
    fn sources_that_read_chinese_are_left_alone() {
        // lolicon / pixiv 标签本身就吃中文
        assert_eq!(to_source_query("风景", false), "风景");
    }

    #[test]
    fn trims_surrounding_whitespace() {
        assert_eq!(to_source_query("  猫  ", true), "cat");
    }

    /// 词表必须能被解析，而且不能是空的。
    /// 这条挡的是"JSON 里少个逗号，所有翻译静默失效"。
    #[test]
    fn term_table_parses_and_is_not_empty() {
        let t = table().expect("search/terms.json 解析失败");

        assert!(t.len() > 50, "词表太小了，只有 {} 条", t.len());
        assert_eq!(t.get("风景").map(String::as_str), Some("landscape"));
    }
}
