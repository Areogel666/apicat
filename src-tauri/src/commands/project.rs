use crate::{db::AppDb, error::CmdResult, types::Project};
use tauri::State;

/// 获取所有项目（按创建时间倒序）
#[tauri::command]
pub async fn list_projects(db: State<'_, AppDb>) -> CmdResult<Vec<Project>> {
    let rows = sqlx::query_as::<_, Project>(
        "SELECT id, name, description, created_at, updated_at FROM projects ORDER BY created_at DESC"
    )
    .fetch_all(&db.0)
    .await?;
    Ok(rows)
}

/// 创建项目
#[tauri::command]
pub async fn create_project(
    db: State<'_, AppDb>,
    name: String,
    description: Option<String>,
) -> CmdResult<Project> {
    let row = sqlx::query_as::<_, Project>(
        "INSERT INTO projects (name, description) VALUES (?, ?) RETURNING id, name, description, created_at, updated_at"
    )
    .bind(&name)
    .bind(&description)
    .fetch_one(&db.0)
    .await?;
    Ok(row)
}

/// 更新项目名称/描述
#[tauri::command]
pub async fn update_project(
    db: State<'_, AppDb>,
    id: i64,
    name: String,
    description: Option<String>,
) -> CmdResult<Project> {
    let row = sqlx::query_as::<_, Project>(
        "UPDATE projects SET name=?, description=?, updated_at=datetime('now') WHERE id=? RETURNING id, name, description, created_at, updated_at"
    )
    .bind(&name)
    .bind(&description)
    .bind(id)
    .fetch_one(&db.0)
    .await?;
    Ok(row)
}

/// 删除项目（级联删除所有子数据，由 SQLite ON DELETE CASCADE 保证）
#[tauri::command]
pub async fn delete_project(db: State<'_, AppDb>, id: i64) -> CmdResult<()> {
    sqlx::query("DELETE FROM projects WHERE id=?")
        .bind(id)
        .execute(&db.0)
        .await?;
    Ok(())
}
