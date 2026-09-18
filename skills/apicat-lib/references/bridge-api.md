# ApiCat Bridge API 端点速查

**完整 URL 格式**：`http://127.0.0.1:{port}/api/v1/{端点名}`
**所有端点都必须带 `/api/v1/` 前缀，不带会 404。**

鉴权：所有端点需 `Authorization: Bearer {token}` 头（`/api/v1/health` 免鉴权，供探活）。

示例（port=17320, token 从 bridge.json 读）：
```bash
curl -s http://127.0.0.1:17320/api/v1/health
curl -s -H "Authorization: Bearer $TOKEN" http://127.0.0.1:17320/api/v1/list_projects
```

## 定位

### GET /list_projects
返回所有项目。
```bash
curl -s -H "Authorization: Bearer $TOKEN" "$BASE/list_projects"
```

### POST /create_project
```bash
curl -s -X POST -H "Authorization: Bearer $TOKEN" -H "Content-Type: application/json" \
  -d '{"name": "新项目", "description": ""}' "$BASE/create_project"
```

### POST /update_project
```bash
curl -s -X POST -H "Authorization: Bearer $TOKEN" -H "Content-Type: application/json" \
  -d '{"id": 1, "name": "新名字", "docsOutputDir": "D:/docs"}' "$BASE/update_project"
```

### POST /delete_project

### GET /list_collections?project_id=N
返回项目下所有目录。
```bash
curl -s -H "Authorization: Bearer $TOKEN" "$BASE/list_collections?project_id=1"
```

### POST /create_collection
```bash
curl -s -X POST -H "Authorization: Bearer $TOKEN" -H "Content-Type: application/json" \
  -d '{"projectId": 1, "parentId": null, "name": "Banner"}' "$BASE/create_collection"
```

### POST /rename_collection
```bash
curl -s -X POST -H "Authorization: Bearer $TOKEN" -H "Content-Type: application/json" \
  -d '{"id": 6, "name": "新目录名"}' "$BASE/rename_collection"
```

### POST /delete_collection

## 接口

### GET /list_requests?collection_id=N
### GET /get_request?id=N
### POST /create_request
**⚠️ 只支持 collectionId / name / method / url 四个字段。params/headers/body 会被忽略。**
建完后必须再调 `update_request` 补参数。
```bash
curl -s -X POST -H "Authorization: Bearer $TOKEN" -H "Content-Type: application/json" \
  -d '{"collectionId": 1, "name": "获取推荐", "method": "GET", "url": "/apm/intl/recommend"}' \
  "$BASE/create_request"
# 返回 {"ok":true,"data":{"id":14,...}}
# 再补参数：
curl -s -X POST -H "Authorization: Bearer $TOKEN" -H "Content-Type: application/json" \
  -d '{"id": 14, "params": "[{\"key\":\"gpId\",\"value\":\"test\",\"enabled\":true}]"}' \
  "$BASE/update_request"
```

### POST /update_request
```bash
curl -s -X POST -H "Authorization: Bearer $TOKEN" -H "Content-Type: application/json" \
  -d '{"id": 5, "name": "新名字", "method": "POST", "url": "/api/x", "params": "[]", "headers": "[]", "bodyType": "raw_json", "body": "{}", "authType": "none", "authConfig": "{}", "description": ""}' \
  "$BASE/update_request"
```

### POST /delete_request
```bash
curl -s -X POST -H "Authorization: Bearer $TOKEN" -H "Content-Type: application/json" \
  -d '{"id": 5}' "$BASE/delete_request"
```

### POST /duplicate_request
```bash
curl -s -X POST -H "Authorization: Bearer $TOKEN" -H "Content-Type: application/json" \
  -d '{"id": 5}' "$BASE/duplicate_request"
```

## 用例

### GET /list_test_cases?request_id=N
可选 `&last_status=failed` 过滤失败用例。
```bash
curl -s -H "Authorization: Bearer $TOKEN" "$BASE/list_test_cases?request_id=5"
curl -s -H "Authorization: Bearer $TOKEN" "$BASE/list_test_cases?request_id=5&last_status=failed"
```

