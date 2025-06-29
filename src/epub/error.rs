use std::io;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, EpubError>;

/// Epub相关的错误类型
#[derive(Error, Debug)]
pub enum EpubError {
    #[error("IO错误: {0}")]
    Io(#[from] io::Error),
    
    #[error("Zip文件错误: {0}")]
    Zip(#[from] zip::result::ZipError),
    
    #[error("文件不是有效的EPUB格式: {0}")]
    InvalidEpub(String),
    
    #[error("缺少mimetype文件")]
    MissingMimetype,
    
    #[error("无效的mimetype: {expected}, 找到: {found}")]
    InvalidMimetype { expected: String, found: String },
    
    #[error("XML解析错误: {0}")]
    XmlError(#[from] quick_xml::Error),
    
    #[error("container.xml解析错误: {0}")]
    ContainerParseError(String),
    
    #[error("OPF文件解析错误: {0}")]
    OpfParseError(String),
    
    #[error("NCX文件解析错误: {0}")]
    NcxParseError(String),
    
    #[error("配置文件错误: {0}")]
    ConfigError(String),
} 