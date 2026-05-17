#!/usr/bin/env python3
"""
Simprint MCP Server — stdio transport + HTTP CDP bridge
Bridges Hermes MCP client ↔ simprint-runtime (via pipes) + Chrome HTTP CDP

Usage:
  mcp_servers:
    simprint:
      command: python3
      args: ["-u", "/home/agentuser/.hermes/skills/simprint-integration/scripts/simprint_mcp_server.py"]
      env:
        RUNTIME_BIN: /home/agentuser/simprint/simprint-runtime-linux/target/release/simprint-runtime
"""

import json, os, queue, select, signal, struct, subprocess, sys, threading, time, msgpack, requests
from typing import Dict, Optional

RUNTIME_BIN = os.environ.get(
    "RUNTIME_BIN",
    "/home/agentuser/simprint/simprint-runtime-linux/target/release/simprint-runtime"
)

# Bypass Paywalls Clean extension — auto-loaded in every browser instance
BYPASS_PAYWALLS_EXT = "/tmp/bypass-paywalls/ext-chrome/bypass-paywalls-chrome-clean-master"

# ── CDP HTTP Helpers ────────────────────────────────────────────────────────

def cdp_http_get(port: int, path: str) -> requests.Response:
    """Make HTTP request to Chrome debugging port."""
    return requests.get(f"http://127.0.0.1:{port}/json{path}", timeout=10)

def cdp_evaluate(port: int, expression: str) -> Optional[dict]:
    """Use Chrome's /json endpoint to evaluate JavaScript via a temporary WebSocket."""
    try:
        # Get WebSocket debugger URL
        targets = requests.get(f"http://127.0.0.1:{port}/json/list", timeout=10).json()
        if not targets:
            return None
        ws_url = targets[0].get("webSocketDebuggerUrl")
        if not ws_url:
            return None
        import websocket  # pip install websocket-client
        ws = websocket.create_connection(ws_url, timeout=15)
        msg_id = 1
        cmd = {"id": msg_id, "method": "Runtime.evaluate", "params": {"expression": expression, "returnByValue": True}}
        ws.send(json.dumps(cmd))
        for _ in range(10):
            resp = ws.recv()
            data = json.loads(resp)
            if data.get("id") == msg_id:
                ws.close()
                return data.get("result", {})
        ws.close()
    except Exception as e:
        return {"exception": str(e)}
    return None

def fetch_page_text(port: int, url: str) -> str:
    """Navigate to URL and extract visible text."""
    try:
        targets = requests.get(f"http://127.0.0.1:{port}/json/list", timeout=10).json()
        if not targets:
            return f"Error: No browser target at port {port}"
        ws_url = targets[0].get("webSocketDebuggerUrl")
        if not ws_url:
            return f"Error: No WebSocket URL at port {port}"
        import websocket
        ws = websocket.create_connection(ws_url, timeout=15)
        msg_id = 1
        ws.send(json.dumps({"id": msg_id, "method": "Page.navigate", "params": {"url": url}}))
        # Wait for navigation
        for _ in range(30):
            resp = ws.recv()
            data = json.loads(resp)
            if data.get("id") == msg_id or data.get("method") == "Page.loadEventFired":
                break
        time.sleep(4)  # Let page render (important for JS-heavy sites)
        # Extract text
        msg_id = 2
        ws.send(json.dumps({"id": msg_id, "method": "Runtime.evaluate", "params": {
            "expression": "document.body.innerText.slice(0, 3000)", "returnByValue": True
        }}))
        for _ in range(10):
            resp = ws.recv()
            data = json.loads(resp)
            if data.get("id") == msg_id:
                ws.close()
                result = data.get("result", {}).get("result", {})
                return result.get("value", "")
        ws.close()
    except Exception as e:
        return f"Error: {e}"
    return ""

# ── Binary IPC Frame Helpers ────────────────────────────────────────────────

MAGIC = b"sprt"
PROTOCOL_VERSION = 3
FRAME_HEADER_SIZE = 9
PAYLOAD_HDR_SIZE = 15

_msg_id_counter = 0
_msg_id_lock = threading.Lock()

