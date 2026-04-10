# ApiCat 表结构速查

> 同步自 `src-tauri/migrations/0001_init.sql`
> 修改 migration 后需同步更新此文件

此文件供 AI 在生成测试用例 JSON 时查阅字段名，防止拼写错误导致插入失败。

## projects

```sql
CREATE TABLE IF NOT EXISTS projects (
  id          INTEGER PRIMARY KEY AUTOINCREMENT,
  name        TEXT    NOT NULL,
  description TEXT,
  created_at  DATETIME DEFAULT CURRENT_TIMESTAMP,
  updated_at  DATETIME DEFAULT CURRENT_TIMESTAMP
);
```

## collections（UI 称"目录"）

```sql
CREATE TABLE IF NOT EXISTS collections (
  id         INTEGER PRIMARY KEY AUTOINCREMENT,
  project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  parent_id  INTEGER REFERENCES collections(id) ON DELETE CASCADE,
  name       TEXT    NOT NULL,
  sort_order INTEGER DEFAULT 0,
  created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
```

## api_requests

```sql
CREATE TABLE IF NOT EXISTS api_requests (
  id            INTEGER PRIMARY KEY AUTOINCREMENT,
  collection_id INTEGER NOT NULL REFERENCES collections(id) ON DELETE CASCADE,
  name          TEXT    NOT NULL,
  method        TEXT    NOT NULL DEFAULT 'GET',
  url           TEXT    NOT NULL DEFAULT '',
  params        TEXT    DEFAULT '[]',    -- JSON 数组：[{key, value, enabled, description}]
  headers       TEXT    DEFAULT '[]',    -- JSON 数组
  body_type     TEXT    DEFAULT 'none',  -- none | raw_json | raw_text | form_data | form_urlencoded
  body          TEXT    DEFAULT '',
  auth_type     TEXT    DEFAULT 'none',  -- none | bearer | basic | api_key
  auth_config   TEXT    DEFAULT '{}',
  sort_order    INTEGER DEFAULT 0,
  created_at    DATETIME DEFAULT CURRENT_TIMESTAMP,
  updated_at    DATETIME DEFAULT CURRENT_TIMESTAMP,
  UNIQUE(collection_id, name)
);
```

## test_cases

```sql
CREATE TABLE IF NOT EXISTS test_cases (
  id               INTEGER PRIMARY KEY AUTOINCREMENT,
  request_id       INTEGER REFERENCES api_requests(id) ON DELETE SET NULL,
  collection_id    INTEGER NOT NULL REFERENCES collections(id) ON DELETE CASCADE,
  name             TEXT    NOT NULL,
  description      TEXT,
  source           TEXT    DEFAULT 'manual',   -- manual | ai_generated
  method           TEXT,                        -- null = 继承 api_requests
  url              TEXT,                        -- null = 继承 api_requests
  headers          TEXT    DEFAULT '[]',        -- JSON 数组，null = 继承
  params           TEXT    DEFAULT '[]',        -- JSON 数组，null = 继承
  body_type        TEXT,                        -- null = 继承
  body             TEXT,                        -- null = 继承
  assertions       TEXT    NOT NULL DEFAULT '[]', -- JSON 数组，见断言格式
  last_run_at      DATETIME,
  last_status      TEXT    DEFAULT 'pending',  -- pending | passed | failed | error
  last_duration_ms INTEGER,
  last_response    TEXT,
  starred          INTEGER DEFAULT 0,
  enabled          INTEGER DEFAULT 1,
  sort_order       INTEGER DEFAULT 0,
  created_at       DATETIME DEFAULT CURRENT_TIMESTAMP,
  updated_at       DATETIME DEFAULT CURRENT_TIMESTAMP
);
```

## 断言格式（test_cases.assertions 字段）

```json
[
  { "type": "status_code",   "operator": "eq",        "expected": 200 },
  { "type": "response_time", "operator": "lt",         "expected": 500 },
  { "type": "body_json",     "operator": "json_path",  "path": "$.code",    "expected": 0 },
  { "type": "body_json",     "operator": "json_path",  "path": "$.data.id", "expected": "not_null" },
  { "type": "header",        "operator": "contains",   "key": "Content-Type", "expected": "application/json" }
]
```

| `type` | 断言对象 |
|--------|---------|
| `status_code` | HTTP 状态码 |
| `response_time` | 响应耗时（ms） |
| `body_json` | JSON 响应体（支持 JSONPath） |
| `body_text` | 响应体原始文本 |
| `header` | 响应头 |

| `operator` | 含义 |
|------------|------|
| `eq` | 等于 |
| `ne` | 不等于 |
| `lt` / `gt` | 小于 / 大于 |
| `contains` | 包含字符串 |
| `json_path` | JSONPath 取值后比较 |
| `not_null` | 值不为 null/空 |

## AI 生成用例写入约定

| 字段 | 约定值 |
|------|--------|
| `source` | 固定 `'ai_generated'` |
| `starred` | 固定 `0` |
| `enabled` | 固定 `1` |
| `last_status` | 保持默认 `'pending'` |
| `collection_id` | 指向「🤖 AI 测试用例」目录 |
