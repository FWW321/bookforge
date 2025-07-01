//! BookForge EPUB 命令行工具
//! 
//! 一个现代化的EPUB文件信息查看器，支持查看书籍信息、章节、封面等功能。

use clap::{Parser, ValueEnum};
use bookforge::{Epub, Result, EpubError};
use std::process;

#[derive(Parser)]
#[command(name = "bookforge")]
#[command(about = "一个现代化的EPUB文件信息查看器")]
#[command(version = bookforge::VERSION)]
struct Args {
    /// EPUB文件路径
    #[arg(help = "要处理的EPUB文件路径")]
    epub_file: String,
    
    /// 显示详细信息
    #[arg(short, long, help = "显示详细信息")]
    verbose: bool,
    
    /// 显示书籍基本信息
    #[arg(short = 'I', long, help = "显示书籍基本信息")]
    info: bool,
    
    /// 显示章节列表
    #[arg(short = 'c', long, help = "显示章节列表")]
    chapters: bool,
    
    /// 显示指定章节内容
    #[arg(short = 'C', long, help = "显示指定章节的内容（使用章节索引，从1开始）")]
    chapter: Option<usize>,
    
    /// 显示封面信息
    #[arg(long, help = "显示封面信息")]
    cover: bool,
    
    /// 显示图片列表
    #[arg(short = 'i', long, help = "显示图片资源列表")]
    images: bool,
    
    /// 列出所有文件
    #[arg(short = 'l', long, help = "列出EPUB中的所有文件")]
    list: bool,
    
    /// 显示目录树
    #[arg(short = 't', long, help = "显示目录树结构")]
    toc: bool,
    
    /// 内容显示格式
    #[arg(long, value_enum, default_value = "summary", help = "章节内容的显示格式")]
    format: ContentFormat,
    
    /// 内容最大显示长度
    #[arg(long, default_value = "1000", help = "章节内容最大显示字符数（0表示不限制）")]
    max_length: usize,
    
    /// 导出所有章节为txt文件
    #[arg(long, help = "将所有章节导出为txt文件")]
    export_txt: bool,
    
    /// 导出特定章节为txt文件
    #[arg(long, help = "导出指定章节为txt文件（使用章节索引，从1开始）")]
    export_chapter: Option<usize>,
    
    /// 导出文件的输出目录
    #[arg(long, help = "txt文件的输出目录（默认为 output/{书籍标题}/）")]
    export_dir: Option<String>,
    
    /// 导出文本格式
    #[arg(long, value_enum, default_value = "formatted", help = "导出的文本格式")]
    export_format: ExportFormat,
    
    /// 创建子目录结构
    #[arg(long, help = "根据目录树结构创建子目录")]
    create_subdirs: bool,
    
    /// 生成索引文件
    #[arg(long, help = "生成包含目录结构的索引文件")]
    with_index: bool,
    
    /// 将所有章节合并为一个txt文件
    #[arg(long, help = "将所有章节合并为一个txt文件，以书籍标题命名")]
    merge_txt: bool,
}

#[derive(ValueEnum, Clone)]
enum ContentFormat {
    /// 仅显示摘要
    Summary,
    /// 完整内容
    Full,
}

#[derive(ValueEnum, Clone)]
enum ExportFormat {
    /// 格式化文本（保持HTML结构）
    Formatted,
    /// 纯文本（移除所有HTML标签）
    Plain,
}

fn main() {
    let args = Args::parse();
    
    if let Err(e) = run(&args) {
        eprintln!("错误: {}", e);
        process::exit(1);
    }
}

