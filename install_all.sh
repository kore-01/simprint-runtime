#!/bin/bash
# Simprint Runtime 一键安装脚本
# 适用系统：Ubuntu 22.04+ / Debian 12+ / Linux Mint 21+
set -e

echo "🔧 Simprint Runtime 安装脚本"
echo "================================"

# 检查 Python
if ! command -v python3 &> /dev/null; then
    echo "❌ 需要 Python3"
    exit 1
fi

# 检查 git
if ! command -v git &> /dev/null; then
    echo "❌ 需要 git"
    exit 1
fi

# 安装系统依赖
echo "📦 安装系统依赖..."
sudo apt update -qq
sudo apt install -y -qq chromium-browser libxdo3 xdotool xclip python3-pip

# 克隆仓库（如未提供则使用默认）
REPO_DIR="${1:-$HOME/simprint-runtime}"
if [ ! -d "$REPO_DIR/.git" ]; then
    echo "📥 克隆仓库到 $REPO_DIR..."
    git clone https://github.com/kore-01/simprint-runtime.git "$REPO_DIR"
else
    echo "✅ 仓库已存在: $REPO_DIR"
fi

cd "$REPO_DIR"

# 安装 Python 依赖
echo "🐍 安装 Python 依赖..."
pip3 install -q msgpack

# 让二进制可执行
chmod +x runtime/simprint-runtime
chmod +x simprint_mcp_server.py
chmod +x scripts/news_hourly.py

echo ""
echo "================================"
echo "✅ 安装完成！"
echo ""
echo "测试运行:"
echo "  cd $REPO_DIR"
echo "  python3 simprint_mcp_server.py"
echo ""
echo "快速测试（获取 NYT 新闻）:"
echo "  echo '{\"jsonrpc\":\"2.0\",\"method\":\"initialize\",\"params\":{\"protocolVersion\":\"2024-11-05\",\"capabilities\":{},\"clientInfo\":{\"name\":\"test\",\"version\":\"1.0\"}},\"id\":1}' | python3 simprint_mcp_server.py"
echo ""
echo "新闻抓取定时任务（每小时）:"
echo "  # 添加到 crontab:"
echo "  echo \"0 * * * * cd $REPO_DIR && python3 scripts/news_hourly.py\" | crontab -"
echo ""
echo "📖 文档: https://github.com/kore-01/simprint-runtime#readme"