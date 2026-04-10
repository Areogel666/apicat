use crate::{db::AppDb, error::{map_unique_name_error, CmdResult}, types::ApiRequest};
use tauri::State;

/// 获取 collection 下所有接口
#[tauri::command]
pub async fn list_requests(
    db: State<'_, AppDb>,
    collection_id: i64,
) -> CmdResult<Vec<ApiRequest>> {
    let rows = sqlx::query_as::<_, ApiRequest>(
        "SELECT id, collection_id, name, method, url, params, headers, body_type, body, auth_type, auth_config, sort_order, created_at, updated_at FROM api_requests WHERE collection_id=? ORDER BY sort_order, id"
    )
    .bind(collection_id)
    .fetch_all(&db.0)
    .await?;
    Ok(rows)
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
    let row = sqlx::query_as::<_, ApiRequest>(
        "INSERT INTO api_requests (collection_id, name, method, url) VALUES (?,?,?,?) RETURNING id, collection_id, name, method, url, params, headers, body_type, body, auth_type, auth_config, sort_order, created_at, updated_at"
    )
    .bind(collection_id)
    .bind(&name)
    .bind(&method)
    .bind(&url)
    .fetch_one(&db.0)
    .await
    .map_err(|e| map_unique_name_error(e, &name))?;
    Ok(row)
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
    let row = sqlx::query_as::<_, ApiRequest>(
        "UPDATE api_requests SET name=?,method=?,url=?,params=?,headers=?,body_type=?,body=?,auth_type=?,auth_config=?,updated_at=datetime('now') WHERE id=? RETURNING id, collection_id, name, method, url, params, headers, body_type, body, auth_type, auth_config, sort_order, created_at, updated_at"
    )
    .bind(&name).bind(&method).bind(&url)
    .bind(&params).bind(&headers)
    .bind(&body_type).bind(&body)
    .bind(&auth_type).bind(&auth_config)
    .bind(id)
    .fetch_one(&db.0)
    .await
    .map_err(|e| map_unique_name_error(e, &name))?;
    Ok(row)
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

/// 复制接口（克隆所有字段，名称自动追加「副本」）
#[tauri::command]
pub async fn duplicate_request(db: State<'_, AppDb>, id: i64) -> CmdResult<ApiRequest> {
    let src = sqlx::query_as::<_, ApiRequest>(
        "SELECT id, collection_id, name, method, url, params, headers, body_type, body, auth_type, auth_config, sort_order, created_at, updated_at FROM api_requests WHERE id=?"
    )
    .bind(id)
    .fetch_one(&db.0)
    .await?;

    let new_name = format!("{} 副本", src.name);
    let row = sqlx::query_as::<_, ApiRequest>(
        "INSERT INTO api_requests (collection_id, name, method, url, params, headers, body_type, body, auth_type, auth_config, sort_order) VALUES (?,?,?,?,?,?,?,?,?,?,?) RETURNING id, collection_id, name, method, url, params, headers, body_type, body, auth_type, auth_config, sort_order, created_at, updated_at"
    )
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
    .bind(src.sort_order + 1)
    .fetch_one(&db.0)
    .await
    .map_err(|e| map_unique_name_error(e, &new_name))?;
    Ok(row)
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
    let row = sqlx::query_as::<_, ApiRequest>(
        "UPDATE api_requests SET collection_id=?, sort_order=?, updated_at=datetime('now') \
         WHERE id=? \
         RETURNING id, collection_id, name, method, url, params, headers, \
                   body_type, body, auth_type, auth_config, sort_order, created_at, updated_at"
    )
    .bind(new_collection_id)
    .bind(sort_order)
    .bind(id)
    .fetch_one(&db.0)
    .await
    .map_err(|e| crate::error::map_unique_name_error(e, ""))?;
    Ok(row)
}