fn run(args: &Args) -> Result<()> {
    println!("🔍 正在分析EPUB文件: {}", args.epub_file);
    
    // 打开EPUB文件
    let epub = Epub::from_path(&args.epub_file)?;
    
    // 如果没有指定任何选项，显示基本信息
    if !args.info && !args.chapters && args.chapter.is_none() && !args.cover && !args.images && !args.list && !args.toc && !args.export_txt && args.export_chapter.is_none() && !args.merge_txt {
        display_basic_info(&epub)?;
        return Ok(());
    }
    
    // 显示书籍信息
    if args.info {
        display_book_info(&epub, args.verbose)?;
    }
    
    // 显示章节列表
    if args.chapters {
        display_chapters(&epub, args.verbose)?;
    }
    
    // 显示指定章节内容
    if let Some(index) = args.chapter {
        display_chapter_content(&epub, index, &args.format, args.max_length)?;
    }
    
    // 显示封面信息
    if args.cover {
        display_cover_info(&epub)?;
    }
    
    // 显示图片列表
    if args.images {
        display_images(&epub, args.verbose)?;
    }
    
    // 列出所有文件
    if args.list {
        display_file_list(&epub, args.verbose)?;
    }
    
    // 显示目录树
    if args.toc {
        display_toc_tree(&epub, args.verbose)?;
    }
    
    // 导出所有章节为txt文件
    if args.export_txt {
        export_all_chapters(&epub, args)?;
    }
    
    // 导出特定章节为txt文件
    if let Some(index) = args.export_chapter {
        export_single_chapter(&epub, index, args)?;
    }
    
    // 合并所有章节为一个txt文件
    if args.merge_txt {
        merge_all_chapters(&epub, args)?;
    }
    
    Ok(())
}

/// 显示基本信息
fn display_basic_info(epub: &Epub) -> Result<()> {
    let info = epub.book_info()?;
    
    println!("\n📚 书籍信息:");
    println!("  标题: {}", info.title);
    
    if !info.authors.is_empty() {
        println!("  作者: {}", info.authors.join(", "));
    }
    
    if let Some(language) = &info.language {
        println!("  语言: {}", language);
    }
    
    if let Some(publisher) = &info.publisher {
        println!("  出版社: {}", publisher);
    }
    
    // 显示章节数量
    let chapters = epub.chapter_list()?;
    println!("  章节数: {}", chapters.len());
    
    // 显示文件数量
    let files = epub.file_list()?;
    println!("  文件数: {}", files.len());
    
    println!("\n💡 使用 --help 查看更多选项");
    
    Ok(())
}

/// 显示详细书籍信息
fn display_book_info(epub: &Epub, verbose: bool) -> Result<()> {
    let info = epub.book_info()?;
    
    println!("\n📚 详细书籍信息:");
    println!("  标题: {}", info.title);
    
    if !info.authors.is_empty() {
        println!("  作者: {}", info.authors.join(", "));
    }
    
    if let Some(language) = &info.language {
        println!("  语言: {}", language);
    }
    
    if let Some(publisher) = &info.publisher {
        println!("  出版社: {}", publisher);
    }
    
    if let Some(isbn) = &info.isbn {
        println!("  ISBN: {}", isbn);
    }
    
    if let Some(description) = &info.description {
        if verbose {
            println!("  描述: {}", description);
        } else {
            let truncated = if description.len() > 200 {
                format!("{}...", &description[..200])
            } else {
                description.clone()
            };
            println!("  描述: {}", truncated);
        }
    }
    
    // 显示组件信息
    if verbose {
        println!("\n🔧 技术信息:");
        
        // 检查是否有NCX和目录树
        if epub.has_ncx()? {
            println!("  ✅ 包含NCX导航文件");
            if epub.has_toc_tree()? {
                println!("  ✅ 支持目录树结构");
            }
        } else {
            println!("  ❌ 不包含NCX导航文件");
            println!("  ❌ 不支持目录树结构");
        }
        
        // 获取OPF信息
        let opf = epub.opf()?;
        println!("  OPF版本: {}", opf.version);
        println!("  清单项目: {}", opf.manifest.len());
        println!("  脊柱项目: {}", opf.spine.len());
    }
    
    // 显示作者信息
    let metadata = &epub.opf()?.metadata;
    println!("\n作者：");
    for (i, creator) in metadata.creators().iter().enumerate() {
        println!("  {}. {}", i + 1, creator.name);
        if let Some(role) = &creator.role {
            println!("     角色：{}", role);
        }
        if let Some(display_seq) = creator.display_seq {
            println!("     显示顺序：{}", display_seq);
        }
        if let Some(id) = &creator.id {
            println!("     ID：{}", id);
        }
    }

    // 显示贡献者信息（如果有）
    let contributors = metadata.contributors();
    if !contributors.is_empty() {
        println!("\n贡献者：");
        for (i, contributor) in contributors.iter().enumerate() {
            println!("  {}. {}", i + 1, contributor.name);
            if let Some(role) = &contributor.role {
                println!("     角色：{}", role);
            }
            if let Some(display_seq) = contributor.display_seq {
                println!("     显示顺序：{}", display_seq);
            }
        }
    }

    // 显示元数据统计信息
    let (dublin_core, name_based, property_based, refines_based) = metadata.get_metadata_stats();
    println!("\n元数据统计：");
    println!("  Dublin Core元数据：{} 个", dublin_core);
    println!("  基于name的meta标签：{} 个", name_based);
    println!("  基于property的meta标签：{} 个", property_based);
    println!("  基于refines的meta标签：{} 个", refines_based);

    // 如果有refines元数据，显示详细信息
    if refines_based > 0 {
        println!("\nEPUB3 Refines元数据详情：");
        let refines_data = metadata.get_refines_based_meta();
        for (refines_id, property, content, scheme) in refines_data {
            println!("  ID: {} -> 属性: {} = {}", refines_id, property, content);
            if let Some(scheme_val) = scheme {
                println!("    方案: {}", scheme_val);
            }
        }
    }
    
    Ok(())
}

