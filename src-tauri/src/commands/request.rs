use crate::{db::AppDb, error::CmdResult, types::ApiRequest};
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

/// 创建接口（带 Collection 内唯一名校验）
#[tauri::command]
pub async fn create_request(
    db: State<'_, AppDb>,
    collection_id: i64,
    name: String,
    method: String,
    url: String,
) -> CmdResult<ApiRequest> {
    // Collection 内唯一性校验
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM api_requests WHERE collection_id=? AND name=?)"
    )
    .bind(collection_id)
    .bind(&name)
    .fetch_one(&db.0)
    .await?;

    if exists {
        return Err(crate::error::AppError::Custom(
            format!("接口名「{}」在当前文件夹中已存在", name)
        ));
    }

    let row = sqlx::query_as::<_, ApiRequest>(
        "INSERT INTO api_requests (collection_id, name, method, url) VALUES (?,?,?,?) RETURNING id, collection_id, name, method, url, params, headers, body_type, body, auth_type, auth_config, sort_order, created_at, updated_at"
    )
    .bind(collection_id)
    .bind(&name)
    .bind(&method)
    .bind(&url)
    .fetch_one(&db.0)
    .await?;
    Ok(row)
}

/// 更新接口（全量保存）
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
    // 名称唯一性：排除自身
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM api_requests WHERE collection_id=(SELECT collection_id FROM api_requests WHERE id=?) AND name=? AND id!=?)"
    )
    .bind(id)
    .bind(&name)
    .bind(id)
    .fetch_one(&db.0)
    .await?;

    if exists {
        return Err(crate::error::AppError::Custom(
            format!("接口名「{}」在当前文件夹中已存在", name)
        ));
    }

    let row = sqlx::query_as::<_, ApiRequest>(
        "UPDATE api_requests SET name=?,method=?,url=?,params=?,headers=?,body_type=?,body=?,auth_type=?,auth_config=?,updated_at=datetime('now') WHERE id=? RETURNING id, collection_id, name, method, url, params, headers, body_type, body, auth_type, auth_config, sort_order, created_at, updated_at"
    )
    .bind(&name).bind(&method).bind(&url)
    .bind(&params).bind(&headers)
    .bind(&body_type).bind(&body)
    .bind(&auth_type).bind(&auth_config)
    .bind(id)
    .fetch_one(&db.0)
    .await?;
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
