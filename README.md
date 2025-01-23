# trxx

一个用于文本文件和图片文件打包和还原的命令行工具。

## 功能特点

- 将目录下的所有文本文件和图片文件打包成单个文件
- 支持还原打包后的文件到原始目录结构
- 智能识别文本文件（支持无扩展名文件）
- 支持常见图片格式（PNG、JPG、JPEG、SVG）
- 自动忽略二进制文件和大文件（>1MB，SVG 除外）
- 自动忽略特定目录（target、node_modules）和文件（.lock）

## 安装

```bash
# 通过 cargo 安装
cargo install trxx
# 重新加载终端配置（Linux/macOS）
source ~/.bashrc  # 或 source ~/.zshrc
# 验证命令是否可用
trxx --version
```

## 使用方法

### 打包文件

将指定目录下的所有文本文件和图片文件打包到 all_content.md：

```bash
# 打包当前目录
trxx
# 打包指定目录
trxx /path/to/directory
```

### 还原文件

将打包文件还原到原始的目录结构：

```bash
# 还原文件
trxx revert all_content.md
# 还原其他名称的打包文件
trxx revert output.md
```

## 支持的文件类型

### 文本文件
- 常见编程语言文件：.rs, .js, .ts, .py, .java, .cpp, .c, .h, .go 等
- Web 相关文件：.html, .css, .jsx, .tsx, .vue
- 配置文件：.json, .yaml, .yml, .toml, .conf, .ini
- 文档文件：.txt, .md
- 其他常见文本文件：.sh, .bat, .ps1, .env, .gitignore 等

### 图片文件
- PNG (.png)
- JPEG (.jpg, .jpeg)
- SVG (.svg) - 作为文本文件处理

## 自动忽略

- 目录：
  - /target/
  - /node_modules/
  - /.git/

- 文件：
  - all_content.md
  - *.lock

- 大于 1MB 的文件（SVG 文件除外）
- 非文本的二进制文件

## 文件处理说明

- 文本文件：直接保存内容
- PNG/JPG：使用 base64 编码保存
- SVG：作为文本文件处理，保持原始格式

## License

[MIT](LICENSE) © JayFate