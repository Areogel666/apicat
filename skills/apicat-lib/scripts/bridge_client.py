#!/usr/bin/env python3
"""
ApiCat Bridge HTTP 客户端
读 bridge.json 获取 port + token，提供基础 HTTP 封装。
供 apicat-test-gen 等技能的脚本调用。

用法：
    from bridge_client import ApiCatBridge
    bridge = ApiCatBridge()
    projects = bridge.get("/list_projects")
    bridge.post("/create_test_case", {...})
"""
import json
import os
import sys
import urllib.request
import urllib.error
from pathlib import Path


def _bridge_json_path() -> Path:
    """按平台优先级定位 bridge.json"""
    if sys.platform == "win32":
        base = Path(os.environ.get("APPDATA", "")) / "com.apicat.app"
    elif sys.platform == "darwin":
        base = Path.home() / "Library" / "Application Support" / "com.apicat.app"
    else:
        base = Path.home() / ".config" / "com.apicat.app"
    return base / "bridge.json"


class ApiCatBridge:
    def __init__(self, bridge_file: str | None = None):
        path = Path(bridge_file) if bridge_file else _bridge_json_path()
        if not path.exists():
            raise FileNotFoundError(
                f"bridge.json 不存在: {path}\n请先启动 ApiCat 应用（Bridge 默认开启）。"
            )
        info = json.loads(path.read_text(encoding="utf-8"))
        if not info.get("enabled", True):
            raise RuntimeError("Bridge 已在设置中关闭，请先开启。")
        self.port = info["port"]
        self.token = info["token"]
        self.base = f"http://127.0.0.1:{self.port}/api/v1"

    def _request(self, method: str, path: str, body: dict | None = None) -> dict:
        url = f"{self.base}{path}"
        headers = {
            "Authorization": f"Bearer {self.token}",
            "Content-Type": "application/json",
        }
        data = json.dumps(body).encode("utf-8") if body is not None else None
        req = urllib.request.Request(url, data=data, headers=headers, method=method)
        try:
            with urllib.request.urlopen(req, timeout=120) as resp:
                result = json.loads(resp.read().decode("utf-8"))
        except urllib.error.HTTPError as e:
            err_body = e.read().decode("utf-8", errors="replace")
            raise RuntimeError(f"HTTP {e.code}: {err_body}") from e
        except urllib.error.URLError as e:
            raise RuntimeError(f"连接失败（ApiCat 未启动？）: {e.reason}") from e
        if not result.get("ok"):
            raise RuntimeError(f"Bridge 错误: {result.get('error', 'unknown')}")
        return result.get("data")

    def get(self, path: str, params: dict | None = None):
        if params:
            qs = "&".join(f"{k}={v}" for k, v in params.items() if v is not None)
            path = f"{path}?{qs}" if qs else path
        return self._request("GET", path)

    def post(self, path: str, body: dict | None = None):
        return self._request("POST", path, body or {})


if __name__ == "__main__":
    # 自检：列出项目
    try:
        b = ApiCatBridge()
        projects = b.get("/list_projects")
        print(f"Bridge 连接成功: {b.base}")
        for p in projects:
            print(f"  [{p['id']}] {p['name']}")
    except Exception as e:
        print(f"错误: {e}", file=sys.stderr)
        sys.exit(1)
