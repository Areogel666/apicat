use crate::{db::AppDb, error::CmdResult, types::Project};
use tauri::State;

/// Project 反序列化列清单；必须与 types.rs::Project 字段严格一致
use crate::sql_cols::PROJECT_COLS;

/// 获取所有项目（按创建时间倒序）
#[tauri::command]
pub async fn list_projects(db: State<'_, AppDb>) -> CmdResult<Vec<Project>> {
    let rows = sqlx::query_as::<_, Project>(&format!(
        "SELECT {PROJECT_COLS} FROM projects ORDER BY created_at DESC"
    ))
    .fetch_all(&db.0)
    .await?;
    Ok(rows)
}

/// 创建项目。UI 与 Bridge 共用。
pub async fn create_project_impl(
    pool: &sqlx::SqlitePool,
    name: &str,
    description: Option<&str>,
) -> Result<Project, crate::error::AppError> {
    let row = sqlx::query_as::<_, Project>(&format!(
        "INSERT INTO projects (name, description) VALUES (?, ?) RETURNING {PROJECT_COLS}"
    ))
    .bind(name)
    .bind(description)
    .fetch_one(pool)
    .await?;
    Ok(row)
}

/// 更新项目（全量覆写语义）。UI 与 Bridge 共用。
///
/// Bridge 需要「只传要改的字段」时，由其 handler 先 SELECT 合并再调本函数，
/// 这样 COALESCE 的便利留在协议层，SQL 只有这一份。
pub async fn update_project_impl(
    pool: &sqlx::SqlitePool,
    id: i64,
    name: &str,
    description: Option<&str>,
    docs_output_dir: Option<&str>,
) -> Result<Project, crate::error::AppError> {
    let row = sqlx::query_as::<_, Project>(&format!(
        "UPDATE projects SET name=?, description=?, docs_output_dir=?, updated_at=datetime('now') \
         WHERE id=? RETURNING {PROJECT_COLS}"
    ))
    .bind(name)
    .bind(description)
    .bind(docs_output_dir)
    .bind(id)
    .fetch_one(pool)
    .await?;
    Ok(row)
}

/// 创建项目
#[tauri::command]
pub async fn create_project(
    db: State<'_, AppDb>,
    name: String,
    description: Option<String>,
) -> CmdResult<Project> {
    create_project_impl(&db.0, &name, description.as_deref()).await
}

/// 更新项目名称/描述/文档输出目录
#[tauri::command]
pub async fn update_project(
    db: State<'_, AppDb>,
    id: i64,
    name: String,
    description: Option<String>,
    docs_output_dir: Option<String>,
) -> CmdResult<Project> {
    update_project_impl(
        &db.0,
        id,
        &name,
        description.as_deref(),
        docs_output_dir.as_deref(),
    )
    .await
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