/// 显示章节列表
fn display_chapters(epub: &Epub, verbose: bool) -> Result<()> {
    let chapters = epub.chapter_list()?;
    
    println!("\n📖 章节列表 (共{}章):", chapters.len());
    
    for (i, chapter) in chapters.iter().enumerate() {
        if verbose {
            println!("  {}. {} (ID: {}, 路径: {})", 
                i + 1, chapter.title, chapter.id, chapter.path);
        } else {
            println!("  {}. {}", i + 1, chapter.title);
        }
    }
    
    Ok(())
}

/// 显示章节内容
fn display_chapter_content(epub: &Epub, index: usize, format: &ContentFormat, max_length: usize) -> Result<()> {
    let chapters = epub.chapter_list()?;
    
    if index == 0 || index > chapters.len() {
        return Err(EpubError::InvalidEpub(format!(
            "章节索引无效。请使用1-{}之间的数字", chapters.len()
        )));
    }
    
    let chapter_info = &chapters[index - 1];
    let chapter = epub.chapter(chapter_info)?;
    
    println!("\n📄 章节 {}: {}", index, chapter.info.title);
    println!("文件路径: {}", chapter.info.path);
    println!("内容长度: {} 字符", chapter.content.len());
    
    match format {
        ContentFormat::Summary => {
            let content_preview = if chapter.content.len() > max_length && max_length > 0 {
                format!("{}...", &chapter.content[..max_length])
            } else {
                chapter.content.clone()
            };
            
            // 简单的HTML标签移除
            let text_content = strip_html_basic(&content_preview);
            println!("\n内容预览:");
            println!("{}", text_content);
        }
        ContentFormat::Full => {
            println!("\n完整内容:");
            println!("{}", chapter.content);
        }
    }
    
    Ok(())
}

/// 显示封面信息
fn display_cover_info(epub: &Epub) -> Result<()> {
    match epub.cover()? {
        Some(cover) => {
            println!("\n🖼️  封面信息:");
            println!("  文件名: {}", cover.filename);
            println!("  格式: {}", cover.format);
            println!("  大小: {} 字节", cover.data.len());
        }
        None => {
            println!("\n❌ 没有找到封面图片");
        }
    }
    
    Ok(())
}

