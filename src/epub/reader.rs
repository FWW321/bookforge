use std::fs::File;
use std::io::Read;
use std::path::Path;
use zip::ZipArchive;

use crate::epub::error::{EpubError, Result};
use crate::epub::container::Container;
use crate::epub::opf::Opf;

/// 表示一个EPUB文件
pub struct Epub {
    archive: ZipArchive<File>,
}

impl Epub {
    /// 从文件路径创建Epub实例
    /// 
    /// # 参数
    /// * `path` - epub文件的路径
    /// 
    /// # 返回值
    /// * `Result<Epub, EpubError>` - 成功返回Epub实例，失败返回错误
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Epub> {
        let file = File::open(path)?;
        let archive = ZipArchive::new(file)?;
        
        let mut epub = Epub { archive };
        epub.validate()?;
        
        Ok(epub)
    }
    
    /// 验证EPUB文件的合法性
    /// 
    /// 检查步骤：
    /// 1. 检查是否存在mimetype文件
    /// 2. 验证mimetype文件的内容是否为"application/epub+zip"
    fn validate(&mut self) -> Result<()> {
        // 检查mimetype文件是否存在
        let mimetype_file = self.archive.by_name("mimetype");
        
        match mimetype_file {
            Ok(mut file) => {
                let mut content = String::new();
                file.read_to_string(&mut content)?;
                
                // 去除可能的换行符和空白字符
                let content = content.trim();
                let expected_mimetype = "application/epub+zip";
                
                if content != expected_mimetype {
                    return Err(EpubError::InvalidMimetype {
                        expected: expected_mimetype.to_string(),
                        found: content.to_string(),
                    });
                }
                
                println!("✅ EPUB验证成功: mimetype文件正确");
                Ok(())
            }
            Err(_) => Err(EpubError::MissingMimetype),
        }
    }
    
    /// 列出EPUB文件中的所有条目
    pub fn list_files(&mut self) -> Result<Vec<String>> {
        let mut files = Vec::new();
        
        for i in 0..self.archive.len() {
            let file = self.archive.by_index(i)?;
            files.push(file.name().to_string());
        }
        
        Ok(files)
    }
    
    /// 提取指定文件的内容
    /// 
    /// # 参数
    /// * `filename` - 要提取的文件名
    /// 
    /// # 返回值
    /// * `Result<String, EpubError>` - 文件内容
    pub fn extract_file(&mut self, filename: &str) -> Result<String> {
        let mut file = self.archive.by_name(filename)?;
        let mut content = String::new();
        file.read_to_string(&mut content)?;
        Ok(content)
    }
    
    /// 提取指定文件的二进制内容
    /// 
    /// # 参数
    /// * `filename` - 要提取的文件名
    /// 
    /// # 返回值
    /// * `Result<Vec<u8>, EpubError>` - 文件的二进制内容
    pub fn extract_binary_file(&mut self, filename: &str) -> Result<Vec<u8>> {
        let mut file = self.archive.by_name(filename)?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;
        Ok(buffer)
    }
    
    /// 解析container.xml文件
    /// 
    /// # 返回值
    /// * `Result<Container, EpubError>` - 解析后的Container信息
    pub fn parse_container(&mut self) -> Result<Container> {
        let container_content = self.extract_file("META-INF/container.xml")?;
        Container::parse_xml(&container_content)
    }
    
    /// 获取主要的OPF文件路径
    /// 
    /// # 返回值
    /// * `Result<String, EpubError>` - OPF文件的完整路径
    pub fn get_opf_path(&mut self) -> Result<String> {
        let container = self.parse_container()?;
        
        container.get_opf_path().ok_or_else(|| {
            EpubError::ContainerParseError(
                "container.xml中没有找到有效的rootfile".to_string()
            )
        })
    }
    
    /// 解析OPF文件
    /// 
    /// # 返回值
    /// * `Result<Opf, EpubError>` - 解析后的OPF信息
    pub fn parse_opf(&mut self) -> Result<Opf> {
        self.parse_opf_with_config(None)
    }

    /// 使用指定的配置文件解析OPF文件
    /// 
    /// # 参数
    /// * `config_path` - 配置文件路径(可选)，如果不提供则使用默认配置
    /// 
    /// # 返回值
    /// * `Result<Opf, EpubError>` - 解析后的OPF信息
    pub fn parse_opf_with_config(&mut self, config_path: Option<&str>) -> Result<Opf> {
        let opf_path = self.get_opf_path()?;
        let opf_content = self.extract_file(&opf_path)?;
        
        Opf::parse_xml_with_config(&opf_content, config_path).map_err(|e| match e {
            EpubError::XmlError(xml_err) => EpubError::OpfParseError(format!("XML解析错误: {}", xml_err)),
            other => other,
        })
    }
    
