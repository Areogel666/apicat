-- 1.0.4：数据字典「字段名绑定」规则与例外（幂等）
-- 规则：项目内 字段名 ↔ 字典（一对一）
-- 例外：接口 × 字段名 的换绑/解绑（优先于规则；dictionary_id NULL = 解除绑定）

CREATE TABLE IF NOT EXISTS field_dictionary_rules (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  project_id   INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  field_name   TEXT NOT NULL,
  dictionary_id INTEGER NOT NULL REFERENCES data_dictionaries(id) ON DELETE CASCADE,
  created_at   DATETIME DEFAULT CURRENT_TIMESTAMP,
  UNIQUE (project_id, field_name)
);

CREATE TABLE IF NOT EXISTS field_dictionary_overrides (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  project_id   INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  request_id   INTEGER NOT NULL REFERENCES api_requests(id) ON DELETE CASCADE,
  field_name   TEXT NOT NULL,
  dictionary_id INTEGER REFERENCES data_dictionaries(id) ON DELETE CASCADE,
  created_at   DATETIME DEFAULT CURRENT_TIMESTAMP,
  UNIQUE (project_id, request_id, field_name)
);