/// 显示图片列表
fn display_images(epub: &Epub, verbose: bool) -> Result<()> {
    let images = epub.images()?;
    
    if images.is_empty() {
        println!("\n❌ 没有找到图片文件");
        return Ok(());
    }
    
    println!("\n🖼️  图片列表 (共{}张):", images.len());
    
    for (i, image) in images.iter().enumerate() {
        if verbose {
            println!("  {}. {} (类型: {}, ID: {})", 
                i + 1, image.path, image.media_type, image.id);
        } else {
            println!("  {}. {}", i + 1, image.path);
        }
    }
    
    Ok(())
}

/// 显示文件列表
fn display_file_list(epub: &Epub, verbose: bool) -> Result<()> {
    let files = epub.file_list()?;
    
    println!("\n📁 文件列表 (共{}个文件):", files.len());
    
    if verbose {
        for (i, file) in files.iter().enumerate() {
            println!("  {}. {}", i + 1, file);
        }
    } else {
        // 按类型分组显示
        let mut html_files = Vec::new();
        let mut css_files = Vec::new();
        let mut image_files = Vec::new();
        let mut other_files = Vec::new();
        
        for file in &files {
            let lower = file.to_lowercase();
            if lower.ends_with(".html") || lower.ends_with(".xhtml") || lower.ends_with(".htm") {
                html_files.push(file);
            } else if lower.ends_with(".css") {
                css_files.push(file);
            } else if lower.ends_with(".jpg") || lower.ends_with(".jpeg") || 
                     lower.ends_with(".png") || lower.ends_with(".gif") || 
                     lower.ends_with(".svg") || lower.ends_with(".webp") {
                image_files.push(file);
            } else {
                other_files.push(file);
            }
        }
        
        if !html_files.is_empty() {
            println!("  📄 HTML文件: {} 个", html_files.len());
        }
        if !css_files.is_empty() {
            println!("  🎨 CSS文件: {} 个", css_files.len());
        }
        if !image_files.is_empty() {
            println!("  🖼️  图片文件: {} 个", image_files.len());
        }
        if !other_files.is_empty() {
            println!("  📦 其他文件: {} 个", other_files.len());
        }
    }
    
    Ok(())
}

/// 显示目录树
fn display_toc_tree(epub: &Epub, verbose: bool) -> Result<()> {
    println!("\n🌳 目录树结构:");
    
    // 检查是否有目录树
    if !epub.has_toc_tree()? {
        println!("  ❌ 此EPUB文件不包含目录树信息");
        println!("  💡 提示: EPUB文件需要包含NCX文件才能显示目录树");
        return Ok(());
    }
    
    // 获取目录树
    match epub.toc_tree()? {
        Some(toc_tree) => {
            if verbose {
                // 显示统计信息
                let stats = toc_tree.get_statistics();
                println!("  📊 统计信息:");
                println!("    总节点数: {}", stats.total_nodes);
                println!("    最大深度: {}", stats.max_depth);
                println!("    叶子节点数: {}", stats.leaf_count);
                println!("    根节点数: {}", stats.root_count);
                println!();
            }
            
            // 显示目录树结构
            println!("  📖 目录结构:");
            if let Some(title) = &toc_tree.title {
                println!("    书名: {}", title);
            }
            
            // 打印目录树
            let tree_output = format!("{}", toc_tree);
            for line in tree_output.lines() {
                println!("    {}", line);
            }
            
            if verbose {
                println!("\n  🔗 所有路径:");
                let paths = toc_tree.get_all_paths();
                for (index, path) in paths.iter().enumerate() {
                    println!("    {}: {}", index + 1, path);
                }
            }
        }
        None => {
            println!("  ⚠️  目录树信息不可用");
        }
    }
    
    Ok(())
}

