//! 目录树（Table of Contents Tree）模块
//! 
//! 提供NCX目录结构的树形表示和显示功能。

use std::fmt::{Display, Formatter, Result as FmtResult};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use crate::epub::ncx::{Ncx, NavPoint};
use crate::epub::{Epub, EpubError, Result};
use scraper::{Html, Selector};

/// 目录树显示样式
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TocTreeStyle {
    /// 使用树状符号（├── └──）
    TreeSymbols,
    /// 使用缩进和符号（• ）
    Indented,
}

/// 目录树来源类型
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TocTreeSource {
    /// 来自NCX文件
    Ncx,
    /// 来自EPUB3 nav文档
    Nav,
    /// 来源未知（向后兼容）
    Unknown,
}

/// 目录树节点
#[derive(Debug, Clone)]
pub struct TocTreeNode {
    /// 播放顺序
    pub play_order: u32,
    /// 标题
    pub title: String,
    /// 源文件路径
    pub src: String,
    /// 节点ID
    pub id: String,
    /// 子节点
    pub children: Vec<TocTreeNode>,
    /// 节点深度
    pub depth: u32,
}

impl TocTreeNode {
    /// 创建新的目录树节点
    pub fn new(play_order: u32, title: String, src: String, id: String, depth: u32) -> Self {
        Self {
            play_order,
            title,
            src,
            id,
            children: Vec::new(),
            depth,
        }
    }

    /// 添加子节点
    pub fn add_child(&mut self, child: TocTreeNode) {
        self.children.push(child);
    }

    /// 获取节点的最大深度
    pub fn get_max_depth(&self) -> u32 {
        let mut max_depth = self.depth;
        for child in &self.children {
            max_depth = max_depth.max(child.get_max_depth());
        }
        max_depth
    }

    /// 获取节点及其所有子节点的数量
    pub fn get_total_nodes(&self) -> usize {
        let mut count = 1; // 当前节点
        for child in &self.children {
            count += child.get_total_nodes();
        }
        count
    }

    /// 收集所有叶子节点（没有子节点的节点）
    pub fn collect_leaf_nodes(&self) -> Vec<&TocTreeNode> {
        if self.children.is_empty() {
            vec![self]
        } else {
            let mut leaves = Vec::new();
            for child in &self.children {
                leaves.extend(child.collect_leaf_nodes());
            }
            leaves
        }
    }

    /// 根据ID查找节点
    pub fn find_by_id(&self, id: &str) -> Option<&TocTreeNode> {
        if self.id == id {
            return Some(self);
        }
        for child in &self.children {
            if let Some(found) = child.find_by_id(id) {
                return Some(found);
            }
        }
        None
    }

    /// 根据源文件路径查找节点
    pub fn find_by_src(&self, src: &str) -> Option<&TocTreeNode> {
        if self.src == src {
            return Some(self);
        }
        for child in &self.children {
            if let Some(found) = child.find_by_src(src) {
                return Some(found);
            }
        }
        None
    }

    /// 根据路径数组获取子节点
    /// 路径数组表示从当前节点开始的索引路径，例如：
    /// - `[0]` 表示第一个子节点
    /// - `[0, 1]` 表示第一个子节点的第二个子节点
    /// - `[]` 表示当前节点本身
    pub fn get_node_by_path(&self, path: &[usize]) -> Option<&TocTreeNode> {
        if path.is_empty() {
            return Some(self);
        }

        let first_index = path[0];
        if first_index >= self.children.len() {
            return None;
        }

        let child = &self.children[first_index];
        if path.len() == 1 {
            Some(child)
        } else {
            child.get_node_by_path(&path[1..])
        }
    }

    /// 获取当前节点在父节点中的索引路径
    /// 返回从根节点到当前节点的完整路径
    pub fn get_path_from_root(&self, roots: &[TocTreeNode]) -> Option<Vec<usize>> {
        // 首先检查是否是根节点
        for (root_index, root) in roots.iter().enumerate() {
            if std::ptr::eq(self as *const _, root as *const _) {
                return Some(vec![root_index]);
            }
            
            // 递归搜索子节点
            if let Some(path) = self.find_path_in_subtree(root, &[root_index]) {
                return Some(path);
            }
        }
        None
    }

    /// 在子树中查找节点路径
    fn find_path_in_subtree(&self, current: &TocTreeNode, current_path: &[usize]) -> Option<Vec<usize>> {
        // 检查当前节点是否是目标节点
        if std::ptr::eq(self as *const _, current as *const _) {
            return Some(current_path.to_vec());
        }

        // 递归搜索子节点
        for (child_index, child) in current.children.iter().enumerate() {
            let mut child_path = current_path.to_vec();
            child_path.push(child_index);
            
            if let Some(path) = self.find_path_in_subtree(child, &child_path) {
                return Some(path);
            }
        }
        
        None
    }

    /// 获取当前节点所代表的章节的HTML内容
    /// 
    /// 该方法会从EPUB文件中提取当前节点对应的HTML文件内容。
    /// 文件路径会根据NCX文件的位置进行解析，因为NCX中的路径是相对于NCX文件的。
    /// 
    /// # 参数
    /// * `epub` - EPUB阅读器的可变引用
    /// 
    /// # 返回值
    /// * `Result<String, EpubError>` - 成功时返回HTML内容，失败时返回错误
    /// 
    /// # 错误处理
    /// * 如果无法获取NCX目录，返回相应错误
    /// * 如果文件不存在或无法读取，返回相应错误
    /// 
    /// # 使用示例
    /// 
    /// ```rust
    /// use bookforge::epub::Epub;
    /// use bookforge::epub::ncx::toc_tree::create_toc_tree_from_ncx;
    /// 
    /// let mut epub = Epub::new("book.epub")?;
    /// let ncx_content = epub.extract_file("toc.ncx")?;
    /// let ncx = bookforge::epub::ncx::Ncx::parse_xml(&ncx_content)?;
    /// let toc_tree = create_toc_tree_from_ncx(&ncx);
    /// 
    /// if let Some(first_node) = toc_tree.get_first_node() {
    ///     match first_node.get_html_content(&mut epub) {
    ///         Ok(html) => println!("章节内容: {}", html),
    ///         Err(e) => println!("获取章节内容失败: {}", e),
    ///     }
    /// }
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn get_html_content(&self, epub: &Epub) -> Result<String> {
        // 获取NCX文件的目录路径，因为NCX中的路径是相对于NCX文件的
        let full_path = match epub.get_ncx_directory()? {
            Some(ncx_dir) => {
                if ncx_dir.is_empty() {
                    // 如果NCX在根目录，直接使用src路径
                    self.src.clone()
                } else {
                    // 使用PathBuf正确处理路径组合和规范化
                    let mut path = PathBuf::from(ncx_dir);
                    path.push(&self.src);
                    
                    // 规范化路径，处理 ../ 等相对路径组件
                    Self::normalize_path(&path)
                }
            }
            None => {
                // 如果没有NCX文件，回退到使用OPF目录（兼容性处理）
                let opf_dir = epub.get_opf_directory()?;
                if opf_dir.is_empty() {
                    self.src.clone()
                } else {
                    // 使用PathBuf正确处理路径组合和规范化
                    let mut path = PathBuf::from(opf_dir);
                    path.push(&self.src);
                    
                    // 规范化路径，处理 ../ 等相对路径组件
                    Self::normalize_path(&path)
                }
            }
        };
        
        // 从EPUB文件中提取HTML内容
        epub.read_chapter_file(&full_path).map_err(|e| {
            EpubError::InvalidEpub(format!(
                "无法读取章节文件 '{}' (节点ID: {}, 标题: '{}'): {}",
                full_path, self.id, self.title, e
            ))
        })
    }

