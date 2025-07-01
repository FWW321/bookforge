//! NCX导航元素数据结构定义
//! 
//! 定义NCX文件中的各种导航元素，包括导航点、导航标签、内容引用等。

use std::collections::HashMap;

/// NCX元数据信息
#[derive(Debug, Clone)]
pub struct NcxMetadata {
    /// 唯一标识符（dtb:uid）
    pub uid: Option<String>,
    /// 导航深度（dtb:depth）
    pub depth: Option<u32>,
    /// 总页数（dtb:totalPageCount）
    pub total_page_count: Option<u32>,
    /// 最大页码（dtb:maxPageNumber）  
    pub max_page_number: Option<u32>,
    /// 其他元数据
    pub other_metadata: HashMap<String, String>,
}

impl NcxMetadata {
    /// 创建新的NCX元数据
    pub fn new() -> Self {
        Self {
            uid: None,
            depth: None,
            total_page_count: None,
            max_page_number: None,
            other_metadata: HashMap::new(),
        }
    }
}

impl Default for NcxMetadata {
    fn default() -> Self {
        Self::new()
    }
}

/// 文档标题
#[derive(Debug, Clone)]
pub struct DocTitle {
    /// 标题文本
    pub text: String,
}

impl DocTitle {
    /// 创建新的文档标题
    pub fn new(text: String) -> Self {
        Self { text }
    }
}



/// 导航标签
#[derive(Debug, Clone)]
pub struct NavLabel {
    /// 标签文本
    pub text: String,
}

impl NavLabel {
    /// 创建新的导航标签
    pub fn new(text: String) -> Self {
        Self {
            text,
        }
    }
}

/// 导航内容引用
#[derive(Debug, Clone)]
pub struct NavContent {
    /// 源文件路径
    pub src: String,
}

impl NavContent {
    /// 创建新的导航内容引用
    pub fn new(src: String) -> Self {
        Self { src }
    }
}

/// 导航点
#[derive(Debug, Clone)]
pub struct NavPoint {
    /// 唯一标识符
    pub id: String,
    /// 播放顺序
    pub play_order: u32,
    /// CSS类名（可选）
    pub class: Option<String>,
    /// 导航标签
    pub nav_label: NavLabel,
    /// 内容引用
    pub content: NavContent,
    /// 子导航点
    pub children: Vec<NavPoint>,
}

impl NavPoint {
    /// 创建新的导航点
    pub fn new(id: String, play_order: u32, nav_label: NavLabel, content: NavContent) -> Self {
        Self {
            id,
            play_order,
            class: None,
            nav_label,
            content,
            children: Vec::new(),
        }
    }

    /// 添加子导航点
    pub fn add_child(&mut self, child: NavPoint) {
        self.children.push(child);
    }

    /// 按playOrder排序子导航点
    pub fn sort_children_by_play_order(&mut self) {
        self.children.sort_by(|a, b| a.play_order.cmp(&b.play_order));
        for child in &mut self.children {
            child.sort_children_by_play_order();
        }
    }

    /// 获取所有导航点（包括子导航点）的平铺列表
    pub fn get_all_nav_points(&self) -> Vec<&NavPoint> {
        let mut points = vec![self];
        for child in &self.children {
            points.extend(child.get_all_nav_points());
        }
        points
    }

    /// 获取导航深度
    pub fn get_depth(&self) -> u32 {
        if self.children.is_empty() {
            1
        } else {
            1 + self.children.iter().map(|child| child.get_depth()).max().unwrap_or(0)
        }
    }
}

/// 导航地图
#[derive(Debug, Clone)]
pub struct NavMap {
    /// 导航点列表
    pub nav_points: Vec<NavPoint>,
}

impl NavMap {
    /// 创建新的导航地图
    pub fn new() -> Self {
        Self {
            nav_points: Vec::new(),
        }
    }

    /// 添加导航点
    pub fn add_nav_point(&mut self, nav_point: NavPoint) {
        self.nav_points.push(nav_point);
    }

    /// 按playOrder排序所有导航点（包括子导航点）
    pub fn sort_by_play_order(&mut self) {
        self.nav_points.sort_by(|a, b| a.play_order.cmp(&b.play_order));
        for nav_point in &mut self.nav_points {
            nav_point.sort_children_by_play_order();
        }
    }

    /// 获取所有导航点的平铺列表
    pub fn get_all_nav_points(&self) -> Vec<&NavPoint> {
        let mut all_points = Vec::new();
        for nav_point in &self.nav_points {
            all_points.extend(nav_point.get_all_nav_points());
        }
        all_points
    }

    /// 获取导航深度
    pub fn get_depth(&self) -> u32 {
        self.nav_points.iter().map(|point| point.get_depth()).max().unwrap_or(0)
    }

    /// 根据ID查找导航点
    pub fn find_nav_point_by_id(&self, id: &str) -> Option<&NavPoint> {
        fn find_in_nav_point<'a>(nav_point: &'a NavPoint, target_id: &str) -> Option<&'a NavPoint> {
            if nav_point.id == target_id {
                return Some(nav_point);
            }
            for child in &nav_point.children {
                if let Some(found) = find_in_nav_point(child, target_id) {
                    return Some(found);
                }
            }
            None
        }

        for nav_point in &self.nav_points {
            if let Some(found) = find_in_nav_point(nav_point, id) {
                return Some(found);
            }
        }
        None
    }
}

impl Default for NavMap {
    fn default() -> Self {
        Self::new()
    }
}

/// 页面目标
#[derive(Debug, Clone)]
pub struct PageTarget {
    /// 唯一标识符
    pub id: String,
    /// 页面类型（normal, front, special等）
    pub page_type: String,
    /// 页面值
    pub value: String,
    /// 播放顺序
    pub play_order: u32,
    /// 导航标签
    pub nav_label: NavLabel,
    /// 内容引用
    pub content: NavContent,
}

impl PageTarget {
    /// 创建新的页面目标
    pub fn new(
        id: String,
        page_type: String,
        value: String,
        play_order: u32,
        nav_label: NavLabel,
        content: NavContent,
    ) -> Self {
        Self {
            id,
            page_type,
            value,
            play_order,
            nav_label,
            content,
        }
    }
}

/// 页面列表
#[derive(Debug, Clone)]
pub struct PageList {
    /// 导航标签
    pub nav_label: Option<NavLabel>,
    /// 页面目标列表
    pub page_targets: Vec<PageTarget>,
}

impl PageList {
    /// 创建新的页面列表
    pub fn new() -> Self {
        Self {
            nav_label: None,
            page_targets: Vec::new(),
        }
    }

    /// 添加页面目标
    pub fn add_page_target(&mut self, page_target: PageTarget) {
        self.page_targets.push(page_target);
    }

    /// 根据页面值查找页面目标
    pub fn find_page_target_by_value(&self, value: &str) -> Option<&PageTarget> {
        self.page_targets.iter().find(|target| target.value == value)
    }

    /// 根据ID查找页面目标
    pub fn find_page_target_by_id(&self, id: &str) -> Option<&PageTarget> {
        self.page_targets.iter().find(|target| target.id == id)
    }
}

impl Default for PageList {
    fn default() -> Self {
        Self::new()
    }
} 