/// 导出所有章节为txt文件
fn export_all_chapters(epub: &Epub, args: &Args) -> Result<()> {
    println!("\n📁 开始导出所有章节为txt文件...");
    
    // 检查是否有目录树
    if !epub.has_toc_tree()? {
        println!("❌ 此EPUB文件不包含目录树信息，无法导出章节");
        println!("💡 提示: EPUB文件需要包含NCX文件才能导出章节");
        return Ok(());
    }
    
    // 获取目录树
    let toc_tree = match epub.toc_tree()? {
        Some(tree) => tree,
        None => {
            println!("❌ 无法获取目录树信息");
            return Ok(());
        }
    };
    
    let output_path = get_export_directory(epub, &args.export_dir)?;
    let output_dir = output_path.as_path();
    let use_formatted_text = matches!(args.export_format, ExportFormat::Formatted);
    
    println!("📂 导出目录: {}", output_dir.display());
    println!("📄 文本格式: {}", if use_formatted_text { "格式化文本" } else { "纯文本" });
    println!("📁 创建子目录: {}", if args.create_subdirs { "是" } else { "否" });
    println!("📋 生成索引: {}", if args.with_index { "是" } else { "否" });
    
    let result = if args.with_index {
        // 生成txt文件并创建索引
        toc_tree.generate_all_txt_files_with_index(
            Some(output_dir),
            use_formatted_text,
            args.create_subdirs,
            Some("目录索引.txt"),
        )?
    } else {
        // 只生成txt文件
        let file_paths = toc_tree.generate_all_txt_files(
            Some(output_dir),
            use_formatted_text,
            args.create_subdirs,
        )?;
        (file_paths, output_dir.join("unused"))
    };
    
    let (file_paths, index_path) = result;
    
    println!("\n✅ 导出完成!");
    println!("📊 生成文件数: {}", file_paths.len());
    
    if args.with_index && index_path.exists() {
        println!("📋 索引文件: {:?}", index_path);
    }
    
    if args.verbose {
        println!("\n📁 生成的文件:");
        for (i, path) in file_paths.iter().enumerate() {
            let relative_path = path.strip_prefix(output_dir).unwrap_or(path);
            println!("  {}. {}", i + 1, relative_path.display());
        }
    }
    
    Ok(())
}

/// 导出单个章节为txt文件
fn export_single_chapter(epub: &Epub, index: usize, args: &Args) -> Result<()> {
    println!("\n📄 开始导出章节 {} 为txt文件...", index);
    
    // 检查是否有目录树
    if !epub.has_toc_tree()? {
        println!("❌ 此EPUB文件不包含目录树信息，无法导出章节");
        println!("💡 提示: EPUB文件需要包含NCX文件才能导出章节");
        return Ok(());
    }
    
    // 获取目录树
    let toc_tree = match epub.toc_tree()? {
        Some(tree) => tree,
        None => {
            println!("❌ 无法获取目录树信息");
            return Ok(());
        }
    };
    
    // 获取所有章节节点的路径
    let all_node_paths = collect_all_node_paths(&toc_tree);
    
    if index == 0 || index > all_node_paths.len() {
        return Err(EpubError::InvalidEpub(format!(
            "章节索引无效。请使用1-{}之间的数字", all_node_paths.len()
        )));
    }
    
    let node_path = &all_node_paths[index - 1];
    let node = toc_tree.get_node_by_path(node_path).ok_or_else(|| {
        EpubError::InvalidEpub("无法找到指定的章节节点".to_string())
    })?;
    let output_path = get_export_directory(epub, &args.export_dir)?;
    let output_dir = output_path.as_path();
    let use_formatted_text = matches!(args.export_format, ExportFormat::Formatted);
    
    println!("📖 章节标题: {}", node.title);
    println!("📂 导出目录: {}", output_dir.display());
    println!("📄 文本格式: {}", if use_formatted_text { "格式化文本" } else { "纯文本" });
    
    // 生成txt文件
    let file_path = node.generate_txt_file(epub, Some(output_dir), use_formatted_text)?;
    
    println!("\n✅ 导出完成!");
    println!("📁 文件路径: {:?}", file_path);
    
    Ok(())
}

