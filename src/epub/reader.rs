use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::sync::Mutex;
use once_cell::sync::OnceCell;
use zip::ZipArchive;

use crate::epub::error::{EpubError, Result};
use crate::epub::container::Container;
use crate::epub::opf::Opf;
use crate::epub::ncx::{Ncx, TocTree, create_toc_tree_from_ncx};

pub struct Epub {
    /// ZIP文件归档（线程安全）
    archive: Mutex<ZipArchive<File>>,
    /// 容器信息（懒加载）
    container: OnceCell<Container>,
    /// OPF包信息（懒加载）
    opf: OnceCell<Opf>,
    /// NCX导航信息（懒加载）
    ncx: OnceCell<Option<Ncx>>,
    /// 书籍基本信息（懒加载）
    book_info: OnceCell<BookInfo>,
    /// 路径缓存
    paths: OnceCell<EpubPaths>,
}

/// EPUB文件路径信息
#[derive(Debug, Clone)]
struct EpubPaths {
    opf_path: String,
    opf_directory: String,
    ncx_path: Option<String>,
}

/// 书籍基本信息
#[derive(Debug, Clone)]
pub struct BookInfo {
    pub title: String,
    pub authors: Vec<String>,
    pub language: Option<String>,
    pub publisher: Option<String>,
    pub isbn: Option<String>,
    pub description: Option<String>,
}

/// 章节信息
#[derive(Debug, Clone)]
pub struct ChapterInfo {
    pub id: String,
    pub title: String,
    pub path: String,
    pub order: Option<u32>,
}

/// 章节内容
#[derive(Debug)]
pub struct Chapter {
    pub info: ChapterInfo,
    pub content: String,
}

/// 图片资源信息
#[derive(Debug, Clone)]
pub struct ImageInfo {
    pub id: String,
    pub path: String,
    pub media_type: String,
}

/// 封面图片
#[derive(Debug)]
pub struct CoverImage {
    pub data: Vec<u8>,
    pub format: String,
    pub filename: String,
}

impl Epub {
    /// 从文件路径创建EPUB实例
    /// 
    /// # 参数
    /// * `path` - EPUB文件路径
    /// 
    /// # 返回值
    /// * `Result<Epub>` - EPUB实例
    /// 
    /// # 错误
    /// * 文件不存在或无法读取
    /// * 文件不是有效的EPUB格式
    /// * mimetype验证失败
    /// 
    /// # 性能说明
    /// 此方法只验证基本的EPUB结构（mimetype文件），其他组件采用懒加载。
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = File::open(path)?;
        let mut archive = ZipArchive::new(file)?;
        
        // 验证EPUB格式
        Self::validate_epub_format(&mut archive)?;
        
