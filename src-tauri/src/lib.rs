mod db;
mod error;
mod types;
mod commands;
mod http;

use commands::{
    collection::{create_collection, delete_collection, list_collections, rename_collection, update_collection_sort},
    cookie::{
        create_cookie, delete_cookie, get_cookies_for_domain, list_cookies, update_cookie,
    },
    environment::{
        activate_environment, create_env_variable, create_environment, deactivate_environment,
        delete_env_variable, delete_environment, list_env_variables, list_environments,
        update_env_variable, update_environment,
    },
    project::{create_project, delete_project, list_projects, update_project},
    request::{create_request, delete_request, duplicate_request, list_requests, update_request, update_request_sort},
    send_request::{list_history, send_request},
    test_case::{create_test_case, delete_test_case, list_test_cases, update_test_case},
    stress::start_stress,
    io::{export_apicat, export_postman, import_apicat, import_postman, import_openapi},
};
use db::{init_db, AppDb};
use tauri::Manager;

/// 示例 Tauri Command（M2 起逐步替换为真实业务 command）
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello from ApiCat, {}!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
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
            tauri::async_runtime::spawn(async move {
                cleanup_old_test_cases(&pool).await;
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            list_projects, create_project, update_project, delete_project,
            list_collections, create_collection, rename_collection, delete_collection, update_collection_sort,
            list_requests, create_request, update_request, delete_request, duplicate_request, update_request_sort,
            send_request, list_history,
            list_environments, create_environment, update_environment, delete_environment,
            activate_environment, deactivate_environment,
            list_env_variables, create_env_variable, update_env_variable, delete_env_variable,
            list_cookies, create_cookie, update_cookie, delete_cookie, get_cookies_for_domain,
            list_test_cases, create_test_case, update_test_case, delete_test_case,
            start_stress,
            export_apicat, export_postman, import_apicat, import_postman, import_openapi,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// 定时清理：删除 30 天前未收藏的测试用例
/// 设计文档 4.2.2 节：两步清理逻辑
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