    /// 规范化路径，处理相对路径组件如 ../ 和 ./
    /// 
    /// 该方法确保生成的路径使用Unix风格的分隔符（/），这是ZIP文件内部的标准格式。
    /// 
    /// # 参数
    /// * `path` - 需要规范化的路径
    /// 
    /// # 返回值
    /// * `String` - 规范化后的路径字符串，使用Unix风格分隔符
    fn normalize_path(path: &Path) -> String {
        let mut components = Vec::new();
        
        for component in path.components() {
            match component {
                std::path::Component::ParentDir => {
                    // 遇到 ".." 时，删除最后一个组件（如果存在）
                    components.pop();
                }
                std::path::Component::CurDir => {
                    // 忽略 "." 组件
                }
                std::path::Component::Normal(name) => {
                    // 正常的路径组件，转换为字符串
                    if let Some(name_str) = name.to_str() {
                        components.push(name_str.to_string());
                    }
                }
                // 其他组件类型（如根路径、前缀等）在EPUB中不常见，忽略处理
                _ => {}
            }
        }
        
        // 使用Unix风格分隔符重新组装路径（ZIP文件内部标准）
        components.join("/")
    }

    /// 获取当前节点的纯文本内容
    /// 
    /// 该方法获取HTML内容后，会尝试移除HTML标签，返回纯文本内容。
    /// 这对于搜索、摘要生成等功能很有用。
    /// 
    /// # 参数
    /// * `epub` - EPUB阅读器的可变引用
    /// 
    /// # 返回值
    /// * `Result<String, EpubError>` - 成功时返回纯文本内容，失败时返回错误
    /// 
    /// # 注意
    /// 当前实现使用简单的正则表达式移除HTML标签，
    /// 对于复杂的HTML结构可能不够准确。
    /// 
    /// # 使用示例
    /// 
    /// ```rust
    /// use bookforge::epub::Epub;
    /// use bookforge::epub::ncx::toc_tree::create_toc_tree_from_ncx;
    /// 
    /// let epub = Epub::from_path("book.epub")?;
    /// let ncx = epub.ncx()?.unwrap();
    /// let toc_tree = create_toc_tree_from_ncx(&ncx);
    /// 
    /// if let Some(first_node) = toc_tree.get_first_node() {
    ///     match first_node.get_text_content(&epub) {
    ///         Ok(text) => println!("章节纯文本: {}", text),
    ///         Err(e) => println!("获取章节文本失败: {}", e),
    ///     }
    /// }
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn get_text_content(&self, epub: &Epub) -> Result<String> {
        let html_content = self.get_html_content(epub)?;
        
        // 简单的HTML标签移除（可以后续优化为更复杂的HTML解析）
        let text_content = Self::strip_html_tags(&html_content);
        
        Ok(text_content)
    }

    /// 获取当前节点的格式化文本内容
    /// 
    /// 该方法获取HTML内容后，会按照HTML结构进行智能转换：
    /// 1. 保持原有的HTML格式结构
    /// 2. 移除图片等媒体元素
    /// 3. 将空的块级元素或只包含空白符的块级元素转换为换行符
    /// 4. 正确处理HTML实体（如&nbsp;、&lt;等）
    /// 
    /// # 参数
    /// * `epub` - EPUB阅读器的可变引用
    /// 
    /// # 返回值
    /// * `Result<String, EpubError>` - 成功时返回格式化的文本内容，失败时返回错误
    /// 
    /// # 使用示例
    /// 
    /// ```rust
    /// use bookforge::epub::Epub;
    /// use bookforge::epub::ncx::toc_tree::create_toc_tree_from_ncx;
    /// 
    /// let mut epub = Epub::new("book.epub")?;
    /// let ncx_content = epub.extract_file("toc.ncx")?;
    /// let ncx = bookforge::epub::ncx::Ncx::parse_xml(&ncx_content)?;
    /// let toc_tree = create_toc_tree_from_ncx(&ncx);
    /// 
    /// if let Some(first_node) = toc_tree.get_first_node() {
    ///     match first_node.get_formatted_text_content(&mut epub) {
    ///         Ok(text) => println!("格式化章节内容:\n{}", text),
    ///         Err(e) => println!("获取格式化章节内容失败: {}", e),
    ///     }
    /// }
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn get_formatted_text_content(&self, epub: &Epub) -> Result<String> {
        let html_content = self.get_html_content(epub)?;
        
        // 使用智能HTML解析器转换为格式化文本
        let formatted_text = Self::convert_html_to_formatted_text(&html_content);
        
        Ok(formatted_text)
    }

    /// 生成当前节点代表章节的txt文件
    /// 
    /// 该方法会将当前节点对应的章节内容保存为txt文件。
    /// 文件名基于章节标题生成，并进行安全性处理以避免文件系统冲突。
    /// 默认使用格式化文本内容，保持原有HTML结构的基本格式。
    /// 
    /// # 参数
    /// * `epub` - EPUB阅读器的引用
    /// * `output_dir` - 输出目录路径，如果为None则使用当前目录
    /// * `use_formatted_text` - 是否使用格式化文本，false则使用纯文本
    /// 
    /// # 返回值
    /// * `Result<PathBuf, EpubError>` - 成功时返回生成的文件路径，失败时返回错误
    /// 
    /// # 错误处理
    /// * 如果无法获取章节内容，返回相应错误
    /// * 如果无法创建输出目录，返回相应错误
    /// * 如果无法写入文件，返回相应错误
    /// 
    /// # 使用示例
    /// 
    /// ```rust
    /// use bookforge::epub::Epub;
    /// use bookforge::epub::ncx::toc_tree::create_toc_tree_from_ncx;
    /// use std::path::Path;
    /// 
    /// let epub = Epub::from_path("book.epub")?;
    /// let ncx = epub.ncx()?.unwrap();
    /// let toc_tree = create_toc_tree_from_ncx(&ncx, &epub);
    /// 
    /// if let Some(first_node) = toc_tree.get_first_node() {
    ///     match first_node.generate_txt_file(&epub, Some(Path::new("chapters")), true) {
    ///         Ok(file_path) => println!("章节已保存到: {:?}", file_path),
    ///         Err(e) => println!("保存章节失败: {}", e),
    ///     }
    /// }
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn generate_txt_file(
        &self,
        epub: &Epub,
        output_dir: Option<&Path>,
        use_formatted_text: bool,
    ) -> Result<PathBuf> {
        // 获取章节内容
        let content = if use_formatted_text {
            self.get_formatted_text_content(epub)?
        } else {
            self.get_text_content(epub)?
        };

        // 确定输出目录
        let dir = output_dir.unwrap_or_else(|| Path::new("output"));
        
        // 创建输出目录（如果不存在）
        if !dir.exists() {
            fs::create_dir_all(dir).map_err(|e| {
                EpubError::InvalidEpub(format!(
                    "无法创建输出目录 '{}': {}",
                    dir.display(),
                    e
                ))
            })?;
        }

        // 生成安全的文件名
        let safe_filename = Self::generate_safe_filename(&self.title, &self.id, self.play_order);
        let file_path = dir.join(format!("{}.txt", safe_filename));

        // 创建文件内容
        let file_content = self.create_file_content(&content);

        // 写入文件
        fs::write(&file_path, file_content).map_err(|e| {
            EpubError::InvalidEpub(format!(
                "无法写入文件 '{}': {}",
                file_path.display(),
                e
            ))
        })?;

        Ok(file_path)
    }

    /// 批量生成当前节点及其所有子节点的txt文件
    /// 
    /// 该方法会递归处理当前节点及其所有子节点，为每个节点生成对应的txt文件。
    /// 文件会按照目录树结构在输出目录中创建相应的子目录。
    /// 
    /// # 参数
    /// * `epub` - EPUB阅读器的引用
    /// * `output_dir` - 输出目录路径，如果为None则使用当前目录
    /// * `use_formatted_text` - 是否使用格式化文本，false则使用纯文本
    /// * `create_subdirs` - 是否根据目录树结构创建子目录
    /// 
    /// # 返回值
    /// * `Result<Vec<PathBuf>, EpubError>` - 成功时返回所有生成的文件路径列表，失败时返回错误
    /// 
    /// # 使用示例
    /// 
    /// ```rust
    /// use bookforge::epub::Epub;
    /// use bookforge::epub::ncx::toc_tree::create_toc_tree_from_ncx;
    /// use std::path::Path;
    /// 
    /// let epub = Epub::from_path("book.epub")?;
    /// let ncx = epub.ncx()?.unwrap();
    /// let toc_tree = create_toc_tree_from_ncx(&ncx, &epub);
    /// 
    /// if let Some(first_node) = toc_tree.get_first_node() {
    ///     match first_node.generate_txt_files_recursive(&epub, Some(Path::new("chapters")), true, true) {
    ///         Ok(file_paths) => {
    ///             println!("已生成 {} 个章节文件:", file_paths.len());
    ///             for path in file_paths {
    ///                 println!("  - {:?}", path);
    ///             }
    ///         }
    ///         Err(e) => println!("批量保存章节失败: {}", e),
    ///     }
    /// }
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn generate_txt_files_recursive(
        &self,
        epub: &Epub,
        output_dir: Option<&Path>,
        use_formatted_text: bool,
        create_subdirs: bool,
    ) -> Result<Vec<PathBuf>> {
        let mut file_paths = Vec::new();
        
        // 确定输出目录
        let base_dir = output_dir.unwrap_or_else(|| Path::new("output"));
        
        // 为当前节点生成文件
        self.generate_txt_files_recursive_impl(
            epub,
            base_dir,
            use_formatted_text,
            create_subdirs,
            0,
            &mut file_paths,
        )?;
        
        Ok(file_paths)
    }

