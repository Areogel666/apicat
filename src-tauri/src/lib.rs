mod db;
mod error;
mod types;
mod commands;

use commands::{
    collection::{create_collection, delete_collection, list_collections, rename_collection},
    project::{create_project, delete_project, list_projects, update_project},
    request::{create_request, delete_request, list_requests, update_request},
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
        .setup(|app| {
            // Tauri 2.x setup 是同步回调，block_on 在当前线程完成 DB 初始化
            let pool = tauri::async_runtime::block_on(init_db(app))?;

            // 将连接池注册到全局状态，后续 Command 通过 State<AppDb> 取用
            app.manage(AppDb(pool.clone()));

            // 启动时静默清理 30 天前的未收藏测试用例（fire-and-forget）
            tauri::async_runtime::spawn(async move {
                cleanup_old_test_cases(&pool).await;
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            list_projects, create_project, update_project, delete_project,
            list_collections, create_collection, rename_collection, delete_collection,
            list_requests, create_request, update_request, delete_request,
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
