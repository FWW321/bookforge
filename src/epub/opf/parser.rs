//! OPF解析器模块
//! 
//! 提供OPF（Open Packaging Format）文件的XML解析功能。

use crate::epub::error::{EpubError, Result};
use crate::epub::opf::{
    metadata::Metadata,
    manifest::ManifestItem,
    spine::SpineItem,
};
use quick_xml::events::Event;
use quick_xml::reader::Reader;
use std::collections::HashMap;

/// OPF文件解析结果
#[derive(Debug, Clone)]
pub struct Opf {
    /// EPUB版本
    pub version: String,
    /// 元数据
    pub metadata: Metadata,
    /// 清单项(文件列表)
    pub manifest: HashMap<String, ManifestItem>,
    /// 脊柱(阅读顺序)
    pub spine: Vec<SpineItem>,
    /// 脊柱的目录引用
    pub spine_toc: Option<String>,
}

impl Opf {
    /// 解析OPF文件内容
    /// 
    /// # 参数
    /// * `xml_content` - OPF文件的XML内容
    /// 
    /// # 返回值
    /// * `Result<Opf, EpubError>` - 解析后的OPF信息
    pub fn parse_xml(xml_content: &str) -> Result<Opf> {
        Self::parse_xml_with_config(xml_content)
    }

    /// 使用指定的配置文件解析OPF文件内容
    /// 
    /// # 参数
    /// * `xml_content` - OPF文件的XML内容
    /// * `config_path` - 配置文件路径(可选)，如果不提供则使用默认配置
    /// 
    /// # 返回值
    /// * `Result<Opf, EpubError>` - 解析后的OPF信息
    pub fn parse_xml_with_config(xml_content: &str) -> Result<Opf> {
        let mut reader = Reader::from_str(xml_content);
        reader.config_mut().trim_text(true);
        reader.config_mut().expand_empty_elements = true;
        
        let mut version = String::new();
        let mut metadata = Metadata::new();
        let mut manifest = HashMap::new();
        let mut spine = Vec::new();
        let mut spine_toc = None;
        
        let mut buf = Vec::new();
        let mut current_section = String::new();
        let mut text_content = String::new();
        let mut current_attributes = HashMap::new();
        let mut current_meta_property = String::new();
        
        loop {
            match reader.read_event_into(&mut buf)? {
                Event::Start(ref e) => {
                    let local_name_bytes = e.local_name();
                    let local_name = String::from_utf8_lossy(local_name_bytes.as_ref());
                    
                    match local_name.as_ref() {
                        "package" => {
                            version = Self::parse_package_version(e)?;
                        }
                        "metadata" => {
                            current_section = "metadata".to_string();
                        }
                        "manifest" => {
                            current_section = "manifest".to_string();
                        }
                        "spine" => {
                            current_section = "spine".to_string();
                            spine_toc = Self::parse_spine_toc(e)?;
                        }
                        "item" if current_section == "manifest" => {
                            Self::parse_manifest_item(e, &mut manifest)?;
                        }
                        "itemref" if current_section == "spine" => {
                            Self::parse_spine_item(e, &mut spine)?;
                        }
                        "meta" if current_section == "metadata" => {
                            current_meta_property = Self::handle_meta_start_tag(e, &mut metadata)?;
                            text_content.clear();
                        }
                        _ if current_section == "metadata" => {
                            // 处理元数据元素
                            Self::handle_metadata_element(e, &mut current_attributes);
                            text_content.clear();
                        }
                        _ => {}
                    }
                }
                Event::Empty(ref e) => {
                    let local_name_bytes = e.local_name();
                    let local_name = String::from_utf8_lossy(local_name_bytes.as_ref());
                    
                    match local_name.as_ref() {
                        "meta" if current_section == "metadata" => {
                            Self::handle_empty_meta_tag(e, &mut metadata)?;
                        }
                        "item" if current_section == "manifest" => {
                            Self::parse_manifest_item(e, &mut manifest)?;
                        }
                        "itemref" if current_section == "spine" => {
                            Self::parse_spine_item(e, &mut spine)?;
                        }
                        _ => {}
                    }
                }
                Event::End(ref e) => {
                    let local_name_bytes = e.local_name();
                    let local_name = String::from_utf8_lossy(local_name_bytes.as_ref());
                    
                    match local_name.as_ref() {
                        "metadata" | "manifest" | "spine" => {
                            current_section.clear();
                        }
                        "meta" if current_section == "metadata" && !current_meta_property.is_empty() => {
                            // 检查是否是refines类型的meta标签
                            if current_meta_property.starts_with("refines:") {
                                let parts: Vec<&str> = current_meta_property.split(':').collect();
                                if parts.len() >= 3 {
                                    let refines_id = parts[1].to_string();
                                    let property = parts[2].to_string();
                                    let scheme = if parts.len() > 3 && !parts[3].is_empty() {
                                        Some(parts[3].to_string())
                                    } else {
                                        None
                                    };
                                    metadata.add_meta_refines_based(refines_id, property, text_content.trim().to_string(), scheme);
                                }
                            } else {
                                metadata.add_meta_property_based(current_meta_property.clone(), text_content.trim().to_string());
                            }
                            current_meta_property.clear();
                        }
                        _ if current_section == "metadata" => {
                            Self::process_metadata_text(&local_name, &text_content, &mut metadata, &current_attributes);
                        }
                        _ => {}
                    }
                }
                Event::Text(e) => {
                    text_content.push_str(&e.unescape()?);
                }
                Event::Eof => break,
                _ => {}
            }
            buf.clear();
        }
        
        Ok(Opf {
            version,
            metadata,
            manifest,
            spine,
            spine_toc,
        })
    }