    /// 递归生成txt文件的内部实现
    fn generate_txt_files_recursive_impl(
        &self,
        epub: &Epub,
        current_dir: &Path,
        use_formatted_text: bool,
        create_subdirs: bool,
        depth: u32,
        file_paths: &mut Vec<PathBuf>,
    ) -> Result<()> {
        // 为当前节点生成文件
        let file_path = self.generate_txt_file(epub, Some(current_dir), use_formatted_text)?;
        file_paths.push(file_path);

        // 如果需要创建子目录且有子节点，为子节点创建目录
        if create_subdirs && !self.children.is_empty() {
            let safe_dirname = Self::generate_safe_filename(&self.title, &self.id, self.play_order);
            let child_dir = current_dir.join(&safe_dirname);
            
            // 创建子目录
            if !child_dir.exists() {
                fs::create_dir_all(&child_dir).map_err(|e| {
                    EpubError::InvalidEpub(format!(
                        "无法创建子目录 '{}': {}",
                        child_dir.display(),
                        e
                    ))
                })?;
            }

            // 递归处理子节点
            for child in &self.children {
                child.generate_txt_files_recursive_impl(
                    epub,
                    &child_dir,
                    use_formatted_text,
                    create_subdirs,
                    depth + 1,
                    file_paths,
                )?;
            }
        } else {
            // 不创建子目录，在当前目录中处理所有子节点
            for child in &self.children {
                child.generate_txt_files_recursive_impl(
                    epub,
                    current_dir,
                    use_formatted_text,
                    create_subdirs,
                    depth + 1,
                    file_paths,
                )?;
            }
        }

        Ok(())
    }

    /// 生成安全的文件名
    /// 
    /// 该方法会处理章节标题，移除或替换不安全的字符，确保生成的文件名在各种文件系统中都能正常使用。
    /// 
    /// # 参数
    /// * `title` - 章节标题
    /// * `id` - 节点ID（作为备用）
    /// * `play_order` - 播放顺序（用于排序和唯一性）
    /// 
    /// # 返回值
    /// * `String` - 安全的文件名（不包含扩展名）
    fn generate_safe_filename(title: &str, id: &str, play_order: u32) -> String {
        // 移除或替换不安全的字符
        let mut safe_title = title
            .chars()
            .map(|c| match c {
                // 文件系统保留字符
                '<' | '>' | ':' | '"' | '|' | '?' | '*' => '_',
                '/' | '\\' => '_',
                // 控制字符
                c if c.is_control() => '_',
                // 其他字符保持不变
                c => c,
            })
            .collect::<String>();

        // 移除开头和结尾的空白字符和点号
        safe_title = safe_title.trim().trim_matches('.').to_string();
        
        // 如果标题为空或只包含无效字符，使用ID作为备用
        if safe_title.is_empty() {
            safe_title = id.to_string();
        }
        
        // 如果仍然为空，使用play_order
        if safe_title.is_empty() {
            safe_title = format!("chapter_{}", play_order);
        }

        // 限制文件名长度（保留空间给序号和扩展名）
        const MAX_FILENAME_LENGTH: usize = 200;
        if safe_title.len() > MAX_FILENAME_LENGTH {
            safe_title.truncate(MAX_FILENAME_LENGTH);
            // 确保不会在Unicode字符中间截断
            while !safe_title.is_char_boundary(safe_title.len()) {
                safe_title.pop();
            }
        }

        // 添加播放顺序作为前缀，确保文件按顺序排列
        format!("{:03}_{}", play_order, safe_title)
    }

    /// 创建文件内容
    /// 
    /// 该方法会创建包含元数据和章节内容的完整文件内容。
    /// 
    /// # 参数
    /// * `content` - 章节文本内容
    /// 
    /// # 返回值
    /// * `String` - 完整的文件内容
    fn create_file_content(&self, content: &str) -> String {
        let mut file_content = String::new();
        
        // 添加文件头部信息
        // file_content.push_str("═══════════════════════════════════════\n");
        // file_content.push_str(&format!("章节标题: {}\n", self.title));
        // file_content.push_str(&format!("节点ID: {}\n", self.id));
        // file_content.push_str(&format!("播放顺序: {}\n", self.play_order));
        // file_content.push_str(&format!("源文件: {}\n", self.src));
        // file_content.push_str("═══════════════════════════════════════\n\n");
        
        // 添加章节内容
        file_content.push_str(content);
        
        // 添加文件尾部
        // file_content.push_str("\n\n");
        // file_content.push_str("═══════════════════════════════════════\n");
        // file_content.push_str("Generated by BookForge EPUB Reader\n");
        // file_content.push_str("═══════════════════════════════════════\n");
        
        file_content
    }

    /// 将HTML转换为格式化文本
    /// 
    /// # 参数
    /// * `html` - HTML内容
    /// 
    /// # 返回值
    /// * `String` - 格式化的文本内容
    fn convert_html_to_formatted_text(html: &str) -> String {
        // 解析HTML文档
        let document = Html::parse_document(html);
        
        // 选择body元素，如果没有body则使用整个文档
        let body_selector = Selector::parse("body").unwrap();
        let content = if let Some(body) = document.select(&body_selector).next() {
            Self::extract_formatted_text_from_element(body)
        } else {
            // 如果没有body标签，处理整个文档
            Self::extract_formatted_text_from_document(&document)
        };
        
        // 清理多余的连续换行符，但保持段落间的分隔
        let cleaned = Self::clean_excessive_newlines(&content);
        
        cleaned
    }

    /// 从HTML元素中提取格式化文本
    fn extract_formatted_text_from_element(element: scraper::ElementRef) -> String {
        let mut result = String::new();
        Self::process_element_for_formatted_text(element, &mut result);
        result
    }

    /// 从HTML文档中提取格式化文本
    fn extract_formatted_text_from_document(document: &Html) -> String {
        let mut result = String::new();
        
        // 选择所有文本内容，跳过head部分
        let body_selector = Selector::parse("body").unwrap();
        if let Some(body) = document.select(&body_selector).next() {
            Self::process_element_for_formatted_text(body, &mut result);
        } else {
            // 如果没有body标签，处理整个文档但跳过head
            let not_head_selector = Selector::parse("body, :not(head):not(head *)").unwrap();
            for element in document.select(&not_head_selector) {
                Self::process_element_for_formatted_text(element, &mut result);
            }
        }
        
        result
    }

    /// 处理HTML元素以提取格式化文本
    fn process_element_for_formatted_text(element: scraper::ElementRef, result: &mut String) {
        let tag_name = element.value().name();
        
        // 跳过文档头部和脚本相关标签
        // if matches!(tag_name, "head" | "script" | "style" | "meta" | "link" | "title" | "base" | "noscript") {
        //     return;
        // }
        
        // 跳过媒体标签和相关元素
        // if matches!(tag_name, 
        //     "img" | "svg" | "video" | "audio" | "canvas" | "embed" | "object" | 
        //     "iframe" | "picture" | "source" | "track" | "param" | "area" | "map"
        // ) {
        //     return;
        // }
        
        // 跳过特定类型的表单输入元素（图像按钮等）
        // if tag_name == "input" {
        //     if let Some(input_type) = element.value().attr("type") {
        //         if matches!(input_type, "image" | "file" | "hidden") {
        //             return;
        //         }
        //     }
        // }
        if matches!(tag_name, "img"){
            return;
        }
        
        // 处理元素的文本内容
        for node in element.children() {
            match node.value() {
                scraper::node::Node::Text(text) => {
                    result.push_str(text);
                }
                scraper::node::Node::Element(_) => {
                    if let Some(child_element) = scraper::ElementRef::wrap(node) {
                        Self::process_element_for_formatted_text(child_element, result);
                    }
                }
                _ => {}
            }
        }
        
        // 根据标签类型添加格式
        match tag_name {
            // 块级元素 - 在结束时添加换行
            // "div" | "p" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
            //     result.push('\n');
            // }
            // 列表和表格元素
            // "ul" | "ol" | "table" | "tbody" | "thead" | "tr" => {
            //     result.push('\n');
            // }
            // 表格单元格
            // "td" | "th" => {
            //     result.push('\t');
            // }
            // 换行标签
            "br" => {
                result.push('\n');
            }
            _ => {}
        }
    }

