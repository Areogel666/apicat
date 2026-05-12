//! 导入导出命令 — 全部使用 sqlx::query_as / sqlx::query（非宏，无需 DATABASE_URL）

use crate::{db::AppDb, error::CmdResult, types::*};
use sqlx::SqliteConnection;
use tauri::State;

// ═══════════════════════════════════════════════════════════
//  URL path 段分类 helpers（Postman/OpenAPI 导入导出共用）
// ═══════════════════════════════════════════════════════════

/// 判断一个 path segment 是否是 `:xxx` 形态的 path variable。
fn is_colon_param(seg: &str) -> bool {
    seg.starts_with(':') && seg.len() > 1 && seg.chars().skip(1).all(is_param_name_char)
}

/// 判断一个 path segment 是否是 `{xxx}` 形态的 path variable。
fn is_brace_param(seg: &str) -> bool {
    seg.len() >= 3
        && seg.starts_with('{')
        && seg.ends_with('}')
        && !seg[1..seg.len() - 1].is_empty()
        && seg[1..seg.len() - 1].chars().all(is_param_name_char)
        // 排除 {{var}} 这种环境变量插值
        && !seg.starts_with("{{")
}

fn is_param_name_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '-'
}

/// 将 Postman URL raw 字符串拆解为 Postman v2.1 URL 对象结构：
///   {
///     "raw": "...",
///     "host": [...],           // host 按 `.` 切分；{{var}} 作为整体
///     "path": ["users", ":id", ...],
///     "query": [...],          // enabled query params
///     "variable": [{"key":"id","value":""}, ...]
///   }
///
/// 关键行为：
/// - 支持 `:id` 和 `{id}` 两种占位符输入；输出 path 数组里统一转为 `:id`（Postman 规范风）
/// - variable 数组根据 path 占位符生成；value 留空（ApiCat DB 里没有存 path variable 的值）
/// - 若 raw 不含协议头且不以 `/` 开头（典型场景：相对路径），host 数组为空
fn build_postman_url(raw: &str, query_params: &[serde_json::Value]) -> serde_json::Value {
    // 拆分 query / hash
    let (path_and_host, _query_str) = match raw.find(|c| c == '?' || c == '#') {
        Some(idx) => (&raw[..idx], &raw[idx..]),
        None => (raw, ""),
    };

    // 拆分协议+host 与 path
    let (host_raw, path_raw) = if let Some(rest) = path_and_host
        .strip_prefix("http://")
        .or_else(|| path_and_host.strip_prefix("https://"))
    {
        match rest.find('/') {
            Some(slash) => (&rest[..slash], &rest[slash..]),
            None => (rest, ""),
        }
    } else {
        // 无协议头：整串视为 path（含环境变量 {{baseUrl}} 前缀或纯相对路径）
        ("", path_and_host)
    };

    // host 数组：按 `.` 切分；但若整个 host 是 {{var}} 形式，保留整体
    let host: Vec<serde_json::Value> = if host_raw.is_empty() {
        Vec::new()
    } else if host_raw.starts_with("{{") && host_raw.ends_with("}}") {
        vec![serde_json::Value::String(host_raw.to_string())]
    } else {
        host_raw.split('.').map(|s| serde_json::Value::String(s.to_string())).collect()
    };

    // path 数组：按 `/` 切分后的非空段；占位符统一化为 `:xxx` 风格
    let mut path_arr: Vec<serde_json::Value> = Vec::new();
    let mut variables: Vec<serde_json::Value> = Vec::new();
    let mut used_keys: std::collections::HashSet<String> = Default::default();

    for seg in path_raw.split('/').filter(|s| !s.is_empty()) {
        let normalized = if is_colon_param(seg) {
            // `:id` 原样
            let name = seg[1..].to_string();
            if used_keys.insert(name.clone()) {
                variables.push(serde_json::json!({"key": name, "value": ""}));
            }
            seg.to_string()
        } else if is_brace_param(seg) {
            // `{id}` → `:id`
            let name = seg[1..seg.len() - 1].to_string();
            if used_keys.insert(name.clone()) {
                variables.push(serde_json::json!({"key": name, "value": ""}));
            }
            format!(":{}", name)
        } else {
            // 静态段 / {{var}} 环境变量 / 字面量 id
            seg.to_string()
        };
        path_arr.push(serde_json::Value::String(normalized));
    }

    // 重建 raw（占位符已统一为 :xxx），保证 raw / path / variable 三方一致
    let proto_prefix = if raw.starts_with("https://") {
        "https://"
    } else if raw.starts_with("http://") {
        "http://"
    } else {
        ""
    };
    let query_suffix = match raw.find(|c| c == '?' || c == '#') {
        Some(idx) => &raw[idx..],
        None => "",
    };
    let path_str: String = path_arr
        .iter()
        .filter_map(|v| v.as_str().map(|s| format!("/{}", s)))
        .collect();
    let rebuilt_raw = format!("{}{}{}{}", proto_prefix, host_raw, path_str, query_suffix);

    let mut obj = serde_json::json!({
        "raw": rebuilt_raw,
        "path": path_arr,
    });
    if !host.is_empty() {
        obj["host"] = serde_json::Value::Array(host);
    }
    if !query_params.is_empty() {
        obj["query"] = serde_json::Value::Array(query_params.to_vec());
    }
    if !variables.is_empty() {
        obj["variable"] = serde_json::Value::Array(variables);
    }
    obj
}

