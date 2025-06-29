use bookforge::{Epub, Result};
use clap::Parser;

/// 📚 BookForge - EPUB文件处理工具
#[derive(Parser)]
#[command(name = "bookforge")]
#[command(about = "一个用于处理EPUB文件的Rust工具")]
#[command(version)]
struct Args {
    /// EPUB文件路径
    #[arg(help = "要处理的EPUB文件路径")]
    epub_file: String,
    
    /// 详细输出模式
    #[arg(short, long, help = "显示详细信息")]
    verbose: bool,
    
    /// 显示元数据信息
    #[arg(short, long, help = "显示EPUB元数据信息")]
    metadata: bool,
    
    /// 显示NCX导航信息
    #[arg(short, long, help = "显示NCX导航控制文件信息")]
    ncx: bool,
    
    /// 显示目录树
    #[arg(short, long, help = "显示EPUB目录树结构")]
    toc: bool,
    
    /// 显示指定章节内容
    #[arg(short, long, help = "显示指定章节的内容（使用章节索引，从1开始）")]
    chapter: Option<usize>,
    
    /// 章节内容显示格式
    #[arg(long, value_enum, default_value = "formatted", help = "章节内容的显示格式")]
    format: ContentFormat,
    
    /// 章节内容最大显示长度
    #[arg(long, default_value = "2000", help = "章节内容最大显示字符数（0表示不限制）")]
    max_length: usize,
}

/// 章节内容显示格式
#[derive(clap::ValueEnum, Clone, Debug)]
enum ContentFormat {
    /// 原始HTML格式
    Html,
    /// 纯文本格式（移除所有HTML标签）
    Text,
    /// 格式化文本（保持结构，智能处理HTML标签）
    Formatted,
}

fn main() {
    let args = Args::parse();
    
    println!("📚 BookForge - EPUB处理工具");
    
    if args.verbose {
        println!("🔍 详细模式已启用");
    }
    
    if args.metadata {
        println!("📊 元数据模式已启用");
    }
    
    if args.toc {
        println!("🌳 目录树模式已启用");
    }
    
    if let Some(chapter_index) = args.chapter {
        println!("📖 章节内容模式已启用 (章节: {}, 格式: {:?})", chapter_index, args.format);
    }
    
    println!("正在检查EPUB文件: {}", args.epub_file);
    
    match process_epub(
        &args.epub_file, 
        args.verbose, 
        args.metadata, 
        args.ncx, 
        args.toc,
        args.chapter,
        args.format,
        args.max_length
    ) {
        Ok(_) => println!("🎉 EPUB文件处理完成！"),
        Err(e) => eprintln!("❌ 错误: {}", e),
    }
}

fn process_epub(
    path: &str, 
    verbose: bool, 
    show_metadata: bool, 
    show_ncx: bool, 
    show_toc: bool,
    chapter_index: Option<usize>,
    content_format: ContentFormat,
    max_length: usize
) -> Result<()> {
    // 创建Epub实例，会自动验证EPUB格式和mimetype
    let mut epub = Epub::new(path)?;
    
    // 列出文件
    println!("\n📁 EPUB文件内容:");
    let files = epub.list_files()?;
    
    if verbose {
        // 详细模式：显示所有文件
        for (i, file) in files.iter().enumerate() {
            println!("  {}. {}", i + 1, file);
        }
    } else {
        // 简洁模式：只显示文件总数
        println!("  共找到 {} 个文件", files.len());
    }
    
    // 解析container.xml并显示OPF路径
    match epub.parse_container() {
        Ok(container) => {
            println!("\n📦 Container.xml信息:");
            println!("  找到 {} 个 rootfile 条目", container.rootfiles.len());
            
            if verbose {
                for (i, rootfile) in container.rootfiles.iter().enumerate() {
                    println!("  {}. {} ({})", i + 1, rootfile.full_path, rootfile.media_type);
                }
            }
            
            if let Some(opf_path) = container.get_opf_path() {
                println!("  📚 主OPF文件路径: {}", opf_path);
            }
        }
        Err(e) => {
            if verbose {
                println!("\n⚠️  无法解析container.xml: {}", e);
            }
        }
    }
    
    // 显示元数据信息
    if show_metadata {
        display_metadata(&mut epub)?;
    }
    
    // 显示NCX导航信息
    if show_ncx {
        display_ncx(&mut epub, verbose)?;
    }
    
    // 显示目录树
    if show_toc {
        display_table_of_contents(&mut epub, verbose)?;
    }
    
    // 显示指定章节内容
    if let Some(index) = chapter_index {
        display_chapter_content(&mut epub, index, content_format, max_length)?;
    }
    
    Ok(())
}

