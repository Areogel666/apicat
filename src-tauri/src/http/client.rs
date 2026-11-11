use crate::types::HttpResponse;
use reqwest::{Method};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// 响应体内存保留上限（安全兜底）。
///
/// 2026-09：由 2MB 提到 32MB。原 2MB 是「响应体全部存 SQLite」时代的保护，但 1.0.5 起
/// 超过 RESPONSE_FILE_THRESHOLD(1MB) 的响应改为落文件系统，此时该上限会**连带截断落盘内容**
/// —— 10MB 的响应存进文件的也只有前 2MB，超出部分永久丢失。
/// 现在 1MB–32MB 全量落文件（覆盖几乎所有真实接口响应），仅超过 32MB 才截断并置 is_truncated，
/// 以保住「病态大响应不把内存打爆」的兜底（`resp.bytes()` 本身已全量读入内存，本上限只约束保留时长）。
const MAX_BODY_SIZE: usize = 32 * 1024 * 1024; // 32MB

/// 前端传入的请求参数（扁平结构，易于 Tauri IPC 序列化）
#[derive(Debug, Serialize, Deserialize, Clone)]  // Serialize 用于 serde_json::to_string 生成 request_snapshot
pub struct SendRequestParams {
    pub method: String,
    pub url: String,
    /// query params: [(key, value, enabled)]
    pub query_params: Vec<ParamItem>,
    /// headers: [(key, value, enabled)]
    pub headers: Vec<ParamItem>,
    pub body_type: String,  // none | raw_json | json_table | raw_text | form_urlencoded | form_data
    pub body: String,
    /// path params 已在前端替换进 url，这里仅作记录快照用
    pub path_params: Vec<ParamItem>,
    /// 认证类型：none | bearer | basic | api_key
    #[serde(default)]
    pub auth_type: String,
    /// 认证配置 JSON：
    ///   bearer:  {"token": "..."}
    ///   basic:   {"username": "...", "password": "..."}
    ///   api_key: {"key": "...", "value": "...", "in": "header"|"query"}
    #[serde(default)]
    pub auth_config: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ParamItem {
    pub key: String,
    pub value: String,
    pub enabled: bool,
    /// 字段类型标记（前端 ParamItem.type，自由文本；"file" = 文件字段，value 存本地路径）。
    /// 旧 body JSON 无此字段，serde default 兜底为空串；导入数据里也可能是显式 null。
    #[serde(rename = "type", default, deserialize_with = "null_as_empty_string")]
    pub field_type: String,
    /// 参数描述（前端 ParamItem.description）。旧数据无此字段时为空串。
    /// 随发送快照落库，历史回填才能恢复描述。
    #[serde(
        default,
        skip_serializing_if = "String::is_empty",
        deserialize_with = "null_as_empty_string"
    )]
    pub description: String,
}

/// 把 JSON null 反序列化成空串；字段缺失仍由 `default` 兜底。
/// 导入/旧数据里存在 `"type": null`、`"description": null`，直接按 String
/// 反序列化会报 `invalid type: null, expected a string`，导致整个 send_request 失败。
fn null_as_empty_string<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Ok(Option::<String>::deserialize(deserializer)?.unwrap_or_default())
}

