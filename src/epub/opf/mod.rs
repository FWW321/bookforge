//! OPF（Open Packaging Format）文件解析模块
//! 
//! 此模块提供EPUB文件中OPF包文件的解析功能，包括元数据、清单、脊柱等信息的提取。

mod config;
mod metadata;
mod manifest;
mod spine;
mod parser;

// 重新导出公共类型以保持API兼容性
pub use config::{MetadataTagConfig, MetadataTagConfigs};
pub use metadata::{
    Creator, 
    Identifier, 
    Metadata, 
    MetadataValue, 
    MetaValue
};
pub use manifest::ManifestItem;
pub use spine::SpineItem;
pub use parser::Opf; 