/// 显示EPUB元数据信息
fn display_metadata(epub: &mut Epub) -> Result<()> {
    println!("\n📊 EPUB元数据信息:");
    
    // 使用配置文件解析OPF，如果配置文件不存在会自动生成
    let config_path = "metadata_tags.yaml";
    match epub.parse_opf_with_config(Some(config_path)) {
        Ok(opf) => {
            println!("  📖 EPUB版本: {}", opf.version);
            
            // 基本信息
            println!("\n  📚 基本信息:");
            if let Some(title) = opf.metadata.title() {
                println!("    标题: {}", title);
            }
            
            let creators = opf.metadata.creators();
            if !creators.is_empty() {
                println!("    作者:");
                for (i, creator) in creators.iter().enumerate() {
                    let mut author_info = format!("      {}. {}", i + 1, creator.name);
                    if let Some(role) = &creator.role {
                        author_info.push_str(&format!(" ({})", role));
                    }
                    if let Some(file_as) = &creator.file_as {
                        author_info.push_str(&format!(" [排序: {}]", file_as));
                    }
                    println!("{}", author_info);
                }
            }
            
            if let Some(language) = opf.metadata.language() {
                println!("    语言: {}", language);
            }
            
            if let Some(publisher) = opf.metadata.publisher() {
                println!("    出版社: {}", publisher);
            }
            
            if let Some(date) = opf.metadata.date() {
                println!("    出版日期: {}", date);
            }
            
            if let Some(description) = opf.metadata.description() {
                println!("    描述: {}", description);
            }
            
            // 标识符信息
            let identifiers = opf.metadata.identifiers();
            if !identifiers.is_empty() {
                println!("\n  🔖 标识符:");
                for (i, identifier) in identifiers.iter().enumerate() {
                    let mut id_info = format!("    {}. {}", i + 1, identifier.value);
                    if let Some(scheme) = &identifier.scheme {
                        id_info.push_str(&format!(" ({})", scheme));
                    }
                    if let Some(id) = &identifier.id {
                        id_info.push_str(&format!(" [ID: {}]", id));
                    }
                    println!("{}", id_info);
                }
            }
            
            // 主题信息
            let subjects = opf.metadata.subjects();
            if !subjects.is_empty() {
                println!("\n  🏷️  主题:");
                for (i, subject) in subjects.iter().enumerate() {
                    println!("    {}. {}", i + 1, subject);
                }
            }
            
            // 其他信息
            if let Some(rights) = opf.metadata.rights() {
                println!("\n  ⚖️  版权: {}", rights);
            }
            
            if let Some(cover) = opf.metadata.cover() {
                println!("  🖼️  封面: {}", cover);
            }
            
            if let Some(modified) = opf.metadata.modified() {
                println!("  🕐 最后修改: {}", modified);
            }
            
            // 贡献者
            let contributors = opf.metadata.contributors();
            if !contributors.is_empty() {
                println!("\n  👥 贡献者:");
                for (i, contributor) in contributors.iter().enumerate() {
                    let mut contrib_info = format!("    {}. {}", i + 1, contributor.name);
                    if let Some(role) = &contributor.role {
                        contrib_info.push_str(&format!(" ({})", role));
                    }
                    println!("{}", contrib_info);
                }
            }
            
            // 自定义元数据
            let custom = opf.metadata.custom();
            if !custom.is_empty() {
                println!("\n  ⚙️  其他元数据:");
                for (key, value) in custom.iter() {
                    println!("    {}: {}", key, value);
                }
            }
            
            // 文件统计
            println!("\n  📁 文件统计:");
            println!("    清单项目: {} 个", opf.manifest.len());
            println!("    脊柱项目: {} 个", opf.spine.len());
            if let Some(nav_path) = opf.get_nav_path() {
                println!("    导航文档: {}", nav_path);
            }
            if let Some(cover_path) = opf.get_cover_image_path() {
                println!("    封面图片: {}", cover_path);
            }
            
            let image_paths = opf.get_image_paths();
            if !image_paths.is_empty() {
                println!("    图片文件: {} 个", image_paths.len());
            }
            
            let css_paths = opf.get_css_paths();
            if !css_paths.is_empty() {
                println!("    样式文件: {} 个", css_paths.len());
            }
            
            // 元数据类型统计
            let (dublin_core_count, name_based_count, property_based_count) = 
                opf.metadata.get_metadata_stats();
            println!("\n  📈 元数据统计:");
            println!("    Dublin Core标签: {} 个", dublin_core_count);
            println!("    Name-based Meta标签: {} 个", name_based_count);
            println!("    Property-based Meta标签: {} 个", property_based_count);
        }
        Err(e) => {
            println!("  ❌ 无法解析OPF文件: {}", e);
        }
    }
    
    Ok(())
}