/// 构建并发送 HTTP 请求，返回 HttpResponse（不含 history_id，由上层写库后填入）
/// client 参数由全局 HttpClient State 提供，复用连接池
pub async fn send(client: &reqwest::Client, params: &SendRequestParams) -> Result<HttpResponse, String> {
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

    // 保留原始 URL 副本，api_key query 模式时用于重建 builder
    let base_url = parsed_url.clone();

    let mut builder = client.request(method.clone(), parsed_url);

    // Form Data 的 Content-Type（含 boundary）由 reqwest 自动生成，手动填写会发坏请求，
    // 发送时直接忽略该 header（前端发送时已有提示，此处兜底 Bridge 等直调入口）
    let skip_manual_ct = params.body_type == "form_data";

    // ── 添加 Headers ──────────────────────────────────────────
    for h in params.headers.iter().filter(|h| {
        h.enabled && !h.key.is_empty()
            && !(skip_manual_ct && h.key.eq_ignore_ascii_case("content-type"))
    }) {
        builder = builder.header(&h.key, &h.value);
    }

    // ── 注入 Auth Header（auth_type 优先级高于手动 headers）──
    if !params.auth_type.is_empty() && params.auth_type != "none" {
        let cfg: serde_json::Value =
            serde_json::from_str(&params.auth_config).unwrap_or(serde_json::Value::Null);

        match params.auth_type.as_str() {
            "bearer" => {
                if let Some(token) = cfg["token"].as_str() {
                    builder = builder.header("Authorization", format!("Bearer {token}"));
                }
            }
            "basic" => {
                let username = cfg["username"].as_str().unwrap_or("");
                let password = cfg["password"].as_str().unwrap_or("");
                use base64::Engine as _;
                let encoded = base64::engine::general_purpose::STANDARD
                    .encode(format!("{username}:{password}"));
                builder = builder.header("Authorization", format!("Basic {encoded}"));
            }
            "api_key" => {
                let key = cfg["key"].as_str().unwrap_or("X-Api-Key");
                let value = cfg["value"].as_str().unwrap_or("");
                let location = cfg["in"].as_str().unwrap_or("header");
                if location == "header" {
                    builder = builder.header(key, value);
                } else {
                    // query 方式：追加到已构建的 URL（使用保留的 base_url 副本）
                    let mut url_with_key = base_url.clone();
                    url_with_key.query_pairs_mut().append_pair(key, value);
                    builder = client.request(method.clone(), url_with_key);
                    // 重新添加 headers（因为 builder 被重置了）；form_data 时同样忽略手动 Content-Type
                    for h in params.headers.iter().filter(|h| {
                        h.enabled && !h.key.is_empty()
                            && !(skip_manual_ct && h.key.eq_ignore_ascii_case("content-type"))
                    }) {
                        builder = builder.header(&h.key, &h.value);
                    }
                }
            }
            _ => {}
        }
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
                .body(encode_form_urlencoded(&params.body));
        }
        "form_data" => {
            // body 为 JSON 数组 [{key, value, enabled, type}]；type="file" 的字段
            // value 存本地绝对路径，发送时读盘构建文件 part（MIME 按扩展名推断）
            let fields: Vec<ParamItem> = serde_json::from_str(&params.body)
                .map_err(|e| format!("Form Data body 解析失败（应为字段 JSON 数组）: {e}"))?;
            let mut form = reqwest::multipart::Form::new();
            for f in fields.iter().filter(|f| f.enabled && !f.key.is_empty()) {
                if f.field_type == "file" {
                    let bytes = std::fs::read(&f.value).map_err(|e| {
                        format!("字段「{}」的文件读取失败（{}）: {e}", f.key, f.value)
                    })?;
                    let part = reqwest::multipart::Part::bytes(bytes)
                        .file_name(file_name_of(&f.value))
                        .mime_str(mime_for_path(&f.value))
                        .map_err(|e| format!("字段「{}」MIME 设置失败: {e}", f.key))?;
                    form = form.part(f.key.clone(), part);
                } else {
                    form = form.text(f.key.clone(), f.value.clone());
                }
            }
            builder = builder.multipart(form);
        }
        "json_table" => {
            // body 为 JSON 数组 [{key, value, enabled, type, description}]，序列化为 JSON 对象
            let fields: Vec<ParamItem> =
                serde_json::from_str(&params.body).unwrap_or_default();
            let mut map = serde_json::Map::new();
            for f in fields.iter().filter(|f| f.enabled && !f.key.is_empty()) {
                // 尝试解析 value 为 JSON（数字/布尔/对象），否则当字符串
                let val = serde_json::from_str::<serde_json::Value>(&f.value)
                    .unwrap_or_else(|_| serde_json::Value::String(f.value.clone()));
                map.insert(f.key.clone(), val);
            }
            let json_body = serde_json::Value::Object(map).to_string();
            builder = builder
                .header("Content-Type", "application/json")
                .body(json_body);
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
        // 同理：cookie 注入发生在 command 层，跳过数也由那里回填
        cookie_skipped: 0,
    })
}

