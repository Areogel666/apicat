---
name: apicat-lib
description: ApiCat 本地库访问底座。定位 HTTP Bridge、鉴权、curl 约定。触发词：apicat、apicat 库、apicat bridge、查看 apicat、看看 apicat、apicat 里有什么
allowed-tools: Bash, Read, Glob
---

# ApiCat Bridge 访问底座

ApiCat 是桌面 API 调试工具（Tauri 2 + SQLite）。1.0.5 起内置 localhost HTTP Bridge，外部技能通过 HTTP 调用全部 IPC 能力。

**⚠️ 铁律：只走 HTTP Bridge，禁止直连 SQLite。**
- ❌ 不要用 `sqlite3`、`python -c "import sqlite3"`、或任何方式直接读写 `apicat.db`
- ❌ 不要去读 `migrations/*.sql` 来查表结构——用 API
- ❌ 不要先搜磁盘找数据库文件——先读 `bridge.json`
- 直写 DB 会绕过 UI 刷新、业务校验、断言引擎
- `bridge.json` 不存在、`enabled: false`、**或文件在却连不上**（ApiCat 已退出，留下的是上次的残留文件）→ **报错退出**，提示用户启动 ApiCat 并确认 Bridge 已开启，**不要回退到 SQLite**

## Step 1：定位 Bridge

**第一件事就是读 `bridge.json`**：

```bash
# Windows（Git Bash / PowerShell 均可）
cat "$APPDATA/com.apicat.app/bridge.json"
# macOS
cat ~/Library/Application\ Support/com.apicat.app/bridge.json
```

```json
{ "port": 17320, "token": "abc123...", "enabled": true }
```

把上面输出里的值填进变量，后续所有调用复用（`$TOKEN` 不是环境变量，必须自己赋值）：

```bash
PORT=<填入你刚读到的 port>
TOKEN=<填入你刚读到的 token>
BASE="http://127.0.0.1:$PORT/api/v1"
```

**刻意不用 `jq`／`python` 去解析**：`jq` 不是 Windows 自带（实测多数机器没装），而「从 JSON 里取两个字段」这件事你自己读一眼就会 —— 少一个安装门槛，纯 curl 就能跑通全链路。
选完项目后同理固化：`PID=<用户选中的项目 id>`；目录/接口 id 用 `CID`/`RID`。

探活 + 验证（`/health` 免鉴权，`list_projects` 验 token）：

**⚠️ `curl` 连不上（exit 7 / connection refused）＝ ApiCat 没在运行。** 此时 `bridge.json` 往往**还在**（上次退出时没清），别被它骗了以为服务正常 —— 直接提示用户启动 ApiCat。

```bash
curl -s "$BASE/health"
# → {"ok":true,"data":{"status":"up"}}
curl -s -H "Authorization: Bearer $TOKEN" "$BASE/list_projects"
# → {"ok":true,"data":[{"id":2,"name":"demo-project",...}]}
```

**⚠️ 所有端点都必须带 `/api/v1/` 前缀**（`$BASE` 里已含），不带会 404。
完整 URL 格式：`http://127.0.0.1:{port}/api/v1/{端点名}`

## Step 2：调用方式

**少量调用 → curl**：

```bash
# 只读（GET + query 参数）
curl -s -H "Authorization: Bearer $TOKEN" "$BASE/list_collections?project_id=1"

# 写操作（POST + JSON body）—— body 走 stdin，见下方「中文坑」
curl -s -X POST -H "Authorization: Bearer $TOKEN" -H "Content-Type: application/json" \
  -d @- "$BASE/create_request" <<'EOF'
{"collectionId": 1, "name": "新接口", "method": "GET", "url": "/api/foo"}
EOF
```

响应统一格式：成功 `{"ok": true, "data": ...}`；失败 `{"ok": false, "error": "..."}`。

**⚠️ 中文坑（必读）**：Git Bash 在 Windows 下会把**命令行参数**里的中文转成 GBK，服务端按 UTF-8 解析，直接报 `Failed to parse the request body as JSON: invalid unicode code point`。所以 **body 里只要可能出现中文（接口名、字典名、label、description…），就必须走 stdin 或文件，禁止内联**：

