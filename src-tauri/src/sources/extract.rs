use serde_json::Value;

use crate::sources::model::ResponseMapping;

/// 空路径返回 value 本身；任何一级不存在则返回 None。
fn get_by_path<'a>(value: &'a Value, path: &str) -> Option<&'a Value> {
    //1.path 空 返回 Some(value)
    if path.is_empty() {
        return Some(value);
    }

    //2.current设为value
    let mut current = value;

    //3.for 每一段 key 切开
    for key in path.split('.') {
        current = current.get(key)?;
    }

    //4.返回Some(current)
    Some(current)
}

/// 从响应里提取出来的一条图片引用。
#[derive(Debug, PartialEq)]
pub struct ImageRef {
    pub image_url: String,
    pub thumbnail_url: Option<String>,
    pub page_url: Option<String>,
    /// 原图尺寸。图源不提供时为 `None`——**未知不等于小**，筛选时不能当 0 处理。
    pub width: Option<u32>,
    pub height: Option<u32>,
}

/// 取某个路径上的字符串值；路径不存在或不是字符串都返回 None。
fn string_at<'a>(item: &'a Value, path: &str) -> Option<&'a str> {
    get_by_path(item, path)?.as_str()
}

/// 取某个路径上的整数；路径不存在、不是整数、超出 u32 范围都返回 None。
///
/// 数字和数字字符串都收：有的 API 返回 `1920`，有的返回 `"1920"`。
/// 只认数字的话，字符串型图源的宽高会静默变成 None，分辨率筛选永远筛不出东西，
/// 而且不报任何错——"安静地不起作用"是最难查的一类 bug。
fn number_at(item: &Value, path: &str) -> Option<u32> {
    let value = get_by_path(item, path)?;

    if let Some(n) = value.as_u64() {
        return u32::try_from(n).ok();
    }

    value.as_str()?.trim().parse::<u32>().ok()
}

