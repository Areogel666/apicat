-- ApiCat v1.0 初始化建表迁移
-- 文件：0001_init.sql
--
-- SQL 文件编写约定（1.0.3 起生效）：
--   - 每个语句以 ; 结尾
--   - 注释中禁止出现独立的 ';' 字符（可能被旧版解析器误拆）
--   - 触发器（含 BEGIN...END 复合块）继续写在 Rust 代码中
--   - 多行注释 /* ... */ 内的 ; 是安全的（is_pure_comment 会识别）

PRAGMA foreign_keys = ON;

-- 1. projects
CREATE TABLE IF NOT EXISTS projects (
  id          INTEGER PRIMARY KEY AUTOINCREMENT,
  name        TEXT    NOT NULL,
  description TEXT,
  created_at  DATETIME DEFAULT CURRENT_TIMESTAMP,
  updated_at  DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- 2. environments
CREATE TABLE IF NOT EXISTS environments (
  id         INTEGER PRIMARY KEY AUTOINCREMENT,
  project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  name       TEXT    NOT NULL,
  base_url   TEXT,
  is_active  INTEGER DEFAULT 0,
  created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- 3. env_variables
CREATE TABLE IF NOT EXISTS env_variables (
  id          INTEGER PRIMARY KEY AUTOINCREMENT,
  env_id      INTEGER NOT NULL REFERENCES environments(id) ON DELETE CASCADE,
  key         TEXT    NOT NULL,
  value       TEXT    NOT NULL DEFAULT '',
  description TEXT,
  enabled     INTEGER DEFAULT 1
);

-- 4. collections
CREATE TABLE IF NOT EXISTS collections (
  id         INTEGER PRIMARY KEY AUTOINCREMENT,
  project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  parent_id  INTEGER REFERENCES collections(id) ON DELETE CASCADE,
  name       TEXT    NOT NULL,
  sort_order INTEGER DEFAULT 0,
  created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- 5. api_requests
CREATE TABLE IF NOT EXISTS api_requests (
  id            INTEGER PRIMARY KEY AUTOINCREMENT,
  collection_id INTEGER NOT NULL REFERENCES collections(id) ON DELETE CASCADE,
  name          TEXT    NOT NULL,
  method        TEXT    NOT NULL DEFAULT 'GET',
  url           TEXT    NOT NULL DEFAULT '',
  params        TEXT    DEFAULT '[]',
  headers       TEXT    DEFAULT '[]',
  body_type     TEXT    DEFAULT 'none',
  body          TEXT    DEFAULT '',
  auth_type     TEXT    DEFAULT 'none',
  auth_config   TEXT    DEFAULT '{}',
  sort_order    INTEGER DEFAULT 0,
  created_at    DATETIME DEFAULT CURRENT_TIMESTAMP,
  updated_at    DATETIME DEFAULT CURRENT_TIMESTAMP,
  UNIQUE(collection_id, name)
);

-- 6. request_history
CREATE TABLE IF NOT EXISTS request_history (
  id               INTEGER PRIMARY KEY AUTOINCREMENT,
  request_id       INTEGER NOT NULL REFERENCES api_requests(id) ON DELETE CASCADE,
  status_code      INTEGER,
  response_time_ms INTEGER,
  request_snapshot TEXT    NOT NULL,
  response_body    TEXT,
  is_truncated     INTEGER DEFAULT 0,
  response_headers TEXT    DEFAULT '{}',
  created_at       DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- 7. cookies
CREATE TABLE IF NOT EXISTS cookies (
  id         INTEGER PRIMARY KEY AUTOINCREMENT,
  scope_type TEXT    NOT NULL DEFAULT 'global',
  project_id INTEGER REFERENCES projects(id) ON DELETE CASCADE,
  domain     TEXT    NOT NULL,
  name       TEXT    NOT NULL,
  value      TEXT    NOT NULL DEFAULT '',
  path       TEXT    DEFAULT '/',
  expires_at DATETIME,
  http_only  INTEGER DEFAULT 0,
  secure     INTEGER DEFAULT 0,
  enabled    INTEGER DEFAULT 1,
  UNIQUE(scope_type, project_id, domain, path, name)
);

-- 8. test_cases
CREATE TABLE IF NOT EXISTS test_cases (
  id               INTEGER PRIMARY KEY AUTOINCREMENT,
  request_id       INTEGER REFERENCES api_requests(id) ON DELETE SET NULL,
  collection_id    INTEGER NOT NULL REFERENCES collections(id) ON DELETE CASCADE,
  name             TEXT    NOT NULL,
  description      TEXT,
  source           TEXT    DEFAULT 'manual',
  method           TEXT,
  url              TEXT,
  headers          TEXT    DEFAULT '[]',
  params           TEXT    DEFAULT '[]',
  body_type        TEXT,
  body             TEXT,
  assertions       TEXT    NOT NULL DEFAULT '[]',
  last_run_at      DATETIME,
  last_status      TEXT    DEFAULT 'pending',
  last_duration_ms INTEGER,
  last_response    TEXT,
  starred          INTEGER DEFAULT 0,
  enabled          INTEGER DEFAULT 1,
  sort_order       INTEGER DEFAULT 0,
  created_at       DATETIME DEFAULT CURRENT_TIMESTAMP,
  updated_at       DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- 外键列索引（级联删除性能保障）
CREATE INDEX IF NOT EXISTS idx_env_proj    ON environments(project_id);
CREATE INDEX IF NOT EXISTS idx_coll_proj   ON collections(project_id);
CREATE INDEX IF NOT EXISTS idx_coll_parent ON collections(parent_id);
CREATE INDEX IF NOT EXISTS idx_req_coll    ON api_requests(collection_id);
CREATE INDEX IF NOT EXISTS idx_hist_req    ON request_history(request_id);
CREATE INDEX IF NOT EXISTS idx_tc_req      ON test_cases(request_id);
CREATE INDEX IF NOT EXISTS idx_tc_coll     ON test_cases(collection_id);

-- 9. test_case_history（M3-C 新增：用例每次执行的历史记录）
-- 每个 test_case_id 保留最新 10 条，由触发器 trg_tch_keep_10 自动滚动淘汰
CREATE TABLE IF NOT EXISTS test_case_history (
  id               INTEGER PRIMARY KEY AUTOINCREMENT,
  test_case_id     INTEGER NOT NULL REFERENCES test_cases(id) ON DELETE CASCADE,
  status_code      INTEGER,                -- 可空：网络层失败（DNS/超时）时无 HTTP status
  duration_ms      INTEGER,                -- 可空：网络层失败时无耗时
  response_preview TEXT,                   -- 响应摘要，前端裁剪到 ≤ 1KB 后落库
  error_message    TEXT,                   -- 网络层错误消息（HTTP 错误进 status_code 不进这里）
  created_at       DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_tch_case      ON test_case_history(test_case_id);
CREATE INDEX IF NOT EXISTS idx_tch_case_time ON test_case_history(test_case_id, created_at DESC);

-- 滚动淘汰触发器 trg_tch_keep_10 不在此文件定义。
-- 原因：触发器含 BEGIN/END 复合块，内部有多条以分号结尾的语句，
-- 而 db/mod.rs 的 migration 解析器是按分号切分的，会把复合块拆坏。
-- 触发器改在 db/mod.rs::run_migrations() 末尾以独立 sqlx::query 调用创建。
-- 注意：本注释里禁止出现单引号包裹的分号字面量，否则会被解析器误拆。