def _next_msg_id():
    global _msg_id_counter
    with _msg_id_lock:
        _msg_id_counter += 1
        return _msg_id_counter

def _make_frame(topic: int, data_bytes: bytes, msg_type: int = 1, msg_id: int = 0) -> bytes:
    payload_hdr = (
        struct.pack("<I", msg_id)
        + struct.pack("B", msg_type)
        + struct.pack("<H", topic)
        + struct.pack("<i", 0)
        + struct.pack("<I", len(data_bytes))
    )
    payload = payload_hdr + data_bytes
    return MAGIC + struct.pack("B", PROTOCOL_VERSION) + struct.pack("<I", len(payload)) + payload

def _parse_frame(data: bytes) -> Optional[Dict]:
    if not data or len(data) < FRAME_HEADER_SIZE:
        return None
    idx = data.find(MAGIC)
    if idx < 0:
        return None
    if idx > 0:
        data = data[idx:]
    if len(data) < FRAME_HEADER_SIZE:
        return None
    payload_len = struct.unpack("<I", data[5:9])[0]
    total_len = FRAME_HEADER_SIZE + payload_len
    if len(data) < total_len:
        return None
    frame = data[:total_len]
    p = frame[FRAME_HEADER_SIZE:]
    msg_id = struct.unpack("<I", p[0:4])[0]
    msg_type = p[4]
    topic = struct.unpack("<H", p[5:7])[0]
    error_code = struct.unpack("<i", p[7:11])[0]
    data_len = struct.unpack("<I", p[11:15])[0]
    resp_data = None
    if data_len > 0 and len(p) >= PAYLOAD_HDR_SIZE + data_len:
        try:
            resp_data = msgpack.unpackb(p[PAYLOAD_HDR_SIZE:PAYLOAD_HDR_SIZE + data_len], raw=False)
        except Exception:
            pass
    return {"msg_id": msg_id, "msg_type": msg_type, "topic": topic, "error_code": error_code, "data_len": data_len, "data": resp_data}

def _find_frames(buf: bytes):
    while len(buf) >= FRAME_HEADER_SIZE:
        idx = buf.find(MAGIC)
        if idx < 0:
            break
        if idx > 0:
            buf = buf[idx:]
            continue
        payload_len = struct.unpack("<I", buf[5:9])[0]
        total = FRAME_HEADER_SIZE + payload_len
        if len(buf) < total:
            break
        parsed = _parse_frame(buf[:total])
        yield parsed, total
        buf = buf[total:]

TOPIC_HANDSHAKE = 0x0100
TOPIC_INIT_CTX = 0x0101
TOPIC_ENV_CMD = 0x0400
MSGTYPE_REQUEST = 1
MSGTYPE_RESPONSE = 2


