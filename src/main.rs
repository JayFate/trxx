use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use glob::glob;
use serde_json;
use base64;

// 我来解释一下 #[command(subcommand)] 这个属性标注的含义：

// 在 Rust 中，这是一个用于 clap crate（命令行参数解析库）的属性宏，它有以下作用：

// 1. 它标注在一个字段上（在你的代码中是 command 字段），表示这个字段将用来处理子命令
// 2. 它告诉 clap 这个字段的类型是一个子命令枚举（在你的代码中是 Commands 枚举）

// 在你的代码中具体表现为：

// ```rust
// struct Cli {
//     #[command(subcommand)]
//     command: Option<Commands>,  // 这里的 Commands 是一个枚举类型
//     // ... 其他字段
// }

// #[derive(Subcommand)]
// enum Commands {
//     Revert { input: String },  // 这是一个子命令
// }
// ```

// 这样设计后，你的命令行工具就可以支持类似这样的使用方式：

// ```bash
// your_program revert input.txt   # 使用 revert 子命令
// your_program                    # 不使用子命令的情况
// ```

// 实际效果：

// - 当用户输入 your_program revert input.txt 时，command 字段会包含 Some(Commands::Revert { input: "input.txt" })
// - 当用户直接输入 your_program 时，command 字段的值为 None

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

    // ```
    // Commands::Revert { input } 是什么语法？
    // ```

    // 这是 Rust 中的模式匹配（Pattern Matching）语法，具体是对枚举类型的解构（Destructuring）。让我来详细解释：

    // 1. Commands::Revert { input } 是一个模式，用于匹配和解构枚举变体中的字段
    // 2. 这种语法叫做结构体风格的枚举变体

    // 在你的代码中，Commands 枚举是这样定义的：

    // ```rust
    // #[derive(Subcommand)]
    // enum Commands {
    //     Revert {
    //         input: String,  // 这是一个命名字段
    //     },
    // }
    // ```

    // 当你使用 Commands::Revert { input } 进行模式匹配时：

    // - input 会直接绑定到枚举变体中的 input 字段的值
    // - 这种写法等同于 Commands::Revert { input: input }，因为字段名和变量名相同，可以简写

    // 举个具体例子：

    // ```rust
    // match cli.command {
    //     // 如果是 Revert 命令，解构出 input 字段的值
    //     Some(Commands::Revert { input }) => revert_files(&input),
    //     // input 变量现在包含了用户输入的字符串
        
    //     // 处理其他情况
    //     None => { /* ... */ }
    // }
    // ```

    // 这种语法的好处是：

    // 可读性强，直观地表明我们要使用枚举变体中的哪些字段

    // 可以直接获取到字段的值，无需额外的解构步骤

    // 编译器会确保我们正确地处理了所有必要的字段



    // 什么是 结构体风格的枚举变体 ？



    // 在 Rust 中，枚举可以有三种风格的变体：

    // 1. 单元变体（Unit Variant）：

    // ```rust
    // enum Message {
    //     Quit,  // 没有任何数据的变体
    // }
    // ```

    // 2. 元组变体（Tuple Variant）：

    // ```rust
    // enum Message {
    //     Write(String),  // 包含匿名字段的变体
    //     Point(i32, i32),  // 可以包含多个值
    // }
    // ```

    // 3. 结构体变体（Struct Variant）：

    // ```rust
    // enum Message {
    //     Move { x: i32, y: i32 },  // 带有命名字段的变体，像结构体一样
    //     ChangeColor { r: u8, g: u8, b: u8 },  // 每个字段都有名字
    // }
    // ```

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
        ".gitignore": "gitignore",
        "wxss": "css",
        "wxml": "xml",
        "ux": "html"
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
            if path.is_file() && !should_ignore_path(&path) && should_process_file(&path) {
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
    let mut result = String::new();
    
    // 添加文件头
    result.push_str(&format!("###  trxx:{}\n\n", rel_path));
    
    if is_binary_file(path) {
        // 处理二进制文件（图片）
        let bytes = fs::read(path)?;
        let base64 = base64::encode(&bytes);
        
        result.push_str("```binary\n");
        result.push_str(&base64);
        result.push_str("\n```\n\n");
    } else {
        // 处理文本文件
        let bytes = fs::read(path)?;
        let content = String::from_utf8(bytes)
            .with_context(|| format!("文件 {} 不是有效的 UTF-8 编码", rel_path))?;
        
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
    }
    
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
    let mut is_binary = false;

    // 创建一个 Set 来记录已创建的目录
    let mut created_dirs = std::collections::HashSet::new();

    for line in content.lines() {
        if line.starts_with("###  trxx:") {
            // 保存前一个文件
            if !current_file.is_empty() && !current_content.is_empty() {
                save_content(&current_file, &current_content, is_binary, &mut created_dirs)?;
            }

            // 提取新文件名
            current_file = line
                .trim_start_matches("###  trxx:")
                .trim()
                .to_string();
            
            current_content = String::new();
            is_header = true;
            in_code_block = false;
            is_binary = false;
        } else if !is_header {
            if line.starts_with("```binary") {
                in_code_block = true;
                is_binary = true;
                current_content.clear();
                continue;
            } else if line.starts_with("```") {
                in_code_block = !in_code_block;
                continue;
            }
            
            if in_code_block {
                current_content.push_str(line);
                current_content.push('\n');
            }
        } else if line.is_empty() {
            is_header = false;
        }
    }

    // 保存最后一个文件
    if !current_file.is_empty() && !current_content.is_empty() {
        save_content(&current_file, &current_content, is_binary, &mut created_dirs)?;
    }

    println!("文件已还原完成");
    Ok(())
}

