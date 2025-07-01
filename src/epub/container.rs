use crate::epub::error::{EpubError, Result};
use quick_xml::events::Event;
use quick_xml::reader::Reader;

/// Container.xml中的rootfile信息
#[derive(Debug, Clone)]
pub struct RootFile {
    pub full_path: String,
    pub media_type: String,
}

/// Container.xml的解析结果
#[derive(Debug, Clone)]
pub struct Container {
    pub rootfiles: Vec<RootFile>,
}

impl Container {
    /// 解析container.xml内容
    /// 
    /// # 参数
    /// * `xml_content` - container.xml的文件内容
    /// 
    /// # 返回值
    /// * `Result<Container, EpubError>` - 解析后的Container信息
    pub fn parse_xml(xml_content: &str) -> Result<Container> {
        let mut reader = Reader::from_str(xml_content);
        reader.config_mut().trim_text(true);
        reader.config_mut().expand_empty_elements = true;
        
        let mut rootfiles = Vec::new();
        let mut buf = Vec::new();
        let mut in_rootfiles = false;
        
        loop {
            match reader.read_event_into(&mut buf)? {
                Event::Start(ref e) | Event::Empty(ref e) => {
                    let local_name = e.local_name();
                    match local_name.as_ref() {
                        b"rootfiles" => {
                            in_rootfiles = true;
                        }
                        b"rootfile" if in_rootfiles => {
                            let mut full_path = String::new();
                            let mut media_type = String::new();
                            
                            // 解析属性
                            for attr_result in e.attributes() {
                                let attr = attr_result.map_err(|e| EpubError::XmlError(quick_xml::Error::InvalidAttr(e)))?;
                                match attr.key.local_name().as_ref() {
                                    b"full-path" => {
                                        full_path = String::from_utf8_lossy(&attr.value).to_string();
                                    }
                                    b"media-type" => {
                                        media_type = String::from_utf8_lossy(&attr.value).to_string();
                                    }
                                    _ => {}
                                }
                            }
                            
                            if !full_path.is_empty() && !media_type.is_empty() {
                                rootfiles.push(RootFile {
                                    full_path,
                                    media_type,
                                });
                            }
                        }
                        _ => {}
                    }
                }
                Event::End(ref e) => {
                    if e.local_name().as_ref() == b"rootfiles" {
                        in_rootfiles = false;
                    }
                }
                Event::Eof => break,
                _ => {}
            }
            buf.clear();
        }
        
        if rootfiles.is_empty() {
            return Err(EpubError::ContainerParseError(
                "没有找到任何rootfile条目".to_string()
            ));
        }
        
        Ok(Container { rootfiles })
    }
    
    /// 获取主要的OPF文件路径
    /// 
    /// # 返回值
    /// * `Option<String>` - OPF文件的完整路径
    pub fn get_opf_path(&self) -> Option<String> {
        // 查找第一个application/oebps-package+xml类型的rootfile
        for rootfile in &self.rootfiles {
            if rootfile.media_type == "application/oebps-package+xml" {
                return Some(rootfile.full_path.clone());
            }
        }
        
        // 如果没有找到标准类型，返回第一个rootfile
        self.rootfiles.first().map(|rf| rf.full_path.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_container_xml() {
        let container_xml = r#"<?xml version="1.0"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
    <rootfiles>
        <rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/>
        <rootfile full-path="OEBPS/toc.ncx" media-type="application/x-dtbncx+xml"/>
    </rootfiles>
</container>"#;
        
        let result = Container::parse_xml(container_xml);
        assert!(result.is_ok());
        
        let container = result.unwrap();
        assert_eq!(container.rootfiles.len(), 2);
        
        let first_rootfile = &container.rootfiles[0];
        assert_eq!(first_rootfile.full_path, "OEBPS/content.opf");
        assert_eq!(first_rootfile.media_type, "application/oebps-package+xml");
        
        let second_rootfile = &container.rootfiles[1];
        assert_eq!(second_rootfile.full_path, "OEBPS/toc.ncx");
        assert_eq!(second_rootfile.media_type, "application/x-dtbncx+xml");
    }

    #[test]
    fn test_get_opf_path() {
        let container = Container {
            rootfiles: vec![
                RootFile {
                    full_path: "OEBPS/content.opf".to_string(),
                    media_type: "application/oebps-package+xml".to_string(),
                },
                RootFile {
                    full_path: "OEBPS/toc.ncx".to_string(),
                    media_type: "application/x-dtbncx+xml".to_string(),
                },
            ],
        };
        
        let opf_path = container.get_opf_path();
        assert_eq!(opf_path, Some("OEBPS/content.opf".to_string()));
    }
    
    #[test]
    fn test_parse_container_xml_with_single_rootfile() {
        let container_xml = r#"<?xml version="1.0"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
    <rootfiles>
        <rootfile full-path="content.opf" media-type="application/oebps-package+xml"/>
    </rootfiles>
</container>"#;
        
        let result = Container::parse_xml(container_xml);
        assert!(result.is_ok());
        
        let container = result.unwrap();
        assert_eq!(container.rootfiles.len(), 1);
        assert_eq!(container.get_opf_path(), Some("content.opf".to_string()));
    }
} 