class RuntimeClient:
    def __init__(self, binary_path: str):
        self.bin = binary_path
        self.proc: Optional[subprocess.Popen] = None
        self._lock = threading.Lock()
        self._connected = False
        self._pending: Dict[int, queue.Queue] = {}
        self._reader_thread: Optional[threading.Thread] = None
        self._shutdown = False
        self._stdout_buf: bytes = b""

    def connect(self) -> bool:
        with self._lock:
            if self._connected:
                return True
            try:
                self.proc = subprocess.Popen(
                    [self.bin],
                    stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                    bufsize=0
                )
                time.sleep(0.5)
                self._drain_stderr()

                handshake_data = msgpack.packb({
                    "protocol_version": PROTOCOL_VERSION,
                    "client_name": "simprint-mcp-server",
                    "client_version": "1.0.0"
                }, use_bin_type=True)
                handshake_frame = _make_frame(TOPIC_HANDSHAKE, handshake_data, msg_id=1)
                self.proc.stdin.write(handshake_frame)
                self.proc.stdin.flush()

                resp = self._wait_frame(TOPIC_HANDSHAKE, timeout=10.0)
                if not resp:
                    sys.stderr.write("[runtime] handshake timeout\n")
                    self._cleanup()
                    return False

                init_data = msgpack.packb({
                    "context": {
                        "user_id": "mcp-hermes",
                        "workspace_id": "ws-default",
                        "attributes": {"debug": True}
                    }
                }, use_bin_type=True)
                init_frame = _make_frame(TOPIC_INIT_CTX, init_data, msg_id=2)
                self.proc.stdin.write(init_frame)
                self.proc.stdin.flush()
                self._wait_frame(TOPIC_INIT_CTX, timeout=10.0)

                self._start_reader()
                self._connected = True
                return True
            except Exception as e:
                sys.stderr.write(f"[runtime] connect error: {e}\n")
                self._cleanup()
                return False

    def _drain_stderr(self):
        try:
            r, _, _ = select.select([self.proc.stderr], [], [], 0.2)
            if r:
                chunk = os.read(self.proc.stderr.fileno(), 4096)
                if chunk:
                    sys.stderr.write(f"[runtime startup] {len(chunk)} bytes\n")
        except:
            pass

    def _wait_frame(self, topic: int, timeout: float = 10.0) -> Optional[Dict]:
        deadline = time.time() + timeout
        while time.time() < deadline:
            r, _, _ = select.select([self.proc.stdout], [], [], 0.5)
            if r:
                chunk = os.read(self.proc.stdout.fileno(), 8192)
                if chunk:
                    self._stdout_buf += chunk
                    for parsed, consumed in _find_frames(self._stdout_buf):
                        self._stdout_buf = self._stdout_buf[consumed:]
                        if parsed and parsed["topic"] == topic:
                            return parsed
            time.sleep(0.05)
        return None

    def _start_reader(self):
        self._shutdown = False
        def reader_loop():
            buf = b""
            while not self._shutdown:
                try:
                    r, _, _ = select.select([self.proc.stdout], [], [], 0.5)
                    if not r:
                        continue
                    chunk = os.read(self.proc.stdout.fileno(), 8192)
                    if not chunk:
                        time.sleep(0.1)
                        continue
                    buf += chunk
                    while len(buf) >= FRAME_HEADER_SIZE:
                        idx = buf.find(MAGIC)
                        if idx < 0:
                            buf = b""
                            break
                        if idx > 0:
                            buf = buf[idx:]
                            continue
                        payload_len = struct.unpack("<I", buf[5:9])[0]
                        total = FRAME_HEADER_SIZE + payload_len
                        if len(buf) < total:
                            break
                        frame = buf[:total]
                        buf = buf[total:]
                        parsed = _parse_frame(frame)
                        if parsed:
                            mid = parsed["msg_id"]
                            with self._lock:
                                if mid in self._pending:
                                    q = self._pending.pop(mid)
                                    q.put_nowait(parsed)
                except OSError as e:
                    if e.errno == 11:
                        continue
                    break
                except Exception as e:
                    sys.stderr.write(f"[reader] {e}\n")
                    time.sleep(0.2)
        self._reader_thread = threading.Thread(target=reader_loop, daemon=True)
        self._reader_thread.start()

    def _call_runtime_sync(self, topic: int, data: bytes, timeout: float = 10.0) -> Optional[Dict]:
        mid = _next_msg_id()
        frame = _make_frame(topic, data, msg_type=MSGTYPE_REQUEST, msg_id=mid)
        q: queue.Queue = queue.Queue()
        with self._lock:
            self._pending[mid] = q
        try:
            self.proc.stdin.write(frame)
            self.proc.stdin.flush()
            result = q.get(timeout=timeout)
            return result
        except queue.Empty:
            with self._lock:
                self._pending.pop(mid, None)
            return None
        except Exception as e:
            sys.stderr.write(f"[_call_runtime_sync] {e}\n")
            with self._lock:
                self._pending.pop(mid, None)
            return None

    def _cleanup(self):
        self._shutdown = True
        if self.proc:
            try:
                self.proc.terminate()
                self.proc.wait(timeout=3)
            except:
                self.proc.kill()
            self.proc = None
        with self._lock:
            self._pending.clear()

    # ── Public API ───────────────────────────────────────────────────────────

    def get_all_statuses(self) -> list:
        data = msgpack.packb({"command": "get_all_environment_statuses"}, use_bin_type=True)
        result = self._call_runtime_sync(TOPIC_ENV_CMD, data)
        if result and result.get("data"):
            return result["data"]
        return []

    def list_environments(self) -> list:
        data = msgpack.packb({"command": "get_connected_environments"}, use_bin_type=True)
        result = self._call_runtime_sync(TOPIC_ENV_CMD, data)
        if result and result.get("data"):
            return result["data"].get("env_ids", [])
        return []

    def start_environment(self, env_uuid: str, exe_path: str = None, extension_dirs: list = None) -> Optional[Dict]:
        env_req = {
            "env_uuid": env_uuid,
            "exe_path": exe_path or "/usr/bin/chromium-browser",
            "user_data_dir": f"/tmp/sp-{env_uuid}",
            "fingerprint_config": None,
            "proxy": None,
            "extension_dirs": extension_dirs,
        }
        data = msgpack.packb({
            "command": "start_environment",
            "request": env_req
        }, use_bin_type=True)
        result = self._call_runtime_sync(TOPIC_ENV_CMD, data, timeout=30.0)
        if result:
            return result.get("data")
        return None

    def stop_environment(self, env_uuid: str) -> bool:
        data = msgpack.packb({"command": "stop_environment", "env_uuid": env_uuid}, use_bin_type=True)
        result = self._call_runtime_sync(TOPIC_ENV_CMD, data)
        return result is not None

    def get_environment_status(self, env_uuid: str) -> Optional[Dict]:
        data = msgpack.packb({"command": "get_connected_environments"}, use_bin_type=True)
        result = self._call_runtime_sync(TOPIC_ENV_CMD, data)
        return result.get("data") if result else None

    def get_environment_cdp_endpoint(self, env_uuid: str) -> Optional[Dict]:
        data = msgpack.packb({"command": "get_cdp_endpoint", "env_uuid": env_uuid}, use_bin_type=True)
        result = self._call_runtime_sync(TOPIC_ENV_CMD, data)
        if result and result.get("data"):
            return result["data"].get("endpoint")
        return None

    def get_environment_port(self, env_uuid: str) -> Optional[int]:
        ep = self.get_environment_cdp_endpoint(env_uuid)
        if ep:
            return ep.get("port")
        return None

    def batch_start_environments(self, env_uuids: list) -> Dict[str, bool]:
        return {uuid: self.start_environment(uuid) is not None for uuid in env_uuids}

    def batch_stop_environments(self, env_uuids: list) -> Dict[str, bool]:
        return {uuid: self.stop_environment(uuid) for uuid in env_uuids}

    def list_groups(self) -> list:
        return []


