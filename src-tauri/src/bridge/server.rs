// HTTP Bridge 服务：axum router + auth + 全量 IPC 镜像 handler

use axum::{
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::SqlitePool;
use std::sync::Arc;
use tauri::Emitter;

use crate::sql_cols::{
    COOKIE_COLS, DICT_COLS, ENV_COLS, ENV_VAR_COLS, PROJECT_COLS, REQUEST_COLS, TEST_CASE_COLS,
};
use crate::types::*;

/// Bridge 共享状态
pub struct BridgeState {
    pub pool: SqlitePool,
    pub http: reqwest::Client,
    pub token: String,
    pub app: tauri::AppHandle,
}

type BState = Arc<BridgeState>;

// ── 响应工具（统一返回 Response，避免 match 臂类型不兼容）────

fn ok<T: serde::Serialize>(data: T) -> axum::response::Response {
    Json(json!({ "ok": true, "data": data })).into_response()
}

fn err(status: StatusCode, msg: &str) -> axum::response::Response {
    (status, Json(json!({ "ok": false, "error": msg }))).into_response()
}

fn server_err(e: impl std::fmt::Display) -> axum::response::Response {
    err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string())
}

// ── Auth 中间件 ─────────────────────────────────────────────

async fn auth_middleware(
    State(state): State<BState>,
    headers: HeaderMap,
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    // /health 免鉴权（供探活）
    if request.uri().path().ends_with("/health") {
        return next.run(request).await;
    }
    let auth = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    // RFC 7235: scheme 大小写不敏感
    let token = auth
        .strip_prefix("Bearer ")
        .or_else(|| auth.strip_prefix("bearer "))
        .unwrap_or("");
    if token != state.token {
        return err(StatusCode::UNAUTHORIZED, "invalid or missing token").into_response();
    }
    next.run(request).await
}

// ── data-changed 广播 ───────────────────────────────────────

fn broadcast(state: &BState, kind: &str) {
    let _ = state.app.emit("bridge-data-changed", json!({ "kind": kind }));
}

// ── JSON body 双键取值 ──────────────────────────────────────
//
// Bridge 调用方混用 camelCase（前端/文档示例）与 snake_case（Rust/脚本），
// 每个字段都要先试 camel 再试 snake。此前 40+ 处手写 .or(body["..."])，
// 漏写 snake 兜底会让该命名风格的参数被静默忽略。

/// 取字符串字段：camelCase 优先，回退 snake_case
fn get_str<'a>(body: &'a Value, camel: &str, snake: &str) -> Option<&'a str> {
    body[camel].as_str().or(body[snake].as_str())
}

/// 取整数字段：camelCase 优先，回退 snake_case
fn get_i64(body: &Value, camel: &str, snake: &str) -> Option<i64> {
    body[camel].as_i64().or(body[snake].as_i64())
}

// ── Query 参数 ──────────────────────────────────────────────

#[derive(Deserialize)]
struct IdQuery {
    id: i64,
}
#[derive(Deserialize)]
struct ProjectQuery {
    project_id: i64,
}
#[derive(Deserialize)]
struct CollectionQuery {
    collection_id: i64,
}
#[derive(Deserialize)]
struct RequestQuery {
    request_id: i64,
    #[serde(default)]
    test_case_id: Option<i64>,
    #[serde(default)]
    last_status: Option<String>,
}
#[derive(Deserialize)]
struct StressReportQuery {
    run_id: i64,
}
#[derive(Deserialize)]
struct DictQuery {
    dictionary_id: i64,
}
#[derive(Deserialize)]
struct FieldOverrideQuery {
    project_id: i64,
    request_id: Option<i64>,
}

// ── Router ──────────────────────────────────────────────────

pub fn build_router(state: BState) -> Router {
    let api = Router::new()
        // 项目
        .route("/list_projects", get(list_projects))
        .route("/create_project", post(create_project))
        .route("/update_project", post(update_project))
        .route("/delete_project", post(delete_project))
        // 目录
        .route("/list_collections", get(list_collections))
        .route("/create_collection", post(create_collection))
        .route("/rename_collection", post(rename_collection))
        .route("/delete_collection", post(delete_collection))
        // 接口
        .route("/list_requests", get(list_requests))
        .route("/get_request", get(get_request))
        .route("/create_request", post(create_request))
        .route("/update_request", post(update_request))
        .route("/delete_request", post(delete_request))
        .route("/duplicate_request", post(duplicate_request))
        // 用例
        .route("/list_test_cases", get(list_test_cases))
        .route("/create_test_case", post(create_test_case))
        .route("/update_test_case", post(update_test_case))
        .route("/delete_test_case", post(delete_test_case))
        .route("/list_test_case_history", get(list_test_case_history))
        .route("/run_test_case", post(run_test_case))
        // 字典
        .route("/list_dictionaries", get(list_dictionaries))
        .route("/list_dictionary_items", get(list_dictionary_items))
        .route("/create_dictionary", post(create_dictionary))
        .route("/create_dictionary_with_items", post(create_dictionary_with_items))
        .route("/update_dictionary", post(update_dictionary))
        .route("/delete_dictionary", post(delete_dictionary))
        .route("/replace_dictionary_items", post(replace_dictionary_items))
        // 字段绑定
        .route("/list_field_rules", get(list_field_rules))
        .route("/set_field_rule", post(set_field_rule))
        .route("/delete_field_rule", post(delete_field_rule))
        .route("/list_field_overrides", get(list_field_overrides))
        .route("/set_field_override", post(set_field_override))
        .route("/delete_field_override", post(delete_field_override))
        // 发送 / 历史
        .route("/send_request", post(send_request))
        .route("/list_history", get(list_history))
        .route("/cleanup_history", post(cleanup_history))
        // 压测
        .route("/start_stress", post(start_stress))
        .route("/list_stress_runs", get(list_stress_runs))
        .route("/stress_report", get(stress_report))
        // 环境
        .route("/list_environments", get(list_environments))
        .route("/list_env_variables", get(list_env_variables))
        .route("/create_environment", post(create_environment))
        .route("/update_environment", post(update_environment))
        .route("/delete_environment", post(delete_environment))
        .route("/activate_environment", post(activate_environment))
        .route("/deactivate_environment", post(deactivate_environment))
        .route("/create_env_variable", post(create_env_variable))
        .route("/update_env_variable", post(update_env_variable))
        .route("/delete_env_variable", post(delete_env_variable))
        // Cookie
        .route("/list_cookies", get(list_cookies))
        .route("/create_cookie", post(create_cookie))
        .route("/update_cookie", post(update_cookie))
        .route("/delete_cookie", post(delete_cookie))
        // 健康检查（auth_middleware 内部放行 /health）
        .route("/health", get(|| async { ok(json!({"status": "up"})) }));

    Router::new()
        .nest("/api/v1", api.layer(axum::middleware::from_fn_with_state(state.clone(), auth_middleware)))
        .with_state(state)
}

