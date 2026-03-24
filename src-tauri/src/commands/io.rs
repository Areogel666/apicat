//! 导入导出命令 — 全部使用 sqlx::query_as / sqlx::query（非宏，无需 DATABASE_URL）

use crate::{db::AppDb, error::CmdResult, types::*};
use tauri::State;

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

                    serde_json::json!({
                        "name": r.name,
                        "request": {
                            "method": r.method,
                            "header": headers,
                            "url": {"raw": r.url},
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

    // 开启事务：保证导入原子性，中途失败自动回滚，不留半成品
    sqlx::query("BEGIN").execute(&db.0).await?;
    let result = import_apicat_inner(&db, project_id, &export).await;
    match result {
        Ok(pid) => {
            sqlx::query("COMMIT").execute(&db.0).await?;
            Ok(pid)
        }
        Err(e) => {
            let _ = sqlx::query("ROLLBACK").execute(&db.0).await;
            Err(e)
        }
    }
}

async fn import_apicat_inner(
    db: &AppDb,
    project_id: i64,
    export: &ApiCatExport,
) -> CmdResult<i64> {
    let pid = ensure_project(
        db,
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
        .fetch_one(&db.0)
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
            .execute(&db.0)
            .await?;
        }
    }

    import_collections_tree(db, pid, None, &export.project.collections).await?;
    Ok(pid)
}

#[async_recursion::async_recursion]
async fn import_collections_tree(
    db: &AppDb,
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
        .fetch_one(&db.0)
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
            .execute(&db.0)
            .await?;
        }

        import_collections_tree(db, project_id, Some(coll_id), &coll.children).await?;
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

    // 开启事务：保证导入原子性，中途失败自动回滚
    sqlx::query("BEGIN").execute(&db.0).await?;
    let result = import_postman_inner(&db, project_id, &coll_name, &v).await;
    match result {
        Ok(pid) => { sqlx::query("COMMIT").execute(&db.0).await?; Ok(pid) }
        Err(e)  => { let _ = sqlx::query("ROLLBACK").execute(&db.0).await; Err(e) }
    }
}

async fn import_postman_inner(
    db: &AppDb,
    project_id: i64,
    coll_name: &str,
    v: &serde_json::Value,
) -> CmdResult<i64> {
    let pid = ensure_project(db, project_id, coll_name, None).await?;
    let empty: Vec<serde_json::Value> = vec![];
    let items = v["item"].as_array().unwrap_or(&empty).clone();
    import_postman_items(db, pid, None, &items, 0).await?;
    Ok(pid)
}

#[async_recursion::async_recursion]
async fn import_postman_items(
    db: &AppDb,
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
        .fetch_one(&db.0)
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
            .fetch_one(&db.0)
            .await?;

            let empty: Vec<serde_json::Value> = vec![];
            let sub = item["item"].as_array().unwrap_or(&empty).clone();
            import_postman_items(db, project_id, Some(coll_id), &sub, 0).await?;
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
                "urlencoded" => ("form_urlencoded".to_string(), "".to_string()),
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
                .execute(&db.0)
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

    // 开启事务：保证导入原子性，中途失败自动回滚
    sqlx::query("BEGIN").execute(&db.0).await?;
    let result = import_openapi_inner(&db, project_id, &v).await;
    match result {
        Ok(pid) => { sqlx::query("COMMIT").execute(&db.0).await?; Ok(pid) }
        Err(e)  => { let _ = sqlx::query("ROLLBACK").execute(&db.0).await; Err(e) }
    }
}

async fn import_openapi_inner(
    db: &AppDb,
    project_id: i64,
    v: &serde_json::Value,
) -> CmdResult<i64> {
    let title = v["info"]["title"]
        .as_str()
        .unwrap_or("OpenAPI 导入")
        .to_string();
    let pid = ensure_project(db, project_id, &title, None).await?;
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
        .fetch_one(&db.0)
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
                .fetch_one(&db.0)
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
                    .fetch_one(&db.0)
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
            .execute(&db.0)
            .await?;

            sort_order += 1;
        }
    }
    Ok(pid)
}

// ── 辅助 ────────────────────────────────────────────────────

async fn ensure_project(
    db: &AppDb,
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
    .fetch_one(&db.0)
    .await?;
    Ok(id)
}
