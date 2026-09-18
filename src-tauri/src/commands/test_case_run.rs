// 1.0.5：跑用例 —— 用用例自身参数快照发请求，按断言求值，回写 last_status

use crate::{
    assertion::{evaluate_all, parse_assertions, AssertionResult},
    db::AppDb,
    error::CmdResult,
    http::{
        client::{send, ParamItem, SendRequestParams},
        variable::replace_variables,
        HttpClient,
    },
    types::{EnvVariable, Environment, TestCase},
};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use tauri::State;

#[derive(Debug, Serialize, Deserialize)]
pub struct RunCaseResult {
    pub test_case_id: i64,
    /// "passed" | "failed" | "error"（error = 网络层失败，断言无法判定）
    pub status: String,
    pub status_code: Option<u16>,
    pub elapsed_ms: u64,
    pub response_body: String,
    pub assertions: Vec<AssertionResult>,
    /// 通过的断言数 / 总断言数
    pub passed_count: usize,
    pub total_count: usize,
    pub error_message: Option<String>,
}

/// 把用例的 JSON 字符串字段解析成 ParamItem 数组（容错：空/坏 JSON → 空 Vec）
fn parse_params(raw: &str) -> Vec<ParamItem> {
    if raw.trim().is_empty() {
        return Vec::new();
    }
    serde_json::from_str(raw).unwrap_or_default()
}