    /// 清理多余的连续换行符
    fn clean_excessive_newlines(text: &str) -> String {
        // 将多个连续的换行符（超过2个）替换为最多2个换行符
        let mut result = String::new();
        let mut newline_count = 0;
        
        for ch in text.chars() {
            if ch == '\n' {
                newline_count += 1;
                if newline_count <= 2 {
                    result.push(ch);
                }
            } else {
                newline_count = 0;
                result.push(ch);
            }
        }
        
        // 移除开头和结尾的空白字符
        result.trim().to_string()
    }

    /// 移除HTML标签的辅助函数（保留用于向后兼容）
    /// 
    /// 使用scraper库移除HTML标签，只保留纯文本内容。
    /// 只处理body标签内的内容。
    /// 
    /// # 参数
    /// * `html` - 包含HTML标签的字符串
    /// 
    /// # 返回值
    /// * `String` - 移除HTML标签后的纯文本
    fn strip_html_tags(html: &str) -> String {
        // 解析HTML文档
        let document = Html::parse_document(html);
        
        // 选择body元素，如果没有body则使用整个文档
        let body_selector = Selector::parse("body").unwrap();
        let text = if let Some(body) = document.select(&body_selector).next() {
            Self::extract_text_from_element(body)
        } else {
            Self::extract_text_from_document(&document)
        };
        
        // 清理多余的空白字符
        text.split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// 从HTML元素中提取纯文本
    fn extract_text_from_element(element: scraper::ElementRef) -> String {
        let mut result = String::new();
        Self::process_element_for_text(element, &mut result);
        result
    }

    /// 从HTML文档中提取纯文本
    fn extract_text_from_document(document: &Html) -> String {
        let mut result = String::new();
        
        // 选择body元素，如果没有则处理整个文档
        let body_selector = Selector::parse("body").unwrap();
        if let Some(body) = document.select(&body_selector).next() {
            Self::process_element_for_text(body, &mut result);
        } else {
            // 如果没有body标签，使用通用选择器
            let all_selector = Selector::parse("*").unwrap();
            for element in document.select(&all_selector) {
                Self::process_element_for_text(element, &mut result);
                break; // 只处理第一个元素（通常是html或body）
            }
        }
        
        result
    }

    /// 处理HTML元素以提取纯文本
    fn process_element_for_text(element: scraper::ElementRef, result: &mut String) {
        let tag_name = element.value().name();
        
        // 跳过文档头部和脚本相关标签
        if matches!(tag_name, "head" | "script" | "style" | "meta" | "link" | 
                  "title" | "base" | "noscript") {
            return;
        }
        
        // 跳过媒体标签和相关元素
        if matches!(tag_name, 
            "img" | "svg" | "video" | "audio" | "canvas" | "embed" | "object" | 
            "iframe" | "picture" | "source" | "track" | "param" | "area" | "map"
        ) {
            return;
        }
        
        // 跳过特定类型的表单输入元素（图像按钮等）
        if tag_name == "input" {
            if let Some(input_type) = element.value().attr("type") {
                if matches!(input_type, "image" | "file" | "hidden") {
                    return;
                }
            }
        }
        
        // 处理元素的文本内容
        for node in element.children() {
            match node.value() {
                scraper::node::Node::Text(text) => {
                    result.push_str(text);
                }
                scraper::node::Node::Element(_) => {
                    if let Some(child_element) = scraper::ElementRef::wrap(node) {
                        Self::process_element_for_text(child_element, result);
                    }
                }
                _ => {}
            }
        }
        
        // 在某些元素后添加空格以避免文本粘连
        match tag_name {
            "div" | "p" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" | 
            "li" | "br" | "td" | "th" => {
                result.push(' ');
            }
            _ => {}
        }
    }
}

/// 目录树结构
pub struct TocTree<'a> {
    /// 文档标题
    pub title: Option<String>,
    /// 根节点列表
    pub roots: Vec<TocTreeNode>,
    /// 显示样式
    pub style: TocTreeStyle,
    /// 是否显示文件路径
    pub show_paths: bool,
    /// 最大显示深度（None表示显示所有）
    pub max_depth: Option<u32>,
    /// EPUB阅读器引用
    pub epub: &'a Epub,
    /// 目录树来源
    pub source: TocTreeSource,
}

impl<'a> TocTree<'a> {
    /// 创建新的目录树
    /// 
    /// # 参数
    /// * `epub` - EPUB阅读器的引用
    /// 
    /// # 使用示例
    /// 
    /// ```rust
    /// use bookforge::epub::ncx::toc_tree::TocTree;
    /// use bookforge::epub::Epub;
    /// 
    /// let epub = Epub::from_path("book.epub")?;
    /// let toc_tree = TocTree::new(&epub);
    /// 
    /// // 获取第一个根节点
    /// if let Some(first_node) = toc_tree.get_first_node() {
    ///     println!("第一个节点: {}", first_node.title);
    /// }
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn new(epub: &'a Epub) -> Self {
        Self {
            title: None,
            roots: Vec::new(),
            style: TocTreeStyle::TreeSymbols,
            show_paths: true,
            max_depth: None,
            epub,
            source: TocTreeSource::Unknown,
        }
    }
    
    /// 创建指定来源的目录树
    /// 
    /// # 参数
    /// * `epub` - EPUB阅读器的引用
    /// * `source` - 目录树来源
    pub fn new_with_source(epub: &'a Epub, source: TocTreeSource) -> Self {
        Self {
            title: None,
            roots: Vec::new(),
            style: TocTreeStyle::TreeSymbols,
            show_paths: true,
            max_depth: None,
            epub,
            source,
        }
    }

    /// 设置文档标题
    pub fn with_title(mut self, title: Option<String>) -> Self {
        self.title = title;
        self
    }

    /// 设置显示样式
    pub fn with_style(mut self, style: TocTreeStyle) -> Self {
        self.style = style;
        self
    }

    /// 设置是否显示文件路径
    pub fn with_show_paths(mut self, show_paths: bool) -> Self {
        self.show_paths = show_paths;
        self
    }

    /// 设置最大显示深度
    pub fn with_max_depth(mut self, max_depth: Option<u32>) -> Self {
        self.max_depth = max_depth;
        self
    }

    /// 添加根节点
    pub fn add_root(&mut self, node: TocTreeNode) {
        self.roots.push(node);
    }

    /// 获取目录树的统计信息
    pub fn get_statistics(&self) -> TocStatistics {
        let mut total_nodes = 0;
        let mut max_depth = 0;
        let mut leaf_count = 0;

        for root in &self.roots {
            total_nodes += root.get_total_nodes();
            max_depth = max_depth.max(root.get_max_depth());
            leaf_count += root.collect_leaf_nodes().len();
        }

        TocStatistics {
            total_nodes,
            max_depth,
            leaf_count,
            root_count: self.roots.len(),
        }
    }

    /// 获取所有章节路径
    pub fn get_all_paths(&self) -> Vec<String> {
        let mut paths = Vec::new();
        for root in &self.roots {
            self.collect_paths(root, &mut paths);
        }
        paths
    }

    /// 递归收集路径
    fn collect_paths(&self, node: &TocTreeNode, paths: &mut Vec<String>) {
        paths.push(node.src.clone());
        for child in &node.children {
            self.collect_paths(child, paths);
        }
    }

    /// 获取所有章节标题
    pub fn get_all_titles(&self) -> Vec<String> {
        let mut titles = Vec::new();
        for root in &self.roots {
            self.collect_titles(root, &mut titles);
        }
        titles
    }

