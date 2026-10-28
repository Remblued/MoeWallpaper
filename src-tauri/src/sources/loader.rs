use crate::sources::model::SourcesFile;

const BUILTIN: &str = include_str!("../../../sources/builtin/example.sources.json");

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
        assert_eq!(file.sources.len(), 1);

        let source = &file.sources[0];
        assert_eq!(source.id, "example");
        assert!(!source.enabled);
        assert!(source.filter.sfw_only);
    }
}