// ════════════════════════════════════════════════════════════
// 项目
// ════════════════════════════════════════════════════════════

async fn list_projects(State(s): State<BState>) -> axum::response::Response {
    match sqlx::query_as::<_, Project>(&format!(
        "SELECT {PROJECT_COLS} FROM projects ORDER BY created_at DESC",
    ))
    .fetch_all(&s.pool)
    .await
    {
        Ok(rows) => ok(rows),
        Err(e) => server_err(e),
    }
}

async fn create_project(State(s): State<BState>, Json(body): Json<Value>) -> axum::response::Response {
    let name = body["name"].as_str().unwrap_or("");
    if name.is_empty() {
        return err(StatusCode::BAD_REQUEST, "name is required");
    }
    // 走与 UI 相同的 impl，列清单只有一份
    match crate::commands::project::create_project_impl(&s.pool, name, body["description"].as_str()).await
    {
        Ok(row) => { broadcast(&s, "projects"); ok(row) }
        Err(e) => server_err(e),
    }
}

async fn update_project(State(s): State<BState>, Json(body): Json<Value>) -> axum::response::Response {
    let Some(id) = body["id"].as_i64() else { return err(StatusCode::BAD_REQUEST, "id required") };
    // 先取当前值，未传字段保留原值（COALESCE 的便利留在协议层，SQL 只有 impl 一份）
    let cur: Project = match sqlx::query_as::<_, Project>(&format!(
        "SELECT {PROJECT_COLS} FROM projects WHERE id=?"
    ))
    .bind(id)
    .fetch_one(&s.pool)
    .await
    {
        Ok(r) => r,
        Err(e) => return server_err(e),
    };
    let desc = body["description"].as_str().or(cur.description.as_deref());
    let dir = get_str(&body, "docsOutputDir", "docs_output_dir").or(cur.docs_output_dir.as_deref());
    match crate::commands::project::update_project_impl(
        &s.pool,
        id,
        body["name"].as_str().unwrap_or(&cur.name),
        desc,
        dir,
    )
    .await
    {
        Ok(row) => { broadcast(&s, "projects"); ok(row) }
        Err(e) => server_err(e),
    }
}

async fn delete_project(State(s): State<BState>, Json(body): Json<Value>) -> axum::response::Response {
    let Some(id) = body["id"].as_i64() else { return err(StatusCode::BAD_REQUEST, "id required") };
    match sqlx::query("DELETE FROM projects WHERE id=?").bind(id).execute(&s.pool).await {
        Ok(_) => { broadcast(&s, "projects"); ok(json!({"deleted": id})) }
        Err(e) => server_err(e),
    }
}

// ════════════════════════════════════════════════════════════
// 目录
// ════════════════════════════════════════════════════════════

async fn list_collections(State(s): State<BState>, Query(q): Query<ProjectQuery>) -> axum::response::Response {
    match sqlx::query_as::<_, Collection>(
        "SELECT id, project_id, parent_id, name, sort_order, created_at FROM collections \
         WHERE project_id=? ORDER BY sort_order, id",
    )
    .bind(q.project_id)
    .fetch_all(&s.pool)
    .await
    {
        Ok(rows) => ok(rows),
        Err(e) => server_err(e),
    }
}

async fn create_collection(State(s): State<BState>, Json(body): Json<Value>) -> axum::response::Response {
    let Some(pid) = get_i64(&body, "projectId", "project_id") else {
        return err(StatusCode::BAD_REQUEST, "projectId required");
    };
    match sqlx::query_as::<_, Collection>(
        "INSERT INTO collections (project_id, parent_id, name) VALUES (?, ?, ?) \
         RETURNING id, project_id, parent_id, name, sort_order, created_at",
    )
    .bind(pid)
    .bind(get_i64(&body, "parentId", "parent_id"))
    .bind(body["name"].as_str().unwrap_or("未命名目录"))
    .fetch_one(&s.pool)
    .await
    {
        Ok(row) => { broadcast(&s, "collections"); ok(row) }
        Err(e) => server_err(e),
    }
}

async fn rename_collection(State(s): State<BState>, Json(body): Json<Value>) -> axum::response::Response {
    let Some(id) = body["id"].as_i64() else { return err(StatusCode::BAD_REQUEST, "id required") };
    match sqlx::query_as::<_, Collection>(
        "UPDATE collections SET name=? WHERE id=? \
         RETURNING id, project_id, parent_id, name, sort_order, created_at",
    )
    .bind(body["name"].as_str().unwrap_or(""))
    .bind(id)
    .fetch_one(&s.pool)
    .await
    {
        Ok(row) => { broadcast(&s, "collections"); ok(row) }
        Err(e) => server_err(e),
    }
}

async fn delete_collection(State(s): State<BState>, Json(body): Json<Value>) -> axum::response::Response {
    let Some(id) = body["id"].as_i64() else { return err(StatusCode::BAD_REQUEST, "id required") };
    match sqlx::query("DELETE FROM collections WHERE id=?").bind(id).execute(&s.pool).await {
        Ok(_) => { broadcast(&s, "collections"); ok(json!({"deleted": id})) }
        Err(e) => server_err(e),
    }
}

// ════════════════════════════════════════════════════════════
// 接口
// ════════════════════════════════════════════════════════════

const REQ_COLS: &str = REQUEST_COLS;

async fn list_requests(State(s): State<BState>, Query(q): Query<CollectionQuery>) -> axum::response::Response {
    let sql = format!("SELECT {REQ_COLS} FROM api_requests WHERE collection_id=? ORDER BY sort_order, id");
    match sqlx::query_as::<_, ApiRequest>(&sql).bind(q.collection_id).fetch_all(&s.pool).await {
        Ok(rows) => ok(rows),
        Err(e) => server_err(e),
    }
}

