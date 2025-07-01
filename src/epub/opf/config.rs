//! 元数据标签配置模块
//! 
//! 提供元数据标签的配置管理功能，支持从YAML文件加载配置。

use crate::epub::error::{EpubError, Result};
use serde::{Deserialize, Serialize};
use std::fs;

/// 默认配置文件路径
const DEFAULT_CONFIG_PATH: &str = "metadata.yaml";

/// 单个元数据类型的标签配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataTagConfig {
    /// 标签列表
    pub tags: Vec<String>,
    /// 可选的描述
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl MetadataTagConfig {
    /// 创建新的标签配置
    pub fn new(tags: Vec<String>) -> Self {
        Self {
            tags,
            description: None,
        }
    }

    /// 创建带描述的标签配置
    pub fn with_description(tags: Vec<String>, description: String) -> Self {
        Self {
            tags,
            description: Some(description),
        }
    }
}

/// 元数据标签配置，定义每种元数据类型对应的可能标签
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataTagConfigs {
    /// 标题标签配置
    pub title: MetadataTagConfig,
    /// 创建者标签配置
    pub creator: MetadataTagConfig,
    /// 贡献者标签配置
    pub contributor: MetadataTagConfig,
    /// 语言标签配置
    pub language: MetadataTagConfig,
    /// 标识符标签配置
    pub identifier: MetadataTagConfig,
    /// 出版社标签配置
    pub publisher: MetadataTagConfig,
    /// 出版日期标签配置
    pub date: MetadataTagConfig,
    /// 描述标签配置
    pub description: MetadataTagConfig,
    /// 主题标签配置
    pub subject: MetadataTagConfig,
    /// 版权标签配置
    pub rights: MetadataTagConfig,
    /// 封面标签配置
    pub cover: MetadataTagConfig,
    /// 修改时间标签配置
    pub modified: MetadataTagConfig,
}

impl MetadataTagConfigs {
    /// 从默认配置文件中加载元数据标签配置
    /// 
    /// 配置文件默认为当前目录下的 `metadata.yaml`
    /// 
    /// # 返回值
    /// 
    /// * `Result<Self>` - 加载成功返回配置实例，失败返回错误
    /// 
    /// # 示例
    /// 
    /// ```rust
    /// use bookforge::epub::opf::MetadataTagConfigs;
    /// let config = MetadataTagConfigs::from_file()?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn from_file() -> Result<Self> {
        let content = fs::read_to_string(DEFAULT_CONFIG_PATH)
            .map_err(|e| EpubError::ConfigError(format!("无法读取配置文件: {}", e)))?;
        
        serde_yml::from_str(&content)
            .map_err(|e| EpubError::ConfigError(format!("配置文件格式错误: {}", e)))
    }

    /// 生成默认配置文件到当前目录
    /// 
    /// 配置文件将生成为当前目录下的 `metadata.yaml`
    /// 
    /// # 返回值
    /// 
    /// * `Result<()>` - 生成成功返回Ok，失败返回错误
    /// 
    /// # 示例
    /// 
    /// ```rust
    /// use bookforge::epub::opf::MetadataTagConfigs;
    /// MetadataTagConfigs::generate_default_config()?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn generate_default_config() -> Result<()> {
        let default_config = Self::default_config();
        let yaml_content = serde_yml::to_string(&default_config)
            .map_err(|e| EpubError::ConfigError(format!("序列化配置失败: {}", e)))?;
        
        // 在YAML内容前添加注释说明
        let content_with_header = format!(
            "# 元数据标签配置文件\n# 定义 EPUB 元数据解析时使用的标签映射\n# 每个配置项可以包含多个可能的标签名称\n\n{}",
            yaml_content
        );
        
        fs::write(DEFAULT_CONFIG_PATH, content_with_header)
            .map_err(|e| EpubError::ConfigError(format!("写入配置文件失败: {}", e)))?;
        
        Ok(())
    }

    /// 获取默认配置
    /// 
    /// # 返回值
    /// 
    /// * `Self` - 默认配置实例
    pub fn default_config() -> Self {
        Self {
            title: MetadataTagConfig::with_description(
                vec!["title".to_string()],
                "书籍标题".to_string()
            ),
            creator: MetadataTagConfig::with_description(
                vec!["creator".to_string(), "author".to_string()],
                "作者/创建者信息".to_string()
            ),
            contributor: MetadataTagConfig::with_description(
                vec!["contributor".to_string()],
                "贡献者信息（编辑、插图等）".to_string()
            ),
            language: MetadataTagConfig::with_description(
                vec!["language".to_string()],
                "书籍语言".to_string()
            ),
            identifier: MetadataTagConfig::with_description(
                vec!["identifier".to_string()],
                "书籍标识符（ISBN、UUID等）".to_string()
            ),
            publisher: MetadataTagConfig::with_description(
                vec!["publisher".to_string()],
                "出版社信息".to_string()
            ),
            date: MetadataTagConfig::with_description(
                vec!["date".to_string()],
                "出版日期".to_string()
            ),
            description: MetadataTagConfig::with_description(
                vec!["description".to_string()],
                "书籍描述/简介".to_string()
            ),
            subject: MetadataTagConfig::with_description(
                vec!["subject".to_string()],
                "书籍主题/分类".to_string()
            ),
            rights: MetadataTagConfig::with_description(
                vec!["rights".to_string()],
                "版权信息".to_string()
            ),
            cover: MetadataTagConfig::with_description(
                vec!["cover".to_string()],
                "封面图片信息".to_string()
            ),
            modified: MetadataTagConfig::with_description(
                vec!["dcterms:modified".to_string()],
                "最后修改时间".to_string()
            ),
        }
    }

    /// 尝试从默认配置文件加载，如果文件不存在则先生成配置文件再加载
    /// 
    /// 配置文件为当前目录下的 `metadata.yaml`
    /// 
    /// # 返回值
    /// 
    /// * `Self` - 配置实例
    /// 
    /// # 示例
    /// 
    /// ```rust
    /// use bookforge::epub::opf::MetadataTagConfigs;
    /// let config = MetadataTagConfigs::new();
    /// ```
    pub fn new() -> Self {
        // 首先尝试从文件加载
        match Self::from_file() {
            Ok(config) => config,
            Err(_) => {
                // 如果文件不存在，先生成默认配置文件
                let _ = Self::generate_default_config();
                Self::default_config()
            }
        }
    }
} 