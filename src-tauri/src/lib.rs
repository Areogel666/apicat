mod db;
mod error;
mod types;
mod sql_cols;
mod commands;
mod http;
mod assertion;
mod bridge;

use commands::{
    collection::{create_collection, delete_collection, list_collections, rename_collection, update_collection_sort, move_collection},
    cookie::{
        create_cookie, delete_cookie, get_cookies_for_domain, list_cookies, update_cookie,
    },
    data_dictionary::{
        create_dictionary, create_dictionary_item, create_dictionary_with_items,
        delete_dictionary, delete_dictionary_item, list_dictionaries, list_dictionary_items,
        replace_dictionary_items, update_dictionary, update_dictionary_item,
        list_field_rules, set_field_rule, delete_field_rule,
        list_field_overrides, set_field_override, delete_field_override,
        copy_dictionary_to_project,
    },
    environment::{
        activate_environment, create_env_variable, create_environment, deactivate_environment,
        delete_env_variable, delete_environment, list_env_variables, list_environments,
        update_env_variable, update_environment,
    },
    project::{create_project, delete_project, list_projects, update_project},
    request::{create_request, delete_request, duplicate_request, list_requests, update_request, update_request_sort, move_request},
    send_request::{get_history_record, list_history, send_request, open_response_file, cleanup_history},
    test_case::{
        create_test_case, delete_test_case, list_test_cases, update_test_case,
        list_test_case_history, delete_test_cases,
    },
    test_case_run::run_test_case,
    stress::{start_stress, list_stress_runs, delete_stress_run, get_stress_report},
    io::{export_apicat, export_postman, import_apicat, import_postman, import_openapi},
    docs::{scan_docs_dir, reveal_in_explorer, open_file_with_default, get_home_dir, read_doc_file},
    skill_installer::{get_skill_targets, install_skills, uninstall_skills},
};
use db::{init_db, AppDb};
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .setup(|app| {
            // Tauri 2.x setup 是同步回调，block_on 在当前线程完成 DB 初始化
            let pool = tauri::async_runtime::block_on(init_db(app))?;

            // 将连接池注册到全局状态，后续 Command 通过 State<AppDb> 取用
            app.manage(AppDb(pool.clone()));

            // 全局 HTTP 客户端：复用连接池和 TLS session cache，避免每次请求重建开销
            let http_client = reqwest::Client::builder()
                .danger_accept_invalid_certs(true)
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .expect("failed to build HTTP client");
            app.manage(http::HttpClient(http_client));

            // 启动时静默清理 30 天前的未收藏测试用例（fire-and-forget）
            let cleanup_pool = pool.clone();
            tauri::async_runtime::spawn(async move {
                cleanup_old_test_cases(&cleanup_pool).await;
            });

            // 1.0.5：启动 Localhost HTTP Bridge（默认开启，token 写 bridge.json）
            if bridge::read_bridge_enabled(app) {
                let bridge_app = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    if let Err(e) = start_bridge(bridge_app, pool).await {
                        eprintln!("[ApiCat] Bridge 启动失败: {e}");
                    }
                });
            }

            // 1.0.5：静默修复悬空的技能链接（App 更新后资源目录被替换的场景）
            commands::skill_installer::repair_skill_links(app.handle());

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_projects, create_project, update_project, delete_project,
            list_collections, create_collection, rename_collection, delete_collection, update_collection_sort, move_collection,
            list_requests, create_request, update_request, delete_request, duplicate_request, update_request_sort, move_request,
            send_request, list_history, get_history_record, open_response_file, cleanup_history,
            list_environments, create_environment, update_environment, delete_environment,
            activate_environment, deactivate_environment,
            list_env_variables, create_env_variable, update_env_variable, delete_env_variable,
            list_cookies, create_cookie, update_cookie, delete_cookie, get_cookies_for_domain,
            list_test_cases, create_test_case, update_test_case, delete_test_case,
            list_test_case_history, delete_test_cases,
            run_test_case,
            list_dictionaries, create_dictionary, update_dictionary, delete_dictionary,
            list_dictionary_items, create_dictionary_item, update_dictionary_item, delete_dictionary_item,
            create_dictionary_with_items, replace_dictionary_items,
            list_field_rules, set_field_rule, delete_field_rule,
            list_field_overrides, set_field_override, delete_field_override,
            copy_dictionary_to_project,
            start_stress,
            list_stress_runs, delete_stress_run, get_stress_report,
            export_apicat, export_postman, import_apicat, import_postman, import_openapi,
            scan_docs_dir, reveal_in_explorer, open_file_with_default, get_home_dir, read_doc_file,
            get_skill_targets, install_skills, uninstall_skills,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// 启动 Localhost HTTP Bridge
/// 固定端口 17320 起，占用则顺延；token 写 bridge.json 供技能发现
async fn start_bridge(
    app: tauri::AppHandle,
    pool: sqlx::SqlitePool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use bridge::token::{generate_token, read_bridge_info, write_bridge_info};

    let app_data_dir = app.path().app_data_dir()?;
    let bridge_path = app_data_dir.join("bridge.json");

    // 已有 bridge.json 则复用 token（重启不换 token，技能无需重新读文件）
    let token = read_bridge_info(&bridge_path)
        .map(|info| info.token)
        .unwrap_or_else(generate_token);

    // 固定端口 17320 起，占用则顺延最多 20 个
    const BASE_PORT: u16 = 17320;
    let mut port = BASE_PORT;
    let listener = loop {
        match tokio::net::TcpListener::bind(("127.0.0.1", port)).await {
            Ok(l) => break l,
            Err(_) if port < BASE_PORT + 20 => port += 1,
            Err(e) => return Err(format!("无法绑定 127.0.0.1:{BASE_PORT}~{port}: {e}").into()),
        }
    };

    let info = bridge::BridgeInfo { port, token: token.clone(), enabled: true };
    write_bridge_info(&bridge_path, &info)?;
    println!("[ApiCat] Bridge listening on http://127.0.0.1:{port}");

    let state = std::sync::Arc::new(bridge::server::BridgeState {
        pool,
        http: reqwest::Client::new(),
        token,
        app,
    });

    let router = bridge::server::build_router(state);
    axum::serve(listener, router).await?;
    Ok(())
}

/// 定时清理：删除 30 天前未收藏的测试用例
async fn cleanup_old_test_cases(pool: &sqlx::SqlitePool) {
    // Step 1 + 2 合并：只删「有收藏用例的接口」中 30 天前的未收藏用例
    // 「没有任何收藏用例」的接口不受影响，其最新用例天然保留
    let result = sqlx::query(
        r#"
        DELETE FROM test_cases
        WHERE starred = 0
          AND created_at < datetime('now', '-30 days')
          AND request_id IN (
            SELECT DISTINCT request_id FROM test_cases WHERE starred = 1
          )
        "#,
    )
    .execute(pool)
    .await;

    match result {
        Ok(r) => println!("[ApiCat] Cleaned up {} old test cases", r.rows_affected()),
        Err(e) => eprintln!("[ApiCat] Cleanup failed: {e}"),
    }
}
