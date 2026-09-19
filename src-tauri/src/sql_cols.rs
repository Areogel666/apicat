//! 各表的 SELECT 列清单常量。
//!
//! 同一张表的列清单此前在 commands/ 与 bridge/server.rs 各写一份（名字还不同），
//! 加列时漏改任意一处会导致 sqlx 反序列化失败。此处集中一份，两侧共用。
//! **列顺序必须与 types.rs 中对应 struct 的字段顺序一致**，否则 query_as 会错位。

/// api_requests
pub const REQUEST_COLS: &str = "id, collection_id, name, method, url, params, headers, \
    body_type, body, auth_type, auth_config, description, p95_threshold_ms, p99_threshold_ms, \
    sort_order, created_at, updated_at";

/// test_cases
pub const TEST_CASE_COLS: &str = "id, request_id, collection_id, name, description, source, case_type, \
    method, url, headers, params, body_type, body, assertions, last_run_at, last_status, \
    last_duration_ms, last_response, starred, enabled, sort_order, created_at, updated_at";

/// test_case_history
pub const TEST_CASE_HISTORY_COLS: &str = "id, test_case_id, status_code, duration_ms, \
    response_preview, error_message, created_at";

/// environments
pub const ENV_COLS: &str = "id, project_id, name, base_url, is_active, created_at";

/// env_variables
pub const ENV_VAR_COLS: &str = "id, env_id, key, value, description, enabled";

/// cookies
pub const COOKIE_COLS: &str = "id, scope_type, project_id, domain, name, value, path, \
    expires_at, http_only, secure, enabled";

/// data_dictionaries
pub const DICT_COLS: &str = "id, code, name, description, builtin, project_id, created_at, updated_at";

/// dictionary_items
pub const DICT_ITEM_COLS: &str = "id, dictionary_id, label, value, description, sort_order";
