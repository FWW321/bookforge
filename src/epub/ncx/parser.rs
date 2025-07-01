//! NCX解析器模块
//! 
//! 提供NCX（Navigation Control file for XML）文件的XML解析功能。

use crate::epub::error::{EpubError, Result};
use crate::epub::ncx::{
    NcxMetadata, DocTitle, NavMap, NavPoint, NavLabel, NavContent,
    PageList, PageTarget,
};
use quick_xml::events::Event;
use quick_xml::reader::Reader;


/// NCX文件解析结果
#[derive(Debug, Clone)]
pub struct Ncx {
    /// NCX版本
    pub version: String,
    /// XML语言
    pub xml_lang: Option<String>,
    /// 元数据
    pub metadata: NcxMetadata,
    /// 文档标题
    pub doc_title: Option<DocTitle>,
    /// 导航地图
    pub nav_map: NavMap,
    /// 页面列表（可选）
    pub page_list: Option<PageList>,
}

impl Ncx {
    /// 解析NCX文件内容
    /// 
    /// # 参数
    /// * `xml_content` - NCX文件的XML内容
    /// 
    /// # 返回值
    /// * `Result<Ncx, EpubError>` - 解析后的NCX信息
    pub fn parse_xml(xml_content: &str) -> Result<Ncx> {
        let mut reader = Reader::from_str(xml_content);
        reader.config_mut().trim_text(true);
        reader.config_mut().expand_empty_elements = true;

        let mut version = String::new();
        let mut xml_lang = None;
        let mut metadata = NcxMetadata::new();
        let mut doc_title = None;
        let mut nav_map = NavMap::new();
        let mut page_list = None;

        let mut buf = Vec::new();
        let mut current_section = String::new();
        let mut text_content = String::new();
        
        // 导航点解析状态
        let mut nav_point_stack: Vec<NavPoint> = Vec::new();
        let mut current_nav_point: Option<NavPoint> = None;
        let mut current_nav_label: Option<NavLabel> = None;
        let mut current_nav_content: Option<NavContent> = None;
        
        // 页面列表解析状态
        let mut current_page_list = PageList::new();
        let mut current_page_target: Option<PageTarget> = None;
        
        // 文档标题和作者解析状态
        let mut current_doc_title: Option<DocTitle> = None;

        loop {
            match reader.read_event_into(&mut buf)? {
                Event::Start(ref e) => {
                    let local_name_bytes = e.local_name();
                    let local_name = String::from_utf8_lossy(local_name_bytes.as_ref());
                    
                    match local_name.as_ref() {
                        "ncx" => {
                            let (ncx_version, ncx_lang) = Self::parse_ncx_attributes(e)?;
                            version = ncx_version;
                            xml_lang = ncx_lang;
                        }
                        "head" => {
                            current_section = "head".to_string();
                        }
                        "docTitle" => {
                            current_section = "docTitle".to_string();
                            current_doc_title = Some(DocTitle::new(String::new()));
                        }
                        "navMap" => {
                            current_section = "navMap".to_string();
                        }
                        "pageList" => {
                            current_section = "pageList".to_string();
                            current_page_list = PageList::new();
                        }
                        "meta" if current_section == "head" => {
                            Self::parse_meta_element(e, &mut metadata)?;
                        }
                        "navPoint" if current_section == "navMap" => {
                            let (id, play_order, class) = Self::parse_nav_point_attributes(e)?;
                            
                            // 如果当前有未完成的导航点，将其推入栈中
                            if let Some(nav_point) = current_nav_point.take() {
                                nav_point_stack.push(nav_point);
                            }
                            
                            current_nav_point = Some(NavPoint {
                                id,
                                play_order,
                                class,
                                nav_label: NavLabel::new(String::new()),
                                content: NavContent::new(String::new()),
                                children: Vec::new(),
                            });
                        }
                        "navLabel" if current_section == "navMap" => {
                            current_nav_label = Some(NavLabel::new(String::new()));
                        }
                        "content" if current_section == "navMap" => {
                            let src = Self::parse_content_src(e)?;
                            current_nav_content = Some(NavContent::new(src));
                        }
                        "pageTarget" if current_section == "pageList" => {
                            let (id, page_type, value, play_order) = Self::parse_page_target_attributes(e)?;
                            current_page_target = Some(PageTarget::new(
                                id,
                                page_type,
                                value,
                                play_order,
                                NavLabel::new(String::new()),
                                NavContent::new(String::new()),
                            ));
                        }
                        "navLabel" if current_section == "pageList" => {
                            // 页面列表中的导航标签处理将在text内容中完成
                        }
                        "content" if current_section == "pageList" => {
                            // 页面目标内容处理将在属性解析中完成
                        }
                        _ => {}
                    }
                    text_content.clear();
                }
                Event::Empty(ref e) => {
                    let local_name_bytes = e.local_name();
                    let local_name = String::from_utf8_lossy(local_name_bytes.as_ref());
                    
                    match local_name.as_ref() {
                        "meta" if current_section == "head" => {
                            Self::parse_meta_element(e, &mut metadata)?;
                        }
                        "content" if current_section == "navMap" => {
                            let src = Self::parse_content_src(e)?;
                            current_nav_content = Some(NavContent::new(src));
                        }
                        _ => {}
                    }
                }
                Event::End(ref e) => {
                    let local_name_bytes = e.local_name();
                    let local_name = String::from_utf8_lossy(local_name_bytes.as_ref());
                    
                    match local_name.as_ref() {
                        "head" | "navMap" => {
                            current_section.clear();
                        }
                        "pageList" => {
                            if !current_page_list.page_targets.is_empty() {
                                page_list = Some(current_page_list.clone());
                            }
                            current_section.clear();
                        }
                        "docTitle" => {
                            if let Some(mut title) = current_doc_title.take() {
                                title.text = text_content.trim().to_string();
                                doc_title = Some(title);
                            }
                            current_section.clear();
                        }
                        "text" if current_section == "navMap" => {
                            if let Some(ref mut nav_label) = current_nav_label {
                                nav_label.text = text_content.trim().to_string();
                            }
                        }
                        "navLabel" if current_section == "navMap" => {
                            if let (Some(nav_label), Some(ref mut nav_point)) = (current_nav_label.take(), current_nav_point.as_mut()) {
                                nav_point.nav_label = nav_label;
                            }
                        }
                        "content" if current_section == "navMap" => {
                            if let (Some(nav_content), Some(ref mut nav_point)) = (current_nav_content.take(), current_nav_point.as_mut()) {
                                nav_point.content = nav_content;
                            }
                        }
                        "navPoint" if current_section == "navMap" => {
                            if let Some(nav_point) = current_nav_point.take() {
                                if let Some(mut parent) = nav_point_stack.pop() {
                                    parent.add_child(nav_point);
                                    current_nav_point = Some(parent);
                                } else {
                                    nav_map.add_nav_point(nav_point);
                                }
                            }
                        }
                        "text" if current_section == "pageList" => {
                            // 处理页面列表中的文本内容
                            if let Some(ref mut page_target) = current_page_target {
                                page_target.nav_label.text = text_content.trim().to_string();
                            } else if current_page_list.nav_label.is_none() {
                                current_page_list.nav_label = Some(NavLabel::new(text_content.trim().to_string()));
                            }
                        }
                        "pageTarget" if current_section == "pageList" => {
                            if let Some(page_target) = current_page_target.take() {
                                current_page_list.add_page_target(page_target);
                            }
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

        // 按playOrder排序所有导航点
        let mut sorted_nav_map = nav_map;
        sorted_nav_map.sort_by_play_order();

        Ok(Ncx {
            version,
            xml_lang,
            metadata,
            doc_title,
            nav_map: sorted_nav_map,
            page_list,
        })
    }

    /// 解析NCX根元素的属性
    fn parse_ncx_attributes(e: &quick_xml::events::BytesStart) -> Result<(String, Option<String>)> {
        let mut version = String::new();
        let mut xml_lang = None;

        for attr_result in e.attributes() {
            let attr = attr_result.map_err(|err| EpubError::XmlError(quick_xml::Error::InvalidAttr(err)))?;
            match attr.key.local_name().as_ref() {
                b"version" => {
                    version = String::from_utf8_lossy(&attr.value).to_string();
                }
                b"xml:lang" => {
                    xml_lang = Some(String::from_utf8_lossy(&attr.value).to_string());
                }
                _ => {}
            }
        }

        Ok((version, xml_lang))
    }

    /// 解析meta元素
    fn parse_meta_element(e: &quick_xml::events::BytesStart, metadata: &mut NcxMetadata) -> Result<()> {
        let mut name = String::new();
        let mut content = String::new();

        for attr_result in e.attributes() {
            let attr = attr_result.map_err(|err| EpubError::XmlError(quick_xml::Error::InvalidAttr(err)))?;
            match attr.key.local_name().as_ref() {
                b"name" => {
                    name = String::from_utf8_lossy(&attr.value).to_string();
                }
                b"content" => {
                    content = String::from_utf8_lossy(&attr.value).to_string();
                }
                _ => {}
            }
        }

        // 处理已知的元数据字段
        match name.as_str() {
            "dtb:uid" => {
                metadata.uid = Some(content);
            }
            "dtb:depth" => {
                metadata.depth = content.parse().ok();
            }
            "dtb:totalPageCount" => {
                metadata.total_page_count = content.parse().ok();
            }
            "dtb:maxPageNumber" => {
                metadata.max_page_number = content.parse().ok();
            }
            _ => {
                metadata.other_metadata.insert(name, content);
            }
        }

        Ok(())
    }

    /// 解析navPoint元素的属性
    fn parse_nav_point_attributes(e: &quick_xml::events::BytesStart) -> Result<(String, u32, Option<String>)> {
        let mut id = String::new();
        let mut play_order = 0;
        let mut class = None;

        for attr_result in e.attributes() {
            let attr = attr_result.map_err(|err| EpubError::XmlError(quick_xml::Error::InvalidAttr(err)))?;
            match attr.key.local_name().as_ref() {
                b"id" => {
                    id = String::from_utf8_lossy(&attr.value).to_string();
                }
                b"playOrder" => {
                    play_order = String::from_utf8_lossy(&attr.value).parse().unwrap_or(0);
                }
                b"class" => {
                    class = Some(String::from_utf8_lossy(&attr.value).to_string());
                }
                _ => {}
            }
        }

        Ok((id, play_order, class))
    }

    /// 解析content元素的src属性
    fn parse_content_src(e: &quick_xml::events::BytesStart) -> Result<String> {
        for attr_result in e.attributes() {
            let attr = attr_result.map_err(|err| EpubError::XmlError(quick_xml::Error::InvalidAttr(err)))?;
            if attr.key.local_name().as_ref() == b"src" {
                return Ok(String::from_utf8_lossy(&attr.value).to_string());
            }
        }
        Ok(String::new())
    }

    /// 解析pageTarget元素的属性
    fn parse_page_target_attributes(e: &quick_xml::events::BytesStart) -> Result<(String, String, String, u32)> {
        let mut id = String::new();
        let mut page_type = "normal".to_string();
        let mut value = String::new();
        let mut play_order = 0;

        for attr_result in e.attributes() {
            let attr = attr_result.map_err(|err| EpubError::XmlError(quick_xml::Error::InvalidAttr(err)))?;
            match attr.key.local_name().as_ref() {
                b"id" => {
                    id = String::from_utf8_lossy(&attr.value).to_string();
                }
                b"type" => {
                    page_type = String::from_utf8_lossy(&attr.value).to_string();
                }
                b"value" => {
                    value = String::from_utf8_lossy(&attr.value).to_string();
                }
                b"playOrder" => {
                    play_order = String::from_utf8_lossy(&attr.value).parse().unwrap_or(0);
                }
                _ => {}
            }
        }

        Ok((id, page_type, value, play_order))
    }

    /// 获取NCX文件的唯一标识符
    pub fn get_uid(&self) -> Option<&String> {
        self.metadata.uid.as_ref()
    }

    /// 获取导航深度
    pub fn get_depth(&self) -> u32 {
        self.metadata.depth.unwrap_or_else(|| self.nav_map.get_depth())
    }

    /// 获取文档标题文本
    pub fn get_title(&self) -> Option<&String> {
        self.doc_title.as_ref().map(|title| &title.text)
    }

    /// 获取所有导航点的平铺列表
    pub fn get_all_nav_points(&self) -> Vec<&NavPoint> {
        self.nav_map.get_all_nav_points()
    }

    /// 根据ID查找导航点
    pub fn find_nav_point_by_id(&self, id: &str) -> Option<&NavPoint> {
        self.nav_map.find_nav_point_by_id(id)
    }

    /// 获取章节路径列表
    pub fn get_chapter_paths(&self) -> Vec<String> {
        self.get_all_nav_points()
            .iter()
            .map(|point| point.content.src.clone())
            .collect()
    }

    /// 检查是否有页面列表
    pub fn has_page_list(&self) -> bool {
        self.page_list.is_some()
    }

    /// 获取页面列表引用
    pub fn get_page_list(&self) -> Option<&PageList> {
        self.page_list.as_ref()
    }

    // 注意：创建目录树现在需要 Epub 实例，请使用 create_toc_tree_from_ncx 函数


} 