/// URL-Encoded body 的「线上形态」是 `k=v&...`，但 1.0.4 起 UI 的「存储形态」
/// 是结构化 ParamItem[]，普通发送由前端 `syncUrlencodedData()` 预先编码。
/// 用例执行 / Bridge 等入口不经过前端，若把 JSON 数组原文当表单发出去，
/// 服务端解析不到任何字段（实测 420「参数不能为空 sdk」的根因）。
///
/// 在发送边界做幂等兜底：能解析为 ParamItem[] 就按表单编码（只取 enabled），
/// 旧 k=v 文本或坏数据原样发送，不影响已有调用方。
fn encode_form_urlencoded(body: &str) -> String {
    let trimmed = body.trim_start();
    if !trimmed.starts_with('[') {
        return body.to_string();
    }
    let Ok(fields) = serde_json::from_str::<Vec<ParamItem>>(trimmed) else {
        return body.to_string();
    };
    // 复用 reqwest::Url 的 query 序列化器，与 query 参数共用同一套
    // application/x-www-form-urlencoded 编码规则（空格 -> +、UTF-8 百分号编码）
    let mut url = reqwest::Url::parse("http://apicat.local/").expect("dummy url 恒定合法");
    {
        let mut pairs = url.query_pairs_mut();
        for f in fields.iter().filter(|f| f.enabled && !f.key.is_empty()) {
            pairs.append_pair(&f.key, &f.value);
        }
    }
    url.query().unwrap_or("").to_string()
}

