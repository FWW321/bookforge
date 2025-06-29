//! 脊柱模块
//! 
//! 提供EPUB包中阅读顺序（脊柱）的结构定义。

/// 脊柱项信息(阅读顺序)
#[derive(Debug, Clone)]
pub struct SpineItem {
    /// 引用的清单项ID
    pub idref: String,
    /// 是否线性阅读
    pub linear: bool,
}

impl SpineItem {
    /// 创建新的脊柱项
    pub fn new(idref: String) -> Self {
        Self {
            idref,
            linear: true,
        }
    }

    /// 创建非线性的脊柱项
    pub fn new_non_linear(idref: String) -> Self {
        Self {
            idref,
            linear: false,
        }
    }

    /// 创建指定线性属性的脊柱项
    pub fn with_linear(idref: String, linear: bool) -> Self {
        Self {
            idref,
            linear,
        }
    }

    /// 检查是否为线性阅读
    pub fn is_linear(&self) -> bool {
        self.linear
    }
} 