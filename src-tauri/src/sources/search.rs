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

    // 词表优先，**纯英文也查**。
    //
    // 原来的写法是"ASCII 就直接小写返回，不过词表"，理由是"英文已经很准确"。
    // 实测下来这句话是错的：`wuwa`、`lycoris` 都是用户会打的英文，也都不等于
    // booru 的标签（`wuthering_waves` / `lycoris_recoil`）。跳过词表等于
    // 把"词表里明明写了"的别名全部作废，而且不作废的那部分用户也发现不了——
    // 界面上只有一句"没搜到"。
    //
    // ASCII 先归一成小写再查：用户会打 `Lycoris`，词表里存的是小写。
    // 中文没有大小写，原样查。
    let lookup = if keyword.is_ascii() {
        keyword.to_ascii_lowercase()
    } else {
        keyword.to_string()
    };

    if let Some(hit) = table().and_then(|t| t.get(&lookup)) {
        return hit.clone();
    }

    // 查不到就原样发出去，不猜、不拆词、不音译
    lookup
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

    /// 英文别名也要走词表。
    ///
    /// 挡的是"`wuwa` 搜不到鸣潮"：用户打的是英文缩写，booru 上的标签是
    /// `wuthering_waves`。词表里写了这条，但代码曾经一律跳过 ASCII 输入，
    /// 于是它永远命中不了——测试要挡的正是这种"配了等于没配"。
    #[test]
    fn ascii_aliases_in_the_table_are_applied() {
        assert_eq!(to_source_query("wuwa", true), "wuthering waves");
        assert_eq!(to_source_query("Lycoris", true), "lycoris recoil");
        assert_eq!(to_source_query("re0", true), "re:zero");
    }

    /// 但词表里没有的英文不能被"猜"成别的词。
    #[test]
    fn unknown_ascii_passes_through_lowercased() {
        assert_eq!(to_source_query("Miku", true), "miku");
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
