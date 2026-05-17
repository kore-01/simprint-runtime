# Simprint Runtime

Linux 原生指纹浏览器自动化运行时 + MCP 服务器。

## 功能

- 🐾 **指纹隔离浏览器**：每次启动创建独立浏览器环境（通过 Linux namespace 隔离）
- 🌐 **MCP 服务器**：通过 stdio 提供 `browse_url` / `screenshot` 等工具
- 🔐 **环境管理**：`list_environments` / `start_environment` / `stop_environment`
- 🎯 **反爬绕过**：集成 Bypass Paywalls Clean 扩展（已解压到 `extensions/bypass-paywalls-chrome-clean-master/`）
- 🌐 **新闻抓取**：稳定获取 NYT/NPR/BBC/CNN/ABCNews/NBCNews/LATimes 等媒体内容

## 架构

```
用户 (Hermes AI Agent)
    ↓ JSON-RPC (stdio)
Python MCP Server (simprint_mcp_server.py)
    ↓ msgpack RPC (localhost)
Rust Runtime (simprint-runtime, 二进制)
    ↓ fork + exec
Chromium Browser (独立 profile / namespace)
```

## 安装

### 方式一：使用预编译二进制（推荐）

```bash
# 克隆仓库
git clone https://github.com/kore-01/simprint-runtime.git
cd simprint-runtime

# 安装依赖
sudo apt install -y chromium-browser libxdo3 xdotool xclip

# 让二进制可执行
chmod +x runtime/simprint-runtime

# 测试运行
./runtime/simprint-runtime --help
```

### 方式二：源码编译

```bash
git clone https://github.com/kore-01/simprint-runtime.git
cd simprint-runtime/simprint-runtime-linux
cargo build --release
# 编译产物：target/release/simprint-runtime
```

## 使用 MCP 服务器

### 作为独立 stdio 服务器

```python
import subprocess, json, sys

proc = subprocess.Popen(
    ['python3', 'simprint_mcp_server.py'],
    stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE
)

def rpc(method, params=None, id_=1):
    req = {'jsonrpc':'2.0','method':method,'params':params or {},'id':id_}
    proc.stdin.write((json.dumps(req)+'\n').encode())
    proc.stdin.flush()
    return json.loads(proc.stdout.readline().decode())

# 初始化
rpc('initialize', {'protocolVersion':'2024-11-05','capabilities':{},'clientInfo':{'name':'my-app','version':'1.0'}})

# 浏览网页
result = rpc('tools/call', {'name':'browse_url','arguments':{'url':'https://www.nytimes.com/','env_uuid':'nyt'}})
print(result['result']['content'][0]['text'][:500])

# 关闭
rpc('shutdown', {}, 99)
```

### MCP 工具列表

| 工具名 | 参数 | 说明 |
|--------|------|------|
| `browse_url` | `url`, `env_uuid?` | 获取页面文本（最多 3000 字符） |
| `screenshot` | `url`, `env_uuid?` | 获取页面截图（base64 PNG） |
| `list_environments` | — | 列出所有环境 |
| `start_environment` | `env_uuid`, `exe_path?` | 启动浏览器环境 |
| `stop_environment` | `env_uuid` | 停止浏览器环境 |
| `close_browser` | `env_uuid` | 关闭浏览器（优雅） |

### 环境 ID 映射

| ID | 适用网站 |
|----|---------|
| `nyt` | New York Times |
| `npr` | NPR |
| `bbc` | BBC News |
| `cnn` | CNN |
| `abc` | ABC News |
| `nbc` | NBC News |
| `la` | Los Angeles Times |
| `news` | 通用新闻（不指定时） |

## 与 Hermes AI Agent 集成

在 `~/.hermes/config.yaml` 中添加：

```yaml
mcp_servers:
  simprint:
    command: python3
    args:
      - /path/to/simprint_mcp_server.py
    env:
      RUNTIME_BIN: /path/to/simprint-runtime
```

## 新闻抓取示例

```bash
python3 scripts/news_hourly.py
# 输出：~/.hermes/cron/output/news_report_latest.txt
```

## 文件结构

```
simprint-runtime/
├── runtime/
│   └── simprint-runtime          # 预编译 Linux 二进制
├── simprint-runtime-linux/        # Rust 源码目录
│   ├── src/                       # Rust 源码
│   ├── Cargo.toml
│   ├── install.sh
│   └── start.sh
├── simprint_mcp_server.py         # Python MCP 服务器
├── extensions/
│   └── bypass-paywalls-chrome-clean-master/  # 已解压的扩展
├── scripts/
│   ├── news_hourly.py             # 每小时新闻抓取脚本
│   └── ...
└── README.md
```

## 系统要求

- Linux x86_64
- Python 3.8+
- Chromium（`chromium-browser` 或 `google-chrome`）
- 依赖：`libxdo3`, `xdotool`, `xclip`

## License

MIT