### POST /create_test_case
```bash
curl -s -X POST -H "Authorization: Bearer $TOKEN" -H "Content-Type: application/json" \
  -d '{"requestId": 5, "collectionId": 2, "name": "Happy Path", "method": "GET", "url": "{{base_url}}/api/x", "headers": "[]", "params": "[{\"key\":\"gpId\",\"value\":\"test\",\"enabled\":true}]", "bodyType": null, "body": null, "caseType": "happy_path", "assertions": "[{\"type\":\"status_code\",\"operator\":\"eq\",\"expected\":\"200\"}]"}' \
  "$BASE/create_test_case"
```

### POST /update_test_case
所有字段可选，传了才更新（COALESCE 语义）。
```bash
curl -s -X POST -H "Authorization: Bearer $TOKEN" -H "Content-Type: application/json" \
  -d '{"id": 10, "assertions": "[{\"type\":\"status_code\",\"operator\":\"eq\",\"expected\":\"200\"}]"}' \
  "$BASE/update_test_case"
```

### POST /delete_test_case
```bash
curl -s -X POST -H "Authorization: Bearer $TOKEN" -H "Content-Type: application/json" \
  -d '{"id": 10}' "$BASE/delete_test_case"
```

### POST /run_test_case
跑用例（发请求 + 断言求值 + 回写 last_status）。阻塞到结束。
```bash
curl -s -X POST -H "Authorization: Bearer $TOKEN" -H "Content-Type: application/json" \
  -d '{"testCaseId": 10, "envId": null}' "$BASE/run_test_case"
```
返回：
```json
{
  "ok": true,
  "data": {
    "test_case_id": 10,
    "status": "passed",
    "status_code": 200,
    "elapsed_ms": 142,
    "response_body": "...",
    "assertions": [
      {"kind": "status_code", "operator": "eq", "expected": "200", "actual": "200", "passed": true, "message": ""}
    ],
    "passed_count": 1,
    "total_count": 1,
    "error_message": null
  }
}
```

### GET /list_test_case_history?id=N
注意参数名是 `id`（test_case_id）。
```bash
curl -s -H "Authorization: Bearer $TOKEN" "$BASE/list_test_case_history?id=10"
```

## 字典

### GET /list_dictionaries?project_id=N
### GET /list_dictionary_items?dictionary_id=N
### POST /create_dictionary
```bash
curl -s -X POST -H "Authorization: Bearer $TOKEN" -H "Content-Type: application/json" \
  -d '{"code": "activity_type", "name": "活动类型", "description": "", "projectId": 1}' \
  "$BASE/create_dictionary"
```

### POST /create_dictionary_with_items
```bash
curl -s -X POST -H "Authorization: Bearer $TOKEN" -H "Content-Type: application/json" \
  -d '{"code": "activity_type", "name": "活动类型", "projectId": 1, "items": [{"label": "特别活动", "value": "1"}, {"label": "限时特惠", "value": "2"}]}' \
  "$BASE/create_dictionary_with_items"
```

### POST /update_dictionary
### POST /delete_dictionary
### POST /replace_dictionary_items
```bash
curl -s -X POST -H "Authorization: Bearer $TOKEN" -H "Content-Type: application/json" \
  -d '{"dictionaryId": 3, "items": [{"label": "A", "value": "1"}, {"label": "B", "value": "2"}]}' \
  "$BASE/replace_dictionary_items"
```

## 字段绑定

### GET /list_field_rules?project_id=N
### GET /list_field_overrides?project_id=N（可加 `&request_id=N`）
### POST /set_field_rule（项目级）
```bash
curl -s -X POST -H "Authorization: Bearer $TOKEN" -H "Content-Type: application/json" \
  -d '{"projectId": 1, "fieldName": "materialTag", "dictionaryId": 3}' \
  "$BASE/set_field_rule"
```

### POST /delete_field_rule
### POST /set_field_override（接口级，dictionaryId=null 表示解绑）
```bash
curl -s -X POST -H "Authorization: Bearer $TOKEN" -H "Content-Type: application/json" \
  -d '{"projectId": 1, "requestId": 5, "fieldName": "materialTag", "dictionaryId": null}' \
  "$BASE/set_field_override"
```

### POST /delete_field_override

## 发送 / 历史

