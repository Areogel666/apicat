use crate::{db::AppDb, error::CmdResult, types::{TestCase, TestCaseHistory}};
use tauri::State;

use crate::sql_cols::TEST_CASE_COLS as SELECT_COLS;

/// 获取某接口的所有测试用例（按 sort_order + id）
#[tauri::command]
pub async fn list_test_cases(
    db: State<'_, AppDb>,
    request_id: i64,
) -> CmdResult<Vec<TestCase>> {
    let sql = format!(
        "SELECT {SELECT_COLS} FROM test_cases WHERE request_id=? AND enabled=1 ORDER BY sort_order, id"
    );
    let rows = sqlx::query_as::<_, TestCase>(&sql)
        .bind(request_id)
        .fetch_all(&db.0)
        .await?;
    Ok(rows)
}

/// 创建测试用例
/// 若该接口当前没有任何用例，自动将新用例标记为收藏（starred=1）
/// 创建测试用例。UI 与 Bridge 共用。
///
/// `description` / `source` / `assertions` 为可选：缺省时 description 落 NULL、
/// source 落表默认 'manual'、assertions 落 '[]'。
/// `case_type` 必填 —— 缺省兜底会把 7 种用例类型静默全标成 happy_path，
/// 由调用方决定默认值（UI 侧传 happy_path，Bridge 侧缺失则报错）。
pub async fn create_test_case_impl(
    pool: &sqlx::SqlitePool,
    request_id: i64,
    collection_id: i64,
    name: &str,
    description: Option<&str>,
    source: &str,
    method: Option<&str>,
    url: Option<&str>,
    headers: Option<&str>,
    params: Option<&str>,
    body_type: Option<&str>,
    body: Option<&str>,
    case_type: &str,
    assertions: Option<&str>,
) -> Result<TestCase, crate::error::AppError> {
    // 统计当前用例数，用于自动命名与末尾插入排序
    let existing_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM test_cases WHERE request_id=?",
    )
    .bind(request_id)
    .fetch_one(pool)
    .await?;

    // 自动命名：「用例 N」（N = existing_count + 1）
    let final_name = if name.is_empty() {
        format!("用例 {}", existing_count + 1)
    } else {
        name.to_string()
    };

    let sql = format!(
        "INSERT INTO test_cases \
            (request_id, collection_id, name, description, source, method, url, headers, params, body_type, body, case_type, assertions, sort_order) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) \
         RETURNING {SELECT_COLS}"
    );
    let row = sqlx::query_as::<_, TestCase>(&sql)
        .bind(request_id)
        .bind(collection_id)
        .bind(&final_name)
        .bind(description)
        .bind(source)
        .bind(method)
        .bind(url)
        .bind(headers.unwrap_or("[]"))
        .bind(params.unwrap_or("[]"))
        .bind(body_type)
        .bind(body)
        .bind(case_type)
        .bind(assertions.unwrap_or("[]"))
        .bind(existing_count)   // sort_order = 当前用例数（末尾插入）
        .fetch_one(pool)
        .await?;
    Ok(row)
}

#[tauri::command]
pub async fn create_test_case(
    db: State<'_, AppDb>,
    request_id: i64,
    collection_id: i64,
    name: String,
    method: Option<String>,
    url: Option<String>,
    headers: Option<String>,
    params: Option<String>,
    body_type: Option<String>,
    body: Option<String>,
    case_type: Option<String>,
) -> CmdResult<TestCase> {
    create_test_case_impl(
        &db.0,
        request_id,
        collection_id,
        &name,
        None,
        "manual",
        method.as_deref(),
        url.as_deref(),
        headers.as_deref(),
        params.as_deref(),
        body_type.as_deref(),
        body.as_deref(),
        case_type.as_deref().unwrap_or("happy_path"),
        None,
    )
    .await
}

/// 更新测试用例名称 / 请求参数 / 类型 / 断言
#[tauri::command]
pub async fn update_test_case(
    db: State<'_, AppDb>,
    id: i64,
    name: String,
    method: Option<String>,
    url: Option<String>,
    headers: Option<String>,
    params: Option<String>,
    body_type: Option<String>,
    body: Option<String>,
    case_type: Option<String>,
    assertions: Option<String>,
) -> CmdResult<TestCase> {
    let sql = format!(
        "UPDATE test_cases SET name=?, method=?, url=?, headers=?, params=?, \
         body_type=?, body=?, case_type=COALESCE(?, case_type), \
         assertions=COALESCE(?, assertions), updated_at=datetime('now') \
         WHERE id=? RETURNING {SELECT_COLS}"
    );
    let row = sqlx::query_as::<_, TestCase>(&sql)
        .bind(&name)
        .bind(&method)
        .bind(&url)
        .bind(headers.as_deref().unwrap_or("[]"))
        .bind(params.as_deref().unwrap_or("[]"))
        .bind(&body_type)
        .bind(&body)
        .bind(&case_type)
        .bind(&assertions)
        .bind(id)
        .fetch_one(&db.0)
        .await?;
    Ok(row)
}

/// 删除测试用例（1.0.5 移除收藏后不再有「最后一个收藏不可删」约束）。
/// UI 与 Bridge 共用此实现，避免两侧业务分叉。
pub async fn delete_test_case_impl(
    pool: &sqlx::SqlitePool,
    id: i64,
) -> Result<(), crate::error::AppError> {
    sqlx::query("DELETE FROM test_cases WHERE id=?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

/// 若为该接口最后一个收藏用例，返回错误（禁止删除）
#[tauri::command]
pub async fn delete_test_case(db: State<'_, AppDb>, id: i64) -> CmdResult<()> {
    delete_test_case_impl(&db.0, id).await
}

// ── 用例执行历史（合并到 request_history）────────────────

/// 列出某用例的最近 10 条历史调用（按时间倒序）
/// 从 request_history 查询，用 AS 别名映射到 TestCaseHistory 结构
#[tauri::command]
pub async fn list_test_case_history(
    db: State<'_, AppDb>,
    test_case_id: i64,
) -> CmdResult<Vec<TestCaseHistory>> {
    let rows = sqlx::query_as::<_, TestCaseHistory>(
        "SELECT id, test_case_id, status_code, \
                response_time_ms as duration_ms, \
                substr(response_body, 1, 1024) as response_preview, \
                error_message, created_at \
         FROM request_history \
         WHERE test_case_id = ? \
         ORDER BY created_at DESC, id DESC LIMIT 10"
    )
    .bind(test_case_id)
    .fetch_all(&db.0)
    .await?;
    Ok(rows)
}

/// 批量删除用例（一次 DB 往返）。
/// 关联的 test_case_history 由 FK CASCADE 自动清理。
/// 注意：批量删除不走 delete_test_case 的"最后一个收藏用例"保护逻辑——
/// 这是 UI 主动的批量操作，由前端 NPopconfirm 二次确认兜底。
#[tauri::command]
pub async fn delete_test_cases(
    db: State<'_, AppDb>,
    ids: Vec<i64>,
) -> CmdResult<u64> {
    if ids.is_empty() {
        return Ok(0);
    }
    // 拼接 IN 子句的占位符（参数化绑定，无注入风险）
    let placeholders = std::iter::repeat("?").take(ids.len()).collect::<Vec<_>>().join(",");
    let sql = format!("DELETE FROM test_cases WHERE id IN ({placeholders})");
    let mut q = sqlx::query(&sql);
    for id in &ids {
        q = q.bind(id);
    }
    let result = q.execute(&db.0).await?;
    Ok(result.rows_affected())
}