    /// 递归收集标题
    fn collect_titles(&self, node: &TocTreeNode, titles: &mut Vec<String>) {
        titles.push(node.title.clone());
        for child in &node.children {
            self.collect_titles(child, titles);
        }
    }

    /// 根据ID查找节点
    pub fn find_by_id(&self, id: &str) -> Option<&TocTreeNode> {
        for root in &self.roots {
            if let Some(found) = root.find_by_id(id) {
                return Some(found);
            }
        }
        None
    }

    /// 根据源文件路径查找节点
    pub fn find_by_src(&self, src: &str) -> Option<&TocTreeNode> {
        for root in &self.roots {
            if let Some(found) = root.find_by_src(src) {
                return Some(found);
            }
        }
        None
    }

    /// 根据路径数组获取节点
    /// 路径数组表示从根节点开始的索引路径，例如：
    /// - `[0]` 表示第一个根节点
    /// - `[0, 1]` 表示第一个根节点的第二个子节点
    /// - `[1, 0, 2]` 表示第二个根节点的第一个子节点的第三个子节点
    /// 如果对应的节点不存在，则返回 None
    pub fn get_node_by_path(&self, path: &[usize]) -> Option<&TocTreeNode> {
        if path.is_empty() {
            return None;
        }

        let root_index = path[0];
        if root_index >= self.roots.len() {
            return None;
        }

        let root = &self.roots[root_index];
        if path.len() == 1 {
            Some(root)
        } else {
            root.get_node_by_path(&path[1..])
        }
    }

    /// 获取第一个根节点
    pub fn get_first_node(&self) -> Option<&TocTreeNode> {
        self.get_node_by_path(&[0])
    }

    /// 获取指定根节点的第一个子节点
    pub fn get_first_child_of_root(&self, root_index: usize) -> Option<&TocTreeNode> {
        self.get_node_by_path(&[root_index, 0])
    }

    /// 获取节点的下一个兄弟节点
    /// 如果是最后一个节点或者没有找到节点，返回 None
    pub fn get_next_sibling(&self, current_path: &[usize]) -> Option<&TocTreeNode> {
        if current_path.is_empty() {
            return None;
        }

        let mut next_path = current_path.to_vec();
        let last_index = next_path.len() - 1;
        next_path[last_index] += 1;

        self.get_node_by_path(&next_path)
    }

    /// 获取节点的上一个兄弟节点
    /// 如果是第一个节点或者没有找到节点，返回 None
    pub fn get_prev_sibling(&self, current_path: &[usize]) -> Option<&TocTreeNode> {
        if current_path.is_empty() {
            return None;
        }

        let mut prev_path = current_path.to_vec();
        let last_index = prev_path.len() - 1;
        
        if prev_path[last_index] == 0 {
            return None; // 已经是第一个节点
        }
        
        prev_path[last_index] -= 1;
        self.get_node_by_path(&prev_path)
    }

    /// 获取指定节点的HTML内容
    /// 
    /// # 参数
    /// * `node` - 目录树节点的引用
    /// 
    /// # 返回值
    /// * `Result<String, EpubError>` - 成功时返回HTML内容，失败时返回错误
    pub fn get_node_html_content(&self, node: &TocTreeNode) -> Result<String> {
        // 获取NCX文件的目录路径，因为NCX中的路径是相对于NCX文件的
        let full_path = match self.epub.get_ncx_directory()? {
            Some(ncx_dir) => {
                if ncx_dir.is_empty() {
                    // 如果NCX在根目录，直接使用src路径
                    node.src.clone()
                } else {
                    // 使用PathBuf正确处理路径组合和规范化
                    let mut path = PathBuf::from(ncx_dir);
                    path.push(&node.src);
                    
                    // 规范化路径，处理 ../ 等相对路径组件
                    TocTreeNode::normalize_path(&path)
                }
            }
            None => {
                // 如果没有NCX文件，回退到使用OPF目录（兼容性处理）
                let opf_dir = self.epub.get_opf_directory()?;
                if opf_dir.is_empty() {
                    node.src.clone()
                } else {
                    // 使用PathBuf正确处理路径组合和规范化
                    let mut path = PathBuf::from(opf_dir);
                    path.push(&node.src);
                    
                    // 规范化路径，处理 ../ 等相对路径组件
                    TocTreeNode::normalize_path(&path)
                }
            }
        };
        
        // 从EPUB文件中提取HTML内容
        self.epub.read_chapter_file(&full_path).map_err(|e| {
            EpubError::InvalidEpub(format!(
                "无法读取章节文件 '{}' (节点ID: {}, 标题: '{}'): {}",
                full_path, node.id, node.title, e
            ))
        })
    }

    /// 获取指定节点的纯文本内容
    /// 
    /// # 参数
    /// * `node` - 目录树节点的引用
    /// 
    /// # 返回值
    /// * `Result<String, EpubError>` - 成功时返回纯文本内容，失败时返回错误
    pub fn get_node_text_content(&self, node: &TocTreeNode) -> Result<String> {
        let html_content = self.get_node_html_content(node)?;
        
        // 简单的HTML标签移除
        let text_content = TocTreeNode::strip_html_tags(&html_content);
        
        Ok(text_content)
    }

    /// 获取指定节点的格式化文本内容
    /// 
    /// # 参数
    /// * `node` - 目录树节点的引用
    /// 
    /// # 返回值
    /// * `Result<String, EpubError>` - 成功时返回格式化文本内容，失败时返回错误
    pub fn get_node_formatted_text_content(&self, node: &TocTreeNode) -> Result<String> {
        let html_content = self.get_node_html_content(node)?;
        
        // 使用智能HTML解析器转换为格式化文本
        let formatted_text = TocTreeNode::convert_html_to_formatted_text(&html_content);
        
        Ok(formatted_text)
    }

    /// 获取所有章节的HTML内容
    /// 
    /// 该方法会遍历目录树中的所有节点，获取每个节点对应的HTML内容。
    /// 返回的结果按照目录树的遍历顺序排列。
    /// 
    /// # 返回值
    /// * `Result<Vec<(String, String, String)>, EpubError>` - 成功时返回(节点ID, 标题, HTML内容)的元组列表
    /// 
    /// # 使用示例
    /// 
    /// ```rust
    /// use bookforge::epub::Epub;
    /// use bookforge::epub::ncx::toc_tree::create_toc_tree_from_ncx;
    /// 
    /// let epub = Epub::from_path("book.epub")?;
    /// let ncx = epub.ncx()?.unwrap();
    /// let toc_tree = create_toc_tree_from_ncx(&ncx, &epub);
    /// 
    /// match toc_tree.get_all_html_contents() {
    ///     Ok(contents) => {
    ///         for (id, title, html) in contents {
    ///             println!("章节: {} ({})", title, id);
    ///             println!("内容长度: {} 字符", html.len());
    ///         }
    ///     }
    ///     Err(e) => println!("获取章节内容失败: {}", e),
    /// }
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn get_all_html_contents(&self) -> Result<Vec<(String, String, String)>> {
        let mut contents = Vec::new();
        
        for root in &self.roots {
            self.collect_html_contents(root, &mut contents)?;
        }
        
        Ok(contents)
    }

    /// 递归收集HTML内容
    fn collect_html_contents(
        &self,
        node: &TocTreeNode,
        contents: &mut Vec<(String, String, String)>,
    ) -> Result<()> {
        // 获取当前节点的HTML内容
        match self.get_node_html_content(node) {
            Ok(html) => {
                contents.push((node.id.clone(), node.title.clone(), html));
            }
            Err(e) => {
                // 记录错误但继续处理其他章节
                eprintln!("警告: 无法读取章节 '{}' ({}): {}", node.title, node.id, e);
            }
        }
        
        // 递归处理子节点
        for child in &node.children {
            self.collect_html_contents(child, contents)?;
        }
        
        Ok(())
    }

