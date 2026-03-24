use crate::{
    db::AppDb,
    error::CmdResult,
    http::{client::{send, SendRequestParams}, HttpClient},
    types::{HistoryRecord, HttpResponse},
};
use tauri::State;

/// 发送 HTTP 请求，写入 request_history，返回响应结果
#[tauri::command]
pub async fn send_request(
    db: State<'_, AppDb>,
    http: State<'_, HttpClient>,
    request_id: i64,
    params: SendRequestParams,
) -> CmdResult<HttpResponse> {
    // 1. 发送请求（复用全局 Client 的连接池）
    let mut resp = send(&http.0, &params).await.map_err(crate::error::AppError::Custom)?;

    // 2. 构造请求快照（JSON 文本，用于历史回填）
    let snapshot = serde_json::to_string(&params)
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