async fn get_request(State(s): State<BState>, Query(q): Query<IdQuery>) -> axum::response::Response {
    let sql = format!("SELECT {REQ_COLS} FROM api_requests WHERE id=?");
    match sqlx::query_as::<_, ApiRequest>(&sql).bind(q.id).fetch_optional(&s.pool).await {
        Ok(Some(row)) => ok(row),
        Ok(None) => err(StatusCode::NOT_FOUND, "request not found"),
        Err(e) => server_err(e),
    }
}

async fn create_request(State(s): State<BState>, Json(body): Json<Value>) -> axum::response::Response {
    let Some(cid) = get_i64(&body, "collectionId", "collection_id") else {
        return err(StatusCode::BAD_REQUEST, "collectionId required");
    };
    // 走与 UI 相同的 impl，UNIQUE 冲突返回友好文案而非原始 SQLite 错误
    match crate::commands::request::create_request_impl(
        &s.pool,
        cid,
        body["name"].as_str().unwrap_or("未命名接口"),
        body["method"].as_str().unwrap_or("GET"),
        body["url"].as_str().unwrap_or(""),
    )
    .await
    {
        Ok(row) => { broadcast(&s, "requests"); ok(row) }
        Err(e) => server_err(e),
    }
}

async fn update_request(State(s): State<BState>, Json(body): Json<Value>) -> axum::response::Response {
    let Some(id) = body["id"].as_i64() else { return err(StatusCode::BAD_REQUEST, "id required") };
    // Bridge 侧保持「传了才更新」的 COALESCE 语义：先读旧值，未传字段用原值补齐，
    // 再走与 UI 同一份全量覆写 SQL。这样 COALESCE 是协议层便利，SQL 只有一份。
    let cur: ApiRequest = match sqlx::query_as::<_, ApiRequest>(&format!(
        "SELECT {REQ_COLS} FROM api_requests WHERE id=?"
    ))
    .bind(id)
    .fetch_one(&s.pool)
    .await
    {
        Ok(r) => r,
        Err(e) => return server_err(e),
    };
    match crate::commands::request::update_request_impl(
        &s.pool,
        id,
        body["name"].as_str().unwrap_or(&cur.name),
        body["method"].as_str().unwrap_or(&cur.method),
        body["url"].as_str().unwrap_or(&cur.url),
        body["params"].as_str().unwrap_or(&cur.params),
        body["headers"].as_str().unwrap_or(&cur.headers),
        get_str(&body, "bodyType", "body_type").unwrap_or(&cur.body_type),
        body["body"].as_str().unwrap_or(&cur.body),
        get_str(&body, "authType", "auth_type").unwrap_or(&cur.auth_type),
        get_str(&body, "authConfig", "auth_config").unwrap_or(&cur.auth_config),
        body["description"].as_str().unwrap_or(&cur.description),
    )
    .await
    {
        Ok(row) => { broadcast(&s, "requests"); ok(row) }
        Err(e) => server_err(e),
    }
}

async fn delete_request(State(s): State<BState>, Json(body): Json<Value>) -> axum::response::Response {
    let Some(id) = body["id"].as_i64() else { return err(StatusCode::BAD_REQUEST, "id required") };
    match sqlx::query("DELETE FROM api_requests WHERE id=?").bind(id).execute(&s.pool).await {
        Ok(_) => { broadcast(&s, "requests"); ok(json!({"deleted": id})) }
        Err(e) => server_err(e),
    }
}

async fn duplicate_request(State(s): State<BState>, Json(body): Json<Value>) -> axum::response::Response {
    let Some(id) = body["id"].as_i64() else { return err(StatusCode::BAD_REQUEST, "id required") };
    // 走与 UI 相同的 impl：保留 description 且 sort_order 紧跟原接口
    match crate::commands::request::duplicate_request_impl(&s.pool, id).await {
        Ok(row) => { broadcast(&s, "requests"); ok(row) }
        Err(e) => server_err(e),
    }
}

// ════════════════════════════════════════════════════════════
// 用例
// ════════════════════════════════════════════════════════════

const TC_COLS: &str = TEST_CASE_COLS;

async fn list_test_cases(State(s): State<BState>, Query(q): Query<RequestQuery>) -> axum::response::Response {
    // last_status 过滤（如 ?last_status=failed）；不传则返回全部 enabled 用例
    let sql = if q.last_status.is_some() {
        format!("SELECT {TC_COLS} FROM test_cases WHERE request_id=? AND enabled=1 AND last_status=? ORDER BY sort_order, id")
    } else {
        format!("SELECT {TC_COLS} FROM test_cases WHERE request_id=? AND enabled=1 ORDER BY sort_order, id")
    };
    let mut query = sqlx::query_as::<_, TestCase>(&sql).bind(q.request_id);
    if let Some(st) = &q.last_status {
        query = query.bind(st);
    }
    match query.fetch_all(&s.pool).await {
        Ok(rows) => ok(rows),
        Err(e) => server_err(e),
    }
}

async fn create_test_case(State(s): State<BState>, Json(body): Json<Value>) -> axum::response::Response {
    let Some(rid) = get_i64(&body, "requestId", "request_id") else {
        return err(StatusCode::BAD_REQUEST, "requestId required");
    };
    let Some(cid) = get_i64(&body, "collectionId", "collection_id") else {
        return err(StatusCode::BAD_REQUEST, "collectionId required");
    };
    // caseType 必填：缺省兜底会把 7 种用例类型静默全标成 happy_path，宁可报错
    let Some(case_type) = get_str(&body, "caseType", "case_type") else {
        return err(StatusCode::BAD_REQUEST, "caseType required (happy_path/missing_required/unauthorized/boundary/empty_list/type_error/invalid_chars)");
    };
    match crate::commands::test_case::create_test_case_impl(
        &s.pool,
        rid,
        cid,
        body["name"].as_str().unwrap_or(""),
        body["description"].as_str(),
        body["source"].as_str().unwrap_or("manual"),
        body["method"].as_str(),
        body["url"].as_str(),
        body["headers"].as_str(),
        body["params"].as_str(),
        get_str(&body, "bodyType", "body_type"),
        body["body"].as_str(),
        case_type,
        body["assertions"].as_str(),
    )
    .await
    {
        Ok(row) => { broadcast(&s, "test_cases"); ok(row) }
        Err(e) => server_err(e),
    }
}

