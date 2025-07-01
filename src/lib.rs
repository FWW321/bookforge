pub mod epub;

// === 核心API重新导出 ===

/// EPUB文件读取器（主要接口）
pub use epub::Epub;

/// 错误处理
pub use epub::{EpubError, Result};

// === 数据结构 ===

/// 书籍基本信息
pub use epub::BookInfo;

/// 章节信息和内容
pub use epub::{ChapterInfo, Chapter};

/// 图片资源信息
pub use epub::{ImageInfo, CoverImage};

// === 底层组件（高级用法） ===

/// 容器组件
pub use epub::{Container, RootFile};

/// OPF组件
pub use epub::{
    Opf, 
    Metadata, 
    Creator, 
    Identifier, 
    ManifestItem, 
    SpineItem,
    MetadataTagConfig,
    MetadataTagConfigs,
};

/// NCX组件
pub use epub::{
    Ncx, 
    NavPoint, 
    NavMap, 
    PageList, 
    DocTitle,
    TocTree, 
    TocTreeNode, 
    TocTreeStyle, 
    TocStatistics,
    create_toc_tree_from_ncx,
};



// === 库信息 ===

/// BookForge库的版本信息
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// BookForge库的描述
pub const DESCRIPTION: &str = "一个现代化、高性能的EPUB文件处理库";

/// 库的主页
pub const HOMEPAGE: &str = "https://github.com/FWW321/bookforge";

// === 类型别名（便于使用） ===

/// EPUB文件读取器的类型别名
pub type EpubReader = Epub;

/// 书籍信息的类型别名
pub type Book = BookInfo;

// === 便捷函数 ===

/// 快速打开EPUB文件
/// 
/// 这是 `Epub::from_path` 的便捷包装函数。
/// 
/// # 参数
/// * `path` - EPUB文件路径
/// 
/// # 返回值
/// * `Result<Epub>` - EPUB实例
/// 
/// # 示例
/// 
/// ```rust
/// use bookforge;
/// 
 /// let epub = bookforge::open("book.epub")?;
/// let info = epub.book_info()?;
/// println!("书名: {}", info.title);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn open<P: AsRef<std::path::Path>>(path: P) -> Result<Epub> {
    Epub::from_path(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!VERSION.is_empty());
        println!("BookForge version: {}", VERSION);
    }

    #[test]
    fn test_description() {
        assert!(!DESCRIPTION.is_empty());
        println!("Description: {}", DESCRIPTION);
    }

    #[test]
    fn test_homepage() {
        assert!(!HOMEPAGE.is_empty());
        println!("Homepage: {}", HOMEPAGE);
    }
} 