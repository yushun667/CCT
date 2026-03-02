#!/bin/bash

# 用法：./clean_dot_underscore.sh [目标目录]

TARGET_DIR="${1:-.}"

if [ ! -d "$TARGET_DIR" ]; then
    echo "错误：目录 '$TARGET_DIR' 不存在。"
    exit 1
fi

echo "正在搜索目录 '$TARGET_DIR' 中以 '._' 开头的文件..."

# 查找所有以 ._ 开头的文件，并存入变量（适用于大多数情况）
files=$(find "$TARGET_DIR" -type f -name "._*")

if [ -z "$files" ]; then
    echo "没有找到任何匹配的文件。"
    exit 0
fi

echo "找到以下文件："
echo "$files"
echo ""

read -p "确定要删除这些文件吗？(y/N) " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    # 逐行读取并删除，避免参数过长或特殊字符问题
    echo "$files" | while IFS= read -r file; do
        rm -v "$file"
    done
    echo "删除完成。"
else
    echo "操作已取消。"
fi