    /// 获取所有章节的纯文本内容
    /// 
    /// 该方法会遍历目录树中的所有节点，获取每个节点对应的纯文本内容。
    /// 这对于全文搜索、内容分析等功能很有用。
    /// 
    /// # 返回值
    /// * `Result<Vec<(String, String, String)>, EpubError>` - 成功时返回(节点ID, 标题, 纯文本内容)的元组列表
    /// 
    /// # 使用示例
    /// 
    /// ```rust
    /// use bookforge::epub::Epub;
    /// use bookforge::epub::ncx::toc_tree::create_toc_tree_from_ncx;
    /// 
    /// let epub = Epub::from_path("book.epub")?;
    /// let ncx = epub.ncx()?.unwrap();
    /// let toc_tree = create_toc_tree_from_ncx(&ncx, &epub);
    /// 
    /// match toc_tree.get_all_text_contents() {
    ///     Ok(contents) => {
    ///         for (id, title, text) in contents {
    ///             println!("章节: {} ({})", title, id);
    ///             println!("文本长度: {} 字符", text.len());
    ///             println!("前100字符: {}", &text[..text.len().min(100)]);
    ///         }
    ///     }
    ///     Err(e) => println!("获取章节文本失败: {}", e),
    /// }
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn get_all_text_contents(&self) -> Result<Vec<(String, String, String)>> {
        let mut contents = Vec::new();
        
        for root in &self.roots {
            self.collect_text_contents(root, &mut contents)?;
        }
        
        Ok(contents)
    }

    /// 递归收集纯文本内容
    fn collect_text_contents(
        &self,
        node: &TocTreeNode,
        contents: &mut Vec<(String, String, String)>,
    ) -> Result<()> {
        // 获取当前节点的纯文本内容
        match self.get_node_text_content(node) {
            Ok(text) => {
                contents.push((node.id.clone(), node.title.clone(), text));
            }
            Err(e) => {
                // 记录错误但继续处理其他章节
                eprintln!("警告: 无法读取章节文本 '{}' ({}): {}", node.title, node.id, e);
            }
        }
        
        // 递归处理子节点
        for child in &node.children {
            self.collect_text_contents(child, contents)?;
        }
        
        Ok(())
    }

    /// 获取所有章节的格式化文本内容
    /// 
    /// 该方法会遍历目录树中的所有节点，获取每个节点对应的格式化文本内容。
    /// 格式化文本会保持原有的HTML结构，正确处理块级元素和HTML实体。
    /// 
    /// # 返回值
    /// * `Result<Vec<(String, String, String)>, EpubError>` - 成功时返回(节点ID, 标题, 格式化文本内容)的元组列表
    /// 
    /// # 使用示例
    /// 
    /// ```rust
    /// use bookforge::epub::Epub;
    /// use bookforge::epub::ncx::toc_tree::create_toc_tree_from_ncx;
    /// 
    /// let epub = Epub::from_path("book.epub")?;
    /// let ncx = epub.ncx()?.unwrap();
    /// let toc_tree = create_toc_tree_from_ncx(&ncx, &epub);
    /// 
    /// match toc_tree.get_all_formatted_text_contents() {
    ///     Ok(contents) => {
    ///         for (id, title, text) in contents {
    ///             println!("章节: {} ({})", title, id);
    ///             println!("格式化文本长度: {} 字符", text.len());
    ///             println!("前200字符:\n{}\n", &text[..text.len().min(200)]);
    ///         }
    ///     }
    ///     Err(e) => println!("获取格式化章节文本失败: {}", e),
    /// }
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn get_all_formatted_text_contents(&self) -> Result<Vec<(String, String, String)>> {
        let mut contents = Vec::new();
        
        for root in &self.roots {
            self.collect_formatted_text_contents(root, &mut contents)?;
        }
        
        Ok(contents)
    }

    /// 递归收集格式化文本内容
    fn collect_formatted_text_contents(
        &self,
        node: &TocTreeNode,
        contents: &mut Vec<(String, String, String)>,
    ) -> Result<()> {
        // 获取当前节点的格式化文本内容
        match self.get_node_formatted_text_content(node) {
            Ok(text) => {
                contents.push((node.id.clone(), node.title.clone(), text));
            }
            Err(e) => {
                // 记录错误但继续处理其他章节
                eprintln!("警告: 无法读取章节格式化文本 '{}' ({}): {}", node.title, node.id, e);
            }
        }
        
        // 递归处理子节点
        for child in &node.children {
            self.collect_formatted_text_contents(child, contents)?;
        }
        
        Ok(())
    }

    /// 为整个目录树生成txt文件
    /// 
    /// 该方法会为目录树中的所有节点生成对应的txt文件。
    /// 支持创建分层目录结构来组织章节文件。
    /// 
    /// # 参数
    /// * `output_dir` - 输出目录路径，如果为None则使用当前目录
    /// * `use_formatted_text` - 是否使用格式化文本，false则使用纯文本
    /// * `create_subdirs` - 是否根据目录树结构创建子目录
    /// 
    /// # 返回值
    /// * `Result<Vec<PathBuf>, EpubError>` - 成功时返回所有生成的文件路径列表，失败时返回错误
    /// 
    /// # 使用示例
    /// 
    /// ```rust
    /// use bookforge::epub::Epub;
    /// use bookforge::epub::ncx::toc_tree::create_toc_tree_from_ncx;
    /// use std::path::Path;
    /// 
    /// let epub = Epub::from_path("book.epub")?;
    /// let ncx = epub.ncx()?.unwrap();
    /// let toc_tree = create_toc_tree_from_ncx(&ncx, &epub);
    /// 
    /// match toc_tree.generate_all_txt_files(Some(Path::new("chapters")), true, true) {
    ///     Ok(file_paths) => {
    ///         println!("已生成 {} 个章节文件:", file_paths.len());
    ///         for path in file_paths {
    ///             println!("  - {:?}", path);
    ///         }
    ///     }
    ///     Err(e) => println!("批量生成章节失败: {}", e),
    /// }
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn generate_all_txt_files(
        &self,
        output_dir: Option<&Path>,
        use_formatted_text: bool,
        create_subdirs: bool,
    ) -> Result<Vec<PathBuf>> {
        let mut all_file_paths = Vec::new();
        
        // 确定输出目录
        let base_dir = output_dir.unwrap_or_else(|| Path::new("."));
        
        // 为所有根节点生成文件
        for root in &self.roots {
            let file_paths = root.generate_txt_files_recursive(
                self.epub,
                Some(base_dir),
                use_formatted_text,
                create_subdirs,
            )?;
            all_file_paths.extend(file_paths);
        }
        
        Ok(all_file_paths)
    }

    /// 为整个目录树生成txt文件，并创建索引文件
    /// 
    /// 该方法不仅会为所有节点生成txt文件，还会创建一个包含所有章节信息的索引文件。
    /// 索引文件包含目录结构和文件路径映射。
    /// 
    /// # 参数
    /// * `output_dir` - 输出目录路径，如果为None则使用当前目录
    /// * `use_formatted_text` - 是否使用格式化文本，false则使用纯文本
    /// * `create_subdirs` - 是否根据目录树结构创建子目录
    /// * `index_filename` - 索引文件名，如果为None则使用默认名称
    /// 
    /// # 返回值
    /// * `Result<(Vec<PathBuf>, PathBuf), EpubError>` - 成功时返回(章节文件路径列表, 索引文件路径)，失败时返回错误
    /// 
    /// # 使用示例
    /// 
    /// ```rust
    /// use bookforge::epub::Epub;
    /// use bookforge::epub::ncx::toc_tree::create_toc_tree_from_ncx;
    /// use std::path::Path;
    /// 
    /// let epub = Epub::from_path("book.epub")?;
    /// let ncx = epub.ncx()?.unwrap();
    /// let toc_tree = create_toc_tree_from_ncx(&ncx, &epub);
    /// 
    /// match toc_tree.generate_all_txt_files_with_index(
    ///     Some(Path::new("chapters")), 
    ///     true, 
    ///     true, 
    ///     Some("目录索引.txt")
    /// ) {
    ///     Ok((file_paths, index_path)) => {
    ///         println!("已生成 {} 个章节文件", file_paths.len());
    ///         println!("索引文件: {:?}", index_path);
    ///     }
    ///     Err(e) => println!("批量生成失败: {}", e),
    /// }
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn generate_all_txt_files_with_index(
        &self,
        output_dir: Option<&Path>,
        use_formatted_text: bool,
        create_subdirs: bool,
        index_filename: Option<&str>,
    ) -> Result<(Vec<PathBuf>, PathBuf)> {
        // 生成所有章节文件
        let file_paths = self.generate_all_txt_files(output_dir, use_formatted_text, create_subdirs)?;
        
        // 确定输出目录和索引文件路径
        let base_dir = output_dir.unwrap_or_else(|| Path::new("."));
        let index_name = index_filename.unwrap_or("目录索引.txt");
        let index_path = base_dir.join(index_name);
        
        // 生成索引文件内容
        let index_content = self.create_index_content(&file_paths, base_dir, use_formatted_text)?;
        
        // 写入索引文件
        fs::write(&index_path, index_content).map_err(|e| {
            EpubError::InvalidEpub(format!(
                "无法写入索引文件 '{}': {}",
                index_path.display(),
                e
            ))
        })?;
        
        Ok((file_paths, index_path))
    }

