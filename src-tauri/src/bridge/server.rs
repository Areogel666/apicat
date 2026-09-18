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
    let auth = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let token = auth.strip_prefix("Bearer ").unwrap_or("");
    if token != state.token {
        return err(StatusCode::UNAUTHORIZED, "invalid or missing token").into_response();
    }
    next.run(request).await
}

// ── data-changed 广播 ───────────────────────────────────────

fn broadcast(state: &BState, kind: &str) {
    let _ = state.app.emit("bridge-data-changed", json!({ "kind": kind }));
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
struct DictQuery {
    dictionary_id: i64,
}
#[derive(Deserialize)]
struct FieldRuleQuery {
    project_id: i64,
    field_name: Option<String>,
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
        // 压测
        .route("/start_stress", post(start_stress))
        .route("/list_stress_runs", get(list_stress_runs))
        // 环境
        .route("/list_environments", get(list_environments))
        .route("/list_env_variables", get(list_env_variables))
        // 健康检查（无需鉴权，在 middleware 之前）
        .route("/health", get(|| async { ok(json!({"status": "up"})) }));

    api.layer(axum::middleware::from_fn_with_state(state.clone(), auth_middleware))
        .with_state(state)
}

// ════════════════════════════════════════════════════════════
// 项目
// ════════════════════════════════════════════════════════════

async fn list_projects(State(s): State<BState>) -> axum::response::Response {
    match sqlx::query_as::<_, Project>(
        "SELECT id, name, description, docs_output_dir, created_at, updated_at FROM projects ORDER BY created_at DESC",
    )
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
    match sqlx::query_as::<_, Project>(
        "INSERT INTO projects (name, description) VALUES (?, ?) \
         RETURNING id, name, description, docs_output_dir, created_at, updated_at",
    )
    .bind(name)
    .bind(body["description"].as_str())
    .fetch_one(&s.pool)
    .await
    {
        Ok(row) => { broadcast(&s, "projects"); ok(row) }
        Err(e) => server_err(e),
    }
}

async fn update_project(State(s): State<BState>, Json(body): Json<Value>) -> axum::response::Response {
    let Some(id) = body["id"].as_i64() else { return err(StatusCode::BAD_REQUEST, "id required") };
    match sqlx::query_as::<_, Project>(
        "UPDATE projects SET name=?, description=?, docs_output_dir=?, updated_at=datetime('now') \
         WHERE id=? RETURNING id, name, description, docs_output_dir, created_at, updated_at",
    )
    .bind(body["name"].as_str().unwrap_or(""))
    .bind(body["description"].as_str())
    .bind(body["docsOutputDir"].as_str().or(body["docs_output_dir"].as_str()))
    .bind(id)
    .fetch_one(&s.pool)
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
    let Some(pid) = body["projectId"].as_i64().or(body["project_id"].as_i64()) else {
        return err(StatusCode::BAD_REQUEST, "projectId required");
    };
    match sqlx::query_as::<_, Collection>(
        "INSERT INTO collections (project_id, parent_id, name) VALUES (?, ?, ?) \
         RETURNING id, project_id, parent_id, name, sort_order, created_at",
    )
    .bind(pid)
    .bind(body["parentId"].as_i64().or(body["parent_id"].as_i64()))
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

const REQ_COLS: &str = "id, collection_id, name, method, url, params, headers, body_type, body, \
    auth_type, auth_config, description, sort_order, created_at, updated_at";

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
    let Some(cid) = body["collectionId"].as_i64().or(body["collection_id"].as_i64()) else {
        return err(StatusCode::BAD_REQUEST, "collectionId required");
    };
    let sql = format!(
        "INSERT INTO api_requests (collection_id, name, method, url) VALUES (?, ?, ?, ?) \
         RETURNING {REQ_COLS}"
    );
    match sqlx::query_as::<_, ApiRequest>(&sql)
        .bind(cid)
        .bind(body["name"].as_str().unwrap_or("未命名接口"))
        .bind(body["method"].as_str().unwrap_or("GET"))
        .bind(body["url"].as_str().unwrap_or(""))
        .fetch_one(&s.pool)
        .await
    {
        Ok(row) => { broadcast(&s, "requests"); ok(row) }
        Err(e) => server_err(e),
    }
}

async fn update_request(State(s): State<BState>, Json(body): Json<Value>) -> axum::response::Response {
    let Some(id) = body["id"].as_i64() else { return err(StatusCode::BAD_REQUEST, "id required") };
    let sql = format!(
        "UPDATE api_requests SET name=?, method=?, url=?, params=?, headers=?, \
         body_type=?, body=?, auth_type=?, auth_config=?, description=?, updated_at=datetime('now') \
         WHERE id=? RETURNING {REQ_COLS}"
    );
    match sqlx::query_as::<_, ApiRequest>(&sql)
        .bind(body["name"].as_str().unwrap_or(""))
        .bind(body["method"].as_str().unwrap_or("GET"))
        .bind(body["url"].as_str().unwrap_or(""))
        .bind(body["params"].as_str().unwrap_or("[]"))
        .bind(body["headers"].as_str().unwrap_or("[]"))
        .bind(body["bodyType"].as_str().or(body["body_type"].as_str()).unwrap_or("none"))
        .bind(body["body"].as_str().unwrap_or(""))
        .bind(body["authType"].as_str().or(body["auth_type"].as_str()).unwrap_or("none"))
        .bind(body["authConfig"].as_str().or(body["auth_config"].as_str()).unwrap_or("{}"))
        .bind(body["description"].as_str().unwrap_or(""))
        .bind(id)
        .fetch_one(&s.pool)
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
    let sql = format!(
        "INSERT INTO api_requests (collection_id, name, method, url, params, headers, body_type, body, auth_type, auth_config, description) \
         SELECT collection_id, name || ' 副本', method, url, params, headers, body_type, body, auth_type, auth_config, description \
         FROM api_requests WHERE id=? RETURNING {REQ_COLS}"
    );
    match sqlx::query_as::<_, ApiRequest>(&sql).bind(id).fetch_one(&s.pool).await {
        Ok(row) => { broadcast(&s, "requests"); ok(row) }
        Err(e) => server_err(e),
    }
}

// ════════════════════════════════════════════════════════════
// 用例
// ════════════════════════════════════════════════════════════

const TC_COLS: &str = "id, request_id, collection_id, name, description, source, case_type, \
    method, url, headers, params, body_type, body, assertions, last_run_at, last_status, \
    last_duration_ms, last_response, starred, enabled, sort_order, created_at, updated_at";

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
    let Some(rid) = body["requestId"].as_i64().or(body["request_id"].as_i64()) else {
        return err(StatusCode::BAD_REQUEST, "requestId required");
    };
    let Some(cid) = body["collectionId"].as_i64().or(body["collection_id"].as_i64()) else {
        return err(StatusCode::BAD_REQUEST, "collectionId required");
    };
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM test_cases WHERE request_id=?")
        .bind(rid).fetch_one(&s.pool).await.unwrap_or(0);
    let starred: i64 = if count == 0 { 1 } else { 0 };
    let name = body["name"].as_str().unwrap_or("");
    let final_name = if name.is_empty() { format!("用例 {}", count + 1) } else { name.to_string() };

    let sql = format!(
        "INSERT INTO test_cases (request_id, collection_id, name, method, url, headers, params, body_type, body, case_type, assertions, starred, sort_order) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) RETURNING {TC_COLS}"
    );
    match sqlx::query_as::<_, TestCase>(&sql)
        .bind(rid).bind(cid).bind(&final_name)
        .bind(body["method"].as_str())
        .bind(body["url"].as_str())
        .bind(body["headers"].as_str().unwrap_or("[]"))
        .bind(body["params"].as_str().unwrap_or("[]"))
        .bind(body["bodyType"].as_str().or(body["body_type"].as_str()))
        .bind(body["body"].as_str())
        .bind(body["caseType"].as_str().or(body["case_type"].as_str()).unwrap_or("happy_path"))
        .bind(body["assertions"].as_str().unwrap_or("[]"))
        .bind(starred)
        .bind(count)
        .fetch_one(&s.pool)
        .await
    {
        Ok(row) => { broadcast(&s, "test_cases"); ok(row) }
        Err(e) => server_err(e),
    }
}

