-- ApiCat v1.0 初始化建表迁移
-- 文件：0001_init.sql

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