        Ok(Epub {
            archive: Mutex::new(archive),
            container: OnceCell::new(),
            opf: OnceCell::new(),
            ncx: OnceCell::new(),
            book_info: OnceCell::new(),
            paths: OnceCell::new(),
        })
    }
    
    /// 获取Container引用
    /// 
    /// # 返回值
    /// * `Result<&Container>` - Container的不可变引用
    pub fn container(&self) -> Result<&Container> {
        self.container.get_or_try_init(|| {
            let container_content = self.read_file("META-INF/container.xml")?;
            Container::parse_xml(&container_content)
        })
    }
    
    /// 获取OPF引用
    /// 
    /// # 返回值
    /// * `Result<&Opf>` - OPF的不可变引用
    pub fn opf(&self) -> Result<&Opf> {
        self.opf.get_or_try_init(|| {
            let paths = self.paths()?;
            let opf_content = self.read_file(&paths.opf_path)?;
            Opf::parse_xml(&opf_content)
        })
    }
    
    /// 使用配置解析OPF
    /// 
    /// # 参数
    /// * `config_path` - 配置文件路径
    /// 
    /// # 返回值
    /// * `Result<&Opf>` - OPF的不可变引用
    pub fn opf_with_config(&self) -> Result<&Opf> {
        self.opf.get_or_try_init(|| {
            let paths = self.paths()?;
            let opf_content = self.read_file(&paths.opf_path)?;
            Opf::parse_xml_with_config(&opf_content)
        })
    }
    
    /// 获取NCX引用（如果存在）
    /// 
    /// # 返回值
    /// * `Result<Option<&Ncx>>` - NCX的不可变引用（如果存在）
    pub fn ncx(&self) -> Result<Option<&Ncx>> {
        let ncx_option = self.ncx.get_or_try_init(|| -> Result<Option<Ncx>> {
            let paths = self.paths()?;
            match &paths.ncx_path {
                Some(ncx_path) => {
                    match self.read_file(ncx_path) {
                        Ok(ncx_content) => {
                            match Ncx::parse_xml(&ncx_content) {
                                Ok(ncx) => Ok(Some(ncx)),
                                Err(e) => {
                                    eprintln!("警告: NCX文件解析失败: {}", e);
                                    Ok(None)
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("警告: 无法读取NCX文件: {}", e);
                            Ok(None)
                        }
                    }
                }
                None => Ok(None),
            }
        })?;
        
        Ok(ncx_option.as_ref())
    }
    

    /// 获取书籍基本信息引用
    /// 
    /// # 返回值
    /// * `Result<&BookInfo>` - 书籍信息的不可变引用
    pub fn book_info(&self) -> Result<&BookInfo> {
        self.book_info.get_or_try_init(|| {
            let opf = self.opf()?;
            let metadata = &opf.metadata;
            
            // 从标识符中查找ISBN
            let isbn = metadata.identifiers()
                .iter()
                .find(|id| {
                    id.scheme.as_ref()
                        .map(|s| s.to_lowercase() == "isbn")
                        .unwrap_or(false)
                })
                .map(|id| id.value.clone());
            
            Ok(BookInfo {
                title: metadata.title().unwrap_or_else(|| "未知标题".to_string()),
                authors: metadata.creators().iter().map(|c| c.name.clone()).collect(),
                language: metadata.language(),
                publisher: metadata.publisher(),
                isbn,
                description: metadata.description(),
            })
        })
    }
    
    /// 获取EPUB版本信息
    /// 
    /// # 返回值
    /// * `Result<&String>` - EPUB版本字符串的不可变引用
    pub fn version(&self) -> Result<&String> {
        let opf = self.opf()?;
        Ok(&opf.version)
    }
    
    /// 检查是否包含NCX文件
    /// 
    /// # 返回值
    /// * `Result<bool>` - 是否包含NCX文件
    pub fn has_ncx(&self) -> Result<bool> {
        Ok(self.paths()?.ncx_path.is_some())
    }
    
    /// 创建目录树（从NCX文件）
    /// 
    /// 从NCX文件构建目录树。目录树提供了章节的树形结构表示，支持层级导航和快速查找。
    /// 
    /// # 返回值
    /// * `Result<Option<TocTree>>` - 目录树实例（如果存在NCX文件）
    /// 
    /// # 性能说明
    /// * 每次调用都会重新创建目录树
    /// * 如果不存在NCX文件，则返回None
    /// 
    /// # 示例
    /// 
    /// ```rust
    /// use bookforge::Epub;
    /// 
    /// let epub = Epub::from_path("book.epub")?;
    /// 
    /// if let Some(toc_tree) = epub.toc_tree()? {
    ///     println!("目录结构:");
    ///     println!("{}", toc_tree);
    /// 
    ///     // 获取第一个章节
    ///     if let Some(first_node) = toc_tree.get_first_node() {
    ///         println!("第一章标题: {}", first_node.title);
    ///     }
    /// }
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn toc_tree(&self) -> Result<Option<TocTree>> {
        // 使用NCX文件创建目录树
        match self.ncx()? {
            Some(ncx) => {
                // 从NCX创建目录树
                let toc_tree = create_toc_tree_from_ncx(ncx, self);
                Ok(Some(toc_tree))
            }
            None => {
                // 没有NCX文件，返回None
                Ok(None)
            }
        }
    }
    
    /// 检查是否包含目录树
    /// 
    /// # 返回值
    /// * `Result<bool>` - 是否包含目录树（基于是否存在NCX文件）
    pub fn has_toc_tree(&self) -> Result<bool> {
        // 检查是否有NCX文件
        self.has_ncx()
    }
    
    /// 获取章节信息列表
    /// 
    /// # 返回值
    /// * `Result<Vec<ChapterInfo>>` - 章节信息列表
    pub fn chapter_list(&self) -> Result<Vec<ChapterInfo>> {
        let opf = self.opf()?;
        let mut chapters = Vec::new();
        
        for (order, spine_item) in opf.spine.iter().enumerate() {
            if let Some(manifest_item) = opf.get_manifest_item(&spine_item.idref) {
                // 从NCX中获取章节标题
                let title = if let Ok(Some(ncx)) = self.ncx() {
                    // 从NCX中查找对应的导航点
                    self.find_chapter_title_in_ncx(ncx, &manifest_item.href)
                        .unwrap_or_else(|| format!("章节 {}", order + 1))
                } else {
                    format!("章节 {}", order + 1)
                };
                
                chapters.push(ChapterInfo {
                    id: spine_item.idref.clone(),
                    title,
                    path: manifest_item.href.clone(),
                    order: Some(order as u32 + 1),
                });
            }
        }
        
        Ok(chapters)
    }
    
    /// 获取指定章节内容
    /// 
    /// # 参数
    /// * `chapter_info` - 章节信息
    /// 
    /// # 返回值
    /// * `Result<Chapter>` - 章节内容
    pub fn chapter(&self, chapter_info: &ChapterInfo) -> Result<Chapter> {
        let paths = self.paths()?;
        let full_path = if paths.opf_directory.is_empty() {
            chapter_info.path.clone()
        } else {
            format!("{}/{}", paths.opf_directory, chapter_info.path)
        };
        
        let content = self.read_file(&full_path)?;
        
        Ok(Chapter {
            info: chapter_info.clone(),
            content,
        })
    }
    
    /// 获取所有章节内容
    /// 
    /// # 返回值
    /// * `Result<Vec<Chapter>>` - 所有章节内容
    pub fn chapters(&self) -> Result<Vec<Chapter>> {
        let chapter_list = self.chapter_list()?;
        let mut chapters = Vec::new();
        
        for chapter_info in chapter_list {
            match self.chapter(&chapter_info) {
                Ok(chapter) => chapters.push(chapter),
                Err(e) => {
                    eprintln!("警告: 无法读取章节 {}: {}", chapter_info.path, e);
                    continue;
                }
            }
        }
        
        Ok(chapters)
    }
    
    /// 获取图片资源列表
    /// 
    /// # 返回值
    /// * `Result<Vec<ImageInfo>>` - 图片资源信息列表
    pub fn images(&self) -> Result<Vec<ImageInfo>> {
        let opf = self.opf()?;
        let mut images = Vec::new();
        
        for (id, item) in &opf.manifest {
            if Self::is_image_media_type(&item.media_type) {
                images.push(ImageInfo {
                    id: id.clone(),
                    path: item.href.clone(),
                    media_type: item.media_type.clone(),
                });
            }
        }
        
        Ok(images)
    }
    
    /// 获取封面图片
    /// 
    /// # 返回值
    /// * `Result<Option<CoverImage>>` - 封面图片（如果存在）
    pub fn cover(&self) -> Result<Option<CoverImage>> {
        let opf = self.opf()?;
        let paths = self.paths()?;
        
        // 1. 尝试从OPF metadata中获取封面
        if let Some(cover_path) = opf.get_cover_image_path() {
            if let Some(cover) = self.extract_cover_image(&paths.opf_directory, &cover_path)? {
                return Ok(Some(cover));
            }
        }
        
        // 2. 尝试从manifest中查找cover-image属性
        for (_, item) in &opf.manifest {
            if let Some(properties) = &item.properties {
                if properties.contains("cover-image") {
                    if let Some(cover) = self.extract_cover_image(&paths.opf_directory, &item.href)? {
                        return Ok(Some(cover));
                    }
                }
            }
        }
        
        // 3. 尝试常见的封面文件名
        let common_cover_names = [
            "cover.jpg", "cover.jpeg", "cover.png", "cover.gif",
            "Cover.jpg", "Cover.jpeg", "Cover.png", "Cover.gif",
        ];
        
        for &cover_name in &common_cover_names {
            if let Some(cover) = self.extract_cover_image(&paths.opf_directory, cover_name)? {
                return Ok(Some(cover));
            }
        }
        
        Ok(None)
    }
    
    /// 获取指定图片数据
    /// 
    /// # 参数
    /// * `image_info` - 图片信息
    /// 
    /// # 返回值
    /// * `Result<Vec<u8>>` - 图片二进制数据
    pub fn image_data(&self, image_info: &ImageInfo) -> Result<Vec<u8>> {
        let paths = self.paths()?;
        let full_path = if paths.opf_directory.is_empty() {
            image_info.path.clone()
        } else {
            format!("{}/{}", paths.opf_directory, image_info.path)
        };
        
        self.read_binary_file(&full_path)
    }
    
    /// 列出所有文件
    /// 
    /// # 返回值
    /// * `Result<Vec<String>>` - 文件路径列表
    pub fn file_list(&self) -> Result<Vec<String>> {
        let mut archive = self.archive.lock()
            .map_err(|_| EpubError::InternalError("无法获取文件归档锁".to_string()))?;
        
        let mut files = Vec::new();
        for i in 0..archive.len() {
            let file = archive.by_index(i)?;
            files.push(file.name().to_string());
        }
        
        Ok(files)
    }
    
    /// 获取OPF目录路径
    /// 
    /// # 返回值
    /// * `Result<String>` - OPF文件所在目录的路径
    pub fn get_opf_directory(&self) -> Result<String> {
        let paths = self.paths()?;
        Ok(paths.opf_directory.clone())
    }
    
    /// 获取NCX文件目录路径
    /// 
    /// # 返回值
    /// * `Result<Option<String>>` - NCX文件所在目录的路径（如果NCX文件存在）
    pub fn get_ncx_directory(&self) -> Result<Option<String>> {
        let paths = self.paths()?;
        match &paths.ncx_path {
            Some(ncx_path) => {
                let ncx_directory = if let Some(last_slash) = ncx_path.rfind('/') {
                    ncx_path[..last_slash].to_string()
                } else {
                    String::new()
                };
                Ok(Some(ncx_directory))
            }
            None => Ok(None),
        }
    }
    
    /// 读取指定文件的内容（公开接口）
    /// 
    /// # 参数
    /// * `filename` - 要读取的文件名
    /// 
    /// # 返回值
    /// * `Result<String>` - 文件内容
    pub fn read_chapter_file(&self, filename: &str) -> Result<String> {
        self.read_file(filename)
    }
    
    // === 内部方法 ===
    
    /// 获取路径信息（懒加载）
    fn paths(&self) -> Result<&EpubPaths> {
        self.paths.get_or_try_init(|| {
            let container = self.container()?;
            let opf_path = container.get_opf_path()
                .ok_or_else(|| EpubError::ContainerParseError(
                    "container.xml中没有找到有效的rootfile".to_string()
                ))?;
            
            let opf_directory = if let Some(last_slash) = opf_path.rfind('/') {
                opf_path[..last_slash].to_string()
            } else {
                String::new()
            };
            
            // 查找NCX路径
            let ncx_path = self.find_ncx_path(&opf_path, &opf_directory)?;
            
            Ok(EpubPaths {
                opf_path,
                opf_directory,
                ncx_path,
            })
        })
    }
    
    /// 查找NCX文件路径
    fn find_ncx_path(&self, opf_path: &str, opf_directory: &str) -> Result<Option<String>> {
        // 首先尝试从OPF中获取
        if let Ok(opf_content) = self.read_file(opf_path) {
            if let Ok(opf) = Opf::parse_xml(&opf_content) {
                if let Some(spine_toc) = &opf.spine_toc {
                    if let Some(manifest_item) = opf.get_manifest_item(spine_toc) {
                        let ncx_path = if opf_directory.is_empty() {
                            manifest_item.href.clone()
                        } else {
                            format!("{}/{}", opf_directory, manifest_item.href)
                        };
                        return Ok(Some(ncx_path));
                    }
                }
            }
        }
        
        // 尝试常见路径
        let common_paths = [
            "toc.ncx",
            "OEBPS/toc.ncx",
            "content/toc.ncx",
            "EPUB/toc.ncx",
        ];
        
        for &path in &common_paths {
            if self.file_exists(path) {
                return Ok(Some(path.to_string()));
            }
        }
        
        Ok(None)
    }
    

    /// 从NCX中查找章节标题
    fn find_chapter_title_in_ncx(&self, ncx: &Ncx, chapter_path: &str) -> Option<String> {
        // 简化的实现，实际可能需要更复杂的匹配逻辑
        for nav_point in &ncx.nav_map.nav_points {
            if nav_point.content.src.contains(chapter_path) {
                return Some(nav_point.nav_label.text.clone());
            }
            // 递归查找子导航点
            if let Some(title) = self.find_title_in_nav_points(&nav_point.children, chapter_path) {
                return Some(title);
            }
        }
        None
    }
    
    /// 在导航点中递归查找标题
    fn find_title_in_nav_points(&self, nav_points: &[crate::epub::ncx::NavPoint], chapter_path: &str) -> Option<String> {
        for nav_point in nav_points {
            if nav_point.content.src.contains(chapter_path) {
                return Some(nav_point.nav_label.text.clone());
            }
            if let Some(title) = self.find_title_in_nav_points(&nav_point.children, chapter_path) {
                return Some(title);
            }
        }
        None
    }
    
    /// 提取封面图片
    fn extract_cover_image(&self, base_dir: &str, file_path: &str) -> Result<Option<CoverImage>> {
        if !Self::is_image_file(file_path) {
            return Ok(None);
        }
        
        let full_path = if base_dir.is_empty() {
            file_path.to_string()
        } else {
            format!("{}/{}", base_dir, file_path)
        };
        
        match self.read_binary_file(&full_path) {
            Ok(data) => {
                let format = Self::detect_image_format(&data, file_path);
                let filename = file_path.split('/').last().unwrap_or(file_path).to_string();
                
                Ok(Some(CoverImage {
                    data,
                    format,
                    filename,
                }))
            }
            Err(_) => Ok(None),
        }
    }
    
    /// 检测图片格式
    fn detect_image_format(data: &[u8], file_path: &str) -> String {
        // 通过文件头检测
        if data.len() >= 2 {
            match &data[0..2] {
                [0xFF, 0xD8] => return "jpeg".to_string(),
                [0x89, 0x50] if data.len() >= 8 && &data[0..8] == b"\x89PNG\r\n\x1a\n" => return "png".to_string(),
                [0x47, 0x49] if data.len() >= 6 && &data[0..6] == b"GIF87a" => return "gif".to_string(),
                [0x47, 0x49] if data.len() >= 6 && &data[0..6] == b"GIF89a" => return "gif".to_string(),
                _ => {}
            }
        }
        
        // 回退到文件扩展名
        std::path::Path::new(file_path)
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("unknown")
            .to_lowercase()
    }
    
    /// 检查是否为图片文件
    fn is_image_file(file_path: &str) -> bool {
        let lower_path = file_path.to_lowercase();
        lower_path.ends_with(".jpg") || 
        lower_path.ends_with(".jpeg") || 
        lower_path.ends_with(".png") || 
        lower_path.ends_with(".gif") || 
        lower_path.ends_with(".webp") ||
        lower_path.ends_with(".svg")
    }
    
    /// 检查媒体类型是否为图片
    fn is_image_media_type(media_type: &str) -> bool {
        media_type.starts_with("image/")
    }
    
    /// 检查文件是否存在
    fn file_exists(&self, filename: &str) -> bool {
        if let Ok(mut archive) = self.archive.lock() {
            archive.by_name(filename).is_ok()
        } else {
            false
        }
    }
    
    /// 读取文本文件
    fn read_file(&self, filename: &str) -> Result<String> {
        let mut archive = self.archive.lock()
            .map_err(|_| EpubError::InternalError("无法获取文件归档锁".to_string()))?;
        
        let mut file = archive.by_name(filename)?;
        let mut content = String::new();
        file.read_to_string(&mut content)?;
        Ok(content)
    }
    
    /// 读取二进制文件
    fn read_binary_file(&self, filename: &str) -> Result<Vec<u8>> {
        let mut archive = self.archive.lock()
            .map_err(|_| EpubError::InternalError("无法获取文件归档锁".to_string()))?;
        
        let mut file = archive.by_name(filename)?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;
        Ok(buffer)
    }
    
    /// 验证EPUB格式
    fn validate_epub_format(archive: &mut ZipArchive<File>) -> Result<()> {
        let mimetype_file = archive.by_name("mimetype");
        
        match mimetype_file {
            Ok(mut file) => {
                let mut content = String::new();
                file.read_to_string(&mut content)?;
                
                let content = content.trim();
                let expected_mimetype = "application/epub+zip";
                
                if content != expected_mimetype {
                    return Err(EpubError::InvalidMimetype {
                        expected: expected_mimetype.to_string(),
                        found: content.to_string(),
                    });
                }
                
                Ok(())
            }
            Err(_) => Err(EpubError::MissingMimetype),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::io::Write;
    use zip::{ZipWriter, write::FileOptions};

    fn create_test_epub(path: &str) -> Result<()> {
        let file = File::create(path)?;
        let mut zip = ZipWriter::new(file);
        
        // mimetype
        zip.start_file("mimetype", FileOptions::<()>::default())?;
        zip.write_all(b"application/epub+zip")?;
        
        // container.xml
        zip.start_file("META-INF/container.xml", FileOptions::<()>::default())?;
        let container_xml = r#"<?xml version="1.0"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
    <rootfiles>
        <rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/>
    </rootfiles>
</container>"#;
        zip.write_all(container_xml.as_bytes())?;
        
        // content.opf
        zip.start_file("OEBPS/content.opf", FileOptions::<()>::default())?;
        let opf_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<package version="3.0" xmlns="http://www.idpf.org/2007/opf" unique-identifier="BookId">
    <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
        <dc:title>测试书籍</dc:title>
        <dc:creator>测试作者</dc:creator>
        <dc:language>zh-CN</dc:language>
        <dc:identifier id="BookId">test-book-001</dc:identifier>
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
        
        // chapter1.xhtml
        zip.start_file("OEBPS/text/chapter1.xhtml", FileOptions::<()>::default())?;
        let chapter1 = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE html>
<html xmlns="http://www.w3.org/1999/xhtml">
<head><title>第一章</title></head>
<body><h1>第一章</h1><p>这是第一章的内容。</p></body>
</html>"#;
        zip.write_all(chapter1.as_bytes())?;
        
        // chapter2.xhtml
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
    fn test_epub_creation() {
        let test_file = "test_epub_creation.epub";
        create_test_epub(test_file).unwrap();
        
        let epub = Epub::from_path(test_file).unwrap();
        assert!(epub.container().is_ok());
        
        let _ = fs::remove_file(test_file);
    }

    #[test]
    fn test_book_info() {
        let test_file = "test_book_info.epub";
        create_test_epub(test_file).unwrap();
        
        let epub = Epub::from_path(test_file).unwrap();
        let info = epub.book_info().unwrap();
        
        assert_eq!(info.title, "测试书籍");
        assert_eq!(info.authors, vec!["测试作者"]);
        
        let _ = fs::remove_file(test_file);
    }

    #[test]
    fn test_chapters() {
        let test_file = "test_chapters.epub";
        create_test_epub(test_file).unwrap();
        
        let epub = Epub::from_path(test_file).unwrap();
        let chapters = epub.chapters().unwrap();
        
        assert_eq!(chapters.len(), 2);
        assert!(chapters[0].content.contains("第一章"));
        assert!(chapters[1].content.contains("第二章"));
        
        let _ = fs::remove_file(test_file);
    }

    #[test]
    fn test_toc_tree_creation() {
        let test_file = "test_toc_tree.epub";
        create_test_epub_with_ncx(test_file).unwrap();
        
        let epub = Epub::from_path(test_file).unwrap();
        
        // 测试检查是否有目录树
        assert!(epub.has_toc_tree().unwrap());
        
        // 第一次调用 - 创建目录树
        let toc_tree1 = epub.toc_tree().unwrap();
        assert!(toc_tree1.is_some());
        
        // 第二次调用 - 再次创建目录树
        let toc_tree2 = epub.toc_tree().unwrap();
        assert!(toc_tree2.is_some());
        
        // 验证目录树内容
        let toc_tree = toc_tree1.unwrap();
        assert_eq!(toc_tree.roots.len(), 2); // 应该有两个根节点
        assert_eq!(toc_tree.roots[0].title, "第一章");
        assert_eq!(toc_tree.roots[1].title, "第二章");
        
        // 验证第二个目录树也有相同内容
        let toc_tree2 = toc_tree2.unwrap();
        assert_eq!(toc_tree2.roots.len(), 2);
        assert_eq!(toc_tree2.roots[0].title, "第一章");
        assert_eq!(toc_tree2.roots[1].title, "第二章");
        
        let _ = fs::remove_file(test_file);
    }

    fn create_test_epub_with_ncx(path: &str) -> Result<()> {
        let file = File::create(path)?;
        let mut zip = ZipWriter::new(file);
        
        // mimetype
        zip.start_file("mimetype", FileOptions::<()>::default())?;
        zip.write_all(b"application/epub+zip")?;
        
        // container.xml
        zip.start_file("META-INF/container.xml", FileOptions::<()>::default())?;
        let container_xml = r#"<?xml version="1.0"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
    <rootfiles>
        <rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/>
    </rootfiles>
</container>"#;
        zip.write_all(container_xml.as_bytes())?;
        
        // content.opf with NCX reference
        zip.start_file("OEBPS/content.opf", FileOptions::<()>::default())?;
        let opf_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<package version="2.0" xmlns="http://www.idpf.org/2007/opf" unique-identifier="BookId">
    <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
        <dc:title>测试书籍（带NCX）</dc:title>
        <dc:creator>测试作者</dc:creator>
        <dc:language>zh-CN</dc:language>
        <dc:identifier id="BookId">test-book-ncx-001</dc:identifier>
    </metadata>
    <manifest>
        <item id="ncx" href="toc.ncx" media-type="application/x-dtbncx+xml"/>
        <item id="chapter1" href="text/chapter1.xhtml" media-type="application/xhtml+xml"/>
        <item id="chapter2" href="text/chapter2.xhtml" media-type="application/xhtml+xml"/>
    </manifest>
    <spine toc="ncx">
        <itemref idref="chapter1"/>
        <itemref idref="chapter2"/>
    </spine>
</package>"#;
        zip.write_all(opf_xml.as_bytes())?;
        
        // toc.ncx
        zip.start_file("OEBPS/toc.ncx", FileOptions::<()>::default())?;
        let ncx_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE ncx PUBLIC "-//NISO//DTD ncx 2005-1//EN" "http://www.daisy.org/z3986/2005/ncx-2005-1.dtd">
<ncx xmlns="http://www.daisy.org/z3986/2005/ncx/" version="2005-1">
    <head>
        <meta name="dtb:uid" content="test-book-ncx-001"/>
        <meta name="dtb:depth" content="1"/>
        <meta name="dtb:totalPageCount" content="0"/>
        <meta name="dtb:maxPageNumber" content="0"/>
    </head>
    <docTitle>
        <text>测试书籍（带NCX）</text>
    </docTitle>
    <navMap>
        <navPoint id="navpoint-1" playOrder="1">
            <navLabel>
                <text>第一章</text>
            </navLabel>
            <content src="text/chapter1.xhtml"/>
        </navPoint>
        <navPoint id="navpoint-2" playOrder="2">
            <navLabel>
                <text>第二章</text>
            </navLabel>
            <content src="text/chapter2.xhtml"/>
        </navPoint>
    </navMap>
</ncx>"#;
        zip.write_all(ncx_xml.as_bytes())?;
        
        // chapter1.xhtml
        zip.start_file("OEBPS/text/chapter1.xhtml", FileOptions::<()>::default())?;
        let chapter1 = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE html>
<html xmlns="http://www.w3.org/1999/xhtml">
<head><title>第一章</title></head>
<body><h1>第一章</h1><p>这是第一章的内容。</p></body>
</html>"#;
        zip.write_all(chapter1.as_bytes())?;
        
        // chapter2.xhtml
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
}