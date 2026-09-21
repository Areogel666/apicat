use crate::{
    db::AppDb,
    error::CmdResult,
    http::{
        client::{send, ParamItem, SendRequestParams},
        variable::replace_variables,
        HttpClient,
    },
    sql_cols::{COOKIE_COLS, ENV_COLS},
    types::{Cookie, EnvVariable, Environment, HistoryRecord, HttpResponse},
};
use sqlx::SqlitePool;
use std::collections::HashSet;
use std::path::PathBuf;
use tauri::State;

/// 响应体超过此阈值时，保存到文件系统而非数据库（100KB）
pub const RESPONSE_FILE_THRESHOLD: usize = 100 * 1024;

/// 获取响应文件存储目录
pub fn get_response_dir() -> Result<PathBuf, crate::error::AppError> {
    #[cfg(target_os = "windows")]
    let home = std::env::var("USERPROFILE").ok();
    #[cfg(not(target_os = "windows"))]
    let home = std::env::var("HOME").ok();

    let home = home.ok_or_else(|| {
        crate::error::AppError::Custom("无法获取用户主目录".to_string())
    })?;

    let dir = PathBuf::from(home).join(".apicat").join("responses");
    std::fs::create_dir_all(&dir).map_err(|e| {
        crate::error::AppError::Custom(format!("创建响应文件目录失败: {e}"))
    })?;
    Ok(dir)
}

/// 保存响应体到文件（超过阈值时），返回 (存储到DB的body, 是否截断)
/// 供 send_request_impl 和 run_test_case_impl 共用
pub fn save_response_body_if_large(
    history_id: i64,
    body: &str,
) -> Result<(String, bool), crate::error::AppError> {
    if body.len() > RESPONSE_FILE_THRESHOLD {
        let file_path = get_response_dir()?.join(format!("{history_id}.txt"));
        std::fs::write(&file_path, body).map_err(|e| {
            crate::error::AppError::Custom(format!("保存响应文件失败: {e}"))
        })?;
        Ok((format!("@file:{history_id}"), true))
    } else {
        Ok((body.to_string(), false))
    }
}

