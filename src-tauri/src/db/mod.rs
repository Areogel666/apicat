use std::fs;
use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};
use tauri::Manager;

/// 全局 DB 状态，通过 Tauri manage() 注入，后续 Command 用 State<AppDb> 取连接池
#[allow(dead_code)]
pub struct AppDb(pub SqlitePool);

/// 初始化数据库：
/// 1. 确保数据目录存在
/// 2. 创建 SQLite 连接池（WAL 模式）
/// 3. 运行内嵌 migration SQL（幂等，CREATE TABLE IF NOT EXISTS）
pub async fn init_db(app: &tauri::App) -> Result<SqlitePool, Box<dyn std::error::Error>> {
    // 数据目录：平台标准 AppData 目录下的 com.apicat.app/
    let app_data_dir = app.path().app_data_dir()?;
    fs::create_dir_all(&app_data_dir)?;

    let db_path = app_data_dir.join("apicat.db");
    // sqlite: URI 格式，mode=rwc 表示读写+不存在时自动创建
    let db_url = format!("sqlite:{}?mode=rwc", db_path.display());

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await?;

    // WAL 模式：读写并发，外部工具（AI Skill）写入时不会死锁
    sqlx::query("PRAGMA journal_mode = WAL").execute(&pool).await?;
    // 启用外键约束（SQLite 默认关闭）
    sqlx::query("PRAGMA foreign_keys = ON").execute(&pool).await?;

    run_migrations(&pool).await?;

    Ok(pool)
}

/// 将 migration SQL 编译时内嵌进二进制，运行时逐条执行
/// 按分号分割，跳过空行和纯注释行
async fn run_migrations(pool: &SqlitePool) -> Result<(), Box<dyn std::error::Error>> {
    let migration_sql = include_str!("../../migrations/0001_init.sql");

    for stmt in migration_sql.split(';') {
        let stmt = stmt.trim();
        if stmt.is_empty() {
            continue;
        }
        // 如果整段没有换行符且以注释开头，才是纯注释，否则里面包含了真实的 SQL，需要执行
        if stmt.starts_with("--") && !stmt.contains('\n') {
            continue;
        }
        sqlx::query(stmt).execute(pool).await?;
    }

    // M3-C 触发器：trg_tch_keep_10
    // 复合 BEGIN/END 块内部含 ';'，不能写在 0001_init.sql 里（会被 split(';') 拆坏）。
    // 单独以一条 query 执行；CREATE TRIGGER IF NOT EXISTS 幂等。
    sqlx::query(
        r#"
        CREATE TRIGGER IF NOT EXISTS trg_tch_keep_10
        AFTER INSERT ON test_case_history
        BEGIN
          DELETE FROM test_case_history
          WHERE test_case_id = NEW.test_case_id
            AND id NOT IN (
              SELECT id FROM test_case_history
              WHERE test_case_id = NEW.test_case_id
              ORDER BY created_at DESC, id DESC
              LIMIT 10
            );
        END
        "#,
    )
    .execute(pool)
    .await?;

    Ok(())
}