    /// 获取书籍的基本信息
    /// 
    /// # 返回值
    /// * `Result<(String, Vec<String>), EpubError>` - (书名, 作者列表)
    pub fn get_book_info(&mut self) -> Result<(String, Vec<String>)> {
        let opf = self.parse_opf()?;
        
        let title = opf.metadata.title()
            .unwrap_or_else(|| "未知标题".to_string());
        
        let authors = opf.metadata.creators()
            .iter()
            .map(|creator| creator.name.clone())
            .collect();
        
        Ok((title, authors))
    }
    
    /// 获取所有章节内容
    /// 
    /// # 返回值
    /// * `Result<Vec<(String, String)>, EpubError>` - (文件路径, 内容)的列表
    pub fn get_chapters(&mut self) -> Result<Vec<(String, String)>> {
        let opf = self.parse_opf()?;
        let chapter_paths = opf.get_chapter_paths();
        
        let mut chapters = Vec::new();
        let opf_dir = self.get_opf_directory()?;
        
        for path in chapter_paths {
            let full_path = if opf_dir.is_empty() {
                path.clone()
            } else {
                format!("{}/{}", opf_dir, path)
            };
            
            match self.extract_file(&full_path) {
                Ok(content) => chapters.push((path, content)),
                Err(e) => {
                    println!("警告: 无法读取章节文件 {}: {}", full_path, e);
                    continue;
                }
            }
        }
        
        Ok(chapters)
    }
    
    /// 获取OPF文件所在的目录
    /// 
    /// # 返回值
    /// * `Result<String, EpubError>` - OPF文件所在的目录路径
    pub fn get_opf_directory(&mut self) -> Result<String> {
        let opf_path = self.get_opf_path()?;
        
        if let Some(parent) = std::path::Path::new(&opf_path).parent() {
            Ok(parent.to_string_lossy().to_string())
        } else {
            Ok(String::new())
        }
    }

    /// 获取封面图片的二进制数据
    /// 
    /// 此方法会尝试多种策略来查找封面：
    /// 1. 检查manifest中具有cover-image属性的项目
    /// 2. 检查metadata中的cover信息
    /// 3. 检查自定义元数据中的cover信息
    /// 4. 尝试根据常见的封面文件名查找（如cover.jpg, cover.png等）
    /// 5. 查找第一个图片文件作为封面候选
    /// 
    /// # 返回值
    /// * `Result<Option<(Vec<u8>, String)>, EpubError>` - 成功时返回(封面二进制数据, 文件扩展名)，没有封面时返回None
    /// 
    /// # 示例
    /// 
    /// ```rust
    /// use bookforge::epub::Epub;
    /// 
    /// let mut epub = Epub::new("book.epub")?;
    /// match epub.get_cover_image()? {
    ///     Some((cover_data, extension)) => {
    ///         println!("找到封面，格式: {}, 大小: {} bytes", extension, cover_data.len());
    ///         // 可以将cover_data写入文件或进行其他处理
    ///     }
    ///     None => {
    ///         println!("未找到封面图片");
    ///     }
    /// }
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn get_cover_image(&mut self) -> Result<Option<(Vec<u8>, String)>> {
        let opf = self.parse_opf()?;
        let opf_dir = self.get_opf_directory()?;
        
        // 策略1: 使用OPF中的get_cover_path方法
        if let Some(cover_path) = opf.get_cover_path() {
            if let Some(result) = self.try_extract_cover(&opf_dir, &cover_path)? {
                return Ok(Some(result));
            }
        }
        
        // 策略2: 尝试常见的封面文件名
        let common_cover_names = [
            "cover.jpg", "cover.jpeg", "cover.png", "cover.gif", "cover.webp",
            "Cover.jpg", "Cover.jpeg", "Cover.png", "Cover.gif", "Cover.webp",
            "COVER.jpg", "COVER.jpeg", "COVER.png", "COVER.gif", "COVER.webp",
            "front.jpg", "front.jpeg", "front.png", "front.gif", "front.webp",
            "title.jpg", "title.jpeg", "title.png", "title.gif", "title.webp",
        ];
        
        for cover_name in &common_cover_names {
            // 在OPF目录中查找
            let full_path = if opf_dir.is_empty() {
                cover_name.to_string()
            } else {
                format!("{}/{}", opf_dir, cover_name)
            };
            
            if let Some(result) = self.try_extract_cover("", &full_path)? {
                return Ok(Some(result));
            }
            
            // 在根目录中查找
            if let Some(result) = self.try_extract_cover("", cover_name)? {
                return Ok(Some(result));
            }
            
            // 在images目录中查找
            let images_path = if opf_dir.is_empty() {
                format!("images/{}", cover_name)
            } else {
                format!("{}/images/{}", opf_dir, cover_name)
            };
            
            if let Some(result) = self.try_extract_cover("", &images_path)? {
                return Ok(Some(result));
            }
        }
        
        // 策略3: 查找manifest中的第一个图片文件
        let image_paths = opf.get_image_paths();
        for image_path in &image_paths {
            let full_path = if opf_dir.is_empty() {
                image_path.clone()
            } else {
                format!("{}/{}", opf_dir, image_path)
            };
            
            if let Some(result) = self.try_extract_cover("", &full_path)? {
                return Ok(Some(result));
            }
        }
        
        // 策略4: 遍历所有文件，查找第一个图片文件
        let all_files = self.list_files()?;
        for file_path in &all_files {
            if self.is_image_file(file_path) {
                if let Some(result) = self.try_extract_cover("", file_path)? {
                    return Ok(Some(result));
                }
            }
        }
        
        // 没有找到封面
        Ok(None)
    }
    