/// 合并所有章节为一个txt文件
fn merge_all_chapters(epub: &Epub, args: &Args) -> Result<()> {
    println!("\n📖 开始合并所有章节为txt文件...");
    
    // 检查是否有目录树
    if !epub.has_toc_tree()? {
        println!("❌ 此EPUB文件不包含目录树信息，无法合并章节");
        println!("💡 提示: EPUB文件需要包含NCX文件才能合并章节");
        return Ok(());
    }
    
    // 获取目录树
    let toc_tree = match epub.toc_tree()? {
        Some(tree) => tree,
        None => {
            println!("❌ 无法获取目录树信息");
            return Ok(());
        }
    };
    
    let output_path = get_export_directory(epub, &args.export_dir)?;
    let output_dir = output_path.as_path();
    let use_formatted_text = matches!(args.export_format, ExportFormat::Formatted);
    
    println!("📂 导出目录: {}", output_dir.display());
    println!("📄 文本格式: {}", if use_formatted_text { "格式化文本" } else { "纯文本" });
    
    // 生成合并的txt文件
    let file_path = toc_tree.generate_merged_txt_file(
        Some(output_dir),
        use_formatted_text,
        None, // 使用默认的书籍标题作为文件名
    )?;
    
    println!("\n✅ 合并完成!");
    println!("📁 文件路径: {:?}", file_path);
    
    // 显示文件大小信息
    if let Ok(metadata) = std::fs::metadata(&file_path) {
        println!("📏 文件大小: {} 字节", metadata.len());
        
        // 转换为更友好的显示单位
        let size_kb = metadata.len() as f64 / 1024.0;
        if size_kb > 1024.0 {
            let size_mb = size_kb / 1024.0;
            println!("              {:.2} MB", size_mb);
        } else {
            println!("              {:.2} KB", size_kb);
        }
    }
    
    Ok(())
}

/// 获取导出目录路径
fn get_export_directory(epub: &Epub, custom_dir: &Option<String>) -> Result<std::path::PathBuf> {
    match custom_dir {
        Some(dir) => Ok(std::path::PathBuf::from(dir)),
        None => {
            // 获取书籍信息以获取标题
            let info = epub.book_info()?;
            
            // 生成安全的目录名
            let safe_title = generate_safe_dirname(&info.title);
            
            // 创建默认路径: output/{书籍标题}/
            let output_path = std::path::PathBuf::from("output").join(safe_title);
            
            Ok(output_path)
        }
    }
}

/// 生成安全的目录名
fn generate_safe_dirname(title: &str) -> String {
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

    // 限制目录名长度
    const MAX_DIRNAME_LENGTH: usize = 100;
    if safe_title.len() > MAX_DIRNAME_LENGTH {
        safe_title.truncate(MAX_DIRNAME_LENGTH);
        // 确保不会在Unicode字符中间截断
        while !safe_title.is_char_boundary(safe_title.len()) {
            safe_title.pop();
        }
    }

    safe_title
}

/// 收集所有节点的路径
fn collect_all_node_paths(toc_tree: &bookforge::epub::ncx::toc_tree::TocTree) -> Vec<Vec<usize>> {
    let mut paths = Vec::new();
    
    for (root_index, root) in toc_tree.roots.iter().enumerate() {
        collect_node_paths_recursive(root, vec![root_index], &mut paths);
    }
    
    paths
}

/// 递归收集节点路径
fn collect_node_paths_recursive(
    node: &bookforge::epub::ncx::toc_tree::TocTreeNode, 
    current_path: Vec<usize>, 
    paths: &mut Vec<Vec<usize>>
) {
    paths.push(current_path.clone());
    
    for (child_index, child) in node.children.iter().enumerate() {
        let mut child_path = current_path.clone();
        child_path.push(child_index);
        collect_node_paths_recursive(child, child_path, paths);
    }
}

/// 简单的HTML标签移除
fn strip_html_basic(html: &str) -> String {
    let mut result = String::new();
    let mut in_tag = false;
    let mut chars = html.chars().peekable();
    
    while let Some(ch) = chars.next() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => {
                result.push(ch);
            }
            _ => {} // 在标签内，忽略字符
        }
    }
    
    // 清理多余的空白字符
    result.split_whitespace().collect::<Vec<&str>>().join(" ")
}
