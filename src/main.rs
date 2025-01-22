use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use std::collections::HashMap;
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

fn should_ignore_dir(path: &Path) -> bool {
    // 检查路径的每一部分
    for ancestor in path.ancestors() {
        if let Some(name) = ancestor.file_name() {
            if let Some(name_str) = name.to_str() {
                if name_str == ".git" || name_str == "target" || name_str == "node_modules" {
                    eprintln!("忽略目录: {}", ancestor.display());
                    return true;
                }
            }
        }
    }
    false
}

fn should_ignore_file(name: &str) -> bool {
    name == "all_content.md" || name.ends_with(".lock")
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

fn pack_files(dir_path: &str) -> Result<()> {
    let extension_map = load_extension_map()?;
    let path = Path::new(dir_path);
    if should_ignore_dir(path) {
        println!("指定的目录被忽略");
        return Ok(());
    }

    let abs_path = fs::canonicalize(path)?;
    let mut all_content = String::new();
    let mut file_list = Vec::new();

    // 收集所有文本文件
    let walker = WalkDir::new(&abs_path)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            let path = e.path();
            
            // 先检查是否是需要忽略的目录
            if should_ignore_dir(path) {
                return false;
            }
            
            // 再检查是否是需要忽略的文件
            if e.file_type().is_file() {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if should_ignore_file(name) {
                        return false;
                    }
                }
            }
            
            true
        });

    for entry in walker {
        let entry = entry?;
        if entry.file_type().is_file() {
            let path = entry.path();
            if is_text_file(path) {
                if let Ok(rel_path) = path.strip_prefix(&abs_path) {
                    let rel_path = rel_path.to_string_lossy().to_string();
                    
                    // 尝试读取文件内容，如果出错则跳过该文件
                    match fs::read(path) {
                        Ok(bytes) => {
                            // 检查文件内容是否为有效的 UTF-8
                            if let Ok(content) = String::from_utf8(bytes) {
                                file_list.push(path.to_path_buf());
                                
                                all_content.push_str(&format!("###  trxx:{}\n\n", rel_path));
                                all_content.push_str("\n\n");
                                
                                // 获取文件扩展名并查找对应的语言标识符
                                if let Some(ext) = Path::new(&rel_path)
                                    .extension()
                                    .and_then(|e| e.to_str())
                                    .map(|e| e.to_lowercase()) {
                                    
                                    if let Some(lang) = extension_map.get(&ext) {
                                        all_content.push_str(&format!("```{}", lang));
                                        all_content.push_str("\n\n");
                                    }
                                }
                                
                                all_content.push_str(&content);
                                all_content.push_str("\n\n");
                                all_content.push_str("```");
                                all_content.push_str("\n\n");
                            } else {
                                eprintln!("警告: 文件 {} 不是有效的 UTF-8 编码，已跳过", rel_path);
                            }
                        }
                        Err(e) => {
                            eprintln!("警告: 读取文件 {} 失败: {}", rel_path, e);
                        }
                    }
                }
            }
        }
    }

    if all_content.is_empty() {
        println!("没有找到任何有效的文本文件");
        return Ok(());
    }

    fs::write("all_content.md", all_content)?;
    println!("文件已打包到 all_content.md");
    Ok(())
}

fn revert_files(input_path: &str) -> Result<()> {
    let content = fs::read_to_string(input_path)
        .with_context(|| format!("无法读取文件 {}", input_path))?;

    let mut current_file = String::new();
    let mut current_content = String::new();
    let mut is_header = true;

    for line in content.lines() {
        if line.starts_with("###  trxx:") {
            // 保存前一个文件
            if !current_file.is_empty() && !current_content.is_empty() {
                save_file(&current_file, &current_content)?;
            }

            // 提取新文件名
            current_file = line
                .trim_start_matches("###  trxx:")
                .trim()
                .to_string();
            current_content = String::new();
            is_header = true;
        } else if !is_header {
            current_content.push_str(line);
            current_content.push('\n');
        } else if line.is_empty() {
            is_header = false;
        }
    }

    // 保存最后一个文件
    if !current_file.is_empty() && !current_content.is_empty() {
        save_file(&current_file, &current_content)?;
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