/// 核心逻辑（供 Tauri command 和 HTTP bridge 共用）
pub async fn run_test_case_impl(
    pool: &SqlitePool,
    http: &reqwest::Client,
    test_case_id: i64,
    env_id: Option<i64>,
) -> Result<RunCaseResult, crate::error::AppError> {
    // 1. 加载用例
    let tc = sqlx::query_as::<_, TestCase>(
        "SELECT id, request_id, collection_id, name, description, source, case_type, \
         method, url, headers, params, body_type, body, assertions, last_run_at, last_status, \
         last_duration_ms, last_response, starred, enabled, sort_order, created_at, updated_at \
         FROM test_cases WHERE id=?",
    )
    .bind(test_case_id)
    .fetch_one(pool)
    .await?;

    let request_id = tc.request_id.ok_or_else(|| {
        crate::error::AppError::Custom("用例未关联接口，无法执行".to_string())
    })?;

    // 2. 组装 SendRequestParams（用例自身快照优先；method/url 为空时回落到接口定义）
    let (method, url, headers, params, body_type, body) = if tc.method.as_deref().unwrap_or("").is_empty()
        || tc.url.as_deref().unwrap_or("").is_empty()
    {
        let req = sqlx::query_as::<_, (String, String, String, String, String, String)>(
            "SELECT method, url, headers, params, body_type, body FROM api_requests WHERE id=?",
        )
        .bind(request_id)
        .fetch_one(pool)
        .await?;
        (
            tc.method.clone().unwrap_or_default().if_empty(req.0),
            tc.url.clone().unwrap_or_default().if_empty(req.1),
            if tc.headers.trim().is_empty() || tc.headers == "[]" { req.2 } else { tc.headers.clone() },
            if tc.params.trim().is_empty() || tc.params == "[]" { req.3 } else { tc.params.clone() },
            tc.body_type.clone().unwrap_or_default().if_empty(req.4),
            tc.body.clone().unwrap_or_default().if_empty(req.5),
        )
    } else {
        (
            tc.method.clone().unwrap_or_default(),
            tc.url.clone().unwrap_or_default(),
            tc.headers.clone(),
            tc.params.clone(),
            tc.body_type.clone().unwrap_or_default(),
            tc.body.clone().unwrap_or_default(),
        )
    };

    let mut send_params = SendRequestParams {
        method,
        url,
        query_params: parse_params(&params),
        headers: parse_params(&headers),
        body_type,
        body,
        path_params: Vec::new(),
        auth_type: String::new(),
        auth_config: String::new(),
    };

    // 3. 环境变量替换（env_id=None 时回落到该项目 is_active=1 的环境）
    let effective_env_id = match env_id {
        Some(eid) => Some(eid),
        None => {
            let pid: Option<i64> = sqlx::query_scalar(
                "SELECT c.project_id FROM test_cases tc \
                 JOIN collections c ON tc.collection_id = c.id WHERE tc.id = ?",
            )
            .bind(test_case_id)
            .fetch_optional(pool)
            .await?
            .flatten();
            match pid {
                Some(pid) => sqlx::query_scalar(
                    "SELECT id FROM environments WHERE project_id = ? AND is_active = 1 LIMIT 1",
                )
                .bind(pid)
                .fetch_optional(pool)
                .await?,
                None => None,
            }
        }
    };
    if let Some(eid) = effective_env_id {
        let env = sqlx::query_as::<_, Environment>(
            "SELECT id, project_id, name, base_url, is_active, created_at FROM environments WHERE id=?",
        )
        .bind(eid)
        .fetch_one(pool)
        .await?;
        let env_vars = sqlx::query_as::<_, EnvVariable>(
            "SELECT id, env_id, key, value, description, enabled FROM env_variables WHERE env_id=? AND enabled=1",
        )
        .bind(eid)
        .fetch_all(pool)
        .await?;

        let mut variables = std::collections::HashMap::new();
        if let Some(base_url) = env.base_url {
            variables.insert("base_url".to_string(), base_url);
        }
        for item in env_vars {
            variables.insert(item.key, item.value);
        }
        send_params.url = replace_variables(&send_params.url, &variables, false);
        let is_json = send_params.body_type == "raw_json";
        send_params.body = replace_variables(&send_params.body, &variables, is_json);
        for h in send_params.headers.iter_mut().filter(|h| h.enabled) {
            h.value = replace_variables(&h.value, &variables, false);
        }

        // 相对 URL（如 /api/test）自动拼 base_url，避免 reqwest 解析失败
        if !send_params.url.starts_with("http://") && !send_params.url.starts_with("https://") {
            if let Some(base) = variables.get("base_url") {
                let base = base.trim_end_matches('/');
                let path = if send_params.url.starts_with('/') {
                    send_params.url.clone()
                } else {
                    format!("/{}", send_params.url)
                };
                send_params.url = format!("{base}{path}");
            }
        }
    }

    // 4. 发请求
    let send_result = send(http, &send_params).await;

    let (status_code, elapsed_ms, response_body, error_message) = match send_result {
        Ok(resp) => (Some(resp.status_code), resp.elapsed_ms, resp.body, None),
        Err(e) => (None, 0, String::new(), Some(e)),
    };

    // 5. 断言求值（网络失败 → status=error）
    let assertions = parse_assertions(&tc.assertions);
    let (case_status, assertion_results) = if let Some(code) = status_code {
        let results = evaluate_all(&assertions, code, &response_body);
        let status = if results.is_empty() {
            if (200..300).contains(&code) { "passed" } else { "failed" }
        } else if results.iter().all(|r| r.passed) {
            "passed"
        } else {
            "failed"
        };
        (status.to_string(), results)
    } else {
        ("error".to_string(), Vec::new())
    };

    let passed_count = assertion_results.iter().filter(|r| r.passed).count();
    let total_count = assertion_results.len();

    // 6. 回写 test_cases 执行结果
    let preview: String = response_body.chars().take(1024).collect();
    sqlx::query(
        "UPDATE test_cases SET last_run_at=datetime('now'), last_status=?, \
         last_duration_ms=?, last_response=?, updated_at=datetime('now') WHERE id=?",
    )
    .bind(&case_status)
    .bind(elapsed_ms as i64)
    .bind(&preview)
    .bind(test_case_id)
    .execute(pool)
    .await?;

    // 7. 写执行历史
    sqlx::query(
        "INSERT INTO test_case_history (test_case_id, status_code, duration_ms, response_preview, error_message) \
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(test_case_id)
    .bind(status_code.map(|c| c as i64))
    .bind(elapsed_ms as i64)
    .bind(&preview)
    .bind(&error_message)
    .execute(pool)
    .await?;

    Ok(RunCaseResult {
        test_case_id,
        status: case_status,
        status_code,
        elapsed_ms,
        response_body,
        assertions: assertion_results,
        passed_count,
        total_count,
        error_message,
    })
}

/// Tauri command 包装
#[tauri::command]
pub async fn run_test_case(
    db: State<'_, AppDb>,
    http: State<'_, HttpClient>,
    test_case_id: i64,
    env_id: Option<i64>,
) -> CmdResult<RunCaseResult> {
    Ok(run_test_case_impl(&db.0, &http.0, test_case_id, env_id).await?)
}

/// String::if_empty —— 空则用 fallback
trait IfEmpty {
    fn if_empty(self, fallback: String) -> String;
}
impl IfEmpty for String {
    fn if_empty(self, fallback: String) -> String {
        if self.trim().is_empty() { fallback } else { self }
    }
}
