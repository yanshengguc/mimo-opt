#!/bin/bash
# 发布脚本：push 代码 + 创建 v0.3.5-beta Release
# 使用方法：./release.sh

set -e

echo "==> 推送代码到 GitHub..."
git push origin master

echo ""
echo "==> 创建并推送 v0.3.5-beta 标签..."
git tag v0.3.5-beta
git push origin v0.3.5-beta

echo ""
echo "==> GitHub Actions 将自动构建 4 个平台的二进制并创建 Release"
echo "    查看进度: https://github.com/yanshengguc/mimo-opt/actions"
echo ""
echo "完成！"