/// 显示NCX导航信息
fn display_ncx(epub: &mut Epub, verbose: bool) -> Result<()> {
    use bookforge::Ncx;
    
    println!("\n🧭 NCX导航信息:");
    
    // 首先获取NCX文件路径
    let ncx_path = match get_ncx_path(epub) {
        Ok(path) => path,
        Err(e) => {
            println!("  ❌ 无法找到NCX文件: {}", e);
            return Ok(());
        }
    };
    
    // 提取NCX文件内容
    let ncx_content = epub.extract_file(&ncx_path)?;
    
    // 解析NCX文件
    match Ncx::parse_xml(&ncx_content) {
        Ok(ncx) => {
            println!("  📖 NCX版本: {}", ncx.version);
            if let Some(lang) = &ncx.xml_lang {
                println!("  🌐 语言: {}", lang);
            }
            
            // NCX元数据信息
            println!("\n  📊 NCX元数据:");
            if let Some(uid) = ncx.get_uid() {
                println!("    唯一标识符: {}", uid);
            }
            println!("    导航深度: {}", ncx.get_depth());
            
            if let Some(total_pages) = ncx.metadata.total_page_count {
                println!("    总页数: {}", total_pages);
            }
            
            if let Some(max_page) = ncx.metadata.max_page_number {
                println!("    最大页码: {}", max_page);
            }
            
            // 文档标题
            if let Some(title) = ncx.get_title() {
                println!("    文档标题: {}", title);
            }
            
            // 导航地图信息
            let nav_points = ncx.get_all_nav_points();
            println!("\n  🗺️  导航地图:");
            println!("    导航点总数: {}", nav_points.len());
            
            if verbose && !nav_points.is_empty() {
                println!("    导航点详情:");
                for (i, nav_point) in nav_points.iter().enumerate() {
                    println!("      {}. {} -> {}", 
                        i + 1, 
                        nav_point.nav_label.text, 
                        nav_point.content.src
                    );
                    if let Some(class) = &nav_point.class {
                        println!("         [类别: {}]", class);
                    }
                    println!("         [播放顺序: {}]", nav_point.play_order);
                }
            }
            
            // 页面列表信息
            if ncx.has_page_list() {
                if let Some(page_list) = ncx.get_page_list() {
                    println!("\n  📄 页面列表:");
                    println!("    页面目标数: {}", page_list.page_targets.len());
                    
                    if verbose && !page_list.page_targets.is_empty() {
                        println!("    页面详情:");
                        for (i, page_target) in page_list.page_targets.iter().enumerate() {
                            println!("      {}. {} ({}) -> {}", 
                                i + 1,
                                page_target.nav_label.text,
                                page_target.page_type,
                                page_target.content.src
                            );
                            println!("         [页面值: {}, 播放顺序: {}]", 
                                page_target.value, 
                                page_target.play_order
                            );
                        }
                    }
                }
            }
            
            // 章节路径
            let chapter_paths = ncx.get_chapter_paths();
            if !chapter_paths.is_empty() {
                println!("\n  📚 章节文件:");
                println!("    章节文件数: {}", chapter_paths.len());
                
                if verbose {
                    for (i, path) in chapter_paths.iter().enumerate() {
                        println!("      {}. {}", i + 1, path);
                    }
                }
            }
            
            // 显示目录树
            println!("\n  🌳 目录树:");
            
            // 创建目录树对象来获取统计信息
            let mut toc_tree = ncx.create_toc_tree();
            let stats = toc_tree.get_statistics();
            println!("    总章节数: {}, 最大深度: {}, 页面列表: {}", 
                stats.total_nodes, 
                stats.max_depth, 
                if ncx.has_page_list() { "是" } else { "否" }
            );
            
            // 根据详细程度显示不同的目录树
            if verbose {
                // 详细模式：显示完整目录树，包含文件路径
                toc_tree = toc_tree.with_show_paths(true);
                println!("\n{}", toc_tree);
            } else {
                // 简洁模式：限制深度为3，不显示文件路径
                toc_tree = toc_tree
                    .with_show_paths(false)
                    .with_max_depth(Some(3));
                println!("\n{}", toc_tree);
            }
            
            // 其他元数据
            if !ncx.metadata.other_metadata.is_empty() {
                println!("\n  ⚙️  其他元数据:");
                for (key, value) in &ncx.metadata.other_metadata {
                    println!("    {}: {}", key, value);
                }
            }
        }
        Err(e) => {
            println!("  ❌ 无法解析NCX文件: {}", e);
        }
    }
    
    Ok(())
}