// ═══════════════════════════════════════════════════════════
//  导出
// ═══════════════════════════════════════════════════════════

#[tauri::command]
pub async fn export_apicat(db: State<'_, AppDb>, project_id: i64) -> CmdResult<String> {
    let (proj_name, proj_desc): (String, Option<String>) =
        sqlx::query_as("SELECT name, description FROM projects WHERE id=?")
            .bind(project_id)
            .fetch_one(&db.0)
            .await
            .map_err(|_| crate::error::AppError::Custom("项目不存在".to_string()))?;

    let envs = export_environments(&db, project_id).await?;
    let collections = export_collections_tree(&db, project_id, None).await?;

    let export = ApiCatExport {
        version: "1.0".to_string(),
        exported_at: chrono::Utc::now().to_rfc3339(),
        project: ExportProject {
            name: proj_name,
            description: proj_desc,
            environments: envs,
            collections,
        },
    };
    serde_json::to_string_pretty(&export)
        .map_err(|e| crate::error::AppError::Custom(format!("序列化失败: {e}")))
}

async fn export_environments(db: &AppDb, project_id: i64) -> CmdResult<Vec<ExportEnv>> {
    let envs: Vec<(i64, String, Option<String>, i64)> = sqlx::query_as(
        "SELECT id, name, base_url, is_active FROM environments WHERE project_id=?",
    )
    .bind(project_id)
    .fetch_all(&db.0)
    .await?;

    let mut result = Vec::new();
    for (env_id, env_name, base_url, is_active) in envs {
        let vars: Vec<(String, String, Option<String>, i64)> = sqlx::query_as(
            "SELECT key, value, description, enabled FROM env_variables WHERE env_id=?",
        )
        .bind(env_id)
        .fetch_all(&db.0)
        .await?;

        result.push(ExportEnv {
            name: env_name,
            base_url,
            is_active,
            variables: vars
                .into_iter()
                .map(|(key, value, description, enabled)| ExportEnvVar {
                    key,
                    value,
                    description,
                    enabled,
                })
                .collect(),
        });
    }
    Ok(result)
}

