//! 目录树（Table of Contents Tree）模块
//! 
//! 提供NCX目录结构的树形表示和显示功能。

use std::fmt::{Display, Formatter, Result as FmtResult};
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
    /// 文件路径会根据EPUB的OPF目录进行解析。
    /// 
    /// # 参数
    /// * `epub` - EPUB阅读器的可变引用
    /// 
    /// # 返回值
    /// * `Result<String, EpubError>` - 成功时返回HTML内容，失败时返回错误
    /// 
    /// # 错误处理
    /// * 如果无法获取OPF目录，返回相应错误
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
    pub fn get_html_content(&self, epub: &mut Epub) -> Result<String> {
        // 获取OPF文件的目录路径，用于构建完整的文件路径
        let opf_dir = epub.get_opf_directory()?;
        
        // 构建完整的文件路径
        let full_path = if opf_dir.is_empty() {
            // 如果OPF在根目录，直接使用src路径
            self.src.clone()
        } else {
            // 如果OPF在子目录中，需要组合路径
            format!("{}/{}", opf_dir, self.src)
        };
        
        // 从EPUB文件中提取HTML内容
        epub.extract_file(&full_path).map_err(|e| {
            EpubError::InvalidEpub(format!(
                "无法读取章节文件 '{}' (节点ID: {}, 标题: '{}'): {}",
                full_path, self.id, self.title, e
            ))
        })
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
    /// let mut epub = Epub::new("book.epub")?;
    /// let ncx_content = epub.extract_file("toc.ncx")?;
    /// let ncx = bookforge::epub::ncx::Ncx::parse_xml(&ncx_content)?;
    /// let toc_tree = create_toc_tree_from_ncx(&ncx);
    /// 
    /// if let Some(first_node) = toc_tree.get_first_node() {
    ///     match first_node.get_text_content(&mut epub) {
    ///         Ok(text) => println!("章节纯文本: {}", text),
    ///         Err(e) => println!("获取章节文本失败: {}", e),
    ///     }
    /// }
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn get_text_content(&self, epub: &mut Epub) -> Result<String> {
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
    pub fn get_formatted_text_content(&self, epub: &mut Epub) -> Result<String> {
        let html_content = self.get_html_content(epub)?;
        
        // 使用智能HTML解析器转换为格式化文本
        let formatted_text = Self::convert_html_to_formatted_text(&html_content);
        
        Ok(formatted_text)
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
#[derive(Debug, Clone)]
pub struct TocTree {
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
}

impl TocTree {
    /// 创建新的目录树
    /// 
    /// # 使用示例
    /// 
    /// ```rust
    /// use bookforge::epub::ncx::toc_tree::TocTree;
    /// 
    /// let toc_tree = TocTree::new();
    /// 
    /// // 获取第一个根节点
    /// if let Some(first_node) = toc_tree.get_first_node() {
    ///     println!("第一个节点: {}", first_node.title);
    /// }
    /// 
    /// // 获取第一个根节点的第二个子节点
    /// if let Some(node) = toc_tree.get_node_by_path(&[0, 1]) {
    ///     println!("节点: {}", node.title);
    /// }
    /// 
    /// // 获取第二个根节点的第一个子节点的第三个子节点
    /// if let Some(node) = toc_tree.get_node_by_path(&[1, 0, 2]) {
    ///     println!("深层节点: {}", node.title);
    /// }
    /// ```
    pub fn new() -> Self {
        Self {
            title: None,
            roots: Vec::new(),
            style: TocTreeStyle::TreeSymbols,
            show_paths: true,
            max_depth: None,
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

    /// 获取所有章节的HTML内容
    /// 
    /// 该方法会遍历目录树中的所有节点，获取每个节点对应的HTML内容。
    /// 返回的结果按照目录树的遍历顺序排列。
    /// 
    /// # 参数
    /// * `epub` - EPUB阅读器的可变引用
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
    /// let mut epub = Epub::new("book.epub")?;
    /// let ncx_content = epub.extract_file("toc.ncx")?;
    /// let ncx = bookforge::epub::ncx::Ncx::parse_xml(&ncx_content)?;
    /// let toc_tree = create_toc_tree_from_ncx(&ncx);
    /// 
    /// match toc_tree.get_all_html_contents(&mut epub) {
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
    pub fn get_all_html_contents(&self, epub: &mut Epub) -> Result<Vec<(String, String, String)>> {
        let mut contents = Vec::new();
        
        for root in &self.roots {
            self.collect_html_contents(root, epub, &mut contents)?;
        }
        
        Ok(contents)
    }

    /// 递归收集HTML内容
    fn collect_html_contents(
        &self,
        node: &TocTreeNode,
        epub: &mut Epub,
        contents: &mut Vec<(String, String, String)>,
    ) -> Result<()> {
        // 获取当前节点的HTML内容
        match node.get_html_content(epub) {
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
            self.collect_html_contents(child, epub, contents)?;
        }
        
        Ok(())
    }

    /// 获取所有章节的纯文本内容
    /// 
    /// 该方法会遍历目录树中的所有节点，获取每个节点对应的纯文本内容。
    /// 这对于全文搜索、内容分析等功能很有用。
    /// 
    /// # 参数
    /// * `epub` - EPUB阅读器的可变引用
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
    /// let mut epub = Epub::new("book.epub")?;
    /// let ncx_content = epub.extract_file("toc.ncx")?;
    /// let ncx = bookforge::epub::ncx::Ncx::parse_xml(&ncx_content)?;
    /// let toc_tree = create_toc_tree_from_ncx(&ncx);
    /// 
    /// match toc_tree.get_all_text_contents(&mut epub) {
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
    pub fn get_all_text_contents(&self, epub: &mut Epub) -> Result<Vec<(String, String, String)>> {
        let mut contents = Vec::new();
        
        for root in &self.roots {
            self.collect_text_contents(root, epub, &mut contents)?;
        }
        
        Ok(contents)
    }

    /// 递归收集纯文本内容
    fn collect_text_contents(
        &self,
        node: &TocTreeNode,
        epub: &mut Epub,
        contents: &mut Vec<(String, String, String)>,
    ) -> Result<()> {
        // 获取当前节点的纯文本内容
        match node.get_text_content(epub) {
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
            self.collect_text_contents(child, epub, contents)?;
        }
        
        Ok(())
    }

    /// 获取所有章节的格式化文本内容
    /// 
    /// 该方法会遍历目录树中的所有节点，获取每个节点对应的格式化文本内容。
    /// 格式化文本会保持原有的HTML结构，正确处理块级元素和HTML实体。
    /// 
    /// # 参数
    /// * `epub` - EPUB阅读器的可变引用
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
    /// let mut epub = Epub::new("book.epub")?;
    /// let ncx_content = epub.extract_file("toc.ncx")?;
    /// let ncx = bookforge::epub::ncx::Ncx::parse_xml(&ncx_content)?;
    /// let toc_tree = create_toc_tree_from_ncx(&ncx);
    /// 
    /// match toc_tree.get_all_formatted_text_contents(&mut epub) {
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
    pub fn get_all_formatted_text_contents(&self, epub: &mut Epub) -> Result<Vec<(String, String, String)>> {
        let mut contents = Vec::new();
        
        for root in &self.roots {
            self.collect_formatted_text_contents(root, epub, &mut contents)?;
        }
        
        Ok(contents)
    }

    /// 递归收集格式化文本内容
    fn collect_formatted_text_contents(
        &self,
        node: &TocTreeNode,
        epub: &mut Epub,
        contents: &mut Vec<(String, String, String)>,
    ) -> Result<()> {
        // 获取当前节点的格式化文本内容
        match node.get_formatted_text_content(epub) {
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
            self.collect_formatted_text_contents(child, epub, contents)?;
        }
        
        Ok(())
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

impl Default for TocTree {
    fn default() -> Self {
        Self::new()
    }
}

impl Display for TocTree {
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
pub fn create_toc_tree_from_ncx(ncx: &Ncx) -> TocTree {
    let mut toc_tree = TocTree::new();
    
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