#!/usr/bin/env python3
"""
ApiCat Bridge HTTP 客户端
读 bridge.json 获取 port + token，提供基础 HTTP 封装。

位于 apicat-lib（底座技能），供 apicat-edit / apicat-test-gen / apicat-doc-gen 共用。

用法：
    import sys; sys.path.insert(0, "<skills>/apicat-lib/scripts")
    from bridge_client import ApiCatBridge
    bridge = ApiCatBridge()                    # 未启动 ApiCat 会抛异常
    projects = bridge.get("/list_projects")    # 返回 data 字段（外壳已剥）
    bridge.post("/create_test_case", {...})    # 失败抛 RuntimeError

命令行自检：
    python bridge_client.py     # 连通则列出所有项目
"""
import json
import os
import sys
import urllib.request
import urllib.error
from pathlib import Path
from typing import Any
from urllib.parse import urlencode


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
    """Bridge 客户端。get/post 返回响应的 data 字段，形状由端点决定（可能是 list/dict/None）。"""

    def __init__(self, bridge_file: str | None = None):
        path = Path(bridge_file) if bridge_file else _bridge_json_path()
        if not path.exists():
            raise FileNotFoundError(
                f"bridge.json 不存在: {path}\n请先启动 ApiCat 应用（Bridge 默认开启）。"
            )
        info = json.loads(path.read_text(encoding="utf-8"))
        if not info.get("enabled", True):
            raise RuntimeError("Bridge 已在设置中关闭，请先开启。")
        self.token = info["token"]
        self.base = f"http://127.0.0.1:{info['port']}/api/v1"

    def _request(self, method: str, path: str, body: dict | None = None) -> Any:
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

    def get(self, path: str, params: dict | None = None) -> Any:
        """GET 请求。params 的 key/value 由本方法负责 percent-encode，调用方传原始值即可。"""
        if params:
            # urlencode 处理空格/&/=/非 ASCII；勿用 doseq，调用方均为标量
            qs = urlencode({k: v for k, v in params.items() if v is not None})
            if qs:
                path = f"{path}?{qs}"
        return self._request("GET", path)

    def post(self, path: str, body: dict | None = None) -> Any:
        return self._request("POST", path, body or {})


def main() -> None:
    # 自检：列出项目
    # Windows 下 Python 默认按 locale(GBK) 写 stdout/stderr，中文会乱码
    for _stream in (sys.stdout, sys.stderr):
        if hasattr(_stream, "reconfigure"):
            _stream.reconfigure(encoding="utf-8")
    try:
        b = ApiCatBridge()
        projects = b.get("/list_projects")
        print(f"Bridge 连接成功: {b.base}")
        for p in projects:
            print(f"  [{p['id']}] {p['name']}")
    except Exception as e:
        print(f"错误: {e}", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
