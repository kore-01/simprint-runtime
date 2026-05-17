#!/bin/bash
# Simprint Linux Runtime 一键安装脚本
# 使用方法: bash <(curl -fsSL https://raw.githubusercontent.com/.../install-linux.sh)
set -e

INSTALL_DIR="/home/agentuser/simprint/simprint-runtime-linux"
BINARY_PATH="$INSTALL_DIR/target/release/simprint-runtime"
SYSTEMD_PATH="/etc/systemd/system/simprint-runtime.service"

echo "=========================================="
echo "  Simprint Linux Runtime Installer"
echo "=========================================="

# 1. 检查系统依赖
echo "[1/6] 检查系统依赖..."
if command -v chromium-browser &> /dev/null; then
    echo "  ✅ Chromium installed"
elif command -v google-chrome &> /dev/null; then
    echo "  ✅ Google Chrome installed"
elif command -v chromium &> /dev/null; then
    echo "  ✅ Chromium installed"
else
    echo "  ⚠️  Chromium not found. Installing..."
    sudo apt update && sudo apt install -y chromium-browser xvfb
fi

# 2. 检查 Rust
echo "[2/6] 检查 Rust..."
if ! command -v cargo &> /dev/null; then
    echo "  ⚠️  Rust not found. Installing..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
fi
echo "  ✅ Rust: $(cargo --version)"

# 3. 检查 Docker 后端
echo "[3/6] 检查 Docker 后端..."
if curl -s http://localhost:40041/health &> /dev/null; then
    echo "  ✅ Simprint backend is running"
else
    echo "  ⚠️  Warning: Simprint backend not running"
    echo "     Start with: cd ~/simprint && docker compose up -d"
fi

# 4. 创建目录
echo "[4/6] 创建目录..."
mkdir -p "$HOME/.simprint/data"
mkdir -p "$HOME/.simprint/environments"
mkdir -p "$HOME/.simprint/profiles"
echo "  ✅ Data directories created"

# 5. 复制二进制
echo "[5/6] 安装 simprint-runtime..."
if [ -f "$BINARY_PATH" ]; then
    sudo cp "$BINARY_PATH" /usr/local/bin/simprint-runtime
    sudo chmod +x /usr/local/bin/simprint-runtime
    echo "  ✅ Binary installed to /usr/local/bin/simprint-runtime"
else
    echo "  ❌ Binary not found. Run: cd $INSTALL_DIR && cargo build --release"
    exit 1
fi

# 6. 安装 systemd 服务
echo "[6/6] 安装 systemd 服务..."
if [ -f "$INSTALL_DIR/simprint-runtime.service" ]; then
    sudo cp "$INSTALL_DIR/simprint-runtime.service" "$SYSTEMD_PATH"
    sudo systemctl daemon-reload
    sudo systemctl enable simprint-runtime
    echo "  ✅ systemd service installed"
    echo ""
    echo "  管理命令:"
    echo "    启动: sudo systemctl start simprint-runtime"
    echo "    查看日志: journalctl -u simprint-runtime -f"
    echo "    状态: sudo systemctl status simprint-runtime"
else
    echo "  ⚠️  Service file not found, skipping systemd installation"
fi

echo ""
echo "=========================================="
echo "  ✅ Installation Complete!"
echo "=========================================="
echo ""
echo "快速启动:"
echo "  直接运行: /usr/local/bin/simprint-runtime"
echo "  或使用脚本: $INSTALL_DIR/start.sh"
echo ""
echo "systemd 管理:"
echo "  sudo systemctl start simprint-runtime"
echo "  sudo systemctl stop simprint-runtime"
echo "  journalctl -u simprint-runtime -f"
echo ""