# ── MCP Server ─────────────────────────────────────────────────────────────

TOOL_DEFINITIONS = [
    {"name": "list_environments", "description": "List all simprint browser environments",
     "inputSchema": {"type": "object", "properties": {}}},
    {"name": "start_environment", "description": "Start a simprint browser environment",
     "inputSchema": {"type": "object", "properties": {
         "env_uuid": {"type": "string", "description": "Unique environment ID"},
         "exe_path": {"type": "string", "description": "Optional Chrome executable path"}
     }, "required": ["env_uuid"]}},
    {"name": "stop_environment", "description": "Stop a running simprint browser environment",
     "inputSchema": {"type": "object", "properties": {"env_uuid": {"type": "string"}}, "required": ["env_uuid"]}},
    {"name": "get_environment_status", "description": "Get status of a specific environment",
     "inputSchema": {"type": "object", "properties": {"env_uuid": {"type": "string"}}, "required": ["env_uuid"]}},
    {"name": "get_environment_cdp_endpoint", "description": "Get Chrome DevTools Protocol endpoint",
     "inputSchema": {"type": "object", "properties": {"env_uuid": {"type": "string"}}, "required": ["env_uuid"]}},
    {"name": "batch_start_environments", "description": "Start multiple environments",
     "inputSchema": {"type": "object", "properties": {"env_uuids": {"type": "array", "items": {"type": "string"}}}, "required": ["env_uuids"]}},
    {"name": "batch_stop_environments", "description": "Stop multiple environments",
     "inputSchema": {"type": "object", "properties": {"env_uuids": {"type": "array", "items": {"type": "string"}}}, "required": ["env_uuids"]}},
    {"name": "list_groups", "description": "List browser environment groups",
     "inputSchema": {"type": "object", "properties": {}}},
    {"name": "browse_url", "description": "Navigate browser to URL and extract visible text (max 3000 chars)",
     "inputSchema": {"type": "object", "properties": {
         "env_uuid": {"type": "string", "description": "Environment ID (default: 'news')"},
         "url": {"type": "string", "description": "URL to navigate to"}
     }, "required": ["url"]}},
]