    /// 解析package元素的version属性
    fn parse_package_version(e: &quick_xml::events::BytesStart) -> Result<String> {
        for attr_result in e.attributes() {
            let attr = attr_result.map_err(|err| EpubError::XmlError(quick_xml::Error::InvalidAttr(err)))?;
            if attr.key.local_name().as_ref() == b"version" {
                return Ok(String::from_utf8_lossy(&attr.value).to_string());
            }
        }
        Ok(String::new())
    }

    /// 解析spine元素的toc属性
    fn parse_spine_toc(e: &quick_xml::events::BytesStart) -> Result<Option<String>> {
        for attr_result in e.attributes() {
            let attr = attr_result.map_err(|err| EpubError::XmlError(quick_xml::Error::InvalidAttr(err)))?;
            if attr.key.local_name().as_ref() == b"toc" {
                return Ok(Some(String::from_utf8_lossy(&attr.value).to_string()));
            }
        }
        Ok(None)
    }
    
    /// 处理meta开始标签，返回property属性值(如果存在)
    fn handle_meta_start_tag(
        e: &quick_xml::events::BytesStart,
        metadata: &mut Metadata,
    ) -> Result<String> {
        let mut name = String::new();
        let mut content = String::new();
        let mut property = String::new();
        let mut refines = String::new();
        let mut scheme = None;
        
        // 解析meta标签属性
        for attr_result in e.attributes() {
            let attr = attr_result.map_err(|err| EpubError::XmlError(quick_xml::Error::InvalidAttr(err)))?;
            match attr.key.local_name().as_ref() {
                b"name" => {
                    name = String::from_utf8_lossy(&attr.value).to_string();
                }
                b"content" => {
                    content = String::from_utf8_lossy(&attr.value).to_string();
                }
                b"property" => {
                    property = String::from_utf8_lossy(&attr.value).to_string();
                }
                b"refines" => {
                    refines = String::from_utf8_lossy(&attr.value).to_string();
                    // 移除开头的#号（如果存在）
                    if refines.starts_with('#') {
                        refines = refines[1..].to_string();
                    }
                }
                b"scheme" => {
                    scheme = Some(String::from_utf8_lossy(&attr.value).to_string());
                }
                _ => {}
            }
        }
        
        // 处理name属性的meta标签
        if !name.is_empty() && !content.is_empty() {
            metadata.add_meta_name_based(name, content);
        }
        
        // 如果是refines类型的meta标签，等待获取文本内容
        if !refines.is_empty() && !property.is_empty() {
            // 这里我们返回特殊格式，包含refines信息，以便后续处理
            return Ok(format!("refines:{}:{}:{}", refines, property, scheme.unwrap_or_default()));
        }
        
        Ok(property)
    }
    
