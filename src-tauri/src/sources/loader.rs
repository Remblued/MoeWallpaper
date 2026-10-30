use crate::sources::model::SourcesFile;

const BUILTIN: &str = include_str!("../../../sources/builtin/sources.json");

pub fn load_builtin() -> Result<SourcesFile, String> {
    serde_json::from_str(BUILTIN).map_err(|e| format!("解析内置图源配置失败: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_sources_parse() {
        let file = load_builtin().unwrap();

        assert_eq!(file.schema_version, 1);
        assert!(
            !file.sources.is_empty(),
            "一个图源都没有，应用打开就是空的下拉框"
        );
    }

    /// 每个内置图源都必须具备"能真的拉到图"的最低条件。
    ///
    /// 这些断言看起来啰嗦，但它们挡的是最丢人的那种 bug：
    /// 打包发版了，用户点开发现下拉框里有个图源是空的 / 地址是空字符串。
    #[test]
    fn every_builtin_source_is_usable() {
        let file = load_builtin().unwrap();

        for source in &file.sources {
            assert!(!source.id.is_empty(), "有图源没有 id");
            assert!(
                !source.name.is_empty(),
                "图源 {} 没有名字，下拉框里会显示空白",
                source.id
            );
            assert!(
                source.request.url.starts_with("https://")
                    || source.request.url.starts_with("http://"),
                "图源 {} 的地址不是 http(s)：{}",
                source.id,
                source.request.url
            );
            assert!(
                !source.response.image_url.is_empty(),
                "图源 {} 没配 imageUrl，拉到数据也取不出图片",
                source.id
            );
        }
    }

    /// id 必须唯一：它是缓存目录名、前端选中值、开关持久化的依据。
    #[test]
    fn builtin_source_ids_are_unique() {
        let file = load_builtin().unwrap();

        let mut ids: Vec<&str> = file.sources.iter().map(|s| s.id.as_str()).collect();
        let total = ids.len();
        ids.sort_unstable();
        ids.dedup();

        assert_eq!(ids.len(), total, "有重复的图源 id");
    }

    /// 配了搜索的图源，`search.url` 里必须有 `{query}`。
    ///
    /// 没有的话关键词会被**静默丢掉**——用户搜"风景"，拿到的是随机图，
    /// 程序不报错、界面不提示，这是最难查的一类 bug。
    /// 这条测试就是防它的。
    #[test]
    fn search_urls_actually_use_the_keyword() {
        let file = load_builtin().unwrap();

        for source in &file.sources {
            let Some(spec) = &source.search else {
                continue;
            };

            assert!(
                spec.url.contains("{query}"),
                "图源 {} 的 search.url 里没有 {{query}}：{}",
                source.id,
                spec.url
            );
            assert!(
                spec.url.starts_with("https://") || spec.url.starts_with("http://"),
                "图源 {} 的 search.url 不是 http(s)：{}",
                source.id,
                spec.url
            );
        }
    }
}