_runtime: Optional[RuntimeClient] = None
_env_cache: Dict[str, str] = {}  # env_uuid -> cdp_port cache

def mcp_initialize(params: Dict) -> Dict:
    global _runtime
    _runtime = RuntimeClient(RUNTIME_BIN)
    if not _runtime.connect():
        sys.stderr.write("[main] runtime connect FAILED\n")
        sys.exit(1)
    sys.stderr.write("[main] runtime connected OK\n")
    return {
        "protocolVersion": "2024-11-05",
        "capabilities": {"tools": {}},
        "serverInfo": {"name": "simprint", "version": "1.0.0"}
    }

def mcp_tools_list() -> Dict:
    return {"tools": TOOL_DEFINITIONS}

def mcp_tools_call(name: str, arguments: Dict) -> Dict:
    if not _runtime:
        return {"content": [{"type": "text", "text": json.dumps({"error": "runtime not initialized"})}],
                "isError": True}
    try:
        if name == "list_environments":
            result = _runtime.list_environments()
            return {"content": [{"type": "text", "text": json.dumps(result, ensure_ascii=False)}]}

        elif name == "start_environment":
            # extension_dirs: list of ext root dirs to load in Chrome
            ext_dirs = arguments.get("extension_dirs", [])
            if ext_dirs:
                # Verify dirs exist before passing
                ext_dirs = [d for d in ext_dirs if os.path.isdir(d)]
            result = _runtime.start_environment(
                arguments.get("env_uuid", ""),
                arguments.get("exe_path"),
                extension_dirs=ext_dirs if ext_dirs else None
            )
            port = None
            if result:
                # Result structure: {"result": {"kind": "started", "endpoint": {...}}}
                ep = result.get("data", result)
                inner = ep.get("result", ep) if isinstance(ep, dict) else ep
                endpoint = inner.get("endpoint", inner) if isinstance(inner, dict) else None
                if endpoint and isinstance(endpoint, dict):
                    port = endpoint.get("port")
                    _env_cache[arguments.get("env_uuid", "")] = port
            return {"content": [{"type": "text", "text": json.dumps(result, ensure_ascii=False)}]}

        elif name == "stop_environment":
            env_uuid = arguments.get("env_uuid", "")
            _env_cache.pop(env_uuid, None)
            ok = _runtime.stop_environment(env_uuid)
            return {"content": [{"type": "text", "text": json.dumps({"ok": ok}, ensure_ascii=False)}]}

        elif name == "get_environment_status":
            result = _runtime.get_environment_status(arguments.get("env_uuid", ""))
            return {"content": [{"type": "text", "text": json.dumps(result, ensure_ascii=False)}]}

        elif name == "get_environment_cdp_endpoint":
            result = _runtime.get_environment_cdp_endpoint(arguments.get("env_uuid", ""))
            if result:
                # Unwrap nested response structure
                ep = result.get("data", result)
                endpoint = ep.get("endpoint", ep) if isinstance(ep, dict) else None
                if endpoint and isinstance(endpoint, dict):
                    _env_cache[arguments.get("env_uuid", "")] = endpoint.get("port")
            return {"content": [{"type": "text", "text": json.dumps(result, ensure_ascii=False)}]}

        elif name == "batch_start_environments":
            result = _runtime.batch_start_environments(arguments.get("env_uuids", []))
            return {"content": [{"type": "text", "text": json.dumps(result, ensure_ascii=False)}]}

        elif name == "batch_stop_environments":
            for uuid in arguments.get("env_uuids", []):
                _env_cache.pop(uuid, None)
            result = _runtime.batch_stop_environments(arguments.get("env_uuids", []))
            return {"content": [{"type": "text", "text": json.dumps(result, ensure_ascii=False)}]}

        elif name == "list_groups":
            result = _runtime.list_groups()
            return {"content": [{"type": "text", "text": json.dumps(result, ensure_ascii=False)}]}

        elif name == "browse_url":
            env_uuid = arguments.get("env_uuid", "news")
            url = arguments.get("url", "")
            if not url:
                return {"content": [{"type": "text", "text": '{"error": "url required"}'}], "isError": True}

            # Get port from cache first (re-use existing environment)
            port = _env_cache.get(env_uuid)
            if not port:
                # Try to get existing environment's CDP endpoint
                ep = _runtime.get_environment_cdp_endpoint(env_uuid)
                if ep:
                    endpoint = ep.get("endpoint", ep.get("result", ep))
                    if isinstance(endpoint, dict):
                        port = endpoint.get("port")
                        if port:
                            _env_cache[env_uuid] = port

            if not port:
                # No existing environment — create one with Bypass Paywalls extension
                ext = [BYPASS_PAYWALLS_EXT] if os.path.isdir(BYPASS_PAYWALLS_EXT) else None
                sys.stderr.write(f"[browse_url] starting env {env_uuid} with ext={ext}\n")
                result = _runtime.start_environment(env_uuid, extension_dirs=ext)
                if result:
                    ep = result.get("data", result)
                    inner = ep.get("result", ep) if isinstance(ep, dict) else ep
                    endpoint = inner.get("endpoint", inner) if isinstance(inner, dict) else None
                    if endpoint and isinstance(endpoint, dict):
                        port = endpoint.get("port")
                        _env_cache[env_uuid] = port
                        sys.stderr.write(f"[browse_url] env started at port {port}\n")
                if not port:
                    sys.stderr.write(f"[browse_url] FAILED to start env {env_uuid}\n")
                    return {"content": [{"type": "text", "text": '{"error": "failed to start environment"}'}], "isError": True}
                # Wait for Chrome to fully start
                time.sleep(2)

            sys.stderr.write(f"[browse_url] using port {port} for {url}\n")
            text = fetch_page_text(port, url)
            if not text:
                sys.stderr.write(f"[browse_url] fetch_page_text returned empty for {url}\n")
            return {"content": [{"type": "text", "text": text[:5000]}]}

        else:
            return {"content": [{"type": "text", "text": json.dumps({"error": f"unknown tool: {name}"})}],
                    "isError": True}
    except Exception as e:
        import traceback
        return {"content": [{"type": "text", "text": json.dumps({"error": str(e), "trace": traceback.format_exc()[:500]})}],
                "isError": True}


def main():
    initialized = False
    sys.stderr.write(f"[main] simprint-mcp-server RUNTIME={RUNTIME_BIN}\n")
    for line in sys.stdin:
        line = line.strip()
        if not line:
            continue
        try:
            req = json.loads(line)
        except:
            continue
        msg_id = req.get("id")
        method = req.get("method")
        params = req.get("params", {})
        if method == "initialize":
            result = mcp_initialize(params)
            resp = {"jsonrpc": "2.0", "id": msg_id, "result": result}
            initialized = True
        elif method == "tools/list" and initialized:
            result = mcp_tools_list()
            resp = {"jsonrpc": "2.0", "id": msg_id, "result": result}
        elif method == "tools/call" and initialized:
            result = mcp_tools_call(params.get("name", ""), params.get("arguments", {}))
            resp = {"jsonrpc": "2.0", "id": msg_id, "result": result}
        else:
            resp = {"jsonrpc": "2.0", "id": msg_id,
                    "error": {"code": -32600, "message": "Invalid Request"}}
        print(json.dumps(resp), flush=True)


if __name__ == "__main__":
    main()