use crate::{db::AppDb, error::{AppError, CmdResult}, types::Collection};
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

/// 移动 collection 到新的父目录（或提升到根层）
/// - new_parent_id=None 表示移到根层
/// - 循环引用防护：若 new_parent_id 是当前 collection 的后代，返回错误
#[tauri::command]
pub async fn move_collection(
    db: State<'_, AppDb>,
    id: i64,
    new_parent_id: Option<i64>,
    sort_order: i64,
) -> CmdResult<Collection> {
    // 循环引用防护：查询 id 的所有后代，确保 new_parent_id 不在其中
    if let Some(target_parent) = new_parent_id {
        // 用递归 CTE 取得所有后代 id
        let descendants: Vec<(i64,)> = sqlx::query_as(
            "WITH RECURSIVE sub(id) AS (
               SELECT id FROM collections WHERE parent_id = ?
               UNION ALL
               SELECT c.id FROM collections c JOIN sub ON c.parent_id = sub.id
             )
             SELECT id FROM sub"
        )
        .bind(id)
        .fetch_all(&db.0)
        .await?;

        let desc_ids: Vec<i64> = descendants.into_iter().map(|(i,)| i).collect();
        if desc_ids.contains(&target_parent) {
            return Err(AppError::Custom(
                "不能将目录移入其自身的子目录（循环引用）".to_string()
            ));
        }
    }

    let row = sqlx::query_as::<_, Collection>(
        "UPDATE collections SET parent_id=?, sort_order=? WHERE id=? \
         RETURNING id, project_id, parent_id, name, sort_order, created_at"
    )
    .bind(new_parent_id)
    .bind(sort_order)
    .bind(id)
    .fetch_one(&db.0)
    .await?;

    Ok(row)
}