async fn update_test_case(State(s): State<BState>, Json(body): Json<Value>) -> axum::response::Response {
    let Some(id) = body["id"].as_i64() else { return err(StatusCode::BAD_REQUEST, "id required") };
    let sql = format!(
        "UPDATE test_cases SET name=COALESCE(?, name), starred=COALESCE(?, starred), \
         method=COALESCE(?, method), url=COALESCE(?, url), \
         headers=COALESCE(?, headers), params=COALESCE(?, params), \
         body_type=COALESCE(?, body_type), body=COALESCE(?, body), \
         case_type=COALESCE(?, case_type), assertions=COALESCE(?, assertions), \
         updated_at=datetime('now') WHERE id=? RETURNING {TC_COLS}"
    );
    match sqlx::query_as::<_, TestCase>(&sql)
        .bind(body["name"].as_str())
        .bind(body["starred"].as_i64())
        .bind(body["method"].as_str())
        .bind(body["url"].as_str())
        .bind(body["headers"].as_str())
        .bind(body["params"].as_str())
        .bind(body["bodyType"].as_str().or(body["body_type"].as_str()))
        .bind(body["body"].as_str())
        .bind(body["caseType"].as_str().or(body["case_type"].as_str()))
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
    match sqlx::query("DELETE FROM test_cases WHERE id=?").bind(id).execute(&s.pool).await {
        Ok(_) => { broadcast(&s, "test_cases"); ok(json!({"deleted": id})) }
        Err(e) => server_err(e),
    }
}

