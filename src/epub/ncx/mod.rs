//! NCX（Navigation Control file for XML）文件解析模块
//! 
//! 此模块提供EPUB文件中NCX导航控制文件的解析功能，包括导航地图、页面列表等信息的提取。
//! NCX文件主要用于定义EPUB的目录结构和导航信息。

pub mod navigation;
pub mod parser;
pub mod toc_tree;

// 重新导出公共类型以保持API兼容性
pub use navigation::{
    NavPoint,
    NavLabel,
    NavContent,
    NavMap,
    PageTarget,
    PageList,
    DocTitle,
    NcxMetadata,
};
pub use parser::Ncx;
pub use toc_tree::*; 