async fn update_test_case(State(s): State<BState>, Json(body): Json<Value>) -> axum::response::Response {
    let Some(id) = body["id"].as_i64() else { return err(StatusCode::BAD_REQUEST, "id required") };
    let sql = format!(
        "UPDATE test_cases SET name=COALESCE(?, name), \
         method=COALESCE(?, method), url=COALESCE(?, url), \
         headers=COALESCE(?, headers), params=COALESCE(?, params), \
         body_type=COALESCE(?, body_type), body=COALESCE(?, body), \
         case_type=COALESCE(?, case_type), assertions=COALESCE(?, assertions), \
         updated_at=datetime('now') WHERE id=? RETURNING {TC_COLS}"
    );
    match sqlx::query_as::<_, TestCase>(&sql)
        .bind(body["name"].as_str())
        .bind(body["method"].as_str())
        .bind(body["url"].as_str())
        .bind(body["headers"].as_str())
        .bind(body["params"].as_str())
        .bind(get_str(&body, "bodyType", "body_type"))
        .bind(body["body"].as_str())
        .bind(get_str(&body, "caseType", "case_type"))
        .bind(body["assertions"].as_str())
        .bind(id)
        .fetch_one(&s.pool)
        .await
    {
        Ok(row) => { broadcast(&s, "test_cases"); ok(row) }
        Err(e) => server_err(e),
    }
}

async fn delete_test_case(State(s): State<BState>, Json(body): Json<Value>) -> axum::response::Response {
    let Some(id) = body["id"].as_i64() else { return err(StatusCode::BAD_REQUEST, "id required") };
    // 走与 UI 相同的 impl，保留「最后一个收藏用例不可删」保护
    match crate::commands::test_case::delete_test_case_impl(&s.pool, id).await {
        Ok(_) => { broadcast(&s, "test_cases"); ok(json!({"deleted": id})) }
        Err(e) => server_err(e),
    }
}

async fn list_test_case_history(State(s): State<BState>, Query(q): Query<IdQuery>) -> axum::response::Response {
    match sqlx::query_as::<_, TestCaseHistory>(
        "SELECT id, test_case_id, status_code, \
                response_time_ms as duration_ms, \
                substr(response_body, 1, 1024) as response_preview, \
                error_message, created_at \
         FROM request_history \
         WHERE test_case_id = ? \
         ORDER BY created_at DESC, id DESC LIMIT 10"
    )
    .bind(q.id)
    .fetch_all(&s.pool)
    .await
    {
        Ok(rows) => ok(rows),
        Err(e) => server_err(e),
    }
}

async fn run_test_case(State(s): State<BState>, Json(body): Json<Value>) -> axum::response::Response {
    let Some(tc_id) = get_i64(&body, "testCaseId", "test_case_id") else {
        return err(StatusCode::BAD_REQUEST, "testCaseId required");
    };
    let env_id = get_i64(&body, "envId", "env_id");
    // 复用 Tauri command 的逻辑：直接调用 crate::commands::test_case_run
    // 但它是 #[tauri::command]，参数是 State —— 这里内联同样逻辑
    match crate::commands::test_case_run::run_test_case_impl(&s.pool, &s.http, tc_id, env_id).await {
        Ok(result) => { broadcast(&s, "test_cases"); ok(result) }
        Err(e) => server_err(e),
    }
}

// ════════════════════════════════════════════════════════════
// 字典
// ════════════════════════════════════════════════════════════

async fn list_dictionaries(State(s): State<BState>, Query(q): Query<ProjectQuery>) -> axum::response::Response {
    // 与 commands/data_dictionary.rs 一致：project_id IS NULL 即全局可见
    // （builtin 是「是否内置」标记，与可见性正交；io.rs 的导出/导入查重也用此条件）
    let sql = format!(
        "SELECT {DICT_COLS} FROM data_dictionaries WHERE project_id IS NULL OR project_id=? ORDER BY builtin DESC, id ASC"
    );
    match sqlx::query_as::<_, DataDictionary>(&sql)
        .bind(q.project_id)
        .fetch_all(&s.pool)
        .await
    {
        Ok(rows) => ok(rows),
        Err(e) => server_err(e),
    }
}

async fn list_dictionary_items(State(s): State<BState>, Query(q): Query<DictQuery>) -> axum::response::Response {
    match sqlx::query_as::<_, DictionaryItem>(
        "SELECT id, dictionary_id, label, value, description, sort_order \
         FROM dictionary_items WHERE dictionary_id=? ORDER BY sort_order, id",
    )
    .bind(q.dictionary_id)
    .fetch_all(&s.pool)
    .await
    {
        Ok(rows) => ok(rows),
        Err(e) => server_err(e),
    }
}

async fn create_dictionary(State(s): State<BState>, Json(body): Json<Value>) -> axum::response::Response {
    // projectId 必填：缺省落 NULL 会静默变成全局共享字典，泄漏到所有项目
    // （与 create_test_case 的 case_type 必填同理）；全局可见性仅保留给 builtin 数据
    let Some(project_id) = get_i64(&body, "projectId", "project_id") else {
        return err(StatusCode::BAD_REQUEST, "projectId is required (dictionary ownership scope)");
    };
    let sql = "INSERT INTO data_dictionaries (code, name, description, project_id) VALUES (?, ?, ?, ?) \
         RETURNING id, code, name, description, builtin, project_id, created_at, updated_at";
    match sqlx::query_as::<_, DataDictionary>(sql)
        .bind(body["code"].as_str().unwrap_or(""))
        .bind(body["name"].as_str().unwrap_or(""))
        .bind(body["description"].as_str().unwrap_or(""))
        .bind(project_id)
        .fetch_one(&s.pool)
        .await
    {
        Ok(row) => { broadcast(&s, "dictionaries"); ok(row) }
        Err(e) => server_err(e),
    }
}