    /// 将所有章节合并为一个txt文件
    /// 
    /// 该方法会将目录树中的所有章节内容按顺序合并到一个txt文件中。
    /// 文件名会基于EPUB的标题生成，每个章节之间会有清晰的分隔。
    /// 
    /// # 参数
    /// * `output_dir` - 输出目录路径，如果为None则使用当前目录
    /// * `use_formatted_text` - 是否使用格式化文本，false则使用纯文本
    /// * `filename` - 自定义文件名，如果为None则使用书籍标题
    /// 
    /// # 返回值
    /// * `Result<PathBuf, EpubError>` - 成功时返回生成的文件路径，失败时返回错误
    /// 
    /// # 使用示例
    /// 
    /// ```rust
    /// use bookforge::epub::Epub;
    /// use bookforge::epub::ncx::toc_tree::create_toc_tree_from_ncx;
    /// use std::path::Path;
    /// 
    /// let epub = Epub::from_path("book.epub")?;
    /// let ncx = epub.ncx()?.unwrap();
    /// let toc_tree = create_toc_tree_from_ncx(&ncx, &epub);
    /// 
    /// match toc_tree.generate_merged_txt_file(
    ///     Some(Path::new("output")), 
    ///     true,
    ///     None
    /// ) {
    ///     Ok(file_path) => println!("合并文件已保存到: {:?}", file_path),
    ///     Err(e) => println!("合并文件失败: {}", e),
    /// }
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn generate_merged_txt_file(
        &self,
        output_dir: Option<&Path>,
        use_formatted_text: bool,
        filename: Option<&str>,
    ) -> Result<PathBuf> {
        // 确定输出目录
        let dir = output_dir.unwrap_or_else(|| Path::new("."));
        
        // 创建输出目录（如果不存在）
        if !dir.exists() {
            fs::create_dir_all(dir).map_err(|e| {
                EpubError::InvalidEpub(format!(
                    "无法创建输出目录 '{}': {}",
                    dir.display(),
                    e
                ))
            })?;
        }

        // 生成文件名
        let safe_filename = if let Some(name) = filename {
            name.to_string()
        } else if let Some(ref title) = self.title {
            Self::generate_safe_book_filename(title)
        } else {
            "merged_book".to_string()
        };
        
        let file_path = dir.join(format!("{}.txt", safe_filename));

        // 收集所有章节内容
        let chapter_contents = if use_formatted_text {
            self.get_all_formatted_text_contents()?
        } else {
            self.get_all_text_contents()?
        };

        // 创建合并文件内容
        let merged_content = self.create_merged_file_content(&chapter_contents, use_formatted_text)?;

        // 写入文件
        fs::write(&file_path, merged_content).map_err(|e| {
            EpubError::InvalidEpub(format!(
                "无法写入合并文件 '{}': {}",
                file_path.display(),
                e
            ))
        })?;

        Ok(file_path)
    }

    /// 生成安全的书籍文件名
    fn generate_safe_book_filename(title: &str) -> String {
        // 移除或替换不安全的字符
        let mut safe_title = title
            .chars()
            .map(|c| match c {
                // 文件系统保留字符
                '<' | '>' | ':' | '"' | '|' | '?' | '*' => '_',
                '/' | '\\' => '_',
                // 控制字符
                c if c.is_control() => '_',
                // 其他字符保持不变
                c => c,
            })
            .collect::<String>();

        // 移除开头和结尾的空白字符和点号
        safe_title = safe_title.trim().trim_matches('.').to_string();
        
        // 如果标题为空或只包含无效字符，使用默认名称
        if safe_title.is_empty() {
            safe_title = "unnamed_book".to_string();
        }

        // 限制文件名长度
        const MAX_FILENAME_LENGTH: usize = 150;
        if safe_title.len() > MAX_FILENAME_LENGTH {
            safe_title.truncate(MAX_FILENAME_LENGTH);
            // 确保不会在Unicode字符中间截断
            while !safe_title.is_char_boundary(safe_title.len()) {
                safe_title.pop();
            }
        }

        safe_title
    }