/// 核心逻辑（供 Tauri command 和 HTTP bridge 共用）
pub async fn send_request_impl(
    pool: &SqlitePool,
    http: &reqwest::Client,
    request_id: Option<i64>,
    test_case_id: Option<i64>,
    params: SendRequestParams,
    env_id: Option<i64>,
    project_id: Option<i64>,
) -> Result<HttpResponse, crate::error::AppError> {
    // 0. 复制参数并按需做变量替换
    let mut resolved_params = params.clone();

    if let Some(env_id) = env_id {
        let env = sqlx::query_as::<_, Environment>(&format!(
            "SELECT {ENV_COLS} FROM environments WHERE id=?"
        ))
        .bind(env_id)
        .fetch_one(pool)
        .await?;

        let env_vars = sqlx::query_as::<_, EnvVariable>(
            "SELECT id, env_id, key, value, description, enabled FROM env_variables WHERE env_id=? AND enabled=1",
        )
        .bind(env_id)
        .fetch_all(pool)
        .await?;

        let mut variables = std::collections::HashMap::new();
        if let Some(base_url) = env.base_url {
            variables.insert("base_url".to_string(), base_url);
        }
        for item in env_vars {
            variables.insert(item.key, item.value);
        }

        resolved_params.url = replace_variables(&resolved_params.url, &variables, false);
        let is_json_body = resolved_params.body_type == "raw_json";
        resolved_params.body = replace_variables(&resolved_params.body, &variables, is_json_body);
        for header in resolved_params.headers.iter_mut().filter(|h| h.enabled) {
            header.value = replace_variables(&header.value, &variables, false);
        }
    }

    // 0.1 注入域名 Cookie
    if let Ok(parsed_url) = reqwest::Url::parse(&resolved_params.url) {
        if let Some(domain) = parsed_url.host_str() {
            let cookie_rows = sqlx::query_as::<_, Cookie>(&format!(
                "SELECT {COOKIE_COLS} FROM cookies WHERE domain=? AND enabled=1 AND (scope_type='global' OR (scope_type='project' AND project_id=?)) ORDER BY scope_type DESC, id DESC"
            ))
            .bind(domain)
            .bind(project_id)
            .fetch_all(pool)
            .await?;

            let mut seen = HashSet::new();
            let mut cookie_pairs = Vec::new();
            for item in cookie_rows {
                let dedup_key = format!("{}\u{0000}{}", item.name, item.path);
                if seen.insert(dedup_key) {
                    cookie_pairs.push(format!("{}={}", item.name, item.value));
                }
            }

            if !cookie_pairs.is_empty() {
                let cookie_value = cookie_pairs.join("; ");
                if let Some(existing) = resolved_params
                    .headers
                    .iter_mut()
                    .find(|h| h.enabled && h.key.eq_ignore_ascii_case("cookie"))
                {
                    if existing.value.is_empty() {
                        existing.value = cookie_value;
                    } else {
                        existing.value = format!("{}; {}", existing.value, cookie_value);
                    }
                } else {
                    resolved_params.headers.push(ParamItem {
                        key: "Cookie".to_string(),
                        value: cookie_value,
                        enabled: true,
                    });
                }
            }
        }
    }

    // 1. 发送请求
    let mut resp = send(http, &resolved_params)
        .await
        .map_err(crate::error::AppError::Custom)?;

    // 2. 构造请求快照
    let snapshot = serde_json::to_string(&resolved_params)
        .unwrap_or_else(|_| "{}".to_string());

    // 3. 响应头序列化
    let resp_headers_json = serde_json::to_string(&resp.headers)
        .unwrap_or_else(|_| "[]".to_string());

    // 4. 写入 request_history（request_id 为空时跳过，避免写 id=0 的孤儿行）
    if let Some(rid) = request_id {
        // 先插入占位符获取 history_id
        let history_id: i64 = sqlx::query_scalar(
            r#"
            INSERT INTO request_history
                (request_id, test_case_id, status_code, response_time_ms, request_snapshot,
                 response_body, is_truncated, response_headers)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            RETURNING id
            "#,
        )
        .bind(rid)
        .bind(test_case_id)
        .bind(resp.status_code as i64)
        .bind(resp.elapsed_ms as i64)
        .bind(&snapshot)
        .bind("")  // 占位，稍后更新
        .bind(if resp.is_truncated { 1i64 } else { 0i64 })
        .bind(&resp_headers_json)
        .fetch_one(pool)
        .await?;

        // 保存响应体（超过阈值时存文件）
        let (stored_body, is_file) = save_response_body_if_large(history_id, &resp.body)?;
        if is_file {
            sqlx::query("UPDATE request_history SET response_body = ? WHERE id = ?")
                .bind(&stored_body)
                .bind(history_id)
                .execute(pool)
                .await?;
            // 替换 resp.body 为标记，前端才能显示占位提示
            resp.body = stored_body;
        } else {
            sqlx::query("UPDATE request_history SET response_body = ? WHERE id = ?")
                .bind(&stored_body)
                .bind(history_id)
                .execute(pool)
                .await?;
        }

        resp.history_id = history_id;
    }
    Ok(resp)
}

/// 发送 HTTP 请求，写入 request_history，返回响应结果
#[tauri::command]
pub async fn send_request(
    db: State<'_, AppDb>,
    http: State<'_, HttpClient>,
    request_id: i64,
    test_case_id: Option<i64>,
    params: SendRequestParams,
    env_id: Option<i64>,
    project_id: Option<i64>,
) -> CmdResult<HttpResponse> {
    Ok(send_request_impl(
        &db.0, &http.0, Some(request_id), test_case_id, params, env_id, project_id,
    ).await?)
}

/// 获取接口最近 20 条历史记录（轻量：不含 response_body / request_snapshot /
/// response_headers，列表 UI 只用 status/time；diff 与回填走 get_history_record 单条补拉）
#[tauri::command]
pub async fn list_history(
    db: State<'_, AppDb>,
    request_id: i64,
    test_case_id: Option<i64>,
) -> CmdResult<Vec<HistoryRecord>> {
    let rows = sqlx::query_as::<_, HistoryRecord>(
        r#"
        SELECT id, request_id, test_case_id, status_code, response_time_ms,
               NULL AS request_snapshot, NULL AS response_body, is_truncated,
               NULL AS response_headers, created_at
        FROM request_history
        WHERE request_id = ? AND (?2 IS NULL OR test_case_id = ?2)
        ORDER BY created_at DESC
        LIMIT 20
        "#,
    )
    .bind(request_id)
    .bind(test_case_id)
    .fetch_all(&db.0)
    .await?;
    Ok(rows)
}