async fn create_dictionary_with_items(State(s): State<BState>, Json(body): Json<Value>) -> axum::response::Response {
    // projectId 必填：缺省落 NULL 会静默变成全局共享字典，泄漏到所有项目
    let Some(project_id) = get_i64(&body, "projectId", "project_id") else {
        return err(StatusCode::BAD_REQUEST, "projectId is required (dictionary ownership scope)");
    };
    let mut tx = match s.pool.begin().await {
        Ok(tx) => tx,
        Err(e) => return server_err(e),
    };
    let dict_result = sqlx::query_as::<_, DataDictionary>(
        "INSERT INTO data_dictionaries (code, name, description, project_id) VALUES (?, ?, ?, ?) \
         RETURNING id, code, name, description, builtin, project_id, created_at, updated_at",
    )
    .bind(body["code"].as_str().unwrap_or(""))
    .bind(body["name"].as_str().unwrap_or(""))
    .bind(body["description"].as_str().unwrap_or(""))
    .bind(project_id)
    .fetch_one(&mut *tx)
    .await;

    let dict = match dict_result {
        Ok(d) => d,
        Err(e) => return server_err(e),
    };

    if let Some(items) = body["items"].as_array() {
        for item in items {
            let r = sqlx::query(
                "INSERT INTO dictionary_items (dictionary_id, label, value, description, sort_order) \
                 VALUES (?, ?, ?, ?, ?)",
            )
            .bind(dict.id)
            .bind(item["label"].as_str().unwrap_or(""))
            .bind(item["value"].as_str().unwrap_or(""))
            .bind(item["description"].as_str().unwrap_or(""))
            .bind(item["sortOrder"].as_i64().or(item["sort_order"].as_i64()).unwrap_or(0))
            .execute(&mut *tx)
            .await;
            if let Err(e) = r { return server_err(e); }
        }
    }

    match tx.commit().await {
        Ok(_) => { broadcast(&s, "dictionaries"); ok(dict) }
        Err(e) => server_err(e),
    }
}

async fn update_dictionary(State(s): State<BState>, Json(body): Json<Value>) -> axum::response::Response {
    let Some(id) = body["id"].as_i64() else { return err(StatusCode::BAD_REQUEST, "id required") };
    match sqlx::query_as::<_, DataDictionary>(
        "UPDATE data_dictionaries SET name=COALESCE(?, name), description=COALESCE(?, description), \
         updated_at=datetime('now') WHERE id=? \
         RETURNING id, code, name, description, builtin, project_id, created_at, updated_at",
    )
    .bind(body["name"].as_str())
    .bind(body["description"].as_str())
    .bind(id)
    .fetch_one(&s.pool)
    .await
    {
        Ok(row) => { broadcast(&s, "dictionaries"); ok(row) }
        Err(e) => server_err(e),
    }
}

async fn delete_dictionary(State(s): State<BState>, Json(body): Json<Value>) -> axum::response::Response {
    let Some(id) = body["id"].as_i64() else { return err(StatusCode::BAD_REQUEST, "id required") };
    match sqlx::query("DELETE FROM data_dictionaries WHERE id=?").bind(id).execute(&s.pool).await {
        Ok(_) => { broadcast(&s, "dictionaries"); ok(json!({"deleted": id})) }
        Err(e) => server_err(e),
    }
}

async fn replace_dictionary_items(State(s): State<BState>, Json(body): Json<Value>) -> axum::response::Response {
    let Some(dict_id) = get_i64(&body, "dictionaryId", "dictionary_id") else {
        return err(StatusCode::BAD_REQUEST, "dictionaryId required");
    };
    let mut tx = match s.pool.begin().await {
        Ok(tx) => tx,
        Err(e) => return server_err(e),
    };
    if let Err(e) = sqlx::query("DELETE FROM dictionary_items WHERE dictionary_id=?").bind(dict_id).execute(&mut *tx).await {
        return server_err(e);
    }
    if let Some(items) = body["items"].as_array() {
        for (i, item) in items.iter().enumerate() {
            let r = sqlx::query(
                "INSERT INTO dictionary_items (dictionary_id, label, value, description, sort_order) \
                 VALUES (?, ?, ?, ?, ?)",
            )
            .bind(dict_id)
            .bind(item["label"].as_str().unwrap_or(""))
            .bind(item["value"].as_str().unwrap_or(""))
            .bind(item["description"].as_str().unwrap_or(""))
            .bind(item["sortOrder"].as_i64().or(item["sort_order"].as_i64()).unwrap_or(i as i64))
            .execute(&mut *tx)
            .await;
            if let Err(e) = r { return server_err(e); }
        }
    }
    match tx.commit().await {
        Ok(_) => {
            broadcast(&s, "dictionaries");
            let rows = sqlx::query_as::<_, DictionaryItem>(
                "SELECT id, dictionary_id, label, value, description, sort_order \
                 FROM dictionary_items WHERE dictionary_id=? ORDER BY sort_order, id",
            )
            .bind(dict_id)
            .fetch_all(&s.pool)
            .await
            .unwrap_or_default();
            ok(rows)
        }
        Err(e) => server_err(e),
    }
}

// ════════════════════════════════════════════════════════════
// 字段绑定
// ════════════════════════════════════════════════════════════

async fn list_field_rules(State(s): State<BState>, Query(q): Query<ProjectQuery>) -> axum::response::Response {
    match sqlx::query_as::<_, FieldDictionaryRule>(
        "SELECT id, project_id, field_name, dictionary_id FROM field_dictionary_rules WHERE project_id=?",
    )
    .bind(q.project_id)
    .fetch_all(&s.pool)
    .await
    {
        Ok(rows) => ok(rows),
        Err(e) => server_err(e),
    }
}

async fn set_field_rule(State(s): State<BState>, Json(body): Json<Value>) -> axum::response::Response {
    let Some(pid) = get_i64(&body, "projectId", "project_id") else {
        return err(StatusCode::BAD_REQUEST, "projectId required");
    };
    let Some(did) = get_i64(&body, "dictionaryId", "dictionary_id") else {
        return err(StatusCode::BAD_REQUEST, "dictionaryId required");
    };
    match sqlx::query(
        "INSERT INTO field_dictionary_rules (project_id, field_name, dictionary_id) VALUES (?, ?, ?) \
         ON CONFLICT(project_id, field_name) DO UPDATE SET dictionary_id=excluded.dictionary_id",
    )
    .bind(pid)
    .bind(get_str(&body, "fieldName", "field_name").unwrap_or(""))
    .bind(did)
    .execute(&s.pool)
    .await
    {
        Ok(_) => { broadcast(&s, "field_bindings"); ok(json!({"ok": true})) }
        Err(e) => server_err(e),
    }
}

