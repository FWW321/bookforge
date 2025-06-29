//! 清单模块
//! 
//! 提供EPUB包中文件清单的结构定义。

/// 清单项信息
#[derive(Debug, Clone)]
pub struct ManifestItem {
    /// 项目ID
    pub id: String,
    /// 文件路径(相对于OPF文件)
    pub href: String,
    /// 媒体类型
    pub media_type: String,
    /// 属性(如nav、cover-image等)
    pub properties: Option<String>,
}

impl ManifestItem {
    /// 创建新的清单项
    pub fn new(id: String, href: String, media_type: String) -> Self {
        Self {
            id,
            href,
            media_type,
            properties: None,
        }
    }

    /// 创建带属性的清单项
    pub fn with_properties(id: String, href: String, media_type: String, properties: String) -> Self {
        Self {
            id,
            href,
            media_type,
            properties: Some(properties),
        }
    }

    /// 检查是否包含指定属性
    pub fn has_property(&self, property: &str) -> bool {
        if let Some(properties) = &self.properties {
            properties.split_whitespace().any(|p| p == property)
        } else {
            false
        }
    }

    /// 检查是否为导航文档
    pub fn is_nav(&self) -> bool {
        self.has_property("nav")
    }

    /// 检查是否为封面图片
    pub fn is_cover_image(&self) -> bool {
        self.has_property("cover-image")
    }

    /// 检查是否为图片文件
    pub fn is_image(&self) -> bool {
        self.media_type.starts_with("image/")
    }

    /// 检查是否为CSS文件
    pub fn is_css(&self) -> bool {
        self.media_type == "text/css"
    }

    /// 检查是否为XHTML文件
    pub fn is_xhtml(&self) -> bool {
        self.media_type == "application/xhtml+xml"
    }
} 