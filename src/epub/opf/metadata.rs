//! 元数据处理模块
//! 
//! 提供EPUB元数据的结构定义和处理功能。

use crate::epub::opf::config::MetadataTagConfigs;
use std::collections::HashMap;

/// 元数据值枚举，表示不同类型的元数据
#[derive(Debug, Clone)]
pub enum MetadataValue {
    /// EPUB规范的Dublin Core标签元数据
    DublinCore {
        /// 元素内容
        value: String,
        /// 元素属性（如 role, file-as, scheme等）
        attributes: HashMap<String, String>,
    },
    /// meta标签的自定义元数据
    Meta(MetaValue),
}

/// meta标签值枚举
#[derive(Debug, Clone)]
pub enum MetaValue {
    /// 基于name属性的meta标签，如 <meta name="cover" content="cover.jpg"/>
    NameBased {
        /// content属性值
        content: String,
    },
    /// 基于property属性的meta标签，如 <meta property="dcterms:modified">2025-06-05T11:24:01Z</meta>
    PropertyBased {
        /// 标签内容
        content: String,
    },
    /// 基于refines属性的meta标签，如 <meta refines="#creator" property="role">aut</meta>
    RefinesBased {
        /// 被精化的元素ID（不包含#前缀）
        refines_id: String,
        /// property属性值（如role、file-as、display-seq等）
        property: String,
        /// 标签内容
        content: String,
        /// scheme属性（可选，如marc:relators）
        scheme: Option<String>,
    },
}

/// 创建者信息(作者、编辑者等)
#[derive(Debug, Clone)]
pub struct Creator {
    /// 创建者姓名
    pub name: String,
    /// 角色(如author、editor等)
    pub role: Option<String>,
    /// 显示顺序
    pub display_seq: Option<u32>,
    /// 元素ID（用于关联refines元数据）
    pub id: Option<String>,
}

/// 标识符信息
#[derive(Debug, Clone)]
pub struct Identifier {
    /// 标识符值
    pub value: String,
    /// 标识符类型(如ISBN、UUID等)
    pub scheme: Option<String>,
    /// 是否为唯一标识符
    pub id: Option<String>,
}

/// OPF文件中的元数据信息
#[derive(Debug, Clone)]
pub struct Metadata {
    /// 原始元数据映射：key为标签名（如"dc:title", "cover", "dcterms:modified"），value为元数据值列表
    raw_metadata: HashMap<String, Vec<MetadataValue>>,
    /// 关联元数据映射：key为被精化的元素ID，value为精化信息列表
    refines_metadata: HashMap<String, Vec<MetaValue>>,
    /// 元数据标签配置，用于查找对应的元数据
    tag_configs: MetadataTagConfigs,
}

impl Metadata {
    /// 创建新的元数据实例
    pub fn new() -> Self {
        Self {
            raw_metadata: HashMap::new(),
            refines_metadata: HashMap::new(),
            tag_configs: MetadataTagConfigs::new(),
        }
    }

    /// 添加Dublin Core元数据
    pub fn add_dublin_core(&mut self, tag: String, value: String, attributes: HashMap<String, String>) {
        let metadata_value = MetadataValue::DublinCore { value, attributes };
        self.raw_metadata
            .entry(tag)
            .or_insert_with(Vec::new)
            .push(metadata_value);
    }

    /// 添加基于name的meta元数据
    pub fn add_meta_name_based(&mut self, name: String, content: String) {
        let metadata_value = MetadataValue::Meta(MetaValue::NameBased { content });
        self.raw_metadata
            .entry(name)
            .or_insert_with(Vec::new)
            .push(metadata_value);
    }

    /// 添加基于property的meta元数据
    pub fn add_meta_property_based(&mut self, property: String, content: String) {
        let metadata_value = MetadataValue::Meta(MetaValue::PropertyBased { content });
        self.raw_metadata
            .entry(property)
            .or_insert_with(Vec::new)
            .push(metadata_value);
    }

    /// 添加基于refines的meta元数据
    pub fn add_meta_refines_based(&mut self, refines_id: String, property: String, content: String, scheme: Option<String>) {
        let meta_value = MetaValue::RefinesBased {
            refines_id: refines_id.clone(),
            property,
            content,
            scheme,
        };
        
        // 同时存储在两个地方：一个用于原始数据，一个用于关联查找
        let metadata_value = MetadataValue::Meta(meta_value.clone());
        self.raw_metadata
            .entry(format!("refines-{}", refines_id))
            .or_insert_with(Vec::new)
            .push(metadata_value);
            
        self.refines_metadata
            .entry(refines_id)
            .or_insert_with(Vec::new)
            .push(meta_value);
    }

    /// 根据标签列表查找元数据值
    fn find_by_tags(&self, tags: &[String]) -> Option<&MetadataValue> {
        for tag in tags {
            if let Some(values) = self.raw_metadata.get(tag) {
                if let Some(value) = values.first() {
                    return Some(value);
                }
            }
        }
        None
    }