async fn delete_field_rule(State(s): State<BState>, Json(body): Json<Value>) -> axum::response::Response {
    let Some(pid) = get_i64(&body, "projectId", "project_id") else {
        return err(StatusCode::BAD_REQUEST, "projectId required");
    };
    match sqlx::query("DELETE FROM field_dictionary_rules WHERE project_id=? AND field_name=?")
        .bind(pid)
        .bind(get_str(&body, "fieldName", "field_name").unwrap_or(""))
        .execute(&s.pool)
        .await
    {
        Ok(_) => { broadcast(&s, "field_bindings"); ok(json!({"ok": true})) }
        Err(e) => server_err(e),
    }
}

async fn list_field_overrides(State(s): State<BState>, Query(q): Query<FieldOverrideQuery>) -> axum::response::Response {
    let rows = if let Some(rid) = q.request_id {
        sqlx::query_as::<_, FieldDictionaryOverride>(
            "SELECT id, project_id, request_id, field_name, dictionary_id \
             FROM field_dictionary_overrides WHERE project_id=? AND request_id=?",
        )
        .bind(q.project_id)
        .bind(rid)
        .fetch_all(&s.pool)
        .await
    } else {
        sqlx::query_as::<_, FieldDictionaryOverride>(
            "SELECT id, project_id, request_id, field_name, dictionary_id \
             FROM field_dictionary_overrides WHERE project_id=?",
        )
        .bind(q.project_id)
        .fetch_all(&s.pool)
        .await
    };
    match rows {
        Ok(rows) => ok(rows),
        Err(e) => server_err(e),
    }
}

async fn set_field_override(State(s): State<BState>, Json(body): Json<Value>) -> axum::response::Response {
    let Some(pid) = get_i64(&body, "projectId", "project_id") else {
        return err(StatusCode::BAD_REQUEST, "projectId required");
    };
    let Some(rid) = get_i64(&body, "requestId", "request_id") else {
        return err(StatusCode::BAD_REQUEST, "requestId required");
    };
    // dictionaryId 可为 null（解绑）
    let did = get_i64(&body, "dictionaryId", "dictionary_id");
    match sqlx::query(
        "INSERT INTO field_dictionary_overrides (project_id, request_id, field_name, dictionary_id) \
         VALUES (?, ?, ?, ?) \
         ON CONFLICT(project_id, request_id, field_name) DO UPDATE SET dictionary_id=excluded.dictionary_id",
    )
    .bind(pid)
    .bind(rid)
    .bind(get_str(&body, "fieldName", "field_name").unwrap_or(""))
    .bind(did)
    .execute(&s.pool)
    .await
    {
        Ok(_) => { broadcast(&s, "field_bindings"); ok(json!({"ok": true})) }
        Err(e) => server_err(e),
    }
}

async fn delete_field_override(State(s): State<BState>, Json(body): Json<Value>) -> axum::response::Response {
    let Some(pid) = get_i64(&body, "projectId", "project_id") else {
        return err(StatusCode::BAD_REQUEST, "projectId required");
    };
    let Some(rid) = get_i64(&body, "requestId", "request_id") else {
        return err(StatusCode::BAD_REQUEST, "requestId required");
    };
    match sqlx::query(
        "DELETE FROM field_dictionary_overrides WHERE project_id=? AND request_id=? AND field_name=?",
    )
    .bind(pid)
    .bind(rid)
    .bind(get_str(&body, "fieldName", "field_name").unwrap_or(""))
    .execute(&s.pool)
    .await
    {
        Ok(_) => { broadcast(&s, "field_bindings"); ok(json!({"ok": true})) }
        Err(e) => server_err(e),
    }
}

// ════════════════════════════════════════════════════════════
// 发送请求 / 历史
// ════════════════════════════════════════════════════════════

async fn send_request(State(s): State<BState>, Json(body): Json<Value>) -> axum::response::Response {
    // body 直接是 SendRequestParams + 可选 requestId/testCaseId/envId/projectId
    let params: crate::http::client::SendRequestParams = match serde_json::from_value(body.clone()) {
        Ok(p) => p,
        Err(e) => return err(StatusCode::BAD_REQUEST, &format!("invalid params: {e}")),
    };
    let request_id = get_i64(&body, "requestId", "request_id");
    let test_case_id = get_i64(&body, "testCaseId", "test_case_id");
    let env_id = get_i64(&body, "envId", "env_id");
    let project_id = get_i64(&body, "projectId", "project_id");

    match crate::commands::send_request::send_request_impl(
        &s.pool, &s.http, request_id, test_case_id, params, env_id, project_id,
    ).await {
        Ok(resp) => ok(resp),
        Err(e) => server_err(e),
    }
}

async fn list_history(State(s): State<BState>, Query(q): Query<RequestQuery>) -> axum::response::Response {
    match sqlx::query_as::<_, HistoryRecord>(
        "SELECT id, request_id, test_case_id, status_code, response_time_ms, request_snapshot, \
         response_body, is_truncated, response_headers, error_message, created_at \
         FROM request_history WHERE request_id=? AND (?2 IS NULL OR test_case_id=?2) \
         ORDER BY created_at DESC LIMIT 20",
    )
    .bind(q.request_id)
    .bind(q.test_case_id)
    .fetch_all(&s.pool)
    .await
    {
        Ok(rows) => ok(rows),
        Err(e) => server_err(e),
    }
}

async fn cleanup_history(State(s): State<BState>, Json(body): Json<Value>) -> axum::response::Response {
    let params = crate::commands::send_request::CleanupHistoryParams {
        days: body["days"].as_i64(),
        keep_per_request: body["keep_per_request"].as_i64().or(body["keepPerRequest"].as_i64()),
        request_id: body["request_id"].as_i64().or(body["requestId"].as_i64()),
        project_id: body["project_id"].as_i64().or(body["projectId"].as_i64()),
        cleanup_files: body["cleanup_files"].as_bool().or(body["cleanupFiles"].as_bool()).unwrap_or(true),
    };

    match crate::commands::send_request::cleanup_history_impl(&s.pool, params).await {
        Ok(result) => { broadcast(&s, "history"); ok(result) }
        Err(e) => server_err(e),
    }
}

// ════════════════════════════════════════════════════════════
// 压测
// ════════════════════════════════════════════════════════════