/// 获取NCX文件路径
fn get_ncx_path(epub: &mut Epub) -> Result<String> {
    // 首先尝试从OPF文件中获取NCX路径
    match epub.parse_opf() {
        Ok(opf) => {
            // 在manifest中查找NCX文件
            for item in opf.manifest.values() {
                if item.media_type == "application/x-dtbncx+xml" {
                    // 需要考虑OPF文件的相对路径
                    let opf_path = epub.get_opf_path()?;
                    let opf_dir = if let Some(pos) = opf_path.rfind('/') {
                        &opf_path[..pos]
                    } else {
                        ""
                    };
                    
                    return Ok(if opf_dir.is_empty() {
                        item.href.clone()
                    } else {
                        format!("{}/{}", opf_dir, item.href)
                    });
                }
            }
            Err(bookforge::EpubError::NcxParseError("在OPF manifest中未找到NCX文件".to_string()))
        }
        Err(_) => {
            // 如果无法解析OPF，尝试常见的NCX文件路径
            let common_paths = vec![
                "OEBPS/toc.ncx",
                "EPUB/toc.ncx", 
                "toc.ncx",
                "content/toc.ncx",
            ];
            
            let files = epub.list_files()?;
            for path in common_paths {
                if files.contains(&path.to_string()) {
                    return Ok(path.to_string());
                }
            }
            
            // 最后尝试在所有文件中寻找.ncx扩展名的文件
            for file in files {
                if file.ends_with(".ncx") {
                    return Ok(file);
                }
            }
            
            Err(bookforge::EpubError::NcxParseError("未找到NCX文件".to_string()))
        }
    }
}

/// 专门显示目录树的函数
fn display_table_of_contents(epub: &mut Epub, verbose: bool) -> Result<()> {
    use bookforge::{Ncx, epub::ncx::TocTreeStyle};
    
    println!("\n🌳 目录树:");
    
    // 首先获取NCX文件路径
    let ncx_path = match get_ncx_path(epub) {
        Ok(path) => path,
        Err(e) => {
            println!("  ❌ 无法找到NCX文件，无法生成目录树: {}", e);
            return Ok(());
        }
    };
    
    // 提取NCX文件内容
    let ncx_content = epub.extract_file(&ncx_path)?;
    
    // 解析NCX文件
    match Ncx::parse_xml(&ncx_content) {
        Ok(ncx) => {
            // 创建目录树对象
            let mut toc_tree = ncx.create_toc_tree()
                .with_style(TocTreeStyle::TreeSymbols);
            
            // 根据详细程度设置显示选项
            if verbose {
                // 详细模式：显示文件路径
                toc_tree = toc_tree.with_show_paths(true);
            } else {  
                // 简洁模式：不显示文件路径，限制深度为3
                toc_tree = toc_tree
                    .with_show_paths(false)
                    .with_max_depth(Some(3));
            }
            
            // 显示统计信息
            let stats = toc_tree.get_statistics();
            println!("  📊 {}", stats);
            
            // 显示目录树
            println!("\n{}", toc_tree);
            
            if verbose {
                // 额外显示章节标题列表
                let titles = toc_tree.get_all_titles();
                if !titles.is_empty() {
                    println!("  📚 章节标题列表:");
                    for (i, title) in titles.iter().enumerate() {
                        println!("    {}. {}", i + 1, title);
                    }
                }
            }
        }
        Err(e) => {
            println!("  ❌ 无法解析NCX文件生成目录树: {}", e);
        }
    }
    
    Ok(())
}