### POST /send_request
**⚠️ params 内部字段必须用 snake_case**（同 start_stress）。
```bash
curl -s -X POST -H "Authorization: Bearer $TOKEN" -H "Content-Type: application/json" \
  -d '{"requestId": 5, "method": "GET", "url": "https://example.com/api", "query_params": [], "headers": [], "body_type": "none", "body": "", "path_params": [], "auth_type": "none", "auth_config": "{}"}' \
  "$BASE/send_request"
```

### GET /list_history?request_id=N（可加 `&test_case_id=N`）

## 压测

### POST /start_stress
阻塞到压测结束，返回最终统计。

**⚠️ params 内部字段必须用 snake_case**（`query_params` / `body_type` / `path_params` / `auth_type` / `auth_config`），camelCase 会 400。
**⚠️ 压测引擎不做 `{{base_url}}` 替换**，`url` 必须传完整地址（`https://...`）。
```bash
curl -s -X POST -H "Authorization: Bearer $TOKEN" -H "Content-Type: application/json" \
  -d '{"requestId": 5, "params": {"method": "GET", "url": "https://example.com/api", "query_params": [], "headers": [], "body_type": "none", "body": "", "path_params": [], "auth_type": "none", "auth_config": "{}"}, "concurrent": 10, "mode": "count", "value": 100}' \
  "$BASE/start_stress"
```
- `concurrent`: 1~500
- `mode`: `"count"`（value=总请求数，≤10000）| `"duration"`（value=秒数）
- `expectStatus`（可选，默认 `"2xx"`）：**业务成功率**的判定口径。逗号分隔，支持 `2xx` 这类百位通配与 `200` 这类精确码，如 `"2xx,3xx"`。

### GET /list_stress_runs?request_id=N

### GET /stress_report?run_id=N
取某条压测历史的报告，**返回 Markdown 全文**（字符串）。与 App 里「📄 报告」看到的内容**完全同源**（同一个 Rust 函数）。

```bash
curl -s -H "Authorization: Bearer $TOKEN" "$BASE/stress_report?run_id=6" \
  | node -e "let s='';process.stdin.on('data',d=>s+=d).on('end',()=>console.log(JSON.parse(s).data))"
```

报告含五节：结论摘要 / 核心指标 / 耗时分布 / 状态码分布 / 压测配置。
可直接交给用户，或自己转成 HTML 分享。

### 两个成功率的区别（⚠️ 别混）
压测结果里有两个独立指标，**含义不同**：

| 字段 | 含义 | 判定依据 |
|---|---|---|
| `success_rate` | **响应率** | 只要拿到 HTTP 响应（哪怕 500）就算成功；只有超时/连接失败/DNS 失败才计失败 |
| `biz_success_rate` | **业务成功率** | 响应状态码命中 `expectStatus` 才算成功 |

所以 `success_rate=100` 而 `biz_success_rate=0` 是**正常结果**，表示接口全返回了非期望状态码（如全是 500）。
`status_counts` 是 `[状态码, 次数]` 数组，`0` 表示网络错误。**1.0.4 及以前的记录没有 `biz_success_rate` / `expect_status` / `latency_hist` / `status_counts` 字段**，不要当 0 处理。

## 环境

### GET /list_environments?project_id=N
### GET /list_env_variables?id=N（注意：参数是 `id` = env_id）
### POST /create_environment
```bash
curl -s -X POST -H "Authorization: Bearer $TOKEN" -H "Content-Type: application/json" \
  -d '{"projectId": 1, "name": "测试环境", "baseUrl": "https://staging.example.com"}' \
  "$BASE/create_environment"
```

### POST /update_environment
### POST /delete_environment
### POST /activate_environment
```bash
curl -s -X POST -H "Authorization: Bearer $TOKEN" -H "Content-Type: application/json" \
  -d '{"projectId": 1, "envId": 3}' "$BASE/activate_environment"
```

### POST /deactivate_environment
### POST /create_env_variable
### POST /update_env_variable
### POST /delete_env_variable

## Cookie

### GET /list_cookies?scope_type=global|project&project_id=N
### POST /create_cookie
```bash
curl -s -X POST -H "Authorization: Bearer $TOKEN" -H "Content-Type: application/json" \
  -d '{"scopeType": "project", "projectId": 1, "domain": "example.com", "name": "session", "value": "abc", "path": "/"}' \
  "$BASE/create_cookie"
```

### POST /update_cookie
### POST /delete_cookie