async fn start_stress(State(s): State<BState>, Json(body): Json<Value>) -> axum::response::Response {
    let Some(request_id) = get_i64(&body, "requestId", "request_id") else {
        return err(StatusCode::BAD_REQUEST, "requestId required");
    };
    let params: crate::http::client::SendRequestParams = match serde_json::from_value(body["params"].clone()) {
        Ok(p) => p,
        Err(e) => return err(StatusCode::BAD_REQUEST, &format!("invalid params: {e}")),
    };
    let concurrent = body["concurrent"].as_u64().unwrap_or(10) as u32;
    let mode = body["mode"].as_str().unwrap_or("count").to_string();
    let value = body["value"].as_u64().unwrap_or(100);
    // 兼容两种写法，与 requestId / request_id 的处理保持一致
    let expect_status = body["expectStatus"]
        .as_str()
        .or(body["expect_status"].as_str())
        .unwrap_or(crate::commands::stress::DEFAULT_EXPECT_STATUS)
        .to_string();
    let p95_threshold_ms = body["p95ThresholdMs"]
        .as_u64()
        .or(body["p95_threshold_ms"].as_u64());
    let p99_threshold_ms = body["p99ThresholdMs"]
        .as_u64()
        .or(body["p99_threshold_ms"].as_u64());

    match crate::commands::stress::start_stress_impl(
        &s.app, &s.pool, request_id, params, concurrent, &mode, value, &expect_status,
        p95_threshold_ms, p99_threshold_ms,
    ).await {
        Ok(stats) => { broadcast(&s, "stress"); ok(stats) }
        Err(e) => server_err(e),
    }
}

async fn list_stress_runs(State(s): State<BState>, Query(q): Query<RequestQuery>) -> axum::response::Response {
    match sqlx::query_as::<_, StressRun>(
        "SELECT id, request_id, config_json, stats_json, created_at \
         FROM stress_runs WHERE request_id=? ORDER BY created_at DESC LIMIT 50",
    )
    .bind(q.request_id)
    .fetch_all(&s.pool)
    .await
    {
        Ok(rows) => ok(rows),
        Err(e) => server_err(e),
    }
}

/// GET /stress_report?run_id=N —— 取某条压测历史的报告（Markdown）。
/// 与 App 预览走的是同一个 `build_stress_report_by_id`，内容完全同源。
async fn stress_report(State(s): State<BState>, Query(q): Query<StressReportQuery>) -> axum::response::Response {
    match crate::commands::stress::build_stress_report_by_id(&s.pool, q.run_id).await {
        Ok(md) => ok(md),
        Err(e) => server_err(e),
    }
}

// ════════════════════════════════════════════════════════════
// 环境
// ════════════════════════════════════════════════════════════

async fn list_environments(State(s): State<BState>, Query(q): Query<ProjectQuery>) -> axum::response::Response {
    let sql = format!(
        "SELECT {ENV_COLS} FROM environments WHERE project_id=? ORDER BY created_at DESC, id DESC"
    );
    match sqlx::query_as::<_, Environment>(&sql)
        .bind(q.project_id)
        .fetch_all(&s.pool)
        .await
    {
        Ok(rows) => ok(rows),
        Err(e) => server_err(e),
    }
}

async fn list_env_variables(State(s): State<BState>, Query(q): Query<IdQuery>) -> axum::response::Response {
    match sqlx::query_as::<_, EnvVariable>(
        "SELECT id, env_id, key, value, description, enabled FROM env_variables WHERE env_id=? ORDER BY id",
    )
    .bind(q.id)
    .fetch_all(&s.pool)
    .await
    {
        Ok(rows) => ok(rows),
        Err(e) => server_err(e),
    }
}

// ── 环境写操作 ──────────────────────────────────────────────

async fn create_environment(State(s): State<BState>, Json(body): Json<Value>) -> axum::response::Response {
    let Some(pid) = get_i64(&body, "projectId", "project_id") else {
        return err(StatusCode::BAD_REQUEST, "projectId required");
    };
    let sql = format!("INSERT INTO environments (project_id, name, base_url) VALUES (?, ?, ?) RETURNING {ENV_COLS}");
    match sqlx::query_as::<_, Environment>(&sql)
        .bind(pid)
        .bind(body["name"].as_str().unwrap_or("未命名环境"))
        .bind(get_str(&body, "baseUrl", "base_url"))
        .fetch_one(&s.pool)
        .await
    {
        Ok(row) => { broadcast(&s, "environments"); ok(row) }
        Err(e) => server_err(e),
    }
}

async fn update_environment(State(s): State<BState>, Json(body): Json<Value>) -> axum::response::Response {
    let Some(id) = body["id"].as_i64() else { return err(StatusCode::BAD_REQUEST, "id required") };
    let sql = format!(
        "UPDATE environments SET name=COALESCE(?, name), base_url=COALESCE(?, base_url) \
         WHERE id=? RETURNING {ENV_COLS}"
    );
    match sqlx::query_as::<_, Environment>(&sql)
        .bind(body["name"].as_str())
        .bind(get_str(&body, "baseUrl", "base_url"))
        .bind(id)
        .fetch_one(&s.pool)
        .await
    {
        Ok(row) => { broadcast(&s, "environments"); ok(row) }
        Err(e) => server_err(e),
    }
}

async fn delete_environment(State(s): State<BState>, Json(body): Json<Value>) -> axum::response::Response {
    let Some(id) = body["id"].as_i64() else { return err(StatusCode::BAD_REQUEST, "id required") };
    match sqlx::query("DELETE FROM environments WHERE id=?").bind(id).execute(&s.pool).await {
        Ok(_) => { broadcast(&s, "environments"); ok(json!({"deleted": id})) }
        Err(e) => server_err(e),
    }
}

async fn activate_environment(State(s): State<BState>, Json(body): Json<Value>) -> axum::response::Response {
    let Some(pid) = get_i64(&body, "projectId", "project_id") else {
        return err(StatusCode::BAD_REQUEST, "projectId required");
    };
    let Some(eid) = get_i64(&body, "envId", "env_id") else {
        return err(StatusCode::BAD_REQUEST, "envId required");
    };
    let mut tx = match s.pool.begin().await { Ok(t) => t, Err(e) => return server_err(e) };
    if let Err(e) = sqlx::query("UPDATE environments SET is_active=0 WHERE project_id=?").bind(pid).execute(&mut *tx).await {
        return server_err(e);
    }
    if let Err(e) = sqlx::query("UPDATE environments SET is_active=1 WHERE id=? AND project_id=?").bind(eid).bind(pid).execute(&mut *tx).await {
        return server_err(e);
    }
    match tx.commit().await {
        Ok(_) => { broadcast(&s, "environments"); ok(json!({"activated": eid})) }
        Err(e) => server_err(e),
    }
}