    /// 尝试提取封面文件
    /// 
    /// # 参数
    /// * `base_dir` - 基础目录
    /// * `file_path` - 文件路径
    /// 
    /// # 返回值
    /// * `Result<Option<(Vec<u8>, String)>, EpubError>` - 成功提取时返回(数据, 扩展名)，文件不存在时返回None
    fn try_extract_cover(&mut self, base_dir: &str, file_path: &str) -> Result<Option<(Vec<u8>, String)>> {
        let full_path = if base_dir.is_empty() {
            file_path.to_string()
        } else {
            format!("{}/{}", base_dir, file_path)
        };
        
        match self.extract_binary_file(&full_path) {
            Ok(data) => {
                if data.is_empty() {
                    return Ok(None);
                }
                
                // 获取文件扩展名
                let extension = std::path::Path::new(file_path)
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .unwrap_or("unknown")
                    .to_lowercase();
                
                Ok(Some((data, extension)))
            }
            Err(EpubError::Zip(zip::result::ZipError::FileNotFound)) => Ok(None),
            Err(e) => Err(e),
        }
    }
    
    /// 检查文件是否为图片文件
    /// 
    /// # 参数
    /// * `file_path` - 文件路径
    /// 
    /// # 返回值
    /// * `bool` - 是否为图片文件
    fn is_image_file(&self, file_path: &str) -> bool {
        if let Some(extension) = std::path::Path::new(file_path)
            .extension()
            .and_then(|ext| ext.to_str())
        {
            match extension.to_lowercase().as_str() {
                "jpg" | "jpeg" | "png" | "gif" | "webp" | "bmp" | "svg" => true,
                _ => false,
            }
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use zip::write::FileOptions;
    use zip::ZipWriter;

    /// 创建一个测试用的有效EPUB文件
    fn create_test_epub(path: &str, mimetype_content: &str) -> Result<()> {
        let file = File::create(path)?;
        let mut zip = ZipWriter::new(file);
        
        // 添加mimetype文件
        zip.start_file("mimetype", FileOptions::<()>::default())?;
        zip.write_all(mimetype_content.as_bytes())?;
        
        // 添加标准的container.xml文件
        zip.start_file("META-INF/container.xml", FileOptions::<()>::default())?;
        let container_xml = r#"<?xml version="1.0"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
    <rootfiles>
        <rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/>
    </rootfiles>
</container>"#;
        zip.write_all(container_xml.as_bytes())?;
        
        // 添加OPF文件
        zip.start_file("OEBPS/content.opf", FileOptions::<()>::default())?;
        let opf_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<package version="3.0" xmlns="http://www.idpf.org/2007/opf" unique-identifier="BookId">
    <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
        <dc:title>测试书籍</dc:title>
        <dc:creator role="aut">测试作者</dc:creator>
        <dc:language>zh-CN</dc:language>
        <dc:identifier id="BookId" scheme="ISBN">978-1234567890</dc:identifier>
    </metadata>
    <manifest>
        <item id="chapter1" href="text/chapter1.xhtml" media-type="application/xhtml+xml"/>
        <item id="chapter2" href="text/chapter2.xhtml" media-type="application/xhtml+xml"/>
    </manifest>
    <spine>
        <itemref idref="chapter1"/>
        <itemref idref="chapter2"/>
    </spine>
</package>"#;
        zip.write_all(opf_xml.as_bytes())?;
        
        // 添加测试章节文件
        zip.start_file("OEBPS/text/chapter1.xhtml", FileOptions::<()>::default())?;
        let chapter1 = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE html>
<html xmlns="http://www.w3.org/1999/xhtml">
<head><title>第一章</title></head>
<body><h1>第一章</h1><p>这是第一章的内容。</p></body>
</html>"#;
        zip.write_all(chapter1.as_bytes())?;
        
        zip.start_file("OEBPS/text/chapter2.xhtml", FileOptions::<()>::default())?;
        let chapter2 = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE html>
<html xmlns="http://www.w3.org/1999/xhtml">
<head><title>第二章</title></head>
<body><h1>第二章</h1><p>这是第二章的内容。</p></body>
</html>"#;
        zip.write_all(chapter2.as_bytes())?;
        
        zip.finish()?;
        Ok(())
    }

    #[test]
    fn test_valid_epub() {
        let test_file = "test_valid.epub";
        create_test_epub(test_file, "application/epub+zip").unwrap();
        
        let result = Epub::new(test_file);
        assert!(result.is_ok());
        
        // 清理测试文件
        let _ = fs::remove_file(test_file);
    }

    #[test]
    fn test_invalid_mimetype() {
        let test_file = "test_invalid.epub";
        create_test_epub(test_file, "invalid/mimetype").unwrap();
        
        let result = Epub::new(test_file);
        assert!(result.is_err());
        
        if let Err(EpubError::InvalidMimetype { expected, found }) = result {
            assert_eq!(expected, "application/epub+zip");
            assert_eq!(found, "invalid/mimetype");
        } else {
            panic!("期望InvalidMimetype错误");
        }
        
        // 清理测试文件
        let _ = fs::remove_file(test_file);
    }

    #[test]
    fn test_parse_container_from_epub() {
        let test_file = "test_container.epub";
        create_test_epub(test_file, "application/epub+zip").unwrap();
        
        let mut epub = Epub::new(test_file).unwrap();
        let container_result = epub.parse_container();
        assert!(container_result.is_ok());
        
        let container = container_result.unwrap();
        assert_eq!(container.rootfiles.len(), 1);
        
        let rootfile = &container.rootfiles[0];
        assert_eq!(rootfile.full_path, "OEBPS/content.opf");
        assert_eq!(rootfile.media_type, "application/oebps-package+xml");
        
        // 清理测试文件
        let _ = fs::remove_file(test_file);
    }

    #[test]
    fn test_get_opf_path() {
        let test_file = "test_opf_path.epub";
        create_test_epub(test_file, "application/epub+zip").unwrap();
        
        let mut epub = Epub::new(test_file).unwrap();
        let opf_path_result = epub.get_opf_path();
        assert!(opf_path_result.is_ok());
        
        let opf_path = opf_path_result.unwrap();
        assert_eq!(opf_path, "OEBPS/content.opf");
        
        // 清理测试文件
        let _ = fs::remove_file(test_file);
    }

    #[test]
    fn test_parse_opf() {
        let test_file = "test_parse_opf.epub";
        create_test_epub(test_file, "application/epub+zip").unwrap();
        
        let mut epub = Epub::new(test_file).unwrap();
        let opf_result = epub.parse_opf();
        assert!(opf_result.is_ok());
        
        let opf = opf_result.unwrap();
        assert_eq!(opf.metadata.title(), Some("测试书籍".to_string()));
        assert_eq!(opf.metadata.creators().len(), 1);
        assert_eq!(opf.metadata.creators()[0].name, "测试作者");
        assert_eq!(opf.manifest.len(), 2);
        assert_eq!(opf.spine.len(), 2);
        
        // 清理测试文件
        let _ = fs::remove_file(test_file);
    }

    #[test]
    fn test_get_book_info() {
        let test_file = "test_book_info.epub";
        create_test_epub(test_file, "application/epub+zip").unwrap();
        
        let mut epub = Epub::new(test_file).unwrap();
        let book_info_result = epub.get_book_info();
        assert!(book_info_result.is_ok());
        
        let (title, authors) = book_info_result.unwrap();
        assert_eq!(title, "测试书籍");
        assert_eq!(authors.len(), 1);
        assert_eq!(authors[0], "测试作者");
        
        // 清理测试文件
        let _ = fs::remove_file(test_file);
    }

    #[test]
    fn test_get_chapters() {
        let test_file = "test_chapters.epub";
        create_test_epub(test_file, "application/epub+zip").unwrap();
        
        let mut epub = Epub::new(test_file).unwrap();
        let chapters_result = epub.get_chapters();
        assert!(chapters_result.is_ok());
        
        let chapters = chapters_result.unwrap();
        assert_eq!(chapters.len(), 2);
        
        // 检查第一个章节
        assert_eq!(chapters[0].0, "text/chapter1.xhtml");
        assert!(chapters[0].1.contains("第一章"));
        assert!(chapters[0].1.contains("这是第一章的内容。"));
        
        // 检查第二个章节
        assert_eq!(chapters[1].0, "text/chapter2.xhtml");
        assert!(chapters[1].1.contains("第二章"));
        assert!(chapters[1].1.contains("这是第二章的内容。"));
        
        // 清理测试文件
        let _ = fs::remove_file(test_file);
    }

    /// 创建一个带封面的测试EPUB文件
    fn create_test_epub_with_cover(path: &str) -> Result<()> {
        let file = File::create(path)?;
        let mut zip = ZipWriter::new(file);
        
        // 添加mimetype文件
        zip.start_file("mimetype", FileOptions::<()>::default())?;
        zip.write_all(b"application/epub+zip")?;
        
        // 添加container.xml文件
        zip.start_file("META-INF/container.xml", FileOptions::<()>::default())?;
        let container_xml = r#"<?xml version="1.0"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
    <rootfiles>
        <rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/>
    </rootfiles>
</container>"#;
        zip.write_all(container_xml.as_bytes())?;
        
        // 添加带封面元数据的OPF文件
        zip.start_file("OEBPS/content.opf", FileOptions::<()>::default())?;
        let opf_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<package version="3.0" xmlns="http://www.idpf.org/2007/opf" unique-identifier="BookId">
    <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
        <dc:title>带封面的测试书籍</dc:title>
        <dc:creator role="aut">测试作者</dc:creator>
        <dc:language>zh-CN</dc:language>
        <dc:identifier id="BookId" scheme="ISBN">978-1234567890</dc:identifier>
        <meta name="cover" content="cover-image"/>
    </metadata>
    <manifest>
        <item id="cover-image" href="images/cover.jpg" media-type="image/jpeg" properties="cover-image"/>
        <item id="chapter1" href="text/chapter1.xhtml" media-type="application/xhtml+xml"/>
    </manifest>
    <spine>
        <itemref idref="chapter1"/>
    </spine>
</package>"#;
        zip.write_all(opf_xml.as_bytes())?;
        
        // 添加封面图片文件（使用简单的JPEG文件头作为测试数据）
        zip.start_file("OEBPS/images/cover.jpg", FileOptions::<()>::default())?;
        // 简单的JPEG文件头
        let jpeg_header = vec![0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46, 0x49, 0x46];
        zip.write_all(&jpeg_header)?;
        
        // 添加测试章节文件
        zip.start_file("OEBPS/text/chapter1.xhtml", FileOptions::<()>::default())?;
        let chapter1 = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE html>
<html xmlns="http://www.w3.org/1999/xhtml">
<head><title>第一章</title></head>
<body><h1>第一章</h1><p>这是第一章的内容。</p></body>
</html>"#;
        zip.write_all(chapter1.as_bytes())?;
        
        zip.finish()?;
        Ok(())
    }

    #[test]
    fn test_get_cover_image() {
        let test_file = "test_cover.epub";
        create_test_epub_with_cover(test_file).unwrap();
        
        let mut epub = Epub::new(test_file).unwrap();
        let cover_result = epub.get_cover_image();
        assert!(cover_result.is_ok());
        
        let cover_option = cover_result.unwrap();
        assert!(cover_option.is_some());
        
        let (cover_data, extension) = cover_option.unwrap();
        assert_eq!(extension, "jpg");
        assert!(!cover_data.is_empty());
        // 检查JPEG文件头
        assert_eq!(cover_data[0], 0xFF);
        assert_eq!(cover_data[1], 0xD8);
        
        // 清理测试文件
        let _ = fs::remove_file(test_file);
    }

    #[test]
    fn test_get_cover_image_no_cover() {
        let test_file = "test_no_cover.epub";
        create_test_epub(test_file, "application/epub+zip").unwrap();
        
        let mut epub = Epub::new(test_file).unwrap();
        let cover_result = epub.get_cover_image();
        assert!(cover_result.is_ok());
        
        let cover_option = cover_result.unwrap();
        assert!(cover_option.is_none());
        
        // 清理测试文件
        let _ = fs::remove_file(test_file);
    }
} 