    /// 处理空的meta标签
    fn handle_empty_meta_tag(
        e: &quick_xml::events::BytesStart,
        metadata: &mut Metadata,
    ) -> Result<()> {
        let mut name = String::new();
        let mut content = String::new();
        let mut property = String::new();
        let mut refines = String::new();
        let mut scheme = None;
        
        // 解析meta标签属性
        for attr_result in e.attributes() {
            let attr = attr_result.map_err(|err| EpubError::XmlError(quick_xml::Error::InvalidAttr(err)))?;
            match attr.key.local_name().as_ref() {
                b"name" => {
                    name = String::from_utf8_lossy(&attr.value).to_string();
                }
                b"content" => {
                    content = String::from_utf8_lossy(&attr.value).to_string();
                }
                b"property" => {
                    property = String::from_utf8_lossy(&attr.value).to_string();
                }
                b"refines" => {
                    refines = String::from_utf8_lossy(&attr.value).to_string();
                    // 移除开头的#号（如果存在）
                    if refines.starts_with('#') {
                        refines = refines[1..].to_string();
                    }
                }
                b"scheme" => {
                    scheme = Some(String::from_utf8_lossy(&attr.value).to_string());
                }
                _ => {}
            }
        }
        
        // 处理name属性的meta标签
        if !name.is_empty() && !content.is_empty() {
            metadata.add_meta_name_based(name, content.clone());
        }
        
        // 处理refines属性的meta标签（空标签，content在属性中）
        if !refines.is_empty() && !property.is_empty() && !content.is_empty() {
            metadata.add_meta_refines_based(refines, property, content, scheme);
        }
        // 处理property属性的meta标签(EPUB3格式，但没有文本内容的情况)
        else if !property.is_empty() && refines.is_empty() {
            metadata.add_meta_property_based(property, String::new());
        }
        
        Ok(())
    }
    
    /// 处理元数据元素的开始标签
    fn handle_metadata_element(
        e: &quick_xml::events::BytesStart,
        current_attributes: &mut HashMap<String, String>,
    ) {
        // 清空当前属性
        current_attributes.clear();
        
        // 收集所有属性
        for attr_result in e.attributes() {
            if let Ok(attr) = attr_result {
                let key = String::from_utf8_lossy(attr.key.local_name().as_ref()).to_string();
                let value = String::from_utf8_lossy(&attr.value).to_string();
                current_attributes.insert(key, value);
            }
        }
    }
    
    /// 处理元数据元素的文本内容
    /// 
    /// 注意：quick_xml解析器使用local_name()方法，会忽略XML命名空间前缀
    /// 例如：<dc:title> 会被解析为 "title"，<dc:language> 会被解析为 "language"
    fn process_metadata_text(
        element_name: &str,
        text_content: &str,
        metadata: &mut Metadata,
        current_attributes: &HashMap<String, String>,
    ) {
        let content = text_content.trim();
        if content.is_empty() {
            return;
        }
        
        // 添加Dublin Core元数据
        // element_name 已经是去掉命名空间前缀的本地名称
        metadata.add_dublin_core(element_name.to_string(), content.to_string(), current_attributes.clone());
    }
    
    /// 解析清单项
    fn parse_manifest_item(
        e: &quick_xml::events::BytesStart,
        manifest: &mut HashMap<String, ManifestItem>,
    ) -> Result<()> {
        let mut item = ManifestItem {
            id: String::new(),
            href: String::new(),
            media_type: String::new(),
            properties: None,
        };
        
        // 解析item属性
        for attr_result in e.attributes() {
            let attr = attr_result.map_err(|e| EpubError::XmlError(quick_xml::Error::InvalidAttr(e)))?;
            match attr.key.local_name().as_ref() {
                b"id" => {
                    item.id = String::from_utf8_lossy(&attr.value).to_string();
                }
                b"href" => {
                    item.href = String::from_utf8_lossy(&attr.value).to_string();
                }
                b"media-type" => {
                    item.media_type = String::from_utf8_lossy(&attr.value).to_string();
                }
                b"properties" => {
                    item.properties = Some(String::from_utf8_lossy(&attr.value).to_string());
                }
                _ => {}
            }
        }
        
        if !item.id.is_empty() && !item.href.is_empty() && !item.media_type.is_empty() {
            manifest.insert(item.id.clone(), item);
        }
        
        Ok(())
    }
    
