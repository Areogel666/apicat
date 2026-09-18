---
name: apicat-lib
description: ApiCat 本地库访问底座。定位 HTTP Bridge、鉴权、curl 约定。被 apicat-edit / apicat-test-gen / apicat-doc-gen 引用。触发词：apicat、apicat 项目、apicat 库、apicat 接口、apicat 数据、apicat bridge、查看 apicat、看看 apicat、apicat 里有什么
allowed-tools: Bash, Read, Glob
---

# ApiCat Bridge 访问底座

ApiCat 是桌面 API 调试工具（Tauri 2 + SQLite）。1.0.5 起内置 localhost HTTP Bridge，外部技能通过 HTTP 调用全部 IPC 能力。

## Step 1：定位 Bridge

读 `%APPDATA%\com.apicat.app\bridge.json`（Windows）或 `~/Library/Application Support/com.apicat.app/bridge.json`（macOS）：

```json
{ "port": 17320, "token": "abc123...", "enabled": true }
```

- 文件不存在或 `enabled: false` → **报错退出**，提示用户启动 ApiCat 并确认 Bridge 已开启
- **不要回退直写 SQLite**（会绕过 UI 刷新和业务校验）

## Step 2：curl 约定

```bash
# 只读（GET + query 参数）
curl -s -H "Authorization: Bearer $TOKEN" "http://127.0.0.1:$PORT/api/v1/list_projects"

# 写操作（POST + JSON body）
curl -s -X POST -H "Authorization: Bearer $TOKEN" -H "Content-Type: application/json" \
  -d '{"collectionId": 1, "name": "新接口", "method": "GET", "url": "/api/foo"}' \
  "http://127.0.0.1:$PORT/api/v1/create_request"
```

响应统一格式：
- 成功：`{"ok": true, "data": ...}`
- 失败：`{"ok": false, "error": "..."}`

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

## 关键约束

- 参数格式必须是 `{key, value, enabled}`，**禁止** OpenAPI 的 `{name, in}`
- GET 参数进 `params`（拼 URL query）；POST/PUT 必须写 `body_type` + `body`
- 接口名在 collection 内唯一（`UNIQUE(collection_id, name)`）；字典 `code` 全局唯一
- 写操作后 App 内 UI 自动刷新（bridge 广播 `bridge-data-changed` 事件）
