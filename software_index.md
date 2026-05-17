# 软件安装索引 / Software Installation Index

> 记录所有已安装软件，方便快速查阅和复装

最后更新：2026-05-17

---

## 📋 索引总览

| # | 软件名称 | 用途 | 类型 | 状态 |
|---|---------|------|------|------|
| 1 | simprint-runtime | 指纹浏览器自动化 | Rust二进制 | ✅ 已安装 |
| 2 | hermes-agent | AI助手主程序 | Python CLI | ✅ 已安装 |
| 3 | Bypass Paywalls Clean | 新闻付费墙绕过 | Chrome扩展 | ✅ 已安装 |
| 4 | AKShare | A股数据获取 | Python库 | ✅ 已安装 |
| 5 | Ashare | 轻量股票行情 | Python库 | ✅ 已安装 |
| 6 | MyTT | 技术指标库 | Python库 | ✅ 已安装 |
| 7 | a-stock-mcp-server | 股票MCP服务器 | MCP Server | ✅ 已配置 |
| 8 | tushare | 股票数据接口 | Python库 | ✅ 已安装 |
| 9 | 微信自动回复 | 消息推送 | Python脚本 | ✅ 已配置 |
| 10 | 飞书Bot | 团队消息 | Webhook Bot | ✅ 已连接 |
| 11 | 中东市场监测 | Cron定时任务 | 新闻监控 | ✅ 运行中 |
| 12 | 大宗商品行情 | Cron定时任务 | 行情感知 | ✅ 运行中 |
| 13 | 五媒体新闻抓取 | Cron定时任务 | 新闻监控 | ✅ 运行中 |

---

## 🔧 详情

### 1. Simprint Runtime
**用途：** 指纹浏览器自动化（MCP 服务器）
**仓库：** https://github.com/kore-01/simprint-runtime

```bash
# 一键安装
git clone https://github.com/kore-01/simprint-runtime.git
cd simprint-runtime
pip3 install msgpack
chmod +x runtime/simprint-runtime simprint_mcp_server.py
python3 simprint_mcp_server.py  # 启动 MCP 服务器
```

**依赖：** chromium-browser, libxdo3, xdotool, xclip
**配置：** `RUNTIME_BIN` 环境变量指向 `runtime/simprint-runtime`
**集成：** Hermes MCP servers → simprint 配置

---

### 2. Hermes Agent
**用途：** AI助手主程序
**路径：** `~/.hermes/hermes-agent/`
**配置：** `~/.hermes/config.yaml`

```bash
# 重启
cd ~/.hermes && python3 -m hermes_agent.cli
```

---

### 3. Bypass Paywalls Clean (BPC)
**用途：** 绕过 NYT/NPR/BBC 等新闻网站付费墙
**版本：** v4.3.7.4
**路径：** `extensions/bypass-paywalls-chrome-clean-master/`

```bash
# Chrome 手动加载
# 1. 打开 chrome://extensions/
# 2. 开启开发者模式
# 3. 加载已解压的扩展程序 → 选择 extensions/bypass-paywalls-chrome-clean-master/

# Simprint 自动加载（已集成到 runtime）
# 启动时自动加载 --load-extension=extensions/bypass-paywalls-chrome-clean-master/
```

---

### 4. AKShare
**用途：** A股数据获取（股票/期货/基金/宏观）
**版本：** 1.18.55

```python
import akshare as ak
ak.stock_zh_a_hist(symbol="603516", period="daily", start_date="20250101", end_date="20260517")
```

```bash
pip install akshare==1.18.55
```

---

### 5. Ashare
**用途：** 轻量股票行情（腾讯/新浪双内核）
**路径：** `~/.hermes/libs/Ashare.py`
**优先级：** AKShare > Ashare > 腾讯 > Tushare

```python
import sys
sys.path.insert(0, '~/.hermes/libs')
import Ashare as stock
stock.raw_hist('603516')
```

---

### 6. MyTT
**用途：** 技术指标计算（MA/MACD/KDJ/布林/RSI）
**路径：** `~/.hermes/libs/MyTT.py`

```python
import sys
sys.path.insert(0, '~/.hermes/libs')
import MyTT as TT
TT.MA(close, 20)  # 20日均线
TT.MACD(close)   # MACD
```

---

### 7. a-stock-mcp-server
**用途：** 股票 MCP 服务器（集成 AkShare）
**配置：** `~/.hermes/config.yaml` → MCP servers

```bash
# 已通过Hermes配置自动启动
# 工具：stock_zh_a_hist / stock_futures_main /宏观数据
```

---

### 8. Tushare
**用途：** 股票数据接口（需token）
**安装：** `pip install tushare`
**备注：** Pro版需申请token

---

### 9. 微信自动回复
**用途：** 微信消息推送与自动回复
**配置：** `~/.hermes/config.yaml` → weixin enabled
**发送目标：** `o9cq801-Px6BNI7SWgq-cCf-SH0U@im.wechat`（Home）

```python
from hermes_tools import send_message
send_message(action='send', message='内容', target='weixin')
```

---

### 10. 飞书Bot
**用途：** 飞书消息推送
**配置：** `~/.hermes/config.yaml` → feishu enabled
**用途：** Cron任务推送（中东市场/大宗商品）

---

### 11. 中东市场监测（Cron）
**Job ID：** `cf57b9027d51`
**调度：** 每30分钟 `*/30 * * * *`
**推送：** 飞书
**Skill：** mideast-market-monitor

---

### 12. 大宗商品行情（Cron）
**Job ID：** `f2b508006a8c`
**调度：** 每30分钟 `*/30 * * * *`
**推送：** 飞书 + 微信
**内容：** 黄金/原油实时价格

---

### 13. 五媒体新闻抓取（Cron）
**Job ID：** `1e02155a5c27`
**调度：** 每60分钟
**推送：** 本地保存 + 微信通知队列
**脚本：** `~/.hermes/scripts/news_hourly.py`
**网站：** NYT / NPR / BBC / CNN / ABCNews / NBCNews / LATimes

---

## 📦 批量安装命令

在一台新电脑上快速恢复所有环境：

```bash
# 1. 安装系统依赖
sudo apt update && sudo apt install -y chromium-browser libxdo3 xdotool xclip python3-pip git

# 2. 克隆 Simprint
git clone https://github.com/kore-01/simprint-runtime.git ~/simprint-runtime
pip3 install msgpack

# 3. 安装 Python 库
pip3 install akshare==1.18.55 tushare msgpack

# 4. 复制配置（如果保留配置备份）
# cp ~/backup/config.yaml ~/.hermes/config.yaml

# 5. 恢复 Cron 任务
# 在 Hermes 中用 cronjob --list 查看 job_id，然后重新配置
```

---

## 🔍 查询本机已安装软件

```bash
# 查看所有已安装的Python包
pip3 list | grep -E "akshare|tushare|msgpack"

# 查看Cron任务
crontab -l

# 查看Hermes MCP配置
cat ~/.hermes/config.yaml | grep -A5 mcp_servers

# 查看Simprint运行时
ls ~/.hermes/skills/simprint-integration/scripts/
```

---

## 📝 添加新软件

编辑本文件，在对应位置添加：
1. 编号和名称
2. 用途说明
3. 安装命令/路径
4. 使用示例
5. 依赖关系