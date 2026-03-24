use serde::{Deserialize, Serialize};

// ── Project ────────────────────────────────────────────────
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Project {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

// ── Collection ─────────────────────────────────────────────
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Collection {
    pub id: i64,
    pub project_id: i64,
    pub parent_id: Option<i64>,
    pub name: String,
    pub sort_order: i64,
    pub created_at: String,
}

// ── ApiRequest ─────────────────────────────────────────────
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct ApiRequest {
    pub id: i64,
    pub collection_id: i64,
    pub name: String,
    pub method: String,
    pub url: String,
    pub params: String,
    pub headers: String,
    pub body_type: String,
    pub body: String,
    pub auth_type: String,
    pub auth_config: String,
    pub sort_order: i64,
    pub created_at: String,
    pub updated_at: String,
}

// ── 发请求响应结果 ───────────────────────────────────────────
#[derive(Debug, Serialize, Deserialize)]
pub struct HttpResponse {
    pub status_code: u16,
    pub status_text: String,
    pub headers: Vec<(String, String)>, // [(name, value)]
    pub body: String,
    pub body_size: usize,
    pub elapsed_ms: u64,
    pub is_truncated: bool,
    pub history_id: i64, // 写入 request_history 后的 ID
}

// ── 历史记录条目（用于 History Tab 列表）─────────────────────
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct HistoryRecord {
    pub id: i64,
    pub request_id: i64,
    pub status_code: Option<i64>,
    pub response_time_ms: Option<i64>,
    pub request_snapshot: String, // JSON
    pub response_body: String,
    pub is_truncated: i64,
    pub response_headers: String, // JSON
    pub created_at: String,
}

// ── 环境 ───────────────────────────────────────────────────
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Environment {
    pub id: i64,
    pub project_id: i64,
    pub name: String,
    pub base_url: Option<String>,
    pub is_active: i64,
    pub created_at: String,
}

// ── 环境变量 ───────────────────────────────────────────────
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct EnvVariable {
    pub id: i64,
    pub env_id: i64,
    pub key: String,
    pub value: String,
    pub description: Option<String>,
    pub enabled: i64,
}

// ── Cookie ─────────────────────────────────────────────────
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Cookie {
    pub id: i64,
    pub scope_type: String,
    pub project_id: Option<i64>,
    pub domain: String,
    pub name: String,
    pub value: String,
    pub path: String,
    pub expires_at: Option<String>,
    pub http_only: i64,
    pub secure: i64,
    pub enabled: i64,
}
