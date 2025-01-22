use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use glob::glob;
use serde_json;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// 目录路径，用于打包文件
    #[arg(default_value = ".")]
    path: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// 还原文件
    Revert {
        /// 输入文件路径
        input: String,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Revert { input }) => revert_files(&input),
        None => {
            let path = cli.path.unwrap_or_else(|| ".".to_string());
            pack_files(&path)
        }
    }
}

fn should_ignore_path(path: &Path) -> bool {
    let path_str = path.to_string_lossy();
    
    // 检查是否包含需要忽略的目录
    if path_str.contains("/.git/") || 
       path_str.contains("/target/") || 
       path_str.contains("/node_modules/") {
        eprintln!("忽略路径: {}", path.display());
        return true;
    }

    // 检查是否是需要忽略的文件
    if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
        if file_name == "all_content.md" || file_name.ends_with(".lock") {
            return true;
        }
    }

    false
}

fn load_extension_map() -> Result<HashMap<String, String>> {
    let map_content = r#"{
        "rs": "rust",
        "json": "json",
        "js": "javascript",
        "ts": "typescript",
        "py": "python",
        "java": "java",
        "cpp": "cpp",
        "c": "c",
        "go": "go",
        "rb": "ruby",
        "php": "php",
        "html": "html",
        "css": "css",
        "md": "markdown",
        "yaml": "yaml",
        "yml": "yaml",
        "toml": "toml",
        "sh": "bash",
        "bash": "bash",
        "sql": "sql",
        "vue": "vue",
        "jsx": "jsx",
        "tsx": "tsx",
        "lua": "lua",
        ".h": "c/c++ header",
        ".conf": "conf",
        ".ini": "ini",
        ".txt": "text",
        ".bat": "batch file",
        ".ps1": "powershell",
        ".env": "env",
        ".gitignore": "gitignore"
    }"#;
    
    let map: HashMap<String, String> = serde_json::from_str(map_content)?;
    Ok(map)
}

