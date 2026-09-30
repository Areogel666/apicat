use crate::{db::AppDb, error::CmdResult, sql_cols::COOKIE_COLS, types::Cookie};
use sqlx::SqlitePool;
use tauri::State;

/// 列出 Cookie（按作用域筛选）
///
/// scope_type: "global" | "project" | "env"
///   - project 需要 project_id
///   - env     需要 env_id
#[tauri::command]
pub async fn list_cookies(
    db: State<'_, AppDb>,
    scope_type: String,
    project_id: Option<i64>,
    env_id: Option<i64>,
) -> CmdResult<Vec<Cookie>> {
    Ok(list_cookies_impl(&db.0, &scope_type, project_id, env_id).await?)
}

/// 核心逻辑（供 Tauri command 与 HTTP bridge 共用）
pub async fn list_cookies_impl(
    pool: &SqlitePool,
    scope_type: &str,
    project_id: Option<i64>,
    env_id: Option<i64>,
) -> Result<Vec<Cookie>, crate::error::AppError> {
    let rows = match scope_type {
        "project" => {
            sqlx::query_as::<_, Cookie>(&format!(
                "SELECT {COOKIE_COLS} FROM cookies WHERE scope_type='project' AND project_id=? ORDER BY id DESC"
            ))
            .bind(project_id)
            .fetch_all(pool)
            .await?
        }
        "env" => {
            sqlx::query_as::<_, Cookie>(&format!(
                "SELECT {COOKIE_COLS} FROM cookies WHERE scope_type='env' AND env_id=? ORDER BY id DESC"
            ))
            .bind(env_id)
            .fetch_all(pool)
            .await?
        }
        _ => {
            sqlx::query_as::<_, Cookie>(&format!(
                "SELECT {COOKIE_COLS} FROM cookies WHERE scope_type='global' ORDER BY id DESC"
            ))
            .fetch_all(pool)
            .await?
        }
    };
    Ok(rows)
}

/// 创建 Cookie
///
/// 1.0.7：补 env_id / expires_at / http_only / secure / enabled 写入
/// （这四列此前是 schema-only 死字段，任何路径都写不进，Q31-B）
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn create_cookie(
    db: State<'_, AppDb>,
    scope_type: String,
    project_id: Option<i64>,
    env_id: Option<i64>,
    domain: String,
    name: String,
    value: String,
    path: String,
    expires_at: Option<String>,
    http_only: Option<bool>,
    secure: Option<bool>,
) -> CmdResult<Cookie> {
    Ok(create_cookie_impl(
        &db.0,
        &CookieUpsert {
            scope_type,
            project_id,
            env_id,
            domain: crate::cookies::normalize_domain(&domain),
            name,
            value,
            path: if path.is_empty() { "/".to_string() } else { path },
            expires_at,
            http_only: http_only.unwrap_or(false),
            secure: secure.unwrap_or(false),
        },
    )
    .await?)
}

/// 更新 Cookie（Q32-A）
///
/// 可改 domain / name / value / path / expires_at / http_only / secure / enabled；
/// **不可改 scope_type / project_id / env_id**（改作用域 = 删了重建，避开唯一约束冲突）。
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn update_cookie(
    db: State<'_, AppDb>,
    id: i64,
    domain: Option<String>,
    name: Option<String>,
    value: Option<String>,
    path: Option<String>,
    expires_at: Option<String>,
    http_only: Option<bool>,
    secure: Option<bool>,
    enabled: Option<i64>,
) -> CmdResult<Cookie> {
    let normalized_domain = domain.as_deref().map(crate::cookies::normalize_domain);
    Ok(update_cookie_impl(
        &db.0,
        id,
        normalized_domain.as_deref(),
        name.as_deref(),
        value.as_deref(),
        path.as_deref(),
        expires_at.as_deref(),
        http_only,
        secure,
        enabled,
    )
    .await?)
}

/// 核心逻辑（供 Tauri command 与 HTTP bridge 共用）
#[allow(clippy::too_many_arguments)]
pub async fn update_cookie_impl(
    pool: &SqlitePool,
    id: i64,
    domain: Option<&str>,
    name: Option<&str>,
    value: Option<&str>,
    path: Option<&str>,
    expires_at: Option<&str>,
    http_only: Option<bool>,
    secure: Option<bool>,
    enabled: Option<i64>,
) -> Result<Cookie, crate::error::AppError> {
    // expires_at 需要支持「显式置 NULL」（改成会话 cookie），故不用 COALESCE，
    // 而是把「传了就覆盖、没传就保留」写在 SQL 里，NULL 语义交给调用方字符串。
    let row = sqlx::query_as::<_, Cookie>(&format!(
        "UPDATE cookies SET \
           domain=COALESCE(?, domain), \
           name=COALESCE(?, name), \
           value=COALESCE(?, value), \
           path=COALESCE(?, path), \
           expires_at=COALESCE(?, expires_at), \
           http_only=COALESCE(?, http_only), \
           secure=COALESCE(?, secure), \
           enabled=COALESCE(?, enabled) \
         WHERE id=? RETURNING {COOKIE_COLS}"
    ))
    .bind(domain)
    .bind(name)
    .bind(value)
    .bind(path)
    .bind(expires_at)
    .bind(http_only.map(|b| b as i64))
    .bind(secure.map(|b| b as i64))
    .bind(enabled)
    .bind(id)
    .fetch_one(pool)
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

/// 获取某 URL 可用 Cookie（子域/path/过期/优先级过滤后）
///
/// 1.0.7：改为委托 `cookies::collect_cookies_for_url`（Q30-A 唯一真源），
/// 不再自己内联一份 SQL。`url` 支持完整 URL 或裸 host。
#[tauri::command]
pub async fn get_cookies_for_domain(
    db: State<'_, AppDb>,
    domain: String,
    project_id: Option<i64>,
    env_id: Option<i64>,
) -> CmdResult<Vec<Cookie>> {
    // 裸 host 补成 URL，好让统一入口解析出 host/path
    let url = if domain.contains("://") {
        domain.clone()
    } else {
        format!("https://{domain}/")
    };
    Ok(crate::cookies::collect_cookies_for_url(
        &db.0,
        &url,
        project_id,
        env_id,
    )
    .await?)
}

/// 写库用的 cookie 归一化载荷（create 与 Set-Cookie 回存共用）
pub struct CookieUpsert {
    pub scope_type: String,
    pub project_id: Option<i64>,
    pub env_id: Option<i64>,
    pub domain: String,
    pub name: String,
    pub value: String,
    pub path: String,
    pub expires_at: Option<String>,
    pub http_only: bool,
    pub secure: bool,
}

/// 核心逻辑（供 Tauri command 与 HTTP bridge 共用）
pub async fn create_cookie_impl(
    pool: &SqlitePool,
    c: &CookieUpsert,
) -> Result<Cookie, crate::error::AppError> {
    let row = sqlx::query_as::<_, Cookie>(&format!(
        "INSERT INTO cookies (scope_type, project_id, env_id, domain, name, value, path, \
           expires_at, http_only, secure, enabled) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 1) RETURNING {COOKIE_COLS}"
    ))
    .bind(&c.scope_type)
    .bind(c.project_id)
    .bind(c.env_id)
    .bind(&c.domain)
    .bind(&c.name)
    .bind(&c.value)
    .bind(&c.path)
    .bind(&c.expires_at)
    .bind(c.http_only as i64)
    .bind(c.secure as i64)
    .fetch_one(pool)
    .await?;
    Ok(row)
}
