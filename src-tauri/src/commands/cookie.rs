use crate::{db::AppDb, error::CmdResult, types::Cookie};
use std::collections::HashSet;
use tauri::State;

/// 列出 Cookie（按作用域筛选）
#[tauri::command]
pub async fn list_cookies(
    db: State<'_, AppDb>,
    scope_type: String,
    project_id: Option<i64>,
) -> CmdResult<Vec<Cookie>> {
    let rows = if scope_type == "global" {
        sqlx::query_as::<_, Cookie>(
            "SELECT id, scope_type, project_id, domain, name, value, path, expires_at, http_only, secure, enabled FROM cookies WHERE scope_type='global' ORDER BY id DESC",
        )
        .fetch_all(&db.0)
        .await?
    } else {
        sqlx::query_as::<_, Cookie>(
            "SELECT id, scope_type, project_id, domain, name, value, path, expires_at, http_only, secure, enabled FROM cookies WHERE scope_type='project' AND project_id=? ORDER BY id DESC",
        )
        .bind(project_id)
        .fetch_all(&db.0)
        .await?
    };
    Ok(rows)
}

/// 创建 Cookie
#[tauri::command]
pub async fn create_cookie(
    db: State<'_, AppDb>,
    scope_type: String,
    project_id: Option<i64>,
    domain: String,
    name: String,
    value: String,
    path: String,
) -> CmdResult<Cookie> {
    let row = sqlx::query_as::<_, Cookie>(
        "INSERT INTO cookies (scope_type, project_id, domain, name, value, path) VALUES (?, ?, ?, ?, ?, ?) RETURNING id, scope_type, project_id, domain, name, value, path, expires_at, http_only, secure, enabled",
    )
    .bind(&scope_type)
    .bind(project_id)
    .bind(&domain)
    .bind(&name)
    .bind(&value)
    .bind(&path)
    .fetch_one(&db.0)
    .await?;
    Ok(row)
}

/// 更新 Cookie
#[tauri::command]
pub async fn update_cookie(
    db: State<'_, AppDb>,
    id: i64,
    value: String,
    path: String,
    enabled: i64,
) -> CmdResult<Cookie> {
    let row = sqlx::query_as::<_, Cookie>(
        "UPDATE cookies SET value=?, path=?, enabled=? WHERE id=? RETURNING id, scope_type, project_id, domain, name, value, path, expires_at, http_only, secure, enabled",
    )
    .bind(&value)
    .bind(&path)
    .bind(enabled)
    .bind(id)
    .fetch_one(&db.0)
    .await?;
    Ok(row)
}

/// 删除 Cookie
#[tauri::command]
pub async fn delete_cookie(db: State<'_, AppDb>, id: i64) -> CmdResult<()> {
    sqlx::query("DELETE FROM cookies WHERE id=?")
        .bind(id)
        .execute(&db.0)
        .await?;
    Ok(())
}

/// 获取某域名可用 Cookie（项目级优先覆盖全局同名+同 path）
#[tauri::command]
pub async fn get_cookies_for_domain(
    db: State<'_, AppDb>,
    domain: String,
    project_id: Option<i64>,
) -> CmdResult<Vec<Cookie>> {
    let rows = sqlx::query_as::<_, Cookie>(
        "SELECT id, scope_type, project_id, domain, name, value, path, expires_at, http_only, secure, enabled FROM cookies WHERE domain=? AND enabled=1 AND (scope_type='global' OR (scope_type='project' AND project_id=?)) ORDER BY scope_type DESC, id DESC",
    )
    .bind(&domain)
    .bind(project_id)
    .fetch_all(&db.0)
    .await?;

    let mut seen = HashSet::new();
    let mut deduped = Vec::new();
    for cookie in rows {
        let dedup_key = format!("{}\u{0000}{}", cookie.name, cookie.path);
        if seen.insert(dedup_key) {
            deduped.push(cookie);
        }
    }

    Ok(deduped)
}