    /// 根据标签列表查找所有元数据值
    fn find_all_by_tags(&self, tags: &[String]) -> Vec<&MetadataValue> {
        let mut result = Vec::new();
        for tag in tags {
            if let Some(values) = self.raw_metadata.get(tag) {
                result.extend(values.iter());
            }
        }
        result
    }

    /// 获取标题
    pub fn title(&self) -> Option<String> {
        self.find_by_tags(&self.tag_configs.title.tags)
            .and_then(|v| self.extract_content(v))
    }

    /// 获取所有创建者
    pub fn creators(&self) -> Vec<Creator> {
        self.find_all_by_tags(&self.tag_configs.creator.tags)
            .into_iter()
            .filter_map(|v| self.extract_creator(v))
            .collect()
    }

    /// 获取所有贡献者
    pub fn contributors(&self) -> Vec<Creator> {
        self.find_all_by_tags(&self.tag_configs.contributor.tags)
            .into_iter()
            .filter_map(|v| self.extract_creator(v))
            .collect()
    }

    /// 获取语言
    pub fn language(&self) -> Option<String> {
        self.find_by_tags(&self.tag_configs.language.tags)
            .and_then(|v| self.extract_content(v))
    }

    /// 获取所有标识符
    pub fn identifiers(&self) -> Vec<Identifier> {
        self.find_all_by_tags(&self.tag_configs.identifier.tags)
            .into_iter()
            .filter_map(|v| self.extract_identifier(v))
            .collect()
    }

    /// 获取出版社
    pub fn publisher(&self) -> Option<String> {
        self.find_by_tags(&self.tag_configs.publisher.tags)
            .and_then(|v| self.extract_content(v))
    }

    /// 获取出版日期
    pub fn date(&self) -> Option<String> {
        self.find_by_tags(&self.tag_configs.date.tags)
            .and_then(|v| self.extract_content(v))
    }

    /// 获取描述
    pub fn description(&self) -> Option<String> {
        self.find_by_tags(&self.tag_configs.description.tags)
            .and_then(|v| self.extract_content(v))
    }

    /// 获取所有主题
    pub fn subjects(&self) -> Vec<String> {
        self.find_all_by_tags(&self.tag_configs.subject.tags)
            .into_iter()
            .filter_map(|v| self.extract_content(v))
            .collect()
    }

    /// 获取版权信息
    pub fn rights(&self) -> Option<String> {
        self.find_by_tags(&self.tag_configs.rights.tags)
            .and_then(|v| self.extract_content(v))
    }

    /// 获取封面信息
    pub fn cover(&self) -> Option<String> {
        self.find_by_tags(&self.tag_configs.cover.tags)
            .and_then(|v| self.extract_content(v))
    }

    /// 获取修改时间
    pub fn modified(&self) -> Option<String> {
        self.find_by_tags(&self.tag_configs.modified.tags)
            .and_then(|v| self.extract_content(v))
    }

    /// 获取其他元数据
    pub fn other(&self) -> HashMap<String, String> {
        let mut other = HashMap::new();
        let known_tags: Vec<String> = [
            &self.tag_configs.title.tags,
            &self.tag_configs.creator.tags,
            &self.tag_configs.contributor.tags,
            &self.tag_configs.language.tags,
            &self.tag_configs.identifier.tags,
            &self.tag_configs.publisher.tags,
            &self.tag_configs.date.tags,
            &self.tag_configs.description.tags,
            &self.tag_configs.subject.tags,
            &self.tag_configs.rights.tags,
            &self.tag_configs.cover.tags,
            &self.tag_configs.modified.tags,
        ].iter().flat_map(|v| v.iter()).cloned().collect();

        for (tag, values) in &self.raw_metadata {
            if !known_tags.contains(tag) && !tag.starts_with("refines-") {
                if let Some(value) = values.first() {
                    if let Some(content) = self.extract_content(value) {
                        other.insert(tag.clone(), content);
                    }
                }
            }
        }
        other
    }

    /// 从元数据值中提取内容
    fn extract_content(&self, value: &MetadataValue) -> Option<String> {
        match value {
            MetadataValue::DublinCore { value, .. } => Some(value.clone()),
            MetadataValue::Meta(meta) => match meta {
                MetaValue::NameBased { content } => Some(content.clone()),
                MetaValue::PropertyBased { content } => Some(content.clone()),
                MetaValue::RefinesBased { content, .. } => Some(content.clone()),
            },
        }
    }