    /// 解析脊柱项
    fn parse_spine_item(
        e: &quick_xml::events::BytesStart,
        spine: &mut Vec<SpineItem>,
    ) -> Result<()> {
        let mut spine_item = SpineItem {
            idref: String::new(),
            linear: true,
        };
        
        // 解析itemref属性
        for attr_result in e.attributes() {
            let attr = attr_result.map_err(|e| EpubError::XmlError(quick_xml::Error::InvalidAttr(e)))?;
            match attr.key.local_name().as_ref() {
                b"idref" => {
                    spine_item.idref = String::from_utf8_lossy(&attr.value).to_string();
                }
                b"linear" => {
                    let linear_value = String::from_utf8_lossy(&attr.value);
                    spine_item.linear = linear_value != "no";
                }
                _ => {}
            }
        }
        
        if !spine_item.idref.is_empty() {
            spine.push(spine_item);
        }
        
        Ok(())
    }
    
    /// 获取导航文档的路径
    /// 
    /// # 返回值
    /// * `Option<String>` - 导航文档的路径
    pub fn get_nav_path(&self) -> Option<String> {
        self.manifest.values()
            .find(|item| item.is_nav())
            .map(|item| item.href.clone())
    }
    
    /// 获取封面图片的路径
    /// 
    /// # 返回值
    /// * `Option<String>` - 封面图片的路径
    pub fn get_cover_image_path(&self) -> Option<String> {
        self.manifest.values()
            .find(|item| item.is_cover_image())
            .map(|item| item.href.clone())
    }
    
    /// 获取封面路径(综合检查多种方式)
    /// 
    /// # 返回值
    /// * `Option<String>` - 封面路径
    pub fn get_cover_path(&self) -> Option<String> {
        // 首先检查manifest中具有cover-image属性的项目
        if let Some(path) = self.get_cover_image_path() {
            return Some(path);
        }
        
        // 然后检查metadata中的cover信息
        if let Some(cover) = self.metadata.cover() {
            // 如果cover是ID，查找对应的manifest项
            if let Some(item) = self.manifest.get(&cover) {
                return Some(item.href.clone());
            }
            // 如果cover不是ID而是直接的文件路径
            return Some(cover);
        }
        
        // 最后检查custom元数据中的cover信息
        let other = self.metadata.other();
        if let Some(cover_id) = other.get("cover") {
            if let Some(item) = self.manifest.get(cover_id) {
                return Some(item.href.clone());
            }
            // 如果cover值不是ID而是直接的文件路径
            return Some(cover_id.clone());
        }
        
        None
    }
    
    /// 获取所有章节文件的路径(按阅读顺序)
    /// 
    /// # 返回值
    /// * `Vec<String>` - 章节文件路径列表
    pub fn get_chapter_paths(&self) -> Vec<String> {
        self.spine.iter()
            .filter(|spine_item| spine_item.is_linear())
            .filter_map(|spine_item| self.manifest.get(&spine_item.idref))
            .map(|manifest_item| manifest_item.href.clone())
            .collect()
    }
    
    /// 根据ID获取清单项
    /// 
    /// # 参数
    /// * `id` - 清单项ID
    /// 
    /// # 返回值
    /// * `Option<&ManifestItem>` - 清单项引用
    pub fn get_manifest_item(&self, id: &str) -> Option<&ManifestItem> {
        self.manifest.get(id)
    }
    
    /// 获取所有图片文件路径
    /// 
    /// # 返回值
    /// * `Vec<String>` - 图片文件路径列表
    pub fn get_image_paths(&self) -> Vec<String> {
        self.manifest.values()
            .filter(|item| item.is_image())
            .map(|item| item.href.clone())
            .collect()
    }
    
