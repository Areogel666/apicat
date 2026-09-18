use serde::{Deserialize, Serialize};

// ── Project ────────────────────────────────────────────────
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Project {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    // 1.0.5：doc-gen 技能的文档输出目录（null = 用默认 ~/.apicat/apidoc/{project}）
    pub docs_output_dir: Option<String>,
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
    pub description: String,  // 1.0.4 新增：接口描述（0002 migration 加列）
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
    pub test_case_id: Option<i64>, // 1.0.4：所属用例（null = 原始参数调试）
    pub status_code: Option<i64>,
    pub response_time_ms: Option<i64>,
    // 三个大字段：列表查询不取（避免每次切 Tab 传输 20 条完整响应体），
    // 为 NULL；diff / 回填时由 get_history_record 单条补拉。
    pub request_snapshot: Option<String>, // JSON
    pub response_body: Option<String>,
    pub is_truncated: i64,
    pub response_headers: Option<String>, // JSON
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

// ── 测试用例 ───────────────────────────────────────────────────
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, Clone)]
pub struct TestCase {
    pub id: i64,
    pub request_id: Option<i64>,
    pub collection_id: i64,
    pub name: String,
    pub description: Option<String>,
    pub source: String, // "manual" | "ai_generated"
    // 1.0.5：用例类型（happy_path/missing_required/unauthorized/boundary/empty_list/type_error/invalid_chars）
    pub case_type: String,
    pub method: Option<String>,
    pub url: Option<String>,
    pub headers: String, // JSON 数组
    pub params: String,  // JSON 数组
    pub body_type: Option<String>,
    pub body: Option<String>,
    pub assertions: String, // JSON 数组
    pub last_run_at: Option<String>,
    pub last_status: String, // "pending"|"passed"|"failed"|"error"
    pub last_duration_ms: Option<i64>,
    pub last_response: Option<String>,
    pub starred: i64, // 0 | 1
    pub enabled: i64, // 0 | 1
    pub sort_order: i64,
    pub created_at: String,
    pub updated_at: String,
}

// ── 导入导出用结构 ──────────────────────────────────────────

/// ApiCat 自定义导出格式（完整项目快照）
#[derive(Debug, Serialize, Deserialize)]
pub struct ApiCatExport {
    pub version: String,     // 格式版本，当前 "1.0"
    pub exported_at: String, // ISO 8601 时间
    pub project: ExportProject,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExportProject {
    pub name: String,
    pub description: Option<String>,
    pub environments: Vec<ExportEnv>,
    pub collections: Vec<ExportCollection>,
    // 1.0.4 fix：项目级数据字典（旧导出文件无此字段，serde(default) 兜底为空）
    #[serde(default)]
    pub dictionaries: Vec<ExportDictionary>,
    // 1.0.4：字段名 ↔ 字典绑定规则（随导出携带，导入还原命中）
    #[serde(default)]
    pub field_rules: Vec<ExportFieldRule>,
    // 1.0.4：接口×字段名例外（换绑/解绑），同样随导出携带还原
    #[serde(default)]
    pub field_overrides: Vec<ExportFieldOverride>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExportEnv {
    pub name: String,
    pub base_url: Option<String>,
    pub is_active: i64,
    pub variables: Vec<ExportEnvVar>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExportEnvVar {
    pub key: String,
    pub value: String,
    pub description: Option<String>,
    pub enabled: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExportCollection {
    pub name: String,
    pub sort_order: i64,
    pub children: Vec<ExportCollection>, // 递归子文件夹
    pub requests: Vec<ExportRequest>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExportRequest {
    pub name: String,
    pub method: String,
    pub url: String,
    pub params: String,  // JSON 数组
    pub headers: String, // JSON 数组
    pub body_type: String,
    pub body: String,
    pub auth_type: String,
    pub auth_config: String, // JSON
    pub sort_order: i64,
    // 1.0.4 fix：接口下挂载的用例（旧导出文件无此字段，serde(default) 兜底为空）
    #[serde(default)]
    pub test_cases: Vec<ExportTestCase>,
}

/// 1.0.4 fix：导出格式里的测试用例
#[derive(Debug, Serialize, Deserialize)]
pub struct ExportTestCase {
    pub name: String,
    pub starred: i64,
    pub method: Option<String>,
    pub url: Option<String>,
    pub headers: String,
    pub params: String,
    pub body_type: Option<String>,
    pub body: Option<String>,
    pub sort_order: i64,
}

/// 1.0.4 fix：导出格式里的数据字典
#[derive(Debug, Serialize, Deserialize)]
pub struct ExportDictionary {
    pub code: String,
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub items: Vec<ExportDictionaryItem>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExportDictionaryItem {
    pub value: String,
    pub label: String,
    pub description: String,
    pub sort_order: i64,
}

/// 1.0.4：导出/导入里的「字段名 ↔ 字典」绑定规则（字典用 code 引用，避免 id 漂移）
#[derive(Debug, Serialize, Deserialize)]
pub struct ExportFieldRule {
    pub field_name: String,
    pub dictionary_code: String,
}

/// 1.0.4：导出/导入里的「接口 × 字段名」例外（换绑/解绑）
/// 接口用 (method, url) 业务键定位（项目内通常唯一）；dictionary_code=None 表示解除绑定。
#[derive(Debug, Serialize, Deserialize)]
pub struct ExportFieldOverride {
    pub field_name: String,
    pub dictionary_code: Option<String>,
    pub request_method: String,
    pub request_url: String,
}

// ── 用例执行历史（M3-C 新增）──────────────────────────────────
// 每用例保留最新 10 条（由触发器 trg_tch_keep_10 滚动淘汰）
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct TestCaseHistory {
    pub id: i64,
    pub test_case_id: i64,
    pub status_code: Option<i64>,    // null = 网络层失败
    pub duration_ms: Option<i64>,    // null = 网络层失败
    pub response_preview: Option<String>,  // 响应摘要（前端 ≤1KB 裁剪后）
    pub error_message: Option<String>,     // 网络层错误（HTTP 错误进 status_code）
    pub created_at: String,
}

// ── 数据字典（1.0.4 新增）────────────────────────────────────
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct DataDictionary {
    pub id: i64,
    pub code: String,
    pub name: String,
    pub description: String,
    pub builtin: i64,
    pub project_id: Option<i64>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct DictionaryItem {
    pub id: i64,
    pub dictionary_id: i64,
    pub label: String,
    pub value: String,
    pub description: String,
    pub sort_order: i64,
}

// ── 字典「字段名绑定」（1.0.4）────────────────────────────────
// 规则：项目内 字段名 ↔ 字典（一对一）
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct FieldDictionaryRule {
    pub id: i64,
    pub project_id: i64,
    pub field_name: String,
    pub dictionary_id: i64,
}

// 例外：接口 × 字段名 的换绑/解绑（dictionary_id None = 解除）
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct FieldDictionaryOverride {
    pub id: i64,
    pub project_id: i64,
    pub request_id: i64,
    pub field_name: String,
    pub dictionary_id: Option<i64>,
}

// ── 压测历史（1.0.4 新增，M3 Task 3 使用）──────────────────────
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct StressRun {
    pub id: i64,
    pub request_id: i64,
    pub config_json: String,
    pub stats_json: String,
    pub created_at: String,
}
