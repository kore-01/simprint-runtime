#!/usr/bin/env python3
"""
每小时新闻抓取脚本 - 获取7大媒体新闻并生成双语报道
Sites: NYT, NPR, BBC, CNN, ABCNews, NBCNews, LATimes
"""
import subprocess, sys, json, time, os

SERVER = '/home/agentuser/.hermes/skills/simprint-integration/scripts/simprint_mcp_server.py'
RUNTIME = '/home/agentuser/simprint/simprint-runtime-linux/target/release/simprint-runtime'

SITES = [
    ('NYT',       'https://www.nytimes.com/',               'nyt'),
    ('NPR',       'https://www.npr.org/',                   'npr'),
    ('BBC',       'https://www.bbc.com/news',               'bbc'),
    ('CNN',       'https://www.cnn.com/',                    'cnn'),
    ('ABCNews',   'https://abcnews.go.com/',                'abc'),
    ('NBCNews',   'https://www.nbcnews.com/',                'nbc'),
    ('LATimes',   'https://www.latimes.com/',                'la'),
]

def send_rpc(proc, method, params=None, id_=1):
    req = {'jsonrpc': '2.0', 'method': method, 'params': params or {}, 'id': id_}
    proc.stdin.write((json.dumps(req) + '\n').encode('utf-8'))
    proc.stdin.flush()
    resp_line = proc.stdout.readline()
    return json.loads(resp_line.decode('utf-8')) if resp_line else None

def fetch_all():
    """启动 MCP server 并抓取所有媒体内容"""
    proc = subprocess.Popen(
        [sys.executable, '-u', SERVER],
        stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
        env={**os.environ, 'RUNTIME_BIN': RUNTIME}, bufsize=0
    )
    try:
        send_rpc(proc, 'initialize', {
            'protocolVersion': '2024-11-05',
            'capabilities': {},
            'clientInfo': {'name': 'news-hourly', 'version': '1.0'}
        }, 1)
        time.sleep(0.5)

        results = {}
        counter = 2
        for name, url, env_id in SITES:
            print(f'[{name}] fetching {url}...', flush=True)
            try:
                resp = send_rpc(proc, 'tools/call', {
                    'name': 'browse_url',
                    'arguments': {'url': url, 'env_uuid': env_id}
                }, counter)
                text = resp.get('result', {}).get('content', [{}])[0].get('text', '')
            except Exception as e:
                text = f'ERROR: {e}'
            results[name] = text
            print(f'  [{name}] {len(text)} chars', flush=True)
            counter += 1
            time.sleep(1)

        send_rpc(proc, 'shutdown', {}, 99)
    finally:
        proc.stdin.close()
        proc.wait()

    return results

def translate_headlines(results):
    """从原始内容中提取标题并翻译（调用 MiniMax）"""
    headlines = {}
    for outlet, raw in results.items():
        if not raw or len(raw) < 100:
            headlines[outlet] = []
            continue
        # 提取前 2000 字符作为摘要
        sample = raw[:2000]
        headlines[outlet] = sample

    # 保存原始数据
    ts = time.strftime('%Y%m%d_%H%M%S')
    raw_path = f'/home/agentuser/.hermes/cron/output/news_raw_{ts}.json'
    os.makedirs(os.path.dirname(raw_path), exist_ok=True)
    with open(raw_path, 'w') as f:
        json.dump({'timestamp': ts, 'results': results}, f, ensure_ascii=False)

    return raw_path

def format_bilingual_report(results):
    """格式化双语新闻报道"""
    ts = time.strftime('%Y-%m-%d %H:%M')
    lines = [
        f"🗞️ 五媒体新闻双语报道 | {ts}",
        "=" * 60,
        ""
    ]

    outlet_names = {
        'NYT': 'The New York Times (纽约时报)',
        'NPR': 'NPR (美国国家公共电台)',
        'BBC': 'BBC (英国广播公司)',
        'CNN': 'CNN (美国有线电视新闻网)',
        'ABCNews': 'ABC News (美国广播公司)',
        'NBCNews': 'NBC News (全国广播公司)',
        'LATimes': 'Los Angeles Times (洛杉矶时报)',
    }

    for name, raw in results.items():
        display = outlet_names.get(name, name)
        lines.append(f"📰 {display}")
        lines.append("-" * 40)

        if not raw or len(raw) < 100:
            lines.append("⚠️ 获取失败\n")
            continue

        # 提取前 500 字符作为内容样例
        sample = raw[:500].replace('\n', ' ').strip()
        lines.append(f"📝 内容预览：\n{sample}...")
        lines.append("")

    lines.append("=" * 60)
    lines.append(f"🕐 报告生成时间: {ts}")
    lines.append(f"💾 原始数据已保存至 cron output 目录")

    return '\n'.join(lines)

def main():
    print(f'[news_hourly] starting at {time.strftime("%Y-%m-%d %H:%M:%S")}', flush=True)

    results = fetch_all()
    print('[news_hourly] all sites fetched', flush=True)

    report = format_bilingual_report(results)
    print('[news_hourly] report formatted', flush=True)

    # 保存最新报告（供其他程序读取）
    latest_path = '/home/agentuser/.hermes/cron/output/news_report_latest.txt'
    with open(latest_path, 'w') as f:
        f.write(report)

    # 保存 JSON 格式的原始数据
    ts = time.strftime('%Y%m%d_%H%M%S')
    raw_path = f'/home/agentuser/.hermes/cron/output/news_raw_{ts}.json'
    report_path = f'/home/agentuser/.hermes/cron/output/news_report_{ts}.txt'
    with open(raw_path, 'w') as f:
        json.dump({'timestamp': ts, 'results': results}, f, ensure_ascii=False, indent=2)
    with open(report_path, 'w') as f:
        f.write(report)

    print(f'[news_hourly] saved: {report_path}', flush=True)
    print(f'[news_hourly] raw data: {raw_path}', flush=True)

    # 将报告写入通知队列（供 Hermes 主进程发送微信）
    notify_path = '/home/agentuser/.hermes/cron/output/news_notify_pending.txt'
    with open(notify_path, 'w') as f:
        f.write(report)
    print('[news_hourly] notification queued', flush=True)

if __name__ == '__main__':
    main()