pub mod error;
pub mod container;
pub mod reader;
pub mod opf;
pub mod ncx;

// 重新导出主要的类型和函数
pub use error::{EpubError, Result};
pub use container::{Container, RootFile};
pub use reader::Epub;
pub use opf::{Opf, Metadata, Creator, Identifier, ManifestItem, SpineItem};
pub use ncx::{Ncx, NavPoint, NavMap, PageList, DocTitle}; 