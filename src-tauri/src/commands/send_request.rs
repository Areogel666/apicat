use crate::{
    db::AppDb,
    error::CmdResult,
    http::{
        client::{send, SendRequestParams},
        variable::replace_variables,
        HttpClient,
    },
    types::{Cookie, EnvVariable, Environment, HistoryRecord, HttpResponse},
};
use std::collections::HashSet;
use tauri::State;

/// 发送 HTTP 请求，写入 request_history，返回响应结果
#[tauri::command]
pub async fn send_request(
    db: State<'_, AppDb>,
    http: State<'_, HttpClient>,
    request_id: i64,
    params: SendRequestParams,
    env_id: Option<i64>,
    project_id: Option<i64>,
) -> CmdResult<HttpResponse> {
    // 0. 复制参数并按需做变量替换
    let mut resolved_params = params.clone();

    if let Some(env_id) = env_id {
        // 加载环境信息（用于 base_url）
        let env = sqlx::query_as::<_, Environment>(
            "SELECT id, project_id, name, base_url, is_active, created_at FROM environments WHERE id=?",
        )
        .bind(env_id)
        .fetch_one(&db.0)
        .await?;

        // 加载启用状态的环境变量
        let env_vars = sqlx::query_as::<_, EnvVariable>(
            "SELECT id, env_id, key, value, description, enabled FROM env_variables WHERE env_id=? AND enabled=1",
        )
        .bind(env_id)
        .fetch_all(&db.0)
        .await?;

        let mut variables = std::collections::HashMap::new();
        if let Some(base_url) = env.base_url {
            variables.insert("base_url".to_string(), base_url);
        }
        for item in env_vars {
            variables.insert(item.key, item.value);
        }

        // URL / Body / Header 值都做替换
        resolved_params.url = replace_variables(&resolved_params.url, &variables, false);
        let is_json_body = resolved_params.body_type == "raw_json";
        resolved_params.body = replace_variables(&resolved_params.body, &variables, is_json_body);
        for header in resolved_params.headers.iter_mut().filter(|h| h.enabled) {
            header.value = replace_variables(&header.value, &variables, false);
        }
    }

    // 0.1 注入域名 Cookie（项目级优先覆盖全局同名+同 path）
    if let Ok(parsed_url) = reqwest::Url::parse(&resolved_params.url) {
        if let Some(domain) = parsed_url.host_str() {
            let cookie_rows = sqlx::query_as::<_, Cookie>(
                "SELECT id, scope_type, project_id, domain, name, value, path, expires_at, http_only, secure, enabled FROM cookies WHERE domain=? AND enabled=1 AND (scope_type='global' OR (scope_type='project' AND project_id=?)) ORDER BY scope_type DESC, id DESC",
            )
            .bind(domain)
            .bind(project_id)
            .fetch_all(&db.0)
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
                if let Some(existing_cookie_header) = resolved_params
                    .headers
                    .iter_mut()
                    .find(|h| h.enabled && h.key.eq_ignore_ascii_case("cookie"))
                {
                    if existing_cookie_header.value.is_empty() {
                        existing_cookie_header.value = cookie_value;
                    } else {
                        existing_cookie_header.value =
                            format!("{}; {}", existing_cookie_header.value, cookie_value);
                    }
                } else {
                    resolved_params.headers.push(crate::http::client::ParamItem {
                        key: "Cookie".to_string(),
                        value: cookie_value,
                        enabled: true,
                    });
                }
            }
        }
    }

    // 1. 发送请求（复用全局 Client 的连接池）
    let mut resp = send(&http.0, &resolved_params)
        .await
        .map_err(crate::error::AppError::Custom)?;

    // 2. 构造请求快照（JSON 文本，用于历史回填）
    let snapshot = serde_json::to_string(&resolved_params)
        .unwrap_or_else(|_| "{}".to_string());

    // 3. 响应头序列化为 JSON 数组 [["Header-Name","value"], ...]（保留顺序和重复 Header）
    let resp_headers_json = serde_json::to_string(&resp.headers)
        .unwrap_or_else(|_| "[]".to_string());

    // 4. 写入 request_history
    let history_id: i64 = sqlx::query_scalar(
        r#"
        INSERT INTO request_history
            (request_id, status_code, response_time_ms, request_snapshot,
             response_body, is_truncated, response_headers)
        VALUES (?, ?, ?, ?, ?, ?, ?)
        RETURNING id
        "#,
    )
    .bind(request_id)
    .bind(resp.status_code as i64)
    .bind(resp.elapsed_ms as i64)
    .bind(&snapshot)
    .bind(&resp.body)
    .bind(if resp.is_truncated { 1i64 } else { 0i64 })
    .bind(&resp_headers_json)
    .fetch_one(&db.0)
    .await?;

    resp.history_id = history_id;
    Ok(resp)
}

/// 获取接口最近 20 条历史记录
#[tauri::command]
pub async fn list_history(
    db: State<'_, AppDb>,
    request_id: i64,
) -> CmdResult<Vec<HistoryRecord>> {
    let rows = sqlx::query_as::<_, HistoryRecord>(
        r#"
        SELECT id, request_id, status_code, response_time_ms,
               request_snapshot, response_body, is_truncated,
               response_headers, created_at
        FROM request_history
        WHERE request_id = ?
        ORDER BY created_at DESC
        LIMIT 20
        "#,
    )
    .bind(request_id)
    .fetch_all(&db.0)
    .await?;
    Ok(rows)
}
