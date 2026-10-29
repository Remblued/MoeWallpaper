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
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_thumbnail_falls_back_to_image() {
        let dto = ImageDto::from(ImageRef {
            image_url: "big.png".to_string(),
            thumbnail_url: None,
            page_url: None,
        });

        assert_eq!(dto.image_url, "big.png");
        assert_eq!(dto.thumbnail_url, "big.png");
        assert!(dto.page_url.is_none());
    }

    #[test]
    fn existing_thumbnail_is_kept() {
        let dto = ImageDto::from(ImageRef {
            image_url: "big.png".to_string(),
            thumbnail_url: Some("small.png".to_string()),
            page_url: Some("post".to_string()),
        });

        assert_eq!(dto.thumbnail_url, "small.png");
        assert_eq!(dto.page_url.as_deref(), Some("post"));
    }

    #[test]
    fn serializes_to_camel_case() {
        let dto = ImageDto::from(ImageRef {
            image_url: "a".to_string(),
            thumbnail_url: Some("b".to_string()),
            page_url: None,
        });
        let json = serde_json::to_string(&dto).unwrap();
        assert!(json.contains("\"imageUrl\""));
        assert!(json.contains("\"thumbnailUrl\""));
    }
}