```bash
# ✅ 对：heredoc 走 stdin（推荐，不落文件）
curl -s -X POST -H "Authorization: Bearer $TOKEN" -H "Content-Type: application/json" \
  -d @- "$BASE/create_request" <<'EOF'
{"collectionId": 1, "name": "中文接口名", "method": "GET", "url": "/api/foo"}
EOF

# ✅ 对：从文件读
curl -s -X POST -H "Authorization: Bearer $TOKEN" -H "Content-Type: application/json" \
  -d @body.json "$BASE/create_request"

# ❌ 错：内联中文，必失败
curl -s -X POST -H "Authorization: Bearer $TOKEN" -H "Content-Type: application/json" \
  -d '{"name": "中文接口名"}' "$BASE/create_request"
```

纯 ASCII 的 body（如 `{"id": 5}`）内联没问题。**报这个错时不要反复重试，改用 heredoc 即可。**

**多步调用（5+ 次连续读写）→ 可选脚本**：`scripts/bridge_client.py`（本技能目录下）封装了 bridge.json 定位、鉴权、超时与错误处理。

**⚠️ 它是可选的，别默认上它**：需要 Python，而 **Windows 不自带 Python** —— 用户没装时跑 `python` 会弹 Microsoft Store 或报「命令不存在」。**单次调用直接用上面的 curl**（零依赖）；只有要连续读写十几条时，用脚本才划算。

```bash
python "<skills>/apicat-lib/scripts/bridge_client.py"   # 自检：连通则列出所有项目
```

```python
import sys; sys.path.insert(0, "<skills>/apicat-lib/scripts")
from bridge_client import ApiCatBridge
b = ApiCatBridge()                      # 自动读 bridge.json；未启动会抛异常
projects = b.get("/list_projects")      # 返回 data 字段（已剥掉 {ok, data} 外壳）
b.post("/create_test_case", {...})      # 失败直接抛 RuntimeError
```

`<skills>` 即技能安装目录，通常是 `~/.claude/skills`。

## Step 3：端点速查

详见 `references/bridge-api.md`。按用途分组：

| 需求 | 端点 |
|------|------|
| 选项目 | `GET /list_projects` |
| 选目录 | `GET /list_collections?project_id=N` |
| 读接口 | `GET /list_requests?collection_id=N`、`GET /get_request?id=N` |
| 建/改接口 | `POST /create_request`、`POST /update_request` |
| 读用例 | `GET /list_test_cases?request_id=N`（可加 `&last_status=failed`） |
| 读字典 | `GET /list_dictionaries?project_id=N`、`GET /list_dictionary_items?dictionary_id=N` |
| 读字段绑定 | `GET /list_field_rules?project_id=N`、`GET /list_field_overrides?project_id=N` |
| 跑用例 | `POST /run_test_case` |
| 压测 | `POST /start_stress` |
| 读历史 | `GET /list_history?request_id=N` |
| 读用例执行历史 | `GET /list_test_case_history?id=N`（注意：参数是 `id`） |
| 读环境 | `GET /list_environments?project_id=N`、`GET /list_env_variables?id=N`（注意：参数是 `id` = env_id） |
| 激活环境 | `POST /activate_environment` |
| 读 Cookie | `GET /list_cookies?scope_type=global&project_id=N` |

**表格只列常用端点。** 写操作（create/update/delete）、环境与 Cookie 的其余端点共约 30 个不在表内 —— 调用前必须读 `references/bridge-api.md` 确认，**不要因为表里没有就断定 Bridge 不支持**。

## 共享约束

以下约定被 `apicat-edit` / `apicat-test-gen` / `apicat-doc-gen` 共同依赖，**只在此处维护一份**，下游技能不重复展开：

- **参数格式必须是 `{key, value, enabled}`**，**禁止** OpenAPI 的 `{name, in}`——前端按 `p.key` 读，用 `{name,in}` 会静默丢参
- GET 参数进 `params`（拼 URL query）；POST/PUT 必须写 `body_type` + `body`
- 接口的 `params`/`headers`、用例的 `headers`/`params`/`assertions` 都是 **JSON 字符串**（不是数组对象）
- **选项目统一用 `AskUserQuestion` 选项卡**，禁止打印列表让用户回复数字
- 接口名在 collection 内唯一（`UNIQUE(collection_id, name)`）；字典 `code` 全局唯一
- 写操作后 App 内 UI 自动刷新（bridge 广播 `bridge-data-changed` 事件）
- 断言 JSON 格式见 `references/apicat_schema.md`