async fn export_collections_tree(
    db: &AppDb,
    project_id: i64,
    parent_id: Option<i64>,
) -> CmdResult<Vec<ExportCollection>> {
    let colls: Vec<(i64, String, i64)> = match parent_id {
        Some(pid) => sqlx::query_as(
            "SELECT id, name, sort_order FROM collections \
             WHERE project_id=? AND parent_id=? ORDER BY sort_order, id",
        )
        .bind(project_id)
        .bind(pid)
        .fetch_all(&db.0)
        .await?,
        None => sqlx::query_as(
            "SELECT id, name, sort_order FROM collections \
             WHERE project_id=? AND parent_id IS NULL ORDER BY sort_order, id",
        )
        .bind(project_id)
        .fetch_all(&db.0)
        .await?,
    };

    let mut result = Vec::new();
    for (coll_id, coll_name, sort_order) in colls {
        let reqs: Vec<(
            String, String, String,
            Option<String>, Option<String>,
            Option<String>, Option<String>,
            Option<String>, Option<String>,
            i64,
        )> = sqlx::query_as(
            "SELECT name, method, url, params, headers, body_type, body, \
                    auth_type, auth_config, sort_order \
             FROM api_requests WHERE collection_id=? ORDER BY sort_order, id",
        )
        .bind(coll_id)
        .fetch_all(&db.0)
        .await?;

        let children =
            Box::pin(export_collections_tree(db, project_id, Some(coll_id))).await?;

        result.push(ExportCollection {
            name: coll_name,
            sort_order,
            children,
            requests: reqs
                .into_iter()
                .map(
                    |(name, method, url, params, headers, body_type, body,
                      auth_type, auth_config, so)| ExportRequest {
                        name,
                        method,
                        url,
                        params: params.unwrap_or_default(),
                        headers: headers.unwrap_or_default(),
                        body_type: body_type.unwrap_or_default(),
                        body: body.unwrap_or_default(),
                        auth_type: auth_type.unwrap_or_default(),
                        auth_config: auth_config.unwrap_or_default(),
                        sort_order: so,
                    },
                )
                .collect(),
        });
    }
    Ok(result)
}

#[tauri::command]
pub async fn export_postman(db: State<'_, AppDb>, project_id: i64) -> CmdResult<String> {
    let (proj_name,): (String,) = sqlx::query_as("SELECT name FROM projects WHERE id=?")
        .bind(project_id)
        .fetch_one(&db.0)
        .await
        .map_err(|_| crate::error::AppError::Custom("项目不存在".to_string()))?;

    let collections = export_collections_tree(&db, project_id, None).await?;
    let postman = build_postman_collection(&proj_name, &collections);
    serde_json::to_string_pretty(&postman)
        .map_err(|e| crate::error::AppError::Custom(format!("序列化失败: {e}")))
}

fn build_postman_collection(name: &str, collections: &[ExportCollection]) -> serde_json::Value {
    fn build_items(colls: &[ExportCollection]) -> Vec<serde_json::Value> {
        let mut items = Vec::new();
        for coll in colls {
            let mut folder_items: Vec<serde_json::Value> = coll
                .requests
                .iter()
                .map(|r| {
                    let headers: Vec<serde_json::Value> =
                        serde_json::from_str::<Vec<serde_json::Value>>(&r.headers)
                            .unwrap_or_default()
                            .into_iter()
                            .filter(|h| {
                                h.get("enabled").and_then(|e| e.as_bool()).unwrap_or(true)
                            })
                            .map(|h| serde_json::json!({"key": h["key"], "value": h["value"]}))
                            .collect();

                    // 将启用的 query params 转为 Postman url.query 条目
                    let query_items: Vec<serde_json::Value> =
                        serde_json::from_str::<Vec<serde_json::Value>>(&r.params)
                            .unwrap_or_default()
                            .into_iter()
                            .filter(|p| {
                                p.get("enabled").and_then(|e| e.as_bool()).unwrap_or(true)
                            })
                            .map(|p| serde_json::json!({"key": p["key"], "value": p["value"]}))
                            .collect();

                    let body = if r.body_type == "raw_json" {
                        serde_json::json!({
                            "mode": "raw", "raw": r.body,
                            "options": {"raw": {"language": "json"}}
                        })
                    } else if r.body_type == "raw_text" {
                        // 保留 body 内容，标注为 text 语言
                        serde_json::json!({
                            "mode": "raw", "raw": r.body,
                            "options": {"raw": {"language": "text"}}
                        })
                    } else if r.body_type == "form_urlencoded" {
                        serde_json::json!({"mode": "urlencoded", "urlencoded": []})
                    } else {
                        serde_json::json!({"mode": "raw", "raw": ""})
                    };

                    // URL 对象：拆分 host/path/variable 数组，path params ({id}/:id) 填入 variable 数组
                    let url_obj = build_postman_url(&r.url, &query_items);

                    serde_json::json!({
                        "name": r.name,
                        "request": {
                            "method": r.method,
                            "header": headers,
                            "url": url_obj,
                            "body": body
                        },
                        "response": []
                    })
                })
                .collect();
            folder_items.extend(build_items(&coll.children));
            items.push(serde_json::json!({"name": coll.name, "item": folder_items}));
        }
        items
    }

    serde_json::json!({
        "info": {
            "name": name,
            "schema": "https://schema.getpostman.com/json/collection/v2.1.0/collection.json",
            "_postman_id": uuid::Uuid::new_v4().to_string(),
        },
        "item": build_items(collections),
    })
}