    /// 获取所有CSS文件路径
    /// 
    /// # 返回值
    /// * `Vec<String>` - CSS文件路径列表
    pub fn get_css_paths(&self) -> Vec<String> {
        self.manifest.values()
            .filter(|item| item.is_css())
            .map(|item| item.href.clone())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_epub3_opf_parsing_with_refines() {
        // 创建一个简化的测试，避免复杂的XML字符串
        let mut opf = Opf {
            version: "3.0".to_string(),
            metadata: Metadata::new(),
            manifest: std::collections::HashMap::new(),
            spine: Vec::new(),
            spine_toc: None,
        };

        // 手动添加EPUB3标准的作者信息
        let mut dc_attributes = std::collections::HashMap::new();
        dc_attributes.insert("id".to_string(), "creator1".to_string());
        opf.metadata.add_dublin_core("creator".to_string(), "J.K. Rowling".to_string(), dc_attributes);
        
        // 添加关联的refines元数据
        opf.metadata.add_meta_refines_based(
            "creator1".to_string(),
            "role".to_string(),
            "aut".to_string(),
            Some("marc:relators".to_string())
        );
        
        opf.metadata.add_meta_refines_based(
            "creator1".to_string(),
            "file-as".to_string(),
            "Rowling, J.K.".to_string(),
            None
        );
        
        opf.metadata.add_meta_refines_based(
            "creator1".to_string(),
            "display-seq".to_string(),
            "1".to_string(),
            None
        );

        // 验证创建者信息被正确提取
        let creators = opf.metadata.creators();
        assert_eq!(creators.len(), 1);
        
        let creator = &creators[0];
        assert_eq!(creator.name, "J.K. Rowling");
        assert_eq!(creator.role, Some("author".to_string()));
        assert_eq!(creator.display_seq, Some(1));
        assert_eq!(creator.id, Some("creator1".to_string()));
    }

    #[test]
    fn test_simple_xml_parsing() {
        let simple_xml = concat!(
            r#"<?xml version="1.0"?>"#,
            r#"<package xmlns="http://www.idpf.org/2007/opf" version="3.0">"#,
            r#"<metadata xmlns:dc="http://purl.org/dc/elements/1.1/">"#,
            r#"<dc:title>Test Book</dc:title>"#,
            r#"<dc:creator id="author1">Test Author</dc:creator>"#,
            r#"</metadata>"#,
            r#"<manifest></manifest>"#,
            r#"<spine></spine>"#,
            r#"</package>"#
        );

        let opf = Opf::parse_xml(simple_xml).expect("解析简单OPF失败");
        assert_eq!(opf.version, "3.0");
        assert_eq!(opf.metadata.title(), Some("Test Book".to_string()));
        
        let creators = opf.metadata.creators();
        assert_eq!(creators.len(), 1);
        assert_eq!(creators[0].name, "Test Author");
        assert_eq!(creators[0].id, Some("author1".to_string()));
    }

    #[test]
    fn test_basic_opf_structure() {
        // 测试基本的OPF结构解析
        let simple_opf = r#"<?xml version="1.0"?>
<package xmlns="http://www.idpf.org/2007/opf" version="3.0">
<metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
<dc:title>Sample Book</dc:title>
<dc:creator>Sample Author</dc:creator>
</metadata>
<manifest>
<item id="item1" href="chapter1.xhtml" media-type="application/xhtml+xml"/>
</manifest>
<spine>
<itemref idref="item1"/>
</spine>
</package>"#;

        let opf = Opf::parse_xml(simple_opf).expect("解析基本OPF失败");
        
        assert_eq!(opf.version, "3.0");
        assert_eq!(opf.metadata.title(), Some("Sample Book".to_string()));
        
        let creators = opf.metadata.creators();
        assert_eq!(creators.len(), 1);
        assert_eq!(creators[0].name, "Sample Author");
        
        assert_eq!(opf.manifest.len(), 1);
        assert_eq!(opf.spine.len(), 1);
    }
} 