async fn list_test_case_history(State(s): State<BState>, Query(q): Query<IdQuery>) -> axum::response::Response {
    match sqlx::query_as::<_, TestCaseHistory>(
        "SELECT id, test_case_id, status_code, duration_ms, response_preview, error_message, created_at \
         FROM test_case_history WHERE test_case_id=? ORDER BY created_at DESC, id DESC LIMIT 10",
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
    let Some(tc_id) = body["testCaseId"].as_i64().or(body["test_case_id"].as_i64()) else {
        return err(StatusCode::BAD_REQUEST, "testCaseId required");
    };
    let env_id = body["envId"].as_i64().or(body["env_id"].as_i64());
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
    match sqlx::query_as::<_, DataDictionary>(
        "SELECT id, code, name, description, builtin, project_id, created_at, updated_at \
         FROM data_dictionaries WHERE builtin=1 OR project_id=? ORDER BY builtin DESC, code",
    )
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
    let sql = "INSERT INTO data_dictionaries (code, name, description, project_id) VALUES (?, ?, ?, ?) \
         RETURNING id, code, name, description, builtin, project_id, created_at, updated_at";
    match sqlx::query_as::<_, DataDictionary>(sql)
        .bind(body["code"].as_str().unwrap_or(""))
        .bind(body["name"].as_str().unwrap_or(""))
        .bind(body["description"].as_str().unwrap_or(""))
        .bind(body["projectId"].as_i64().or(body["project_id"].as_i64()))
        .fetch_one(&s.pool)
        .await
    {
        Ok(row) => { broadcast(&s, "dictionaries"); ok(row) }
        Err(e) => server_err(e),
    }
}

async fn create_dictionary_with_items(State(s): State<BState>, Json(body): Json<Value>) -> axum::response::Response {
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
    .bind(body["projectId"].as_i64().or(body["project_id"].as_i64()))
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
    let Some(dict_id) = body["dictionaryId"].as_i64().or(body["dictionary_id"].as_i64()) else {
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
    let Some(pid) = body["projectId"].as_i64().or(body["project_id"].as_i64()) else {
        return err(StatusCode::BAD_REQUEST, "projectId required");
    };
    let Some(did) = body["dictionaryId"].as_i64().or(body["dictionary_id"].as_i64()) else {
        return err(StatusCode::BAD_REQUEST, "dictionaryId required");
    };
    match sqlx::query(
        "INSERT INTO field_dictionary_rules (project_id, field_name, dictionary_id) VALUES (?, ?, ?) \
         ON CONFLICT(project_id, field_name) DO UPDATE SET dictionary_id=excluded.dictionary_id",
    )
    .bind(pid)
    .bind(body["fieldName"].as_str().or(body["field_name"].as_str()).unwrap_or(""))
    .bind(did)
    .execute(&s.pool)
    .await
    {
        Ok(_) => { broadcast(&s, "field_bindings"); ok(json!({"ok": true})) }
        Err(e) => server_err(e),
    }
}

async fn delete_field_rule(State(s): State<BState>, Json(body): Json<Value>) -> axum::response::Response {
    let Some(pid) = body["projectId"].as_i64().or(body["project_id"].as_i64()) else {
        return err(StatusCode::BAD_REQUEST, "projectId required");
    };
    match sqlx::query("DELETE FROM field_dictionary_rules WHERE project_id=? AND field_name=?")
        .bind(pid)
        .bind(body["fieldName"].as_str().or(body["field_name"].as_str()).unwrap_or(""))
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
    let Some(pid) = body["projectId"].as_i64().or(body["project_id"].as_i64()) else {
        return err(StatusCode::BAD_REQUEST, "projectId required");
    };
    let Some(rid) = body["requestId"].as_i64().or(body["request_id"].as_i64()) else {
        return err(StatusCode::BAD_REQUEST, "requestId required");
    };
    // dictionaryId 可为 null（解绑）
    let did = body["dictionaryId"].as_i64().or(body["dictionary_id"].as_i64());
    match sqlx::query(
        "INSERT INTO field_dictionary_overrides (project_id, request_id, field_name, dictionary_id) \
         VALUES (?, ?, ?, ?) \
         ON CONFLICT(project_id, request_id, field_name) DO UPDATE SET dictionary_id=excluded.dictionary_id",
    )
    .bind(pid)
    .bind(rid)
    .bind(body["fieldName"].as_str().or(body["field_name"].as_str()).unwrap_or(""))
    .bind(did)
    .execute(&s.pool)
    .await
    {
        Ok(_) => { broadcast(&s, "field_bindings"); ok(json!({"ok": true})) }
        Err(e) => server_err(e),
    }
}

async fn delete_field_override(State(s): State<BState>, Json(body): Json<Value>) -> axum::response::Response {
    let Some(pid) = body["projectId"].as_i64().or(body["project_id"].as_i64()) else {
        return err(StatusCode::BAD_REQUEST, "projectId required");
    };
    let Some(rid) = body["requestId"].as_i64().or(body["request_id"].as_i64()) else {
        return err(StatusCode::BAD_REQUEST, "requestId required");
    };
    match sqlx::query(
        "DELETE FROM field_dictionary_overrides WHERE project_id=? AND request_id=? AND field_name=?",
    )
    .bind(pid)
    .bind(rid)
    .bind(body["fieldName"].as_str().or(body["field_name"].as_str()).unwrap_or(""))
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
    let request_id = body["requestId"].as_i64().or(body["request_id"].as_i64());
    let test_case_id = body["testCaseId"].as_i64().or(body["test_case_id"].as_i64());
    let env_id = body["envId"].as_i64().or(body["env_id"].as_i64());
    let project_id = body["projectId"].as_i64().or(body["project_id"].as_i64());

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
         response_body, is_truncated, response_headers, created_at \
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

// ════════════════════════════════════════════════════════════
// 压测
// ════════════════════════════════════════════════════════════

async fn start_stress(State(s): State<BState>, Json(body): Json<Value>) -> axum::response::Response {
    let Some(request_id) = body["requestId"].as_i64().or(body["request_id"].as_i64()) else {
        return err(StatusCode::BAD_REQUEST, "requestId required");
    };
    let params: crate::http::client::SendRequestParams = match serde_json::from_value(body["params"].clone()) {
        Ok(p) => p,
        Err(e) => return err(StatusCode::BAD_REQUEST, &format!("invalid params: {e}")),
    };
    let concurrent = body["concurrent"].as_u64().unwrap_or(10) as u32;
    let mode = body["mode"].as_str().unwrap_or("count").to_string();
    let value = body["value"].as_u64().unwrap_or(100);

    match crate::commands::stress::start_stress_impl(
        &s.app, &s.pool, request_id, params, concurrent, &mode, value,
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

// ════════════════════════════════════════════════════════════
// 环境
// ════════════════════════════════════════════════════════════

async fn list_environments(State(s): State<BState>, Query(q): Query<ProjectQuery>) -> axum::response::Response {
    match sqlx::query_as::<_, Environment>(
        "SELECT id, project_id, name, base_url, is_active, created_at FROM environments WHERE project_id=? ORDER BY id",
    )
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