/// 打开响应文件所在位置（资源管理器定位）
#[tauri::command]
pub async fn open_response_file(history_id: i64) -> CmdResult<()> {
    let file_path = get_response_dir()?.join(format!("{history_id}.txt"));

    if !file_path.exists() {
        return Err(crate::error::AppError::Custom(format!(
            "响应文件不存在: {}",
            file_path.display()
        )));
    }

    // 复用 docs.rs 的 reveal_in_explorer 逻辑
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(format!("/select,{}", file_path.display()))
            .spawn()
            .map_err(|e| crate::error::AppError::Custom(format!("打开资源管理器失败: {e}")))?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg("-R")
            .arg(&file_path)
            .spawn()
            .map_err(|e| crate::error::AppError::Custom(format!("打开 Finder 失败: {e}")))?;
    }
    #[cfg(target_os = "linux")]
    {
        let dir = file_path.parent().unwrap_or(&file_path);
        std::process::Command::new("xdg-open")
            .arg(dir)
            .spawn()
            .map_err(|e| crate::error::AppError::Custom(format!("打开文件管理器失败: {e}")))?;
    }

    Ok(())
}

/// 清理历史记录的策略
#[derive(Debug, serde::Deserialize)]
pub struct CleanupHistoryParams {
    /// 按天数清理：删除 N 天前的历史记录（None 表示不按天数清理）
    pub days: Option<i64>,
    /// 按数量清理：每个接口只保留最近 N 条历史（None 表示不按数量清理）
    pub keep_per_request: Option<i64>,
    /// 按接口清理：只清理指定接口的历史记录（None 表示清理所有接口）
    pub request_id: Option<i64>,
    /// 按项目清理：只清理指定项目的历史记录（None 表示清理所有项目）
    pub project_id: Option<i64>,
    /// 是否同时清理响应文件（默认 true）
    #[serde(default = "default_true")]
    pub cleanup_files: bool,
    /// 是否同时清理用例执行历史（默认 true）
    #[serde(default = "default_true")]
    pub cleanup_test_case_history: bool,
}

fn default_true() -> bool { true }

/// 清理历史记录（支持按天数、按数量、按接口三种策略）
#[tauri::command]
pub async fn cleanup_history(
    db: State<'_, AppDb>,
    params: CleanupHistoryParams,
) -> CmdResult<CleanupHistoryResult> {
    Ok(cleanup_history_impl(&db.0, params).await?)
}

