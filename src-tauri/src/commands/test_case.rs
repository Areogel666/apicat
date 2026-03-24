use crate::{db::AppDb, error::CmdResult, types::TestCase};
use tauri::State;

const SELECT_COLS: &str = "id, request_id, collection_id, name, description, source, method, url, \
    headers, params, body_type, body, assertions, last_run_at, last_status, last_duration_ms, \
    last_response, starred, enabled, sort_order, created_at, updated_at";

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
) -> CmdResult<TestCase> {
    // 判断是否为该接口的第一个用例 → 自动收藏
    let existing_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM test_cases WHERE request_id=?",
    )
    .bind(request_id)
    .fetch_one(&db.0)
    .await?;
    let starred: i64 = if existing_count == 0 { 1 } else { 0 };

    // 自动命名：「用例 N」（N = existing_count + 1）
    let final_name = if name.is_empty() {
        format!("用例 {}", existing_count + 1)
    } else {
        name
    };

    let sql = format!(
        "INSERT INTO test_cases \
            (request_id, collection_id, name, method, url, headers, params, body_type, body, starred, sort_order) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) \
         RETURNING {SELECT_COLS}"
    );
    let row = sqlx::query_as::<_, TestCase>(&sql)
        .bind(request_id)
        .bind(collection_id)
        .bind(&final_name)
        .bind(&method)
        .bind(&url)
        .bind(headers.as_deref().unwrap_or("[]"))
        .bind(params.as_deref().unwrap_or("[]"))
        .bind(&body_type)
        .bind(&body)
        .bind(starred)
        .bind(existing_count)   // sort_order = 当前用例数（末尾插入）
        .fetch_one(&db.0)
        .await?;
    Ok(row)
}

/// 更新测试用例名称 / 收藏状态 / 请求参数
#[tauri::command]
pub async fn update_test_case(
    db: State<'_, AppDb>,
    id: i64,
    name: String,
    starred: i64,
    method: Option<String>,
    url: Option<String>,
    headers: Option<String>,
    params: Option<String>,
    body_type: Option<String>,
    body: Option<String>,
) -> CmdResult<TestCase> {
    let sql = format!(
        "UPDATE test_cases SET name=?, starred=?, method=?, url=?, headers=?, params=?, \
         body_type=?, body=?, updated_at=datetime('now') \
         WHERE id=? RETURNING {SELECT_COLS}"
    );
    let row = sqlx::query_as::<_, TestCase>(&sql)
        .bind(&name)
        .bind(starred)
        .bind(&method)
        .bind(&url)
        .bind(headers.as_deref().unwrap_or("[]"))
        .bind(params.as_deref().unwrap_or("[]"))
        .bind(&body_type)
        .bind(&body)
        .bind(id)
        .fetch_one(&db.0)
        .await?;
    Ok(row)
}

/// 删除测试用例
/// 若为该接口最后一个收藏用例，返回错误（禁止删除）
#[tauri::command]
pub async fn delete_test_case(db: State<'_, AppDb>, id: i64) -> CmdResult<()> {
    // 查出该用例归属的 request_id 和 starred 状态
    let (request_id, starred): (Option<i64>, i64) =
        sqlx::query_as("SELECT request_id, starred FROM test_cases WHERE id=?")
            .bind(id)
            .fetch_one(&db.0)
            .await?;

    // 若为收藏用例，检查是否为最后一个
    if starred == 1 {
        if let Some(rid) = request_id {
            let starred_count: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM test_cases WHERE request_id=? AND starred=1",
            )
            .bind(rid)
            .fetch_one(&db.0)
            .await?;
            if starred_count <= 1 {
                return Err(crate::error::AppError::Custom(
                    "不能删除最后一个收藏用例".to_string(),
                ));
            }
        }
    }

    sqlx::query("DELETE FROM test_cases WHERE id=?")
        .bind(id)
        .execute(&db.0)
        .await?;
    Ok(())
}