async fn deactivate_environment(State(s): State<BState>, Json(body): Json<Value>) -> axum::response::Response {
    let Some(pid) = get_i64(&body, "projectId", "project_id") else {
        return err(StatusCode::BAD_REQUEST, "projectId required");
    };
    match sqlx::query("UPDATE environments SET is_active=0 WHERE project_id=?").bind(pid).execute(&s.pool).await {
        Ok(_) => { broadcast(&s, "environments"); ok(json!({"ok": true})) }
        Err(e) => server_err(e),
    }
}

// ── 环境变量写操作 ──────────────────────────────────────────

async fn create_env_variable(State(s): State<BState>, Json(body): Json<Value>) -> axum::response::Response {
    let Some(eid) = get_i64(&body, "envId", "env_id") else {
        return err(StatusCode::BAD_REQUEST, "envId required");
    };
    let sql = format!("INSERT INTO env_variables (env_id, key, value, description) VALUES (?, ?, ?, ?) RETURNING {ENV_VAR_COLS}");
    match sqlx::query_as::<_, EnvVariable>(&sql)
        .bind(eid)
        .bind(body["key"].as_str().unwrap_or(""))
        .bind(body["value"].as_str().unwrap_or(""))
        .bind(body["description"].as_str())
        .fetch_one(&s.pool)
        .await
    {
        Ok(row) => { broadcast(&s, "environments"); ok(row) }
        Err(e) => server_err(e),
    }
}

async fn update_env_variable(State(s): State<BState>, Json(body): Json<Value>) -> axum::response::Response {
    let Some(id) = body["id"].as_i64() else { return err(StatusCode::BAD_REQUEST, "id required") };
    let sql = format!(
        "UPDATE env_variables SET key=COALESCE(?, key), value=COALESCE(?, value), \
         description=COALESCE(?, description), enabled=COALESCE(?, enabled) \
         WHERE id=? RETURNING {ENV_VAR_COLS}"
    );
    match sqlx::query_as::<_, EnvVariable>(&sql)
        .bind(body["key"].as_str())
        .bind(body["value"].as_str())
        .bind(body["description"].as_str())
        .bind(body["enabled"].as_i64())
        .bind(id)
        .fetch_one(&s.pool)
        .await
    {
        Ok(row) => { broadcast(&s, "environments"); ok(row) }
        Err(e) => server_err(e),
    }
}

async fn delete_env_variable(State(s): State<BState>, Json(body): Json<Value>) -> axum::response::Response {
    let Some(id) = body["id"].as_i64() else { return err(StatusCode::BAD_REQUEST, "id required") };
    match sqlx::query("DELETE FROM env_variables WHERE id=?").bind(id).execute(&s.pool).await {
        Ok(_) => { broadcast(&s, "environments"); ok(json!({"deleted": id})) }
        Err(e) => server_err(e),
    }
}

// ── Cookie ──────────────────────────────────────────────────

async fn list_cookies(State(s): State<BState>, Query(q): Query<serde_json::Value>) -> axum::response::Response {
    let scope = q.get("scope_type").and_then(|v| v.as_str()).unwrap_or("global");
    let pid = q.get("project_id").and_then(|v| v.as_str()).and_then(|v| v.parse::<i64>().ok());
    let sql = if scope == "global" {
        format!("SELECT {COOKIE_COLS} FROM cookies WHERE scope_type='global' ORDER BY id DESC")
    } else {
        format!("SELECT {COOKIE_COLS} FROM cookies WHERE scope_type='project' AND project_id=? ORDER BY id DESC")
    };
    let mut query = sqlx::query_as::<_, Cookie>(&sql);
    if scope != "global" {
        query = query.bind(pid.unwrap_or(0));
    }
    match query.fetch_all(&s.pool).await {
        Ok(rows) => ok(rows),
        Err(e) => server_err(e),
    }
}

async fn create_cookie(State(s): State<BState>, Json(body): Json<Value>) -> axum::response::Response {
    let sql = format!(
        "INSERT INTO cookies (scope_type, project_id, domain, name, value, path) \
         VALUES (?, ?, ?, ?, ?, ?) RETURNING {COOKIE_COLS}"
    );
    match sqlx::query_as::<_, Cookie>(&sql)
        .bind(get_str(&body, "scopeType", "scope_type").unwrap_or("global"))
        .bind(get_i64(&body, "projectId", "project_id"))
        .bind(body["domain"].as_str().unwrap_or(""))
        .bind(body["name"].as_str().unwrap_or(""))
        .bind(body["value"].as_str().unwrap_or(""))
        .bind(body["path"].as_str().unwrap_or("/"))
        .fetch_one(&s.pool)
        .await
    {
        Ok(row) => { broadcast(&s, "cookies"); ok(row) }
        Err(e) => server_err(e),
    }
}

async fn update_cookie(State(s): State<BState>, Json(body): Json<Value>) -> axum::response::Response {
    let Some(id) = body["id"].as_i64() else { return err(StatusCode::BAD_REQUEST, "id required") };
    let sql = format!(
        "UPDATE cookies SET value=COALESCE(?, value), path=COALESCE(?, path), \
         enabled=COALESCE(?, enabled) WHERE id=? RETURNING {COOKIE_COLS}"
    );
    match sqlx::query_as::<_, Cookie>(&sql)
        .bind(body["value"].as_str())
        .bind(body["path"].as_str())
        .bind(body["enabled"].as_i64())
        .bind(id)
        .fetch_one(&s.pool)
        .await
    {
        Ok(row) => { broadcast(&s, "cookies"); ok(row) }
        Err(e) => server_err(e),
    }
}

async fn delete_cookie(State(s): State<BState>, Json(body): Json<Value>) -> axum::response::Response {
    let Some(id) = body["id"].as_i64() else { return err(StatusCode::BAD_REQUEST, "id required") };
    match sqlx::query("DELETE FROM cookies WHERE id=?").bind(id).execute(&s.pool).await {
        Ok(_) => { broadcast(&s, "cookies"); ok(json!({"deleted": id})) }
        Err(e) => server_err(e),
    }
}
