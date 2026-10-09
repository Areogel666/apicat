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
            // env_id 不传 = 取所有环境层 cookie（Cookie 管理的「环境」筛选需要）
            match env_id {
                Some(eid) => {
                    sqlx::query_as::<_, Cookie>(&format!(
                        "SELECT {COOKIE_COLS} FROM cookies WHERE scope_type='env' AND env_id=? ORDER BY id DESC"
                    ))
                    .bind(eid)
                    .fetch_all(pool)
                    .await?
                }
                None => {
                    sqlx::query_as::<_, Cookie>(&format!(
                        "SELECT {COOKIE_COLS} FROM cookies WHERE scope_type='env' ORDER BY id DESC"
                    ))
                    .fetch_all(pool)
                    .await?
                }
            }
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
///
/// `expires_at` 三态约定（与 bridge-api.md 一致）：
///   - 不传        → 保留原值
///   - 传非空串    → 覆盖
///   - **传空串 `""`** → 置 NULL（改回会话 cookie）
///
/// 空串作清空哨兵而不是用 JSON `null`：Bridge 侧 `serde_json::Value` 里
/// 「键缺失」与「值为 null」都取不到 `as_str()`，区分不了；COALESCE 也表达不了
/// 「显式置 NULL」。故用 `CASE WHEN ? THEN ? ELSE expires_at END`。
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
    let expires_provided = expires_at.is_some() as i64;
    let expires_value: Option<&str> = expires_at.filter(|s| !s.is_empty());

    let row = sqlx::query_as::<_, Cookie>(&format!(
        "UPDATE cookies SET \
           domain=COALESCE(?, domain), \
           name=COALESCE(?, name), \
           value=COALESCE(?, value), \
           path=COALESCE(?, path), \
           expires_at=CASE WHEN ? THEN ? ELSE expires_at END, \
           http_only=COALESCE(?, http_only), \
           secure=COALESCE(?, secure), \
           enabled=COALESCE(?, enabled) \
         WHERE id=? RETURNING {COOKIE_COLS}"
    ))
    .bind(domain)
    .bind(name)
    .bind(value)
    .bind(path)
    .bind(expires_provided)
    .bind(expires_value)
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

/// 作用域中文名（仅用于错误提示）
fn scope_label(scope_type: &str) -> &'static str {
    match scope_type {
        "project" => "项目",
        "env" => "环境",
        _ => "全局",
    }
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
///
/// 重复键**拦截**而非覆盖：命中同组（作用域 + domain + path + name）时返回友好错误。
/// 覆盖语义由 `update_cookie` 承担；Set-Cookie 回存的覆盖走 `cookies::upsert_cookie`，
/// 是另一条路径，不受此处影响。
pub async fn create_cookie_impl(
    pool: &SqlitePool,
    c: &CookieUpsert,
) -> Result<Cookie, crate::error::AppError> {
    // 查重必须在应用层——表的 UNIQUE 约束对含 NULL 列的作用域失效（见 find_cookie_id 注释）
    if crate::cookies::find_cookie_id(
        pool,
        &c.scope_type,
        c.project_id,
        c.env_id,
        &c.domain,
        &c.path,
        &c.name,
    )
    .await?
    .is_some()
    {
        return Err(crate::error::AppError::Custom(format!(
            "已存在同名 Cookie（{} · {} · {}），请编辑已有条目",
            scope_label(&c.scope_type),
            c.domain,
            c.name
        )));
    }

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
    // 空串 = 未指定过期时间，落成 NULL（会话 cookie）
    .bind(c.expires_at.as_deref().filter(|s| !s.is_empty()))
    .bind(c.http_only as i64)
    .bind(c.secure as i64)
    .fetch_one(pool)
    .await?;
    Ok(row)
}
