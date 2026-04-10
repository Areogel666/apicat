#!/usr/bin/env python3
"""
自动定位 ApiCat SQLite 数据库路径。
找到则打印绝对路径到 stdout，exit(0)；
未找到则打印错误到 stderr，exit(1)。

路径优先级：
  1. ./apicat/apicat.db          — 项目内开发模式（npm run tauri dev）
  2. %APPDATA%/com.apicat.app/   — Windows 生产
  3. ~/Library/.../com.apicat.app/ — macOS 生产
  4. ~/.config/com.apicat.app/   — Linux 生产
"""
import os, sys, pathlib

def candidates():
    # 1. 项目内开发模式（优先）
    yield pathlib.Path("./apicat/apicat.db").resolve()

    # 2. Windows 生产
    appdata = os.environ.get("APPDATA", "")
    if appdata:
        yield pathlib.Path(appdata) / "com.apicat.app" / "apicat.db"

    # 3. macOS 生产
    home = pathlib.Path.home()
    yield home / "Library" / "Application Support" / "com.apicat.app" / "apicat.db"

    # 4. Linux 生产
    xdg = os.environ.get("XDG_CONFIG_HOME", str(home / ".config"))
    yield pathlib.Path(xdg) / "com.apicat.app" / "apicat.db"

for path in candidates():
    if path.exists():
        print(str(path))
        sys.exit(0)

print("ApiCat 数据库未找到，请先启动 ApiCat 应用", file=sys.stderr)
sys.exit(1)
