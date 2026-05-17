#!/bin/bash
# Simprint Linux Runtime 启动脚本
set -e

SIMPRINT_DIR="/home/agentuser/.simprint"
RUNTIME_BIN="/home/agentuser/simprint/simprint-runtime-linux/target/release/simprint-runtime"

# 检查 Chrome/Chromium 是否安装
check_chrome() {
    if command -v chromium-browser &> /dev/null; then
        echo "✅ Chromium found: $(which chromium-browser)"
    elif command -v google-chrome &> /dev/null; then
        echo "✅ Google Chrome found: $(which google-chrome)"
    elif command -v chromium &> /dev/null; then
        echo "✅ Chromium found: $(which chromium)"
    else
        echo "⚠️  Warning: Chromium not found. Install with:"
        echo "   sudo apt install chromium-browser  # Ubuntu/Debian"
        echo "   sudo yum install chromium         # CentOS/RHEL"
    fi
}

# 创建数据目录
setup_dirs() {
    mkdir -p "$SIMPRINT_DIR/data"
    mkdir -p "$SIMPRINT_DIR/environments"
    mkdir -p "$SIMPRINT_DIR/profiles"
    echo "✅ Data directories created: $SIMPRINT_DIR"
}

# 检查 Docker 后端是否运行
check_docker_backend() {
    echo "🔍 Checking Simprint Docker backend..."
    if curl -s http://localhost:40041/health &> /dev/null; then
        echo "✅ Simprint backend (port 40041) is running"
    else
        echo "⚠️  Warning: Simprint backend not running on port 40041"
        echo "   Start with: cd ~/simprint && docker compose up -d"
    fi
}

# 主函数
main() {
    echo "=========================================="
    echo "  Simprint Runtime (Linux) - Starting"
    echo "=========================================="
    
    check_chrome
    setup_dirs
    check_docker_backend
    
    echo ""
    echo "🚀 Starting simprint-runtime..."
    echo "   Binary: $RUNTIME_BIN"
    echo "   Data Dir: $SIMPRINT_DIR"
    echo "   Log Level: ${RUST_LOG:-info}"
    echo ""
    
    # 设置环境变量并启动
    export SIMPRINT_DATA_DIR
    export RUST_LOG=${RUST_LOG:-info}
    
    exec "$RUNTIME_BIN"
}

main "$@"
