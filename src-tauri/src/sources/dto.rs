//! 跨过 IPC 边界的数据形状。
//!
//! 为什么不让 `ImageRef` 直接返回给前端：
//! `ImageRef` 是**内部**结构，字段可以随重构随便改；
//! `ImageDto` 是**契约**，一旦前端按它写完界面，改名就是破坏性变更。
//! 两者分开，中间这层转换就是"缓冲垫"。

use serde::Serialize;

use crate::sources::extract::ImageRef;

/// 一条可以直接给前端渲染的图片信息。
///
/// 字段全部非空——"没有缩略图就用原图"这种兜底判断**只在这里做一次**，
/// 前端不需要写 `if (thumbnailUrl == null)`。
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageDto {
    pub image_url: String,
    pub thumbnail_url: String,
    pub page_url: Option<String>,
    /// 原图宽高，图源不提供时为 `None`。
    ///
    /// **故意不兜底成 0**：0 会被当成"特别小"，分辨率筛选就会把未知尺寸的图
    /// 全筛掉，而用户看到的是"这个图源筛完就空了"——把"不知道"伪装成"很小"，
    /// 是最容易骗过 review 的一种错。
    pub width: Option<u32>,
    pub height: Option<u32>,
}

impl From<ImageRef> for ImageDto {
    fn from(r: ImageRef) -> Self {
        // 先把兜底算出来，再移动字段。
        // 顺序反了会借用检查报错：借了 image_url 之后就不能再 move 它。
        let thumbnail_url = r
            .thumbnail_url
            .clone()
            .unwrap_or_else(|| r.image_url.clone());

        Self {
            image_url: r.image_url,
            thumbnail_url,
            page_url: r.page_url,
            width: r.width,
            height: r.height,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试里大部分用例不关心尺寸，给个默认值省得每处都写两行。
    fn ref_with(image_url: &str, thumbnail_url: Option<&str>, page_url: Option<&str>) -> ImageRef {
        ImageRef {
            image_url: image_url.to_string(),
            thumbnail_url: thumbnail_url.map(str::to_string),
            page_url: page_url.map(str::to_string),
            width: None,
            height: None,
        }
    }

    #[test]
    fn missing_thumbnail_falls_back_to_image() {
        let dto = ImageDto::from(ref_with("big.png", None, None));

        assert_eq!(dto.image_url, "big.png");
        assert_eq!(dto.thumbnail_url, "big.png");
        assert!(dto.page_url.is_none());
    }

    #[test]
    fn existing_thumbnail_is_kept() {
        let dto = ImageDto::from(ref_with("big.png", Some("small.png"), Some("post")));

        assert_eq!(dto.thumbnail_url, "small.png");
        assert_eq!(dto.page_url.as_deref(), Some("post"));
    }

    #[test]
    fn dimensions_pass_through() {
        let mut source = ref_with("a.png", None, None);
        source.width = Some(1920);
        source.height = Some(1080);

        let dto = ImageDto::from(source);

        assert_eq!(dto.width, Some(1920));
        assert_eq!(dto.height, Some(1080));
    }

    #[test]
    fn unknown_dimensions_stay_none() {
        // 未知 ≠ 0。渲染成 0 会让前端的筛选把它当成"特别小"而筛掉。
        let dto = ImageDto::from(ref_with("a.png", None, None));

        assert_eq!(dto.width, None);
        assert_eq!(dto.height, None);
    }

    #[test]
    fn serializes_to_camel_case() {
        let dto = ImageDto::from(ref_with("a", Some("b"), None));
        let json = serde_json::to_string(&dto).unwrap();
        assert!(json.contains("\"imageUrl\""));
        assert!(json.contains("\"thumbnailUrl\""));
    }
}
