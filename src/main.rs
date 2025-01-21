use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

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

fn pack_files(dir_path: &str) -> Result<()> {
    let abs_path = fs::canonicalize(dir_path)?;
    let mut all_content = String::new();
    let mut file_list = Vec::new();

    // 收集所有文本文件
    for entry in WalkDir::new(&abs_path)
        .follow_links(true)
        .into_iter()
        .filter_entry(|e| !should_ignore_path(e.path())) {
            
        let entry = entry?;
        if entry.file_type().is_file() {
            let path = entry.path();
            if is_text_file(path) {
                let rel_path = path.strip_prefix(&abs_path)?.to_string_lossy().to_string();
                file_list.push(path.to_path_buf());
                
                let mut content = String::new();
                File::open(path)?.read_to_string(&mut content)?;
                
                all_content.push_str(&format!("==========  {}  ==========\n\n", rel_path));
                all_content.push_str(&content);
                all_content.push_str("\n\n");
            }
        }
    }

    fs::write("all_content.txt", all_content)?;
    println!("文件已打包到 all_content.txt");
    Ok(())
}

fn revert_files(input_path: &str) -> Result<()> {
    let content = fs::read_to_string(input_path)
        .with_context(|| format!("无法读取文件 {}", input_path))?;

    let mut current_file = String::new();
    let mut current_content = String::new();
    let mut is_header = true;

    for line in content.lines() {
        if line.starts_with("==========") && line.ends_with("==========") {
            // 保存前一个文件
            if !current_file.is_empty() && !current_content.is_empty() {
                save_file(&current_file, &current_content)?;
            }

            // 提取新文件名
            current_file = line
                .trim_matches('=')
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

fn should_ignore_path(path: &Path) -> bool {
    let path_str = path.to_string_lossy();
    
    // 忽略特定目录
    if path_str.contains("/target/") || 
       path_str.contains("/node_modules/") {
        return true;
    }

    // 获取文件名
    if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
        // 忽略特定文件
        if file_name == "all_content.txt" || 
           file_name.ends_with(".lock") {
            return true;
        }
    }

    false
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
    if let Ok(mut file) = File::open(path) {
        let mut buffer = [0; 512];
        if let Ok(read_count) = file.read(&mut buffer) {
            // 检查前512字节是否包含空字节（二进制文件通常包含空字节）
            for &byte in &buffer[..read_count] {
                if byte == 0 {
                    return false;
                }
            }
            return true;
        }
    }
    false
} 