    /// 创建合并文件内容
    fn create_merged_file_content(
        &self,
        chapter_contents: &[(String, String, String)],
        use_formatted_text: bool,
    ) -> Result<String> {
        let mut content = String::new();
        
        // 添加文件头部
        content.push_str("═══════════════════════════════════════\n");
        content.push_str("           BookForge EPUB 完整内容\n");
        content.push_str("═══════════════════════════════════════\n\n");
        
        // 添加书籍信息
        if let Some(ref title) = self.title {
            content.push_str(&format!("书籍标题: {}\n", title));
        }
        
        let stats = self.get_statistics();
        content.push_str(&format!("章节总数: {}\n", stats.total_nodes));
        content.push_str(&format!("文本格式: {}\n", if use_formatted_text { "格式化文本" } else { "纯文本" }));
        
        // 获取当前时间
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        content.push_str(&format!("生成时间: Unix时间戳 {}\n", now));
        content.push_str("\n");
        
        // 添加目录概览
        content.push_str("═══════════════════════════════════════\n");
        content.push_str("                目录概览\n");
        content.push_str("═══════════════════════════════════════\n\n");
        
        for (index, (_, title, _)) in chapter_contents.iter().enumerate() {
            content.push_str(&format!("{}. {}\n", index + 1, title));
        }
        content.push_str("\n");
        
        // 添加章节内容
        content.push_str("═══════════════════════════════════════\n");
        content.push_str("                正文内容\n");
        content.push_str("═══════════════════════════════════════\n\n");
        
        for (index, (id, title, chapter_content)) in chapter_contents.iter().enumerate() {
            // 章节标题分隔
            content.push_str("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
            content.push_str(&format!("第 {} 章: {}\n", index + 1, title));
            content.push_str(&format!("章节ID: {}\n", id));
            content.push_str("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n\n");
            
            // 章节内容
            content.push_str(chapter_content);
            content.push_str("\n\n");
            
            // 章节结束分隔
            content.push_str("─────────────────────────────────────\n");
            content.push_str(&format!("第 {} 章结束\n", index + 1));
            content.push_str("─────────────────────────────────────\n\n\n");
        }
        
        // 添加文件尾部
        content.push_str("═══════════════════════════════════════\n");
        content.push_str("                全书结束\n");
        content.push_str("═══════════════════════════════════════\n");
        content.push_str("Generated by BookForge EPUB Reader\n");
        content.push_str("═══════════════════════════════════════\n");
        
        Ok(content)
    }

    /// 创建索引文件内容
    fn create_index_content(
        &self,
        file_paths: &[PathBuf],
        base_dir: &Path,
        use_formatted_text: bool,
    ) -> Result<String> {
        let mut content = String::new();
        
        // 添加索引文件头部
        content.push_str("═══════════════════════════════════════\n");
        content.push_str("           BookForge EPUB 章节索引\n");
        content.push_str("═══════════════════════════════════════\n\n");
        
        // 添加基本信息
        if let Some(ref title) = self.title {
            content.push_str(&format!("电子书标题: {}\n", title));
        }
        
        let stats = self.get_statistics();
        content.push_str(&format!("章节总数: {}\n", stats.total_nodes));
        content.push_str(&format!("根章节数: {}\n", stats.root_count));
        content.push_str(&format!("最大深度: {}\n", stats.max_depth));
        content.push_str(&format!("文本格式: {}\n", if use_formatted_text { "格式化文本" } else { "纯文本" }));
        // 获取当前时间
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        content.push_str(&format!("生成时间: Unix时间戳 {}\n", now));
        content.push_str(&format!("文件总数: {}\n\n", file_paths.len()));
        
        // 添加目录树结构
        content.push_str("═══════════════════════════════════════\n");
        content.push_str("                目录结构\n");
        content.push_str("═══════════════════════════════════════\n\n");
        
        // 渲染目录树（不显示文件路径）
        let tree_content = self.render_tree_for_index();
        content.push_str(&tree_content);
        content.push_str("\n");
        
        // 添加文件路径映射
        content.push_str("═══════════════════════════════════════\n");
        content.push_str("                文件路径映射\n");
        content.push_str("═══════════════════════════════════════\n\n");
        
        // 收集所有节点信息和对应的文件路径
        let node_info_list = self.collect_node_info_list();
        
        for (index, (node_info, file_path)) in node_info_list.iter().zip(file_paths.iter()).enumerate() {
            let relative_path = file_path.strip_prefix(base_dir)
                .unwrap_or(file_path)
                .display();
            
            content.push_str(&format!(
                "{:3}. [{}] {} \n     文件: {}\n     源文件: {}\n\n",
                index + 1,
                node_info.play_order,
                node_info.title,
                relative_path,
                node_info.src
            ));
        }
        
        // 添加尾部信息
        content.push_str("═══════════════════════════════════════\n");
        content.push_str("Generated by BookForge EPUB Reader\n");
        content.push_str("═══════════════════════════════════════\n");
        
        Ok(content)
    }

    /// 为索引文件渲染目录树
    fn render_tree_for_index(&self) -> String {
        let mut result = String::new();
        
        // 渲染根节点
        for (index, root) in self.roots.iter().enumerate() {
            let is_last = index == self.roots.len() - 1;
            self.render_node_for_index(root, 0, is_last, "", &mut result);
        }
        
        result
    }

    /// 为索引文件渲染单个节点
    fn render_node_for_index(
        &self,
        node: &TocTreeNode,
        current_depth: u32,
        is_last: bool,
        prefix: &str,
        result: &mut String,
    ) {
        let current_prefix = if is_last { "└── " } else { "├── " };
        
        // 格式化节点内容（不显示文件路径）
        let content = format!("[{}] {}", node.play_order, node.title);
        result.push_str(&format!("{}{}{}\n", prefix, current_prefix, content));

        // 渲染子节点
        let child_prefix = format!("{}{}", prefix, if is_last { "    " } else { "│   " });
        for (index, child) in node.children.iter().enumerate() {
            let is_child_last = index == node.children.len() - 1;
            self.render_node_for_index(child, current_depth + 1, is_child_last, &child_prefix, result);
        }
    }

    /// 收集所有节点信息
    fn collect_node_info_list(&self) -> Vec<NodeInfo> {
        let mut node_info_list = Vec::new();
        
        for root in &self.roots {
            self.collect_node_info_recursive(root, &mut node_info_list);
        }
        
        node_info_list
    }

    /// 递归收集节点信息
    fn collect_node_info_recursive(&self, node: &TocTreeNode, info_list: &mut Vec<NodeInfo>) {
        info_list.push(NodeInfo {
            play_order: node.play_order,
            title: node.title.clone(),
            src: node.src.clone(),
        });
        
        for child in &node.children {
            self.collect_node_info_recursive(child, info_list);
        }
    }

    /// 渲染单个节点
    fn render_node(
        &self,
        node: &TocTreeNode,
        current_depth: u32,
        is_last: bool,
        prefix: &str,
        result: &mut String,
    ) {
        // 检查深度限制
        if let Some(max_depth) = self.max_depth {
            if current_depth >= max_depth {
                return;
            }
        }

        match self.style {
            TocTreeStyle::TreeSymbols => {
                self.render_tree_style(node, current_depth, is_last, prefix, result);
            }
            TocTreeStyle::Indented => {
                self.render_indent_style(node, current_depth, result);
            }
        }
    }

    /// 渲染树状符号风格
    fn render_tree_style(
        &self,
        node: &TocTreeNode,
        current_depth: u32,
        is_last: bool,
        prefix: &str,
        result: &mut String,
    ) {
        let current_prefix = if is_last { "└── " } else { "├── " };
        
        // 格式化节点内容
        let content = if self.show_paths {
            format!("[{}] {} → {}", node.play_order, node.title, node.src)
        } else {
            format!("[{}] {}", node.play_order, node.title)
        };
        
        result.push_str(&format!("{}{}{}\n", prefix, current_prefix, content));

        // 渲染子节点
        if let Some(max_depth) = self.max_depth {
            if current_depth + 1 >= max_depth {
                return;
            }
        }

        let child_prefix = format!("{}{}", prefix, if is_last { "    " } else { "│   " });
        for (index, child) in node.children.iter().enumerate() {
            let is_child_last = index == node.children.len() - 1;
            self.render_node(child, current_depth + 1, is_child_last, &child_prefix, result);
        }
    }

    /// 渲染缩进风格
    fn render_indent_style(&self, node: &TocTreeNode, current_depth: u32, result: &mut String) {
        let indent = "  ".repeat(current_depth as usize);
        
        // 格式化节点内容
        let content = if self.show_paths {
            format!("• [{}] {} → {}", node.play_order, node.title, node.src)
        } else {
            format!("• [{}] {}", node.play_order, node.title)
        };
        
        result.push_str(&format!("{}{}\n", indent, content));

        // 渲染子节点
        if let Some(max_depth) = self.max_depth {
            if current_depth + 1 >= max_depth {
                return;
            }
        }

        for child in &node.children {
            self.render_indent_style(child, current_depth + 1, result);
        }
    }
}

// Note: TocTree 不再实现 Default trait，因为需要 epub 引用参数

impl<'a> Display for TocTree<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let mut result = String::new();
        
        // 添加文档标题
        if let Some(ref title) = self.title {
            let depth_info = if let Some(max_depth) = self.max_depth {
                format!(" (深度限制: {})", max_depth)
            } else {
                String::new()
            };
            result.push_str(&format!("📖 {}{}\n", title, depth_info));
            result.push_str("═══════════════════════════════════════\n\n");
        }
        
        // 渲染根节点
        for (index, root) in self.roots.iter().enumerate() {
            let is_last = index == self.roots.len() - 1;
            self.render_node(root, 0, is_last, "", &mut result);
        }
        
        write!(f, "{}", result)
    }
}

/// 目录树统计信息
#[derive(Debug, Clone)]
pub struct TocStatistics {
    /// 总节点数
    pub total_nodes: usize,
    /// 最大深度
    pub max_depth: u32,
    /// 叶子节点数
    pub leaf_count: usize,
    /// 根节点数
    pub root_count: usize,
}

/// 节点信息结构体（用于避免生命周期问题）
#[derive(Debug, Clone)]
struct NodeInfo {
    /// 播放顺序
    pub play_order: u32,
    /// 标题
    pub title: String,
    /// 源文件路径
    pub src: String,
}

impl Display for TocStatistics {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(
            f,
            "目录统计: {} 个章节, {} 个根节点, {} 个叶子节点, 最大深度: {}",
            self.total_nodes, self.root_count, self.leaf_count, self.max_depth
        )
    }
}

/// 从NCX创建目录树
pub fn create_toc_tree_from_ncx<'a>(ncx: &Ncx, epub: &'a Epub) -> TocTree<'a> {
    let mut toc_tree = TocTree::new(epub);
    
    // 设置文档标题
    toc_tree.title = ncx.get_title().map(|t| t.clone());
    
    // 转换导航点为目录树节点
    for nav_point in &ncx.nav_map.nav_points {
        let toc_node = convert_nav_point_to_toc_node(nav_point, 0);
        toc_tree.add_root(toc_node);
    }
    
    toc_tree
}

/// 递归转换导航点为目录树节点
fn convert_nav_point_to_toc_node(nav_point: &NavPoint, depth: u32) -> TocTreeNode {
    let mut toc_node = TocTreeNode::new(
        nav_point.play_order,
        nav_point.nav_label.text.clone(),
        nav_point.content.src.clone(),
        nav_point.id.clone(),
        depth,
    );
    
    // 转换子节点
    for child in &nav_point.children {
        let child_node = convert_nav_point_to_toc_node(child, depth + 1);
        toc_node.add_child(child_node);
    }
    
    toc_node
} 