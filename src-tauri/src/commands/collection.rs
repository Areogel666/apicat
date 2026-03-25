use crate::{db::AppDb, error::CmdResult, types::Collection};
use tauri::State;

/// 获取项目下所有 collection（一次性拉全部，内存组装树，避免 N+1）
#[tauri::command]
pub async fn list_collections(
    db: State<'_, AppDb>,
    project_id: i64,
) -> CmdResult<Vec<Collection>> {
    let rows = sqlx::query_as::<_, Collection>(
        "SELECT id, project_id, parent_id, name, sort_order, created_at FROM collections WHERE project_id=? ORDER BY sort_order, id"
    )
    .bind(project_id)
    .fetch_all(&db.0)
    .await?;
    Ok(rows)
}

/// 创建 collection（文件夹）
#[tauri::command]
pub async fn create_collection(
    db: State<'_, AppDb>,
    project_id: i64,
    parent_id: Option<i64>,
    name: String,
) -> CmdResult<Collection> {
    let row = sqlx::query_as::<_, Collection>(
        "INSERT INTO collections (project_id, parent_id, name) VALUES (?,?,?) RETURNING id, project_id, parent_id, name, sort_order, created_at"
    )
    .bind(project_id)
    .bind(parent_id)
    .bind(&name)
    .fetch_one(&db.0)
    .await?;
    Ok(row)
}

/// 重命名 collection
#[tauri::command]
pub async fn rename_collection(
    db: State<'_, AppDb>,
    id: i64,
    name: String,
) -> CmdResult<Collection> {
    let row = sqlx::query_as::<_, Collection>(
        "UPDATE collections SET name=? WHERE id=? RETURNING id, project_id, parent_id, name, sort_order, created_at"
    )
    .bind(&name)
    .bind(id)
    .fetch_one(&db.0)
    .await?;
    Ok(row)
}

/// 删除 collection（级联删除子 collection 和接口，由 SQLite ON DELETE CASCADE 保证）
#[tauri::command]
pub async fn delete_collection(db: State<'_, AppDb>, id: i64) -> CmdResult<()> {
    sqlx::query("DELETE FROM collections WHERE id=?")
        .bind(id)
        .execute(&db.0)
        .await?;
    Ok(())
}

/// 批量更新 collection 排序（拖拽后调用）
#[tauri::command]
pub async fn update_collection_sort(
    db: State<'_, AppDb>,
    items: Vec<(i64, i64)>,  // (id, sort_order)
) -> CmdResult<()> {
    for (id, sort) in items {
        sqlx::query("UPDATE collections SET sort_order=? WHERE id=?")
            .bind(sort)
            .bind(id)
            .execute(&db.0)
            .await?;
    }
    Ok(())
}