/// 核心逻辑（供 Tauri command 和 HTTP bridge 共用）
pub async fn cleanup_history_impl(
    pool: &SqlitePool,
    params: CleanupHistoryParams,
) -> Result<CleanupHistoryResult, crate::error::AppError> {
    let mut deleted_count = 0i64;
    let mut deleted_files = 0i64;
    let mut deleted_test_case_history = 0i64;

    // 收集要删除的历史记录 ID（用于清理文件）
    let mut ids_to_delete: Vec<i64> = Vec::new();

    // 构建 WHERE 条件
    let mut conditions: Vec<String> = Vec::new();

    if let Some(rid) = params.request_id {
        conditions.push(format!("request_id = {}", rid));
    }

    if let Some(pid) = params.project_id {
        // 通过 request_id 关联到 api_requests，再关联到 collections 获取 project_id
        conditions.push(format!(
            "request_id IN (SELECT r.id FROM api_requests r JOIN collections c ON r.collection_id = c.id WHERE c.project_id = {})",
            pid
        ));
    }

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("AND {}", conditions.join(" AND "))
    };

    // 策略 1：按天数清理
    if let Some(days) = params.days {
        let sql = format!(
            "SELECT id, response_body FROM request_history WHERE created_at < datetime('now', '-{} days') {}",
            days, where_clause
        );
        let rows: Vec<(i64, Option<String>)> = sqlx::query_as(&sql)
            .fetch_all(pool)
            .await?;

        for (id, response_body) in rows {
            ids_to_delete.push(id);
            let _ = response_body; // 稍后统一清理文件
        }
    }

    // 策略 2：按数量清理（每个接口只保留最近 N 条）
    if let Some(keep) = params.keep_per_request {
        let sql = format!(
            r#"
            SELECT id, response_body FROM request_history
            WHERE id NOT IN (
                SELECT id FROM request_history h1
                WHERE 1=1 {}
                  AND (
                    SELECT COUNT(*) FROM request_history h2
                    WHERE h2.request_id = h1.request_id
                      AND h2.created_at >= h1.created_at
                ) <= ?
            )
            {}
            "#,
            where_clause, where_clause
        );
        let rows: Vec<(i64, Option<String>)> = sqlx::query_as(&sql)
            .bind(keep)
            .fetch_all(pool)
            .await?;

        for (id, _response_body) in rows {
            if !ids_to_delete.contains(&id) {
                ids_to_delete.push(id);
            }
        }
    }

    // 策略 3：按接口或项目清理（清理所有历史）
    if (params.request_id.is_some() || params.project_id.is_some())
        && params.days.is_none()
        && params.keep_per_request.is_none()
    {
        let sql = format!(
            "SELECT id, response_body FROM request_history WHERE 1=1 {}",
            where_clause
        );
        let rows: Vec<(i64, Option<String>)> = sqlx::query_as(&sql)
            .fetch_all(pool)
            .await?;

        for (id, _response_body) in rows {
            if !ids_to_delete.contains(&id) {
                ids_to_delete.push(id);
            }
        }
    }

    // 执行删除
    if !ids_to_delete.is_empty() {
        // 清理响应文件
        if params.cleanup_files {
            let response_dir = get_response_dir()?;
            for id in &ids_to_delete {
                let file_path = response_dir.join(format!("{id}.txt"));
                if file_path.exists() {
                    std::fs::remove_file(&file_path).ok();
                    deleted_files += 1;
                }
            }
        }

        // 删除数据库记录
        for id in &ids_to_delete {
            sqlx::query("DELETE FROM request_history WHERE id = ?")
                .bind(id)
                .execute(pool)
                .await?;
            deleted_count += 1;
        }
    }

    // 同步清理用例执行历史
    if params.cleanup_test_case_history {
        let tc_where = if let Some(pid) = params.project_id {
            format!(
                "AND test_case_id IN (
                    SELECT tc.id FROM test_cases tc
                    JOIN collections c ON tc.collection_id = c.id
                    WHERE c.project_id = {}
                )",
                pid
            )
        } else if let Some(rid) = params.request_id {
            format!(
                "AND test_case_id IN (SELECT id FROM test_cases WHERE request_id = {})",
                rid
            )
        } else {
            String::new()
        };

        let sql = format!(
            "DELETE FROM test_case_history WHERE 1=1 {}",
            tc_where
        );
        let result = sqlx::query(&sql).execute(pool).await?;
        deleted_test_case_history = result.rows_affected() as i64;
    }

    Ok(CleanupHistoryResult {
        deleted_records: deleted_count,
        deleted_files,
        deleted_test_case_history,
    })
}

/// 清理历史记录的结果
#[derive(Debug, serde::Serialize)]
pub struct CleanupHistoryResult {
    /// 删除的历史记录数
    pub deleted_records: i64,
    /// 删除的响应文件数
    pub deleted_files: i64,
    /// 删除的用例执行历史数
    pub deleted_test_case_history: i64,
}

/// 按 id 取单条完整历史记录（含 response_body / request_snapshot / response_headers）
#[tauri::command]
pub async fn get_history_record(
    db: State<'_, AppDb>,
    id: i64,
) -> CmdResult<HistoryRecord> {
    let row = sqlx::query_as::<_, HistoryRecord>(
        r#"
        SELECT id, request_id, test_case_id, status_code, response_time_ms,
               request_snapshot, response_body, is_truncated,
               response_headers, created_at
        FROM request_history WHERE id = ?
        "#,
    )
    .bind(id)
    .fetch_one(&db.0)
    .await?;
    Ok(row)
}