// ═══════════════════════════════════════════════════════════
//  导入
// ═══════════════════════════════════════════════════════════

#[tauri::command]
pub async fn import_apicat(
    db: State<'_, AppDb>,
    project_id: i64,
    json_content: String,
) -> CmdResult<i64> {
    let export: ApiCatExport = serde_json::from_str(&json_content)
        .map_err(|e| crate::error::AppError::Custom(format!("解析失败: {e}")))?;

    // 使用 sqlx 事务 API，避免连接池复用导致的嵌套事务错误
    let mut tx = db.0.begin().await?;
    let pid = import_apicat_inner(&mut *tx, project_id, &export).await?;
    tx.commit().await?;
    Ok(pid)
}

async fn import_apicat_inner(
    conn: &mut SqliteConnection,
    project_id: i64,
    export: &ApiCatExport,
) -> CmdResult<i64> {
    let pid = ensure_project(
        conn,
        project_id,
        &export.project.name,
        export.project.description.as_deref(),
    )
    .await?;

    for env in &export.project.environments {
        let (env_id,): (i64,) = sqlx::query_as(
            "INSERT INTO environments (project_id, name, base_url, is_active) \
             VALUES (?,?,?,?) RETURNING id",
        )
        .bind(pid)
        .bind(&env.name)
        .bind(&env.base_url)
        .bind(env.is_active)
        .fetch_one(&mut *conn)
        .await?;

        for var in &env.variables {
            sqlx::query(
                "INSERT INTO env_variables (env_id, key, value, description, enabled) \
                 VALUES (?,?,?,?,?)",
            )
            .bind(env_id)
            .bind(&var.key)
            .bind(&var.value)
            .bind(&var.description)
            .bind(var.enabled)
            .execute(&mut *conn)
            .await?;
        }
    }

    import_collections_tree(conn, pid, None, &export.project.collections).await?;
    Ok(pid)
}

#[async_recursion::async_recursion]
async fn import_collections_tree(
    conn: &mut SqliteConnection,
    project_id: i64,
    parent_id: Option<i64>,
    collections: &[ExportCollection],
) -> CmdResult<()> {
    for coll in collections {
        let (coll_id,): (i64,) = sqlx::query_as(
            "INSERT INTO collections (project_id, parent_id, name, sort_order) \
             VALUES (?,?,?,?) RETURNING id",
        )
        .bind(project_id)
        .bind(parent_id)
        .bind(&coll.name)
        .bind(coll.sort_order)
        .fetch_one(&mut *conn)
        .await?;

        for req in &coll.requests {
            sqlx::query(
                "INSERT INTO api_requests \
                 (collection_id, name, method, url, params, headers, body_type, body, \
                  auth_type, auth_config, sort_order) \
                 VALUES (?,?,?,?,?,?,?,?,?,?,?)",
            )
            .bind(coll_id)
            .bind(&req.name)
            .bind(&req.method)
            .bind(&req.url)
            .bind(&req.params)
            .bind(&req.headers)
            .bind(&req.body_type)
            .bind(&req.body)
            .bind(&req.auth_type)
            .bind(&req.auth_config)
            .bind(req.sort_order)
            .execute(&mut *conn)
            .await?;
        }

        import_collections_tree(conn, project_id, Some(coll_id), &coll.children).await?;
    }
    Ok(())
}

#[tauri::command]
pub async fn import_postman(
    db: State<'_, AppDb>,
    project_id: i64,
    json_content: String,
) -> CmdResult<i64> {
    let v: serde_json::Value = serde_json::from_str(&json_content)
        .map_err(|e| crate::error::AppError::Custom(format!("解析失败: {e}")))?;

    let coll_name = v["info"]["name"]
        .as_str()
        .unwrap_or("导入的接口集合")
        .to_string();

    // 使用 sqlx 事务 API，失败时依赖 Drop 自动回滚
    let mut tx = db.0.begin().await?;
    let pid = import_postman_inner(&mut *tx, project_id, &coll_name, &v).await?;
    tx.commit().await?;
    Ok(pid)
}