    /// 从元数据值中提取创建者信息（支持EPUB3的refines关联）
    fn extract_creator(&self, value: &MetadataValue) -> Option<Creator> {
        match value {
            MetadataValue::DublinCore { value, attributes } => {
                let mut creator = Creator {
                    name: value.clone(),
                    role: attributes.get("role").cloned(),
                    display_seq: None,
                    id: attributes.get("id").cloned(),
                };

                // 如果有ID，查找相关的refines元数据
                if let Some(id) = &creator.id {
                    if let Some(refines_list) = self.refines_metadata.get(id) {
                        for refines in refines_list {
                            if let MetaValue::RefinesBased { property, content, .. } = refines {
                                match property.as_str() {
                                    "role" => {
                                        // 处理角色信息，支持marc:relators scheme
                                        creator.role = Some(match content.as_str() {
                                            "aut" => "author".to_string(),
                                            "edt" => "editor".to_string(),
                                            "trl" => "translator".to_string(),
                                            "ill" => "illustrator".to_string(),
                                            _ => content.clone(),
                                        });
                                    }
                                    "display-seq" => {
                                        creator.display_seq = content.parse::<u32>().ok();
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }

                Some(creator)
            }
            MetadataValue::Meta(meta) => {
                let name = match meta {
                    MetaValue::NameBased { content } => content.clone(),
                    MetaValue::PropertyBased { content } => content.clone(),
                    MetaValue::RefinesBased { content, .. } => content.clone(),
                };
                Some(Creator {
                    name,
                    role: None,
                    display_seq: None,
                    id: None,
                })
            }
        }
    }

    /// 从元数据值中提取标识符信息
    fn extract_identifier(&self, value: &MetadataValue) -> Option<Identifier> {
        match value {
            MetadataValue::DublinCore { value, attributes } => Some(Identifier {
                value: value.clone(),
                scheme: attributes.get("scheme").cloned(),
                id: attributes.get("id").cloned(),
            }),
            MetadataValue::Meta(meta) => {
                let identifier_value = match meta {
                    MetaValue::NameBased { content } => content.clone(),
                    MetaValue::PropertyBased { content } => content.clone(),
                    MetaValue::RefinesBased { content, .. } => content.clone(),
                };
                Some(Identifier {
                    value: identifier_value,
                    scheme: None,
                    id: None,
                })
            }
        }
    }

    /// 获取原始元数据映射
    pub fn raw_metadata(&self) -> &HashMap<String, Vec<MetadataValue>> {
        &self.raw_metadata
    }

    /// 获取关联元数据映射
    pub fn refines_metadata(&self) -> &HashMap<String, Vec<MetaValue>> {
        &self.refines_metadata
    }

    /// 根据标签名查找原始元数据
    pub fn find_raw_by_tag(&self, tag: &str) -> Option<&Vec<MetadataValue>> {
        self.raw_metadata.get(tag)
    }

    /// 获取所有Dublin Core元数据
    pub fn get_dublin_core_metadata(&self) -> Vec<(String, String, HashMap<String, String>)> {
        let mut result = Vec::new();
        for (tag, values) in &self.raw_metadata {
            for value in values {
                if let MetadataValue::DublinCore { value, attributes } = value {
                    result.push((tag.clone(), value.clone(), attributes.clone()));
                }
            }
        }
        result
    }

    /// 获取所有基于name的meta标签
    pub fn get_name_based_meta(&self) -> Vec<(String, String)> {
        let mut result = Vec::new();
        for (tag, values) in &self.raw_metadata {
            for value in values {
                if let MetadataValue::Meta(MetaValue::NameBased { content }) = value {
                    result.push((tag.clone(), content.clone()));
                }
            }
        }
        result
    }

    /// 获取所有基于property的meta标签
    pub fn get_property_based_meta(&self) -> Vec<(String, String)> {
        let mut result = Vec::new();
        for (tag, values) in &self.raw_metadata {
            for value in values {
                if let MetadataValue::Meta(MetaValue::PropertyBased { content }) = value {
                    result.push((tag.clone(), content.clone()));
                }
            }
        }
        result
    }

    /// 获取所有基于refines的meta标签
    pub fn get_refines_based_meta(&self) -> Vec<(String, String, String, Option<String>)> {
        let mut result = Vec::new();
        for values in self.refines_metadata.values() {
            for meta in values {
                if let MetaValue::RefinesBased { refines_id, property, content, scheme } = meta {
                    result.push((refines_id.clone(), property.clone(), content.clone(), scheme.clone()));
                }
            }
        }
        result
    }

    /// 获取元数据的统计信息
    pub fn get_metadata_stats(&self) -> (usize, usize, usize, usize) {
        let mut dublin_core_count = 0;
        let mut name_based_count = 0;
        let mut property_based_count = 0;
        let mut refines_based_count = 0;

        for values in self.raw_metadata.values() {
            for value in values {
                match value {
                    MetadataValue::DublinCore { .. } => dublin_core_count += 1,
                    MetadataValue::Meta(MetaValue::NameBased { .. }) => name_based_count += 1,
                    MetadataValue::Meta(MetaValue::PropertyBased { .. }) => property_based_count += 1,
                    MetadataValue::Meta(MetaValue::RefinesBased { .. }) => refines_based_count += 1,
                }
            }
        }

        (dublin_core_count, name_based_count, property_based_count, refines_based_count)
    }
} 