/// 从路径提取文件名（multipart part 的 filename，服务器多靠它识别文件类型）
pub(crate) fn file_name_of(path: &str) -> String {
    std::path::Path::new(path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("file")
        .to_string()
}

/// 按扩展名推断文件 part 的 MIME；推断不出用 octet-stream 兜底。
/// 覆盖常见办公/图片/压缩/媒体类型即可，表格/图片/二进制同一条上传路径
fn mime_for_path(path: &str) -> &'static str {
    let ext = std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match ext.as_str() {
        "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        "xls" => "application/vnd.ms-excel",
        "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "doc" => "application/msword",
        "pptx" => "application/vnd.openxmlformats-officedocument.presentationml.presentation",
        "pdf" => "application/pdf",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "bmp" => "image/bmp",
        "svg" => "image/svg+xml",
        "csv" => "text/csv",
        "json" => "application/json",
        "xml" => "application/xml",
        "txt" | "log" | "md" => "text/plain",
        "html" | "htm" => "text/html",
        "zip" => "application/zip",
        "gz" => "application/gzip",
        "tar" => "application/x-tar",
        "mp3" => "audio/mpeg",
        "mp4" => "video/mp4",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 导入数据里可能出现 `"type": null` / `"description": null`，不能因此让整次发送失败
    #[test]
    fn param_item_tolerates_null_metadata() {
        let p: ParamItem = serde_json::from_str(
            r#"{"key":"ram_total","value":"4294967296","enabled":true,"type":null,"description":null}"#,
        )
        .expect("null type/description should deserialize");
        assert_eq!(p.field_type, "");
        assert_eq!(p.description, "");

        // 字段缺失同样兜底
        let p: ParamItem = serde_json::from_str(
            r#"{"key":"a","value":"1","enabled":true}"#,
        )
        .expect("missing type/description should deserialize");
        assert_eq!(p.field_type, "");
        assert_eq!(p.description, "");
    }

    #[test]
    fn mime_for_path_covers_common_types() {
        assert_eq!(
            mime_for_path("/a/b.xlsx"),
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
        );
        assert_eq!(mime_for_path("/a/b.PNG"), "image/png"); // 大小写不敏感
        assert_eq!(mime_for_path("/a/b.unknownext"), "application/octet-stream");
        assert_eq!(mime_for_path("noext"), "application/octet-stream");
    }

    #[test]
    fn file_name_of_extracts_last_segment() {
        assert_eq!(file_name_of("/home/u/导入表.xlsx"), "导入表.xlsx");
        assert_eq!(file_name_of("noext"), "noext");
    }

    /// 端到端：form_data 文件字段真的以 multipart part 发出（filename / MIME / 内容 /
    /// boundary 自动生成），且手动 Content-Type 被忽略
    #[tokio::test]
    async fn form_data_file_part_is_sent_as_multipart() {
        let dir = std::env::temp_dir().join("apicat_formdata_test");
        std::fs::create_dir_all(&dir).unwrap();
        let fpath = dir.join("t.xlsx");
        std::fs::write(&fpath, b"hello").unwrap();

        let app = axum::Router::new().route(
            "/upload",
            axum::routing::post(
                |headers: axum::http::HeaderMap, body: String| async move {
                    let ct = headers.get("content-type").unwrap().to_str().unwrap();
                    assert!(ct.starts_with("multipart/form-data; boundary="));
                    assert!(body.contains("name=\"file\""));
                    assert!(body.contains("filename=\"t.xlsx\""));
                    assert!(body.contains(
                        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
                    ));
                    assert!(body.contains("hello"));
                    axum::http::StatusCode::OK
                },
            ),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

        let params = SendRequestParams {
            method: "POST".into(),
            url: format!("http://{addr}/upload"),
            query_params: vec![],
            headers: vec![ParamItem {
                key: "Content-Type".into(),
                value: "text/plain".into(), // 手动值：应被忽略而非覆盖 multipart 头
                enabled: true,
                field_type: String::new(),
                description: String::new(),
            }],
            body_type: "form_data".into(),
            body: format!(
                "[{{\"key\":\"file\",\"value\":\"{}\",\"enabled\":true,\"type\":\"file\"}}]",
                fpath.display()
            ),
            path_params: vec![],
            auth_type: String::new(),
            auth_config: String::new(),
        };
        let client = reqwest::Client::new();
        let resp = send(&client, &params).await.unwrap();
        assert_eq!(resp.status_code, 200);
    }

    /// 端到端：结构化 ParamItem[] body（UI 的存储形态）在发送边界必须编码成 k=v。
    /// 用例执行 / Bridge 等入口没有前端 syncUrlencodedData() 那一步，漏编码会把
    /// JSON 数组原文当表单发出，服务端收不到任何字段（实测 420「参数不能为空 sdk」）。
    #[tokio::test]
    async fn form_urlencoded_structured_body_is_sent_as_kv() {
        let app = axum::Router::new().route(
            "/form",
            axum::routing::post(|headers: axum::http::HeaderMap, body: String| async move {
                let ct = headers.get("content-type").unwrap().to_str().unwrap();
                assert!(ct.starts_with("application/x-www-form-urlencoded"));
                assert!(body.contains("sdk=37"), "表单里缺 sdk=37，实际 body: {body}");
                assert!(body.contains("resolution=1600"), "表单里缺 resolution，实际 body: {body}");
                assert!(!body.contains("disabled="), "disabled 字段不应发送，实际 body: {body}");
                assert!(!body.starts_with('['), "不得把结构化 JSON 数组原文当表单发送");
                axum::http::StatusCode::OK
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

        let params = SendRequestParams {
            method: "POST".into(),
            url: format!("http://{addr}/form"),
            query_params: vec![],
            headers: vec![],
            body_type: "form_urlencoded".into(),
            body: r#"[{"key":"sdk","value":"37","enabled":true},{"key":"resolution","value":"1600*2560","enabled":true},{"key":"disabled","value":"x","enabled":false}]"#.into(),
            path_params: vec![],
            auth_type: String::new(),
            auth_config: String::new(),
        };
        let client = reqwest::Client::new();
        let resp = send(&client, &params).await.unwrap();
        assert_eq!(resp.status_code, 200);
    }

    /// 旧格式（已是 k=v 文本）必须原样透传，不能被二次编码
    #[tokio::test]
    async fn form_urlencoded_legacy_kv_body_passes_through() {
        let app = axum::Router::new().route(
            "/form",
            axum::routing::post(|body: String| async move {
                assert_eq!(body, "a=1&b=2");
                axum::http::StatusCode::OK
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

        let params = SendRequestParams {
            method: "POST".into(),
            url: format!("http://{addr}/form"),
            query_params: vec![],
            headers: vec![],
            body_type: "form_urlencoded".into(),
            body: "a=1&b=2".into(),
            path_params: vec![],
            auth_type: String::new(),
            auth_config: String::new(),
        };
        let client = reqwest::Client::new();
        let resp = send(&client, &params).await.unwrap();
        assert_eq!(resp.status_code, 200);
    }
}