async fn import_postman_inner(
    conn: &mut SqliteConnection,
    project_id: i64,
    coll_name: &str,
    v: &serde_json::Value,
) -> CmdResult<i64> {
    let pid = ensure_project(conn, project_id, coll_name, None).await?;
    let empty: Vec<serde_json::Value> = vec![];
    let items = v["item"].as_array().unwrap_or(&empty).clone();
    import_postman_items(conn, pid, None, &items, 0).await?;
    Ok(pid)
}

#[async_recursion::async_recursion]
async fn import_postman_items(
    conn: &mut SqliteConnection,
    project_id: i64,
    parent_collection_id: Option<i64>,
    items: &[serde_json::Value],
    sort_offset: i64,
) -> CmdResult<()> {
    // 若根层有裸请求，需先建一个默认 collection 容纳
    let has_bare = items.iter().any(|i| i.get("item").is_none());
    let bare_coll_id: Option<i64> = if has_bare && parent_collection_id.is_none() {
        let (id,): (i64,) = sqlx::query_as(
            "INSERT INTO collections (project_id, parent_id, name, sort_order) \
             VALUES (?,NULL,'默认',0) RETURNING id",
        )
        .bind(project_id)
        .fetch_one(&mut *conn)
        .await?;
        Some(id)
    } else {
        parent_collection_id
    };

    for (idx, item) in items.iter().enumerate() {
        let name = item["name"].as_str().unwrap_or("未命名").to_string();

        if item.get("item").is_some() {
            // 文件夹
            let (coll_id,): (i64,) = sqlx::query_as(
                "INSERT INTO collections (project_id, parent_id, name, sort_order) \
                 VALUES (?,?,?,?) RETURNING id",
            )
            .bind(project_id)
            .bind(parent_collection_id)
            .bind(&name)
            .bind(sort_offset + idx as i64)
            .fetch_one(&mut *conn)
            .await?;

            let empty: Vec<serde_json::Value> = vec![];
            let sub = item["item"].as_array().unwrap_or(&empty).clone();
            import_postman_items(conn, project_id, Some(coll_id), &sub, 0).await?;
        } else {
            // 单个请求
            let req = &item["request"];
            let method = req["method"].as_str().unwrap_or("GET").to_string();
            let url = if let Some(s) = req["url"].as_str() {
                s.to_string()
            } else {
                req["url"]["raw"].as_str().unwrap_or("").to_string()
            };

            let headers: Vec<serde_json::Value> = req["header"]
                .as_array()
                .unwrap_or(&vec![])
                .iter()
                .map(|h| {
                    serde_json::json!({
                        "key": h["key"].as_str().unwrap_or(""),
                        "value": h["value"].as_str().unwrap_or(""),
                        "enabled": true
                    })
                })
                .collect();

            let (body_type, body) = match req["body"]["mode"].as_str().unwrap_or("none") {
                "raw" => {
                    let lang = req["body"]["options"]["raw"]["language"]
                        .as_str()
                        .unwrap_or("text");
                    let bt = if lang == "json" { "raw_json" } else { "raw_text" };
                    (
                        bt.to_string(),
                        req["body"]["raw"].as_str().unwrap_or("").to_string(),
                    )
                }
                "urlencoded" => {
                    // 从 Postman urlencoded 数组中提取 key=value&... 格式（简单 percent-encode）
                    let encoded_body = req["body"]["urlencoded"]
                        .as_array()
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|item| {
                                    let key = item["key"].as_str().unwrap_or("");
                                    let value = item["value"].as_str().unwrap_or("");
                                    let disabled = item["disabled"].as_bool().unwrap_or(false);
                                    if !key.is_empty() && !disabled {
                                        Some(format!(
                                            "{}={}",
                                            simple_percent_encode(key),
                                            simple_percent_encode(value)
                                        ))
                                    } else {
                                        None
                                    }
                                })
                                .collect::<Vec<_>>()
                                .join("&")
                        })
                        .unwrap_or_default();
                    ("form_urlencoded".to_string(), encoded_body)
                }
                _ => ("none".to_string(), "".to_string()),
            };

            if let Some(coll_id) = bare_coll_id.or(parent_collection_id) {
                sqlx::query(
                    "INSERT INTO api_requests \
                     (collection_id, name, method, url, headers, body_type, body, sort_order) \
                     VALUES (?,?,?,?,?,?,?,?)",
                )
                .bind(coll_id)
                .bind(&name)
                .bind(&method)
                .bind(&url)
                .bind(serde_json::to_string(&headers).unwrap_or_default())
                .bind(&body_type)
                .bind(&body)
                .bind(idx as i64)
                .execute(&mut *conn)
                .await?;
            }
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn import_openapi(
    db: State<'_, AppDb>,
    project_id: i64,
    content: String,
    is_yaml: bool,
) -> CmdResult<i64> {
    let v: serde_json::Value = if is_yaml {
        let y: serde_yaml::Value = serde_yaml::from_str(&content)
            .map_err(|e| crate::error::AppError::Custom(format!("YAML 解析失败: {e}")))?;
        serde_json::to_value(y)
            .map_err(|e| crate::error::AppError::Custom(format!("转换失败: {e}")))?
    } else {
        serde_json::from_str(&content)
            .map_err(|e| crate::error::AppError::Custom(format!("JSON 解析失败: {e}")))?
    };

    // 使用 sqlx 事务 API，失败时由事务析构自动回滚
    let mut tx = db.0.begin().await?;
    let pid = import_openapi_inner(&mut *tx, project_id, &v).await?;
    tx.commit().await?;
    Ok(pid)
}

