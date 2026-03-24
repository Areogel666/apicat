use crate::types::HttpResponse;
use reqwest::{Method};
use serde::{Deserialize, Serialize};
use std::time::Instant;

const MAX_BODY_SIZE: usize = 2 * 1024 * 1024; // 2MB，设计文档 §10.5

/// 前端传入的请求参数（扁平结构，易于 Tauri IPC 序列化）
#[derive(Debug, Serialize, Deserialize)]  // Serialize 用于 serde_json::to_string 生成 request_snapshot
pub struct SendRequestParams {
    pub method: String,
    pub url: String,
    /// query params: [(key, value, enabled)]
    pub query_params: Vec<ParamItem>,
    /// headers: [(key, value, enabled)]
    pub headers: Vec<ParamItem>,
    pub body_type: String,  // none | raw_json | raw_text | form_urlencoded
    pub body: String,
    /// path params 已在前端替换进 url，这里仅作记录快照用
    pub path_params: Vec<ParamItem>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ParamItem {
    pub key: String,
    pub value: String,
    pub enabled: bool,
}

/// 构建并发送 HTTP 请求，返回 HttpResponse（不含 history_id，由上层写库后填入）
pub async fn send(params: &SendRequestParams) -> Result<HttpResponse, String> {
    let method = params
        .method
        .parse::<Method>()
        .map_err(|e| format!("无效的 HTTP Method: {e}"))?;

    // ── 构建 URL + Query Params（使用 reqwest::Url 正确编码）────────
    let mut parsed_url = reqwest::Url::parse(&params.url)
        .map_err(|e| format!("无效的 URL: {e}"))?;
    {
        let enabled_query: Vec<_> = params
            .query_params
            .iter()
            .filter(|p| p.enabled && !p.key.is_empty())
            .collect();
        if !enabled_query.is_empty() {
            let mut pairs = parsed_url.query_pairs_mut();
            for p in enabled_query {
                pairs.append_pair(&p.key, &p.value);
            }
        }
    }

    // ── 构建 Client（忽略自签证书，设置超时）─────────────────────
    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("构建 HTTP Client 失败: {e}"))?;

    let mut builder = client.request(method, parsed_url);

    // ── 添加 Headers ──────────────────────────────────────────
    for h in params.headers.iter().filter(|h| h.enabled && !h.key.is_empty()) {
        builder = builder.header(&h.key, &h.value);
    }

    // ── 设置 Body ─────────────────────────────────────────────
    match params.body_type.as_str() {
        "raw_json" => {
            builder = builder
                .header("Content-Type", "application/json")
                .body(params.body.clone());
        }
        "raw_text" => {
            builder = builder
                .header("Content-Type", "text/plain")
                .body(params.body.clone());
        }
        "form_urlencoded" => {
            builder = builder
                .header("Content-Type", "application/x-www-form-urlencoded")
                .body(params.body.clone());
        }
        _ => {} // none: 不设置 body
    }

    // ── 发送请求 + 计时 ───────────────────────────────────────
    let start = Instant::now();
    let resp = builder.send().await.map_err(|e| format!("请求失败: {e}"))?;
    let elapsed_ms = start.elapsed().as_millis() as u64;

    let status_code = resp.status().as_u16();
    let status_text = resp.status().canonical_reason().unwrap_or("").to_string();

    // ── 响应 Headers ──────────────────────────────────────────
    let resp_headers: Vec<(String, String)> = resp
        .headers()
        .iter()
        .map(|(k, v)| {
            (
                k.as_str().to_string(),
                v.to_str().unwrap_or("").to_string(),
            )
        })
        .collect();

    // ── 响应 Body（截断保护）─────────────────────────────────
    let bytes = resp.bytes().await.map_err(|e| format!("读取响应体失败: {e}"))?;
    let body_size = bytes.len();
    let (body_str, is_truncated) = if body_size > MAX_BODY_SIZE {
        (
            String::from_utf8_lossy(&bytes[..MAX_BODY_SIZE]).into_owned(),
            true,
        )
    } else {
        (String::from_utf8_lossy(&bytes).into_owned(), false)
    };

    Ok(HttpResponse {
        status_code,
        status_text,
        headers: resp_headers,
        body: body_str,
        body_size,
        elapsed_ms,
        is_truncated,
        history_id: 0, // 由 send_request command 写库后填入
    })
}
