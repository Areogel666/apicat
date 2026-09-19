use crate::{db::AppDb, error::{map_unique_name_error, CmdResult}, types::ApiRequest};
use tauri::State;

/// ApiRequest 反序列化列清单；必须与 types.rs::ApiRequest 字段严格一致
use crate::sql_cols::REQUEST_COLS;

/// 获取 collection 下所有接口
#[tauri::command]
pub async fn list_requests(
    db: State<'_, AppDb>,
    collection_id: i64,
) -> CmdResult<Vec<ApiRequest>> {
    let sql = format!(
        "SELECT {REQUEST_COLS} FROM api_requests WHERE collection_id=? ORDER BY sort_order, id"
    );
    let rows = sqlx::query_as::<_, ApiRequest>(&sql)
    .bind(collection_id)
    .fetch_all(&db.0)
    .await?;
    Ok(rows)
}

/// 创建接口。UI 与 Bridge 共用。
/// Collection 内名称唯一性由 DB UNIQUE 约束保证，冲突时翻译为友好文案。
pub async fn create_request_impl(
    pool: &sqlx::SqlitePool,
    collection_id: i64,
    name: &str,
    method: &str,
    url: &str,
) -> Result<ApiRequest, crate::error::AppError> {
    let row = sqlx::query_as::<_, ApiRequest>(&format!(
        "INSERT INTO api_requests (collection_id, name, method, url) VALUES (?,?,?,?) RETURNING {REQUEST_COLS}"
    ))
    .bind(collection_id)
    .bind(name)
    .bind(method)
    .bind(url)
    .fetch_one(pool)
    .await
    .map_err(|e| map_unique_name_error(e, name))?;
    Ok(row)
}

/// 更新接口（全量覆写语义）。UI 与 Bridge 共用。
///
/// Bridge 需要「只传要改的字段」时，由其 handler 先 SELECT 合并再调本函数，
/// 这样 COALESCE 的便利留在协议层，SQL 只有这一份。
pub async fn update_request_impl(
    pool: &sqlx::SqlitePool,
    id: i64,
    name: &str,
    method: &str,
    url: &str,
    params: &str,
    headers: &str,
    body_type: &str,
    body: &str,
    auth_type: &str,
    auth_config: &str,
    description: &str,
) -> Result<ApiRequest, crate::error::AppError> {
    let row = sqlx::query_as::<_, ApiRequest>(&format!(
        "UPDATE api_requests SET name=?,method=?,url=?,params=?,headers=?,body_type=?,body=?,auth_type=?,auth_config=?,description=?,updated_at=datetime('now') WHERE id=? RETURNING {REQUEST_COLS}"
    ))
    .bind(name).bind(method).bind(url)
    .bind(params).bind(headers)
    .bind(body_type).bind(body)
    .bind(auth_type).bind(auth_config)
    .bind(description)
    .bind(id)
    .fetch_one(pool)
    .await
    .map_err(|e| map_unique_name_error(e, name))?;
    Ok(row)
}

/// 创建接口（Collection 内名称唯一性由 DB UNIQUE 约束保证，原子操作无 TOCTOU）
#[tauri::command]
pub async fn create_request(
    db: State<'_, AppDb>,
    collection_id: i64,
    name: String,
    method: String,
    url: String,
) -> CmdResult<ApiRequest> {
    create_request_impl(&db.0, collection_id, &name, &method, &url).await
}

/// 更新接口（全量保存，Collection 内名称唯一性由 DB UNIQUE 约束保证）
#[tauri::command]
pub async fn update_request(
    db: State<'_, AppDb>,
    id: i64,
    name: String,
    method: String,
    url: String,
    params: String,
    headers: String,
    body_type: String,
    body: String,
    auth_type: String,
    auth_config: String,
) -> CmdResult<ApiRequest> {
    // UI 走全量覆写；description 不在 IPC 参数里，按原值保留
    let cur = sqlx::query_as::<_, ApiRequest>(&format!(
        "SELECT {REQUEST_COLS} FROM api_requests WHERE id=?"
    ))
    .bind(id)
    .fetch_one(&db.0)
    .await?;
    update_request_impl(
        &db.0, id, &name, &method, &url, &params, &headers,
        &body_type, &body, &auth_type, &auth_config, &cur.description,
    )
    .await
}

/// 删除接口
#[tauri::command]
pub async fn delete_request(db: State<'_, AppDb>, id: i64) -> CmdResult<()> {
    sqlx::query("DELETE FROM api_requests WHERE id=?")
        .bind(id)
        .execute(&db.0)
        .await?;
    Ok(())
}

/// 复制接口（克隆所有字段，名称自动追加「副本」）。
/// UI 与 Bridge 共用：副本保留 description 且 sort_order 紧跟原接口。
pub async fn duplicate_request_impl(
    pool: &sqlx::SqlitePool,
    id: i64,
) -> Result<ApiRequest, crate::error::AppError> {
    let src = sqlx::query_as::<_, ApiRequest>(&format!(
        "SELECT {REQUEST_COLS} FROM api_requests WHERE id=?"
    ))
    .bind(id)
    .fetch_one(pool)
    .await?;

    let new_name = format!("{} 副本", src.name);
    let row = sqlx::query_as::<_, ApiRequest>(&format!(
        "INSERT INTO api_requests (collection_id, name, method, url, params, headers, body_type, body, auth_type, auth_config, description, p95_threshold_ms, p99_threshold_ms, sort_order) \
         VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?) RETURNING {REQUEST_COLS}"
    ))
    .bind(src.collection_id)
    .bind(&new_name)
    .bind(&src.method)
    .bind(&src.url)
    .bind(&src.params)
    .bind(&src.headers)
    .bind(&src.body_type)
    .bind(&src.body)
    .bind(&src.auth_type)
    .bind(&src.auth_config)
    .bind(&src.description)
    .bind(src.p95_threshold_ms)
    .bind(src.p99_threshold_ms)
    .bind(src.sort_order + 1)
    .fetch_one(pool)
    .await
    .map_err(|e| map_unique_name_error(e, &new_name))?;
    Ok(row)
}

/// 复制接口（克隆所有字段，名称自动追加「副本」）
#[tauri::command]
pub async fn duplicate_request(db: State<'_, AppDb>, id: i64) -> CmdResult<ApiRequest> {
    duplicate_request_impl(&db.0, id).await
}

/// 批量更新接口排序（拖拽后调用）
#[tauri::command]
pub async fn update_request_sort(
    db: State<'_, AppDb>,
    items: Vec<(i64, i64)>,  // (id, sort_order)
) -> CmdResult<()> {
    for (id, sort) in items {
        sqlx::query("UPDATE api_requests SET sort_order=? WHERE id=?")
            .bind(sort)
            .bind(id)
            .execute(&db.0)
            .await?;
    }
    Ok(())
}

/// 将接口移动到新的 collection，并设置其在目标 collection 中的排序位置
#[tauri::command]
pub async fn move_request(
    db: State<'_, AppDb>,
    id: i64,
    new_collection_id: i64,
    sort_order: i64,
) -> CmdResult<ApiRequest> {
    let row = sqlx::query_as::<_, ApiRequest>(&format!(
        "UPDATE api_requests SET collection_id=?, sort_order=?, updated_at=datetime('now') \
         WHERE id=? \
         RETURNING {REQUEST_COLS}"
    ))
    .bind(new_collection_id)
    .bind(sort_order)
    .bind(id)
    .fetch_one(&db.0)
    .await
    .map_err(|e| crate::error::map_unique_name_error(e, ""))?;
    Ok(row)
}