/// 显示指定章节的内容
fn display_chapter_content(
    epub: &mut Epub, 
    chapter_index: usize, 
    format: ContentFormat, 
    max_length: usize
) -> Result<()> {
    use bookforge::{Ncx, epub::ncx::toc_tree::create_toc_tree_from_ncx};
    
    println!("\n📖 章节内容:");
    
    // 获取NCX文件路径并解析
    let ncx_path = match get_ncx_path(epub) {
        Ok(path) => path,
        Err(e) => {
            println!("  ❌ 无法找到NCX文件: {}", e);
            return Ok(());
        }
    };
    
    let ncx_content = epub.extract_file(&ncx_path)?;
    let ncx = match Ncx::parse_xml(&ncx_content) {
        Ok(ncx) => ncx,
        Err(e) => {
            println!("  ❌ 无法解析NCX文件: {}", e);
            return Ok(());
        }
    };
    
    // 创建目录树
    let toc_tree = create_toc_tree_from_ncx(&ncx);
    
    // 获取所有节点的平铺列表
    let mut all_nodes = Vec::new();
    for root in &toc_tree.roots {
        collect_all_nodes(root, &mut all_nodes);
    }
    
    // 检查章节索引是否有效（用户输入从1开始）
    if chapter_index == 0 || chapter_index > all_nodes.len() {
        println!("  ❌ 无效的章节索引: {}。可用范围: 1-{}", chapter_index, all_nodes.len());
        
        // 显示可用章节列表
        println!("  📚 可用章节列表:");
        for (i, node) in all_nodes.iter().enumerate() {
            println!("    {}. {}", i + 1, node.title);
        }
        return Ok(());
    }
    
    // 获取指定章节（索引减1，因为用户输入从1开始）
    let selected_node = &all_nodes[chapter_index - 1];
    
    println!("  📄 章节 {}: {}", chapter_index, selected_node.title);
    println!("  🆔 节点ID: {}", selected_node.id);
    println!("  📁 源文件: {}", selected_node.src);
    println!("  🎯 播放顺序: {}", selected_node.play_order);
    println!("  📊 显示格式: {:?}", format);
    
    // 根据格式获取章节内容
    let content = match format {
        ContentFormat::Html => {
            match selected_node.get_html_content(epub) {
                Ok(html) => html,
                Err(e) => {
                    println!("  ❌ 无法获取HTML内容: {}", e);
                    return Ok(());
                }
            }
        }
        ContentFormat::Text => {
            match selected_node.get_text_content(epub) {
                Ok(text) => text,
                Err(e) => {
                    println!("  ❌ 无法获取纯文本内容: {}", e);
                    return Ok(());
                }
            }
        }
        ContentFormat::Formatted => {
            match selected_node.get_formatted_text_content(epub) {
                Ok(text) => text,
                Err(e) => {
                    println!("  ❌ 无法获取格式化文本内容: {}", e);
                    return Ok(());
                }
            }
        }
    };
    
    // 显示内容长度信息
    println!("  📏 内容长度: {} 字符", content.chars().count());
    
    // 根据最大长度限制显示内容
    let display_content = if max_length > 0 && content.chars().count() > max_length {
        let truncated: String = content.chars().take(max_length).collect();
        println!("  ✂️  内容已截断到 {} 字符", max_length);
        truncated
    } else {
        content.clone()
    };
    
    println!("\n  📝 章节内容:");
    println!("{}━{}━{}━{}━{}━{}━{}━{}━{}━{}━", "━", "━", "━", "━", "━", "━", "━", "━", "━", "━");
    println!("{}", display_content);
    println!("{}━{}━{}━{}━{}━{}━{}━{}━{}━{}━", "━", "━", "━", "━", "━", "━", "━", "━", "━", "━");
    
    // 如果内容被截断，提供提示
    if max_length > 0 && content.chars().count() > max_length {
        let remaining = content.chars().count() - max_length;
        println!("  💡 提示: 还有 {} 个字符未显示。使用 --max-length 0 显示完整内容。", remaining);
    }
    
    Ok(())
}

/// 递归收集所有节点到平铺列表中
fn collect_all_nodes<'a>(node: &'a bookforge::epub::ncx::toc_tree::TocTreeNode, nodes: &mut Vec<&'a bookforge::epub::ncx::toc_tree::TocTreeNode>) {
    nodes.push(node);
    for child in &node.children {
        collect_all_nodes(child, nodes);
    }
}