/// 按图源配置的映射规则，从一份 API 响应里提取图片列表。
///
/// 错误处理的粒度：
/// - 列表路径找不到 / 不是数组 → Err（配置错或 API 变了，用户必须知道）
/// - 单条缺 image_url → 跳过该条（一条坏数据不该毁掉整页）
/// - 空数组 → Ok(vec![])（翻到最后一页是正常情况，不是错误）
pub fn extract_images(
    mapping: &ResponseMapping,
    response: &Value,
) -> Result<Vec<ImageRef>, String> {
    let items = match mapping.items_path.as_deref() {
        None | Some("") => response,
        Some(path) => {
            get_by_path(response, path).ok_or_else(|| format!("响应里找不到列表路径 {path:?}"))?
        }
    };

    let array = items
        .as_array()
        .ok_or_else(|| "列表路径指向的不是数组".to_string())?;

    let mut out = Vec::with_capacity(array.len());

    for item in array {
        let Some(image_url) = string_at(item, &mapping.image_url) else {
            continue;
        };

        out.push(ImageRef {
            image_url: image_url.to_string(),
            thumbnail_url: mapping
                .thumbnail_url
                .as_deref()
                .and_then(|path| string_at(item, path))
                .map(str::to_string),
            page_url: mapping
                .page_url
                .as_deref()
                .and_then(|path| string_at(item, path))
                .map(str::to_string),
            width: mapping.width.as_deref().and_then(|p| number_at(item, p)),
            height: mapping.height.as_deref().and_then(|p| number_at(item, p)),
        });
    }

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = include_str!("../../../sources/examples/example.response.json");

    fn sample() -> Value {
        serde_json::from_str(SAMPLE).unwrap()
    }
    fn mapping(items_path: &str) -> ResponseMapping {
        ResponseMapping {
            items_path: if items_path.is_empty() {
                None
            } else {
                Some(items_path.to_string())
            },
            image_url: "file_url".to_string(),
            thumbnail_url: Some("sample_url".to_string()),
            page_url: Some("post_url".to_string()),
            tags: Some("tags".to_string()),
            width: Some("width".to_string()),
            height: Some("height".to_string()),
        }
    }

    // 路径不存在 → None
    #[test]
    fn missing_path_returns_none() {
        let v = sample();
        assert!(get_by_path(&v, "data.nope").is_none());
        assert!(get_by_path(&v, "totally.wrong").is_none());
    }

    // 多层路径 + 数字比较
    #[test]
    fn nested_path_finds_value() {
        let v: Value = serde_json::from_str(r#"{"a":{"b":{"c":42}}}"#).unwrap();
        assert_eq!(get_by_path(&v, "a.b.c").and_then(|x| x.as_i64()), Some(42));
    }

    // 空路径 → 返回整个对象
    #[test]
    fn empty_path_returns_whole_object() {
        let v = sample();
        assert_eq!(
            get_by_path(&v, ""),
            Some(&v),
            "Empty path should return the root object"
        );
    }
    // "data" → 是数组
    #[test]
    fn array_value_is_supported() {
        let v = serde_json::json!({"data":["apple","banana"]});
        assert_eq!(
            get_by_path(&v, "data"),
            Some(&serde_json::json!(["apple", "banana"])),
            "Should be able to get the whole array"
        );
    }
    // "data.0" → 必须是 None（记录"不支持数组下标"）
    #[test]
    fn array_index_returns_none() {
        let v = serde_json::json!({"data":["apple","banana"]});
        assert!(
            get_by_path(&v, "data.0").is_none(),
            "Array index access should return None"
        );
    }

    #[test]
    fn extracts_all_items() {
        let v = sample();
        let images = extract_images(&mapping("data"), &v).unwrap();
        assert_eq!(images.len(), 3);
        assert!(images[0].thumbnail_url.is_some());
        assert!(images[2].thumbnail_url.is_none());
    }

    // 决策 ①：配置错必须报错
    #[test]
    fn missing_items_path_errors() {
        let result = extract_images(&mapping("nope"), &sample());
        assert!(result.is_err());
    }

    // 决策 ②：不是数组必须报错
    #[test]
    fn non_array_errors() {
        let v = serde_json::json!({"data": {"a": 1}});
        assert!(extract_images(&mapping("data"), &v).is_err());
    }

    // 决策 ③：跳过，不是失败
    #[test]
    fn skips_items_without_image_url() {
        let v = serde_json::json!({
            "data": [
                {"file_url": "a"},
                {"x": 1},
                {"file_url": "c"},
            ]
        });
        let images = extract_images(&mapping("data"), &v).unwrap();
        assert_eq!(images.len(), 2);
    }

    // 决策 ④：空 ≠ 错误
    #[test]
    fn empty_array_is_ok() {
        let v = serde_json::json!({"data": []});
        let result = extract_images(&mapping("data"), &v);
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn dimensions_work_for_numbers_and_numeric_strings() {
        let v = serde_json::json!({
            "data": [
                {"file_url": "a", "width": 1920, "height": 1080},
                {"file_url": "b", "width": "2560", "height": "1440"},
                {"file_url": "c", "width": "nope", "height": 1080},
                {"file_url": "d"},
            ]
        });
        let images = extract_images(&mapping("data"), &v).unwrap();

        assert_eq!(
            (images[0].width, images[0].height),
            (Some(1920), Some(1080))
        );
        // 字符串形态的数字也要认，否则这类图源的分辨率筛选会静默失效
        assert_eq!(
            (images[1].width, images[1].height),
            (Some(2560), Some(1440))
        );
        // 坏值只影响它自己那个字段，不影响同一条的另一个字段
        assert_eq!((images[2].width, images[2].height), (None, Some(1080)));
        // 没有就是 None，不能变成 0
        assert_eq!((images[3].width, images[3].height), (None, None));
    }

    #[test]
    fn unconfigured_dimensions_are_none() {
        let mut map = mapping("data");
        map.width = None;
        map.height = None;

        let v = serde_json::json!({"data":[{"file_url":"a","width":1920,"height":1080}]});
        let images = extract_images(&map, &v).unwrap();

        assert_eq!((images[0].width, images[0].height), (None, None));
    }
}