fn escape_markdown_content(content: &str, is_markdown: bool) -> String {
    if !is_markdown {
        return content.to_string();
    }

    content.lines()
        .map(|line| {
            if line.starts_with("```") {
                format!("\\{}", line)
            } else if line.starts_with('#') {
                format!("\\{}", line)
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<String>>()
        .join("\n")
}

fn collect_files(dir_path: &Path) -> Result<Vec<PathBuf>> {
    let pattern = format!("{}/**/*", dir_path.display());
    let mut files = Vec::new();
    
    for entry in glob(&pattern)? {
        if let Ok(path) = entry {
            if path.is_file() && !should_ignore_path(&path) && is_text_file(&path) {
                files.push(path);
            }
        }
    }
    
    Ok(files)
}

fn pack_files(dir_path: &str) -> Result<()> {
    let extension_map = load_extension_map()?;
    let abs_path = fs::canonicalize(dir_path)?;
    let mut all_content = String::new();
    
    // 先收集所有符合条件的文件
    let files = collect_files(&abs_path)?;
    
    if files.is_empty() {
        println!("没有找到任何有效的文本文件");
        return Ok(());
    }

    // 处理每个文件
    for path in files {
        let rel_path = path.strip_prefix(&abs_path)?.to_string_lossy().to_string();
        
        // 检查是否是 markdown 文件
        let is_markdown = path.extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
            .map(|ext| ext == "md")
            .unwrap_or(false);
        
        // 读取并处理文件内容
        let content = process_file(&path, &rel_path, &extension_map, is_markdown)?;
        all_content.push_str(&content);
    }

    fs::write("all_content.md", all_content)?;
    println!("文件已打包到 all_content.md");
    Ok(())
}

fn process_file(path: &Path, rel_path: &str, extension_map: &HashMap<String, String>, is_markdown: bool) -> Result<String> {
    let bytes = fs::read(path)?;
    let content = String::from_utf8(bytes)
        .with_context(|| format!("文件 {} 不是有效的 UTF-8 编码", rel_path))?;
    
    let mut result = String::new();
    
    // 添加文件头
    result.push_str(&format!("###  trxx:{}\n\n", rel_path));
    result.push_str("\n\n");
    
    // 添加语言标识符
    if let Some(ext) = path.extension().and_then(|e| e.to_str()).map(|e| e.to_lowercase()) {
        if let Some(lang) = extension_map.get(&ext) {
            result.push_str(&format!("```{}", lang));
        } else {
            result.push_str("```");
        }
    } else {
        result.push_str("```");
    }
    result.push_str("\n\n");
    
    // 处理内容
    let processed_content = escape_markdown_content(&content, is_markdown);
    result.push_str(&processed_content);
    result.push_str("\n\n");
    result.push_str("```");
    result.push_str("\n\n");
    
    Ok(result)
}

fn unescape_markdown_content(line: &str, is_markdown: bool) -> String {
    if !is_markdown {
        return line.to_string();
    }

    if line.starts_with("\\```") {
        line.trim_start_matches('\\').to_string()
    } else if line.starts_with("\\#") {
        line.trim_start_matches('\\').to_string()
    } else {
        line.to_string()
    }
}

fn revert_files(input_path: &str) -> Result<()> {
    let content = fs::read_to_string(input_path)
        .with_context(|| format!("无法读取文件 {}", input_path))?;

    let mut current_file = String::new();
    let mut current_content = String::new();
    let mut is_header = true;
    let mut in_code_block = false;
    let mut is_markdown = false;

    for line in content.lines() {
        if line.starts_with("###  trxx:") {
            // 保存前一个文件
            if !current_file.is_empty() && !current_content.is_empty() {
                // 去除开头和结尾的空行
                let trimmed_content = current_content.trim_matches('\n');
                save_file(&current_file, trimmed_content)?;
            }

            // 提取新文件名
            current_file = line
                .trim_start_matches("###  trxx:")
                .trim()
                .to_string();
                
            // 检查是否是 markdown 文件
            is_markdown = current_file.ends_with(".md");
            current_content = String::new();
            is_header = true;
            in_code_block = false;
        } else if !is_header {
            // 跳过代码块标记
            if line.starts_with("```") {
                in_code_block = !in_code_block;
                continue;
            }
            
            // 只有在代码块内的内容才添加到 current_content
            if in_code_block {
                // 处理转义字符
                let unescaped_line = unescape_markdown_content(line, is_markdown);
                current_content.push_str(&unescaped_line);
                current_content.push('\n');
            }
        } else if line.is_empty() {
            is_header = false;
        }
    }

    // 保存最后一个文件
    if !current_file.is_empty() && !current_content.is_empty() {
        // 去除开头和结尾的空行
        let trimmed_content = current_content.trim_matches('\n');
        save_file(&current_file, trimmed_content)?;
    }

    println!("文件已还原完成");
    Ok(())
}

fn save_file(file_path: &str, content: &str) -> Result<()> {
    let path = Path::new(file_path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, content)?;
    Ok(())
}

fn is_text_file(path: &Path) -> bool {
    // 如果文件大于 1MB，认为不是文本文件
    if let Ok(metadata) = path.metadata() {
        if metadata.len() > 1024 * 1024 {
            return false;
        }
    }

    let extension = path.extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("");

    // 如果没有扩展名，尝试读取文件开头来判断是否为文本文件
    if extension.is_empty() {
        return is_probably_text(path);
    }

    matches!(extension.to_lowercase().as_str(),
        "txt" | "md" | "rs" | "js" | "ts" | "json" | "yaml" | "yml" 
        | "toml" | "css" | "html" | "htm" | "xml" | "conf" | "cfg"
        | "ini" | "log" | "sh" | "bash" | "py" | "java" | "cpp" | "c"
        | "h" | "hpp" | "cs" | "go" | "rb" | "php" | "sql" | "vue"
        | "jsx" | "tsx" | "gitignore" | "env" | "rc" | "editorconfig"
        | "gradle" | "properties" | "bat" | "cmd" | "ps1" | "dockerfile"
        | "lock" | "config" | "template" | "vim" | "lua")
}

fn is_probably_text(path: &Path) -> bool {
    if let Ok(bytes) = fs::read(path) {
        // 检查文件是否为有效的 UTF-8
        if String::from_utf8(bytes.clone()).is_ok() {
            // 检查前512字节是否包含空字节
            for &byte in bytes.iter().take(512) {
                if byte == 0 {
                    return false;
                }
            }
            return true;
        }
    }
    false
} 