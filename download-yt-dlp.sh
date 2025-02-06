#!/bin/bash

# curl -fsSL https://raw.githubusercontent.com/JayFate/trxx/main/download-yt-dlp.sh | bash

# 创建目录
mkdir -p ~/yt-dlp

# 生成日期数组函数
generate_dates() {
    local start_date=$(date +%Y%m%d)
    local dates=()
    
    for ((i=0; i<200; i++)); do
        local current_date=$(date -d "$start_date - $i days" +%Y.%m.%d)
        dates+=("$current_date")
    done
    echo "${dates[@]}"
}

# 下载函数
download_yt_dlp() {
    local date=$1
    local url="https://gh.api.99988866.xyz/https://github.com/yt-dlp/yt-dlp/releases/download/${date}/yt-dlp_linux"
    echo "尝试下载版本: ${date}"
    
    if curl -L -o ~/yt-dlp/yt-dlp_linux "$url" --fail --silent; then
        echo "下载成功: ${date} 版本"
        return 0
    else
        echo "下载失败: ${date} 版本"
        return 1
    fi
}

# 生成最近200天的日期数组
echo "生成日期列表..."
VERSIONS=($(generate_dates))

# 主下载逻辑
success=false

echo "开始尝试下载，将尝试最近 ${#VERSIONS[@]} 天的版本..."

for version in "${VERSIONS[@]}"; do
    echo "尝试版本: $version"
    if download_yt_dlp "$version"; then
        success=true
        break
    fi
    # 添加短暂延迟，避免请求过快
    sleep 1
done

if [ "$success" = false ]; then
    echo "错误: 所有版本尝试后仍未能下载 yt-dlp"
    exit 1
fi

# 设置执行权限
echo "设置执行权限..."
chmod +x ~/yt-dlp/yt-dlp_linux

# 安装 ffmpeg
echo "更新包管理器并安装 ffmpeg..."
if ! sudo apt update && sudo apt install -y ffmpeg; then
    echo "警告: ffmpeg 安装失败"
    exit 1
fi

# 安装 trxx
echo "安装 trxx..."
if ! cargo install trxx; then
    echo "警告: trxx 安装失败"
    exit 1
fi

echo "所有操作完成!"
