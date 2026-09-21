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

/// 判断一个 SQL 片段是否为纯注释（所有非空行都以 -- 或 /* 开头）
/// 用于过滤掉被 split(';') 误拆出的注释片段，防止注释中的 ';' 字面量被当成分隔符
fn is_pure_comment(stmt: &str) -> bool {
    stmt.lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .all(|line| line.starts_with("--") || line.starts_with("/*") || line.starts_with('*'))
}

/// 增强版 SQL 切分：简单 split(';')，但过滤掉纯注释片段
fn split_sql_statements(sql: &str) -> Vec<&str> {
    sql.split(';')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .filter(|s| !is_pure_comment(s))
        .collect()
}

/// 将 migration SQL 编译时内嵌进二进制，运行时逐条执行
/// 使用增强版 split_sql_statements 过滤多行注释中的 ';' 字面量
async fn run_migrations(pool: &SqlitePool) -> Result<(), Box<dyn std::error::Error>> {
    let migration_sql = include_str!("../../migrations/0001_init.sql");

    for stmt in split_sql_statements(migration_sql) {
        sqlx::query(stmt).execute(pool).await?;
    }

    // 1.0.4：数据字典 / 压测历史等新表（0002，纯幂等 CREATE）
    let migration_sql_2 = include_str!("../../migrations/0002_data_dictionary.sql");
    for stmt in split_sql_statements(migration_sql_2) {
        sqlx::query(stmt).execute(pool).await?;
    }

    // 1.0.4：字典「字段名绑定」规则与例外（0003，纯幂等 CREATE）
    let migration_sql_3 = include_str!("../../migrations/0003_field_rules.sql");
    for stmt in split_sql_statements(migration_sql_3) {
        sqlx::query(stmt).execute(pool).await?;
    }

    // 1.0.4：api_requests.description 幂等加列
    // ALTER TABLE ADD COLUMN 非幂等，不能写进 0002 SQL 文件；
    // 用 PRAGMA table_info 检测列是否存在，不存在才 ALTER。
    // PRAGMA 第 2 列（name）才是列名，query_scalar 默认取第 1 列（cid, INTEGER）——
    // 显式 `SELECT name FROM pragma_table_info(...)` 避免 String/INTEGER 类型错配。
    let cols: Vec<String> =
        sqlx::query_scalar("SELECT name FROM pragma_table_info('api_requests')")
            .fetch_all(pool)
            .await?;
    if !cols.iter().any(|c| c == "description") {
        sqlx::query("ALTER TABLE api_requests ADD COLUMN description TEXT DEFAULT ''")
            .execute(pool)
            .await?;
    }

    // 1.0.4：request_history.test_case_id 幂等加列（History 按用例分桶）
    let hist_cols: Vec<String> =
        sqlx::query_scalar("SELECT name FROM pragma_table_info('request_history')")
            .fetch_all(pool)
            .await?;
    if !hist_cols.iter().any(|c| c == "test_case_id") {
        sqlx::query(
            "ALTER TABLE request_history ADD COLUMN test_case_id INTEGER
             REFERENCES test_cases(id) ON DELETE SET NULL",
        )
        .execute(pool)
        .await?;
    }

    // 1.0.5：test_cases.case_type 幂等加列（AI 技能用例类型分类）
    let tc_cols: Vec<String> =
        sqlx::query_scalar("SELECT name FROM pragma_table_info('test_cases')")
            .fetch_all(pool)
            .await?;
    if !tc_cols.iter().any(|c| c == "case_type") {
        sqlx::query(
            "ALTER TABLE test_cases ADD COLUMN case_type TEXT NOT NULL DEFAULT 'happy_path'",
        )
        .execute(pool)
        .await?;
    }

    // 1.0.5：projects.docs_output_dir 幂等加列（doc-gen 技能的文档输出目录）
    let proj_cols: Vec<String> =
        sqlx::query_scalar("SELECT name FROM pragma_table_info('projects')")
            .fetch_all(pool)
            .await?;
    if !proj_cols.iter().any(|c| c == "docs_output_dir") {
        sqlx::query("ALTER TABLE projects ADD COLUMN docs_output_dir TEXT")
            .execute(pool)
            .await?;
    }

    // 1.0.5：压测参考线阈值 —— 项目级默认 + 接口级覆盖
    let proj_cols2: Vec<String> =
        sqlx::query_scalar("SELECT name FROM pragma_table_info('projects')")
            .fetch_all(pool)
            .await?;
    if !proj_cols2.iter().any(|c| c == "p95_threshold_ms") {
        sqlx::query("ALTER TABLE projects ADD COLUMN p95_threshold_ms INTEGER DEFAULT 500")
            .execute(pool)
            .await?;
    }
    if !proj_cols2.iter().any(|c| c == "p99_threshold_ms") {
        sqlx::query("ALTER TABLE projects ADD COLUMN p99_threshold_ms INTEGER DEFAULT 1000")
            .execute(pool)
            .await?;
    }

    let req_cols2: Vec<String> =
        sqlx::query_scalar("SELECT name FROM pragma_table_info('api_requests')")
            .fetch_all(pool)
            .await?;
    if !req_cols2.iter().any(|c| c == "p95_threshold_ms") {
        sqlx::query("ALTER TABLE api_requests ADD COLUMN p95_threshold_ms INTEGER")
            .execute(pool)
            .await?;
    }
    if !req_cols2.iter().any(|c| c == "p99_threshold_ms") {
        sqlx::query("ALTER TABLE api_requests ADD COLUMN p99_threshold_ms INTEGER")
            .execute(pool)
            .await?;
    }

    // 合并 test_case_history 到 request_history
    // 守卫：request_history 增加 error_message 列
    let has_err_msg = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM pragma_table_info('request_history') WHERE name='error_message'"
    ).fetch_one(pool).await?;
    if has_err_msg == 0 {
        sqlx::query("ALTER TABLE request_history ADD COLUMN error_message TEXT")
            .execute(pool).await?;
    }

    // 数据迁移：test_case_history → request_history（幂等：旧表存在才迁移）
    let has_old_table = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='test_case_history'"
    ).fetch_one(pool).await?;

    if has_old_table > 0 {
        sqlx::query(
            "INSERT INTO request_history \
             (request_id, test_case_id, status_code, response_time_ms, \
              request_snapshot, response_body, error_message, created_at) \
             SELECT tc.request_id, tch.test_case_id, tch.status_code, \
                    tch.duration_ms, '{}', tch.response_preview, tch.error_message, tch.created_at \
             FROM test_case_history tch \
             JOIN test_cases tc ON tch.test_case_id = tc.id \
             WHERE tc.request_id IS NOT NULL"
        ).execute(pool).await?;

        sqlx::query("DROP TABLE IF EXISTS test_case_history").execute(pool).await?;
        sqlx::query("DROP TRIGGER IF EXISTS trg_tch_keep_10").execute(pool).await?;
    }

    // 新触发器：用例执行历史保留 10 条（test_case_id IS NOT NULL 时生效）
    sqlx::query(
        r#"
        CREATE TRIGGER IF NOT EXISTS trg_rh_keep_10_per_case
        AFTER INSERT ON request_history
        WHEN NEW.test_case_id IS NOT NULL
        BEGIN
          DELETE FROM request_history
          WHERE test_case_id = NEW.test_case_id
            AND id NOT IN (
              SELECT id FROM request_history
              WHERE test_case_id = NEW.test_case_id
              ORDER BY created_at DESC, id DESC
              LIMIT 10
            );
        END
        "#,
    )
    .execute(pool)
    .await?;

    // 1.0.5：废弃用例收藏。守卫式 DROP（列存在才删，幂等）。
    // SQLite 3.35+ 支持 DROP COLUMN；若列被索引引用会失败，本列无索引依赖。
    let has_starred = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM pragma_table_info('test_cases') WHERE name='starred'"
    ).fetch_one(pool).await?;
    if has_starred > 0 {
        sqlx::query("ALTER TABLE test_cases DROP COLUMN starred")
            .execute(pool)
            .await?;
    }

    Ok(())
}