fn save_content(file_path: &str, content: &str, is_binary: bool, created_dirs: &mut std::collections::HashSet<PathBuf>) -> Result<()> {
    let path = Path::new(file_path);
    
    // 确保父目录存在
    if let Some(parent) = path.parent() {
        let parent_path = parent.to_path_buf();
        if !created_dirs.contains(&parent_path) {
            fs::create_dir_all(&parent_path)
                .with_context(|| format!("无法创建目录 {}", parent_path.display()))?;
            created_dirs.insert(parent_path);
        }
    }

    // 根据文件类型保存内容
    if is_binary {
        let bytes = base64::decode(content.trim())
            .with_context(|| format!("无法解码文件 {}", file_path))?;
        fs::write(path, bytes)
            .with_context(|| format!("无法写入文件 {}", file_path))?;
    } else {
        let trimmed_content = content.trim_matches('\n');
        fs::write(path, trimmed_content)
            .with_context(|| format!("无法写入文件 {}", file_path))?;
    }

    Ok(())
}

fn should_process_file(path: &Path) -> bool {
    // 获取文件扩展名
    let extension = path.extension()
        .and_then(|ext| ext.to_str())
        .map(|e| e.to_lowercase())
        .unwrap_or_default();

    // 如果是图片文件，直接返回 true
    if matches!(extension.as_str(), "png" | "jpg" | "jpeg" | "svg") {
        return true;
    }

    // 如果文件大于 1MB，且不是 SVG，则跳过
    if let Ok(metadata) = path.metadata() {
        if metadata.len() > 1024 * 1024 && extension != "svg" {
            return false;
        }
    }

    // 如果没有扩展名，尝试检测是否为文本文件
    if extension.is_empty() {
        return is_probably_text(path);
    }

    // 检查是否是支持的文本文件类型
    matches!(extension.as_str(),
        "txt" | "md" | "rs" | "js" | "ts" | "json" | "yaml" | "yml" 
        | "toml" | "css" | "html" | "htm" | "xml" | "conf" | "cfg"
        | "ini" | "log" | "sh" | "bash" | "py" | "java" | "cpp" | "c"
        | "h" | "hpp" | "cs" | "go" | "rb" | "php" | "sql" | "vue"
        | "jsx" | "tsx" | "gitignore" | "env" | "rc" | "editorconfig"
        | "gradle" | "properties" | "bat" | "cmd" | "ps1" | "dockerfile"
        | "lock" | "config" | "template" | "vim" | "lua" | "svg"
        | "wxss" | "wxml" | "ux")  // 添加小程序和快应用文件类型
}

fn is_binary_file(path: &Path) -> bool {
    let extension = path.extension()
        .and_then(|ext| ext.to_str())
        .map(|e| e.to_lowercase())
        .unwrap_or_default();

    matches!(extension.as_str(), "png" | "jpg" | "jpeg")
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
