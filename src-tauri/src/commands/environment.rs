use crate::{
    db::AppDb,
    error::CmdResult,
    types::{EnvVariable, Environment},
};
use tauri::State;

/// 获取项目下环境列表
#[tauri::command]
pub async fn list_environments(
    db: State<'_, AppDb>,
    project_id: i64,
) -> CmdResult<Vec<Environment>> {
    let rows = sqlx::query_as::<_, Environment>(
        "SELECT id, project_id, name, base_url, is_active, created_at FROM environments WHERE project_id=? ORDER BY created_at DESC, id DESC",
    )
    .bind(project_id)
    .fetch_all(&db.0)
    .await?;
    Ok(rows)
}

/// 创建环境
#[tauri::command]
pub async fn create_environment(
    db: State<'_, AppDb>,
    project_id: i64,
    name: String,
    base_url: Option<String>,
) -> CmdResult<Environment> {
    let row = sqlx::query_as::<_, Environment>(
        "INSERT INTO environments (project_id, name, base_url) VALUES (?, ?, ?) RETURNING id, project_id, name, base_url, is_active, created_at",
    )
    .bind(project_id)
    .bind(&name)
    .bind(&base_url)
    .fetch_one(&db.0)
    .await?;
    Ok(row)
}

/// 更新环境
#[tauri::command]
pub async fn update_environment(
    db: State<'_, AppDb>,
    id: i64,
    name: String,
    base_url: Option<String>,
) -> CmdResult<Environment> {
    let row = sqlx::query_as::<_, Environment>(
        "UPDATE environments SET name=?, base_url=? WHERE id=? RETURNING id, project_id, name, base_url, is_active, created_at",
    )
    .bind(&name)
    .bind(&base_url)
    .bind(id)
    .fetch_one(&db.0)
    .await?;
    Ok(row)
}

/// 删除环境
#[tauri::command]
pub async fn delete_environment(db: State<'_, AppDb>, id: i64) -> CmdResult<()> {
    sqlx::query("DELETE FROM environments WHERE id=?")
        .bind(id)
        .execute(&db.0)
        .await?;
    Ok(())
}

/// 激活指定环境（先清空项目下激活状态，事务保证原子性）
#[tauri::command]
pub async fn activate_environment(
    db: State<'_, AppDb>,
    project_id: i64,
    env_id: i64,
) -> CmdResult<()> {
    let mut tx = db.0.begin().await?;

    sqlx::query("UPDATE environments SET is_active=0 WHERE project_id=?")
        .bind(project_id)
        .execute(&mut *tx)
        .await?;

    sqlx::query("UPDATE environments SET is_active=1 WHERE id=? AND project_id=?")
        .bind(env_id)
        .bind(project_id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;
    Ok(())
}

/// 取消项目下全部环境激活状态
#[tauri::command]
pub async fn deactivate_environment(db: State<'_, AppDb>, project_id: i64) -> CmdResult<()> {
    sqlx::query("UPDATE environments SET is_active=0 WHERE project_id=?")
        .bind(project_id)
        .execute(&db.0)
        .await?;
    Ok(())
}

/// 获取环境变量列表
#[tauri::command]
pub async fn list_env_variables(
    db: State<'_, AppDb>,
    env_id: i64,
) -> CmdResult<Vec<EnvVariable>> {
    let rows = sqlx::query_as::<_, EnvVariable>(
        "SELECT id, env_id, key, value, description, enabled FROM env_variables WHERE env_id=? ORDER BY id",
    )
    .bind(env_id)
    .fetch_all(&db.0)
    .await?;
    Ok(rows)
}

/// 创建环境变量
#[tauri::command]
pub async fn create_env_variable(
    db: State<'_, AppDb>,
    env_id: i64,
    key: String,
    value: String,
    description: Option<String>,
) -> CmdResult<EnvVariable> {
    let row = sqlx::query_as::<_, EnvVariable>(
        "INSERT INTO env_variables (env_id, key, value, description) VALUES (?, ?, ?, ?) RETURNING id, env_id, key, value, description, enabled",
    )
    .bind(env_id)
    .bind(&key)
    .bind(&value)
    .bind(&description)
    .fetch_one(&db.0)
    .await?;
    Ok(row)
}

/// 更新环境变量
#[tauri::command]
pub async fn update_env_variable(
    db: State<'_, AppDb>,
    id: i64,
    key: String,
    value: String,
    description: Option<String>,
    enabled: i64,
) -> CmdResult<EnvVariable> {
    let row = sqlx::query_as::<_, EnvVariable>(
        "UPDATE env_variables SET key=?, value=?, description=?, enabled=? WHERE id=? RETURNING id, env_id, key, value, description, enabled",
    )
    .bind(&key)
    .bind(&value)
    .bind(&description)
    .bind(enabled)
    .bind(id)
    .fetch_one(&db.0)
    .await?;
    Ok(row)
}

/// 删除环境变量
#[tauri::command]
pub async fn delete_env_variable(db: State<'_, AppDb>, id: i64) -> CmdResult<()> {
    sqlx::query("DELETE FROM env_variables WHERE id=?")
        .bind(id)
        .execute(&db.0)
        .await?;
    Ok(())
}
