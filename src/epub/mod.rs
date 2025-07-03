pub mod error;
pub mod container;
pub mod reader;
pub mod opf;
pub mod ncx;

// 重新导出错误处理
pub use error::{EpubError, Result};

// 重新导出容器相关
pub use container::{Container, RootFile};

// 重新导出EPUB读取器和新的数据结构
pub use reader::{
    Epub, 
    BookInfo, 
    ChapterInfo, 
    Chapter, 
    ImageInfo, 
    CoverImage
};

// 重新导出OPF相关
pub use opf::{
    Opf,
    Metadata, 
    Creator, 
    Identifier, 
    ManifestItem, 
    SpineItem,
    MetadataTagConfig, 
    MetadataTagConfigs
};

// 重新导出NCX相关
pub use ncx::{
    Ncx, 
    NavPoint, 
    NavMap, 
    PageList, 
    DocTitle,
    TocTree, 
    TocTreeNode, 
    TocTreeStyle, 
    TocStatistics,
    create_toc_tree_from_ncx
};

 