async fn import_openapi_inner(
    conn: &mut SqliteConnection,
    project_id: i64,
    v: &serde_json::Value,
) -> CmdResult<i64> {
    let title = v["info"]["title"]
        .as_str()
        .unwrap_or("OpenAPI 导入")
        .to_string();
    let pid = ensure_project(conn, project_id, &title, None).await?;
    let base_url = v["servers"][0]["url"].as_str().map(|s| s.to_string());

    // 从全局 tags 列表预建 Collection
    let empty_tags: Vec<serde_json::Value> = vec![];
    let tags: Vec<String> = v["tags"]
        .as_array()
        .unwrap_or(&empty_tags)
        .iter()
        .map(|t| t["name"].as_str().unwrap_or("默认").to_string())
        .collect();

    let mut tag_map: std::collections::HashMap<String, i64> = Default::default();
    for (idx, tag) in tags.iter().enumerate() {
        let (coll_id,): (i64,) = sqlx::query_as(
            "INSERT INTO collections (project_id, parent_id, name, sort_order) \
             VALUES (?,NULL,?,?) RETURNING id",
        )
        .bind(pid)
        .bind(tag)
        .bind(idx as i64)
        .fetch_one(&mut *conn)
        .await?;
        tag_map.insert(tag.clone(), coll_id);
    }

    // lazy 创建"其他"集合：仅在有无 tag 的请求时才 INSERT，避免产生空集合
    let mut default_coll_id: Option<i64> = None;

    let empty_obj = serde_json::Map::new();
    let paths = v["paths"].as_object().unwrap_or(&empty_obj);
    let mut sort_order: i64 = 0;

    for (path, path_item) in paths {
        for method_str in &["get", "post", "put", "delete", "patch", "head", "options"] {
            let op = &path_item[method_str];
            if op.is_null() {
                continue;
            }

            let op_name = op["summary"]
                .as_str()
                .or_else(|| op["operationId"].as_str())
                .unwrap_or(path.as_str());
            let req_name = format!("{} {}", method_str.to_uppercase(), op_name);

            let first_tag = op["tags"][0].as_str().unwrap_or("");
            let coll_id = if let Some(&id) = tag_map.get(first_tag) {
                id
            } else if !first_tag.is_empty() {
                // operation 上存在但全局 tags 未声明的 tag：按需创建对应 Collection
                let sort = tag_map.len() as i64;
                let (id,): (i64,) = sqlx::query_as(
                    "INSERT INTO collections (project_id, parent_id, name, sort_order) \
                     VALUES (?,NULL,?,?) RETURNING id",
                )
                .bind(pid)
                .bind(first_tag)
                .bind(sort)
                .fetch_one(&mut *conn)
                .await?;
                tag_map.insert(first_tag.to_string(), id);
                id
            } else {
                // 首次遇到无 tag 请求时才创建"其他"集合
                if default_coll_id.is_none() {
                    let (id,): (i64,) = sqlx::query_as(
                        "INSERT INTO collections (project_id, parent_id, name, sort_order) \
                         VALUES (?,NULL,'其他',999) RETURNING id",
                    )
                    .bind(pid)
                    .fetch_one(&mut *conn)
                    .await?;
                    default_coll_id = Some(id);
                }
                default_coll_id.unwrap()
            };

            let empty_params: Vec<serde_json::Value> = vec![];
            let parameters = op["parameters"].as_array().unwrap_or(&empty_params);
            let mut headers_json: Vec<serde_json::Value> = vec![];
            let mut params_json: Vec<serde_json::Value> = vec![];
            for param in parameters {
                let pn = param["name"].as_str().unwrap_or("").to_string();
                // 参数默认值转换为字符串，支持 number/bool/string 类型
                let pv = match &param["schema"]["default"] {
                    serde_json::Value::Null => "".to_string(),
                    other => other.as_str().map(|s| s.to_string())
                        .unwrap_or_else(|| other.to_string()),
                };
                match param["in"].as_str().unwrap_or("") {
                    "header" => headers_json
                        .push(serde_json::json!({"key":pn,"value":pv,"enabled":true})),
                    "query" => params_json
                        .push(serde_json::json!({"key":pn,"value":pv,"enabled":true})),
                    // "path" 参数：OpenAPI 里 path 参数以 {xxx} 形式出现在路径字符串本身中
                    // （如 /users/{id}），下面 full_url 构建时已原样保留，
                    // MainPanel.parseUrl 会自动识别 {xxx} 花括号占位符并展示在 Path Params 面板。
                    // 此处 schema.default / example 值暂不导入 —— pathParamValues 不落 DB，
                    // 与 Postman 导入行为保持一致。
                    "path" => {}
                    // "cookie" 参数：本应用暂不支持在接口级别管理 Cookie（走全局 CookieManager）
                    "cookie" => {}
                    _ => {}
                }
            }

            let (body_type, body) = {
                let cn = &op["requestBody"]["content"];
                if !cn["application/json"].is_null() {
                    (
                        "raw_json".to_string(),
                        serde_json::to_string_pretty(&cn["application/json"]["schema"])
                            .unwrap_or_default(),
                    )
                } else {
                    ("none".to_string(), "".to_string())
                }
            };

            let full_url = match &base_url {
                Some(base) => format!("{}{}", base.trim_end_matches('/'), path),
                None => path.clone(),
            };

            sqlx::query(
                "INSERT INTO api_requests \
                 (collection_id, name, method, url, headers, params, body_type, body, sort_order) \
                 VALUES (?,?,?,?,?,?,?,?,?)",
            )
            .bind(coll_id)
            .bind(&req_name)
            .bind(method_str.to_uppercase())
            .bind(&full_url)
            .bind(serde_json::to_string(&headers_json).unwrap_or_default())
            .bind(serde_json::to_string(&params_json).unwrap_or_default())
            .bind(&body_type)
            .bind(&body)
            .bind(sort_order)
            .execute(&mut *conn)
            .await?;

            sort_order += 1;
        }
    }
    Ok(pid)
}

// ── 辅助 ────────────────────────────────────────────────────

async fn ensure_project(
    conn: &mut SqliteConnection,
    project_id: i64,
    name: &str,
    description: Option<&str>,
) -> CmdResult<i64> {
    if project_id > 0 {
        return Ok(project_id);
    }
    let (id,): (i64,) = sqlx::query_as(
        "INSERT INTO projects (name, description) VALUES (?,?) RETURNING id",
    )
    .bind(name)
    .bind(description)
    .fetch_one(&mut *conn)
    .await?;
    Ok(id)
}

/// 简单的 application/x-www-form-urlencoded percent-encoding
/// 将字符串中非 unreserved 字符转为 %XX 格式
fn simple_percent_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 3);
    for byte in s.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9'
            | b'-' | b'_' | b'.' | b'~' => {
                out.push(*byte as char);
            }
            b' ' => out.push('+'),
            b => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}
