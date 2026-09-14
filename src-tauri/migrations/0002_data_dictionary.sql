-- 1.0.4：数据字典 + 压测历史（幂等，可重复执行）
-- 注意：本文件只放幂等 DDL（CREATE TABLE IF NOT EXISTS），
-- 不写 ALTER TABLE（非幂等）与触发器（含分号字面量），
-- 后者由 db/mod.rs 用 Rust 代码单独执行。

CREATE TABLE IF NOT EXISTS data_dictionaries (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  code TEXT NOT NULL UNIQUE,
  name TEXT NOT NULL,
  description TEXT DEFAULT '',
  builtin INTEGER DEFAULT 0,
  project_id INTEGER REFERENCES projects(id) ON DELETE CASCADE,
  created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
  updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS dictionary_items (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  dictionary_id INTEGER NOT NULL REFERENCES data_dictionaries(id) ON DELETE CASCADE,
  label TEXT NOT NULL,
  value TEXT NOT NULL,
  description TEXT DEFAULT '',
  sort_order INTEGER DEFAULT 0,
  UNIQUE (dictionary_id, value)
);

CREATE TABLE IF NOT EXISTS stress_runs (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  request_id INTEGER NOT NULL REFERENCES api_requests(id) ON DELETE CASCADE,
  config_json TEXT NOT NULL DEFAULT '{}',
  stats_json TEXT NOT NULL DEFAULT '{}',
  created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);