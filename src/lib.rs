pub mod epub;

// 重新导出主要的类型，方便使用
pub use epub::{
    Epub, EpubError, Result, Container, RootFile, Ncx, NavPoint, NavMap, PageList, DocTitle,
    // 导出目录树相关类型
    ncx::{TocTree, TocTreeNode, TocTreeStyle, TocStatistics, create_toc_tree_from_ncx}
};

/// BookForge库的版本信息
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// BookForge库的描述
pub const DESCRIPTION: &str = "一个用于处理EPUB文件的Rust库";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!VERSION.is_empty());
    }

    #[test]
    fn test_description() {
        assert!(!DESCRIPTION.is_empty());
    }
} 