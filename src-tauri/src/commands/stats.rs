//! 使用统计聚合（纯本地只读）。全部指标从现有表派生，无埋点、无网络。
//! 口径（共识基线）：请求类只算调试发送（test_case_id IS NULL）；
//! 成功率 = 2xx/3xx 成功，NULL 或 >=400 失败；压测次数 = stress_runs 全量 COUNT。
//! TS 镜像接口见 src/types/index.ts 的 UsageStats（字段顺序一致）。

use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};
use tauri::State;

use crate::{db::AppDb, error::CmdResult};

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct KpiStats {
    pub total_interfaces: i64,
    /// 窗口内请求数（窗口由 range 决定，'all' 为累计）
    pub requests_count: i64,
    /// 0-100 的一位小数；窗口内无请求时为 -1（前端显示「—」）
    pub success_rate: f64,
    /// 窗口内压测次数
    pub stress_run_count: i64,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DailyCount {
    /// YYYY-MM-DD（UTC，与 created_at 存储口径一致）
    pub date: String,
    pub count: i64,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NameCount {
    pub name: String,
    pub count: i64,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DictStats {
    pub dict_count: i64,
    pub item_count: i64,
    pub rule_count: i64,
    pub override_count: i64,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DocsStats {
    pub file_count: i64,
    pub total_size: i64,
    pub last_modified: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Trend {
    /// 趋势粒度：窗口 <=90 天按日，更久/全部按周（点日期为周一）
    pub granularity: String,
    pub points: Vec<DailyCount>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UsageStats {
    pub kpi: KpiStats,
    pub trend: Trend,
    pub status_dist: Vec<NameCount>,
    pub method_dist: Vec<NameCount>,
    pub dictionary: DictStats,
    pub docs: DocsStats,
}

/// 恒定过滤条件：请求统计只算调试发送，用例执行（触发器每用例仅留 10 条）不污染趋势
const REQ_FILTER: &str = "test_case_id IS NULL";

/// 范围筛选（1.0.9 级联）：接口级直接 request_id（已有索引）；项目级经
/// api_requests→collections 两跳定位。request_history 与 stress_runs 的
/// request_id 列同名同义，同一片段两处复用。
/// 参数为 Tauri IPC 类型化的 i64（非数字在反序列化即被拒），拼接无注入面。
fn scope_filter_sql(project_id: Option<i64>, request_id: Option<i64>) -> String {
    match (project_id, request_id) {
        (_, Some(rid)) => format!("request_id = {rid}"),
        (Some(pid), None) => format!(
            "request_id IN (SELECT r.id FROM api_requests r \
             JOIN collections c ON r.collection_id = c.id WHERE c.project_id = {pid})"
        ),
        (None, None) => "1=1".to_string(),
    }
}

async fn collect_usage_stats(
    pool: &SqlitePool,
    range: &str,
    project_id: Option<i64>,
    request_id: Option<i64>,
) -> Result<UsageStats, sqlx::Error> {
    // 档位 -> 窗口天数（None = 累计全部）
    let window_days: Option<i64> = match range {
        "all" => None,
        "180d" => Some(180),
        "365d" => Some(365),
        _ => Some(30),
    };
    let window = window_days
        .map(|d| format!("created_at >= datetime('now','-{d} days')"))
        .unwrap_or_else(|| "1=1".to_string());
    let scope_filter = scope_filter_sql(project_id, request_id);
    let dist_filter = format!("{REQ_FILTER} AND {scope_filter} AND {window}");
    // 趋势粒度：窗口 <=90 天按日，更久/全部按周（每条记录归到所属周一）
    let granularity = if window_days.map_or(false, |d| d <= 90) { "day" } else { "week" };

    // ---- KPI ----
    // 接口总数跟随范围：接口级 = 1（选中的那个），项目级 = 该项目存量，全部 = 全量
    let total_interfaces: i64 = match (project_id, request_id) {
        (_, Some(rid)) => sqlx::query_scalar(&format!(
            "SELECT COUNT(*) FROM api_requests WHERE id = {rid}"
        ))
        .fetch_one(pool)
        .await?,
        (Some(pid), None) => sqlx::query_scalar(&format!(
            "SELECT COUNT(*) FROM api_requests \
             WHERE collection_id IN (SELECT id FROM collections WHERE project_id = {pid})"
        ))
        .fetch_one(pool)
        .await?,
        (None, None) => sqlx::query_scalar("SELECT COUNT(*) FROM api_requests")
            .fetch_one(pool)
            .await?,
    };
    let requests_count: i64 = sqlx::query_scalar(&format!(
        "SELECT COUNT(*) FROM request_history WHERE {dist_filter}"
    ))
    .fetch_one(pool)
    .await?;
    let (ok_n, all_n): (i64, i64) = sqlx::query_as(&format!(
        "SELECT \
           COALESCE(SUM(CASE WHEN status_code BETWEEN 200 AND 399 THEN 1 ELSE 0 END), 0), \
           COUNT(*) \
         FROM request_history WHERE {dist_filter}"
    ))
    .fetch_one(pool)
    .await?;
    let success_rate = if all_n == 0 { -1.0 } else {
        ((ok_n as f64) / (all_n as f64) * 1000.0).round() / 10.0
    };
    let stress_run_count: i64 = sqlx::query_scalar(&format!(
        "SELECT COUNT(*) FROM stress_runs WHERE {window} AND {scope_filter}"
    ))
    .fetch_one(pool)
    .await?;
    // ---- 趋势（粒度自适应，空档不返回，由前端补零） ----
    // 周分组：先回退 6 天再取下一个周一 = 归到所属周一。
    // 注意顺序不可反：'weekday 1' 遇当天已是周一会原地不动，
    // 若先 'weekday 1' 后 '-6 days'，周一记录会错归上一周（实测踩坑）。
    let bucket = if granularity == "day" {
        "date(created_at)"
    } else {
        "date(created_at, '-6 days', 'weekday 1')"
    };
    let trend_rows = sqlx::query(&format!(
        "SELECT {bucket} AS d, COUNT(*) AS c FROM request_history \
         WHERE {dist_filter} GROUP BY d ORDER BY d"
    ))
    .fetch_all(pool)
    .await?;

    let trend_points = trend_rows
        .iter()
        .map(|r| DailyCount { date: r.get::<String, _>("d"), count: r.get::<i64, _>("c") })
        .collect();

    // ---- 状态分布（NULL → 网络失败；<400 → 2xx/3xx；<500 → 4xx；else → 5xx） ----
    let status_rows = sqlx::query(&format!(
        "SELECT CASE \
             WHEN status_code IS NULL THEN '网络失败' \
             WHEN status_code < 400 THEN '2xx/3xx' \
             WHEN status_code < 500 THEN '4xx' \
             ELSE '5xx' END AS name, \
           COUNT(*) AS c \
         FROM request_history WHERE {dist_filter} \
         GROUP BY name ORDER BY c DESC"
    ))
    .fetch_all(pool)
    .await?;
    let status_dist = status_rows
        .iter()
        .map(|r| NameCount { name: r.get::<String, _>("name"), count: r.get::<i64, _>("c") })
        .collect();

    // ---- 方法分布（快照是 ApiRequest 的 serde 序列化，method 为单词键，大小写免疫） ----
    let method_rows = sqlx::query(&format!(
        "SELECT COALESCE(json_extract(request_snapshot, '$.method'), '未知') AS name, COUNT(*) AS c \
         FROM request_history WHERE {dist_filter} \
         GROUP BY name ORDER BY c DESC LIMIT 8"
    ))
    .fetch_all(pool)
    .await?;
    let method_dist = method_rows
        .iter()
        .map(|r| NameCount { name: r.get::<String, _>("name"), count: r.get::<i64, _>("c") })
        .collect();

    // ---- 字典（存量跟随项目级筛选；接口级时前端隐藏该区块，直接零值跳过全部查询）----
    // 字典树展示「项目自有 + 全局共享」，口径与之一致：project_id = ? OR IS NULL
    let (dict_count, item_count, rule_count, override_count): (i64, i64, i64, i64) = if request_id.is_some() {
        (0, 0, 0, 0)
    } else {
        match project_id {
        Some(pid) => {
            let dict_count: i64 = sqlx::query_scalar(&format!(
                "SELECT COUNT(*) FROM data_dictionaries WHERE project_id = {pid} OR project_id IS NULL"
            ))
            .fetch_one(pool)
            .await?;
            let item_count: i64 = sqlx::query_scalar(&format!(
                "SELECT COUNT(*) FROM dictionary_items WHERE dictionary_id IN \
                 (SELECT id FROM data_dictionaries WHERE project_id = {pid} OR project_id IS NULL)"
            ))
            .fetch_one(pool)
            .await?;
            // rules/overrides 的 project_id NOT NULL（0003 迁移），纯项目行
            let rule_count: i64 = sqlx::query_scalar(&format!(
                "SELECT COUNT(*) FROM field_dictionary_rules WHERE project_id = {pid}"
            ))
            .fetch_one(pool)
            .await?;
            let override_count: i64 = sqlx::query_scalar(&format!(
                "SELECT COUNT(*) FROM field_dictionary_overrides WHERE project_id = {pid}"
            ))
            .fetch_one(pool)
            .await?;
            (dict_count, item_count, rule_count, override_count)
        }
        None => {
            let dict_count: i64 =
                sqlx::query_scalar("SELECT COUNT(*) FROM data_dictionaries").fetch_one(pool).await?;
            let item_count: i64 =
                sqlx::query_scalar("SELECT COUNT(*) FROM dictionary_items").fetch_one(pool).await?;
            let rule_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM field_dictionary_rules")
                .fetch_one(pool)
                .await?;
            let override_count: i64 =
                sqlx::query_scalar("SELECT COUNT(*) FROM field_dictionary_overrides")
                    .fetch_one(pool)
                    .await?;
            (dict_count, item_count, rule_count, override_count)
        }
        }
    };

    // ---- 文档（仅扫描已配置 docs_output_dir 的项目；目录缺失按 0 计不拖垮整体）----
    // 接口级筛选时前端隐藏该区块，直接零值跳过磁盘扫描；项目级筛选时只扫该项目的目录
    let mut docs = DocsStats { file_count: 0, total_size: 0, last_modified: None };
    if request_id.is_none() {
        let docs_project_clause =
            project_id.map(|pid| format!(" AND id = {pid}")).unwrap_or_default();
        let dir_rows = sqlx::query(&format!(
            "SELECT docs_output_dir FROM projects \
             WHERE docs_output_dir IS NOT NULL AND docs_output_dir != ''{docs_project_clause}"
        ))
        .fetch_all(pool)
        .await?;
        for row in dir_rows {
            let dir: String = row.get("docs_output_dir");
            let root = std::path::PathBuf::from(&dir);
            if !root.is_dir() {
                continue; // 目录缺失/无权限按 0 计
            }
            let mut files = Vec::new();
            crate::commands::docs::scan_md_files(&root, &root, &mut files);
            docs.file_count += files.len() as i64;
            docs.total_size += files.iter().map(|f| f.size as i64).sum::<i64>();
            if let Some(latest) = files.iter().map(|f| f.modified_at.clone()).max() {
                if docs.last_modified.as_ref().map_or(true, |cur| latest > *cur) {
                    docs.last_modified = Some(latest);
                }
            }
        }
    }

    Ok(UsageStats {
        kpi: KpiStats { total_interfaces, requests_count, success_rate, stress_run_count },
        trend: Trend { granularity: granularity.to_string(), points: trend_points },
        status_dist,
        method_dist,
        dictionary: DictStats { dict_count, item_count, rule_count, override_count },
        docs,
    })
}

#[tauri::command]
pub async fn get_usage_stats(
    db: State<'_, AppDb>,
    range: String,
    project_id: Option<i64>,
    request_id: Option<i64>,
) -> CmdResult<UsageStats> {
    Ok(collect_usage_stats(&db.0, &range, project_id, request_id).await?)
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn mem_db() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        crate::db::run_migrations(&pool).await.unwrap();
        pool
    }

    /// 建一条完整的 项目→集合→接口 链，返回 request_id
    async fn seed_chain(pool: &SqlitePool) -> i64 {
        let pid: i64 = sqlx::query_scalar(
            "INSERT INTO projects (name) VALUES ('p') RETURNING id",
        ).fetch_one(pool).await.unwrap();
        let cid: i64 = sqlx::query_scalar(
            "INSERT INTO collections (project_id, name) VALUES (?, 'c') RETURNING id",
        ).bind(pid).fetch_one(pool).await.unwrap();
        sqlx::query_scalar(
            "INSERT INTO api_requests (collection_id, name, method, url) \
             VALUES (?, 'r', 'GET', 'http://x') RETURNING id",
        ).bind(cid).fetch_one(pool).await.unwrap()
    }

    async fn seed_history(
        pool: &SqlitePool, request_id: i64, days_ago: i64,
        status_code: Option<i64>, test_case_id: Option<i64>, method: &str,
    ) {
        sqlx::query(
            "INSERT INTO request_history \
             (request_id, test_case_id, status_code, response_time_ms, request_snapshot, created_at) \
             VALUES (?, ?, ?, 100, ?, datetime('now', ? || ' days'))",
        )
        .bind(request_id)
        .bind(test_case_id)
        .bind(status_code)
        .bind(serde_json::json!({ "method": method }).to_string())
        .bind(format!("-{days_ago}"))
        .execute(pool)
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn stats_counts_buckets_and_excludes_test_case_rows() {
        let pool = mem_db().await;
        let rid = seed_chain(&pool).await;

        // 近30天：200/304/500/NULL 各一条 + 40 天前的 200（不计入30d）+ 用例行（排除）
        seed_history(&pool, rid, 0, Some(200), None, "GET").await;
        seed_history(&pool, rid, 1, Some(304), None, "GET").await;
        seed_history(&pool, rid, 2, Some(500), None, "POST").await;
        seed_history(&pool, rid, 0, None, None, "GET").await;
        seed_history(&pool, rid, 40, Some(200), None, "GET").await;
        let tcid: i64 = sqlx::query_scalar(
            "INSERT INTO test_cases (request_id, collection_id, name) \
             SELECT ?, id, 'tc' FROM collections LIMIT 1 RETURNING id",
        ).bind(rid).fetch_one(&pool).await.unwrap();
        seed_history(&pool, rid, 0, Some(200), Some(tcid), "GET").await;

        let s = collect_usage_stats(&pool, "30d", None, None).await.unwrap();

        assert_eq!(s.kpi.total_interfaces, 1);
        assert_eq!(s.kpi.requests_count, 4); // 5 条调试发送 - 1 条 40 天前
        assert_eq!(s.kpi.success_rate, 50.0); // 2 成功 / 4 总
        assert_eq!(s.kpi.stress_run_count, 0);
        assert_eq!(s.trend.granularity, "day");
        assert_eq!(s.trend.points.iter().map(|d| d.count).sum::<i64>(), 4);

        let bucket = |n: &str| s.status_dist.iter().find(|b| b.name == n).map(|b| b.count).unwrap_or(0);
        assert_eq!(bucket("2xx/3xx"), 2);
        assert_eq!(bucket("5xx"), 1);
        assert_eq!(bucket("网络失败"), 1);

        let m = |n: &str| s.method_dist.iter().find(|b| b.name == n).map(|b| b.count).unwrap_or(0);
        assert_eq!(m("GET"), 3);
        assert_eq!(m("POST"), 1);
    }

    #[tokio::test]
    async fn stats_empty_db_returns_zeroed_structure() {
        // Review Focus 1：空库不得报错，成功率哨兵 -1
        let pool = mem_db().await;
        let s = collect_usage_stats(&pool, "30d", None, None).await.unwrap();
        assert_eq!(s.kpi.total_interfaces, 0);
        assert_eq!(s.kpi.requests_count, 0);
        assert_eq!(s.kpi.success_rate, -1.0);
        assert_eq!(s.trend.granularity, "day");
        assert!(s.trend.points.is_empty());
        assert!(s.status_dist.is_empty());
        assert_eq!(s.docs.file_count, 0);
    }

    #[tokio::test]
    async fn stats_week_granularity_and_wider_window() {
        let pool = mem_db().await;
        let rid = seed_chain(&pool).await;

        // 180d 窗口内：1 天前、40 天前、100 天前各一条；400 天前不计入
        seed_history(&pool, rid, 1, Some(200), None, "GET").await;
        seed_history(&pool, rid, 40, Some(500), None, "POST").await;
        seed_history(&pool, rid, 100, Some(200), None, "GET").await;
        seed_history(&pool, rid, 400, Some(200), None, "GET").await;

        // 压测：10 天前与 100 天前各一次
        sqlx::query("INSERT INTO stress_runs (request_id, created_at) VALUES (?, datetime('now','-10 days'))")
            .bind(rid).execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO stress_runs (request_id, created_at) VALUES (?, datetime('now','-100 days'))")
            .bind(rid).execute(&pool).await.unwrap();

        let wide = collect_usage_stats(&pool, "180d", None, None).await.unwrap();
        assert_eq!(wide.kpi.requests_count, 3); // 400 天前那条不计入
        assert_eq!(wide.trend.granularity, "week"); // 180 天 > 90 天阈值
        assert_eq!(wide.trend.points.iter().map(|d| d.count).sum::<i64>(), 3);
        assert_eq!(wide.trend.points.len(), 3); // 三条记录分属三个不同周
        // 周桶正确性：每个桶日期必须是周一（strftime %w：1=Monday）
        for pt in &wide.trend.points {
            let dow: String = sqlx::query_scalar("SELECT strftime('%w', ?)")
                .bind(&pt.date).fetch_one(&pool).await.unwrap();
            assert_eq!(dow, "1", "周桶 {} 不是周一", pt.date);
        }
        assert_eq!(wide.kpi.stress_run_count, 2);

        // 切回 30 天：仅剩 1 条请求 + 1 次压测，粒度回到按日
        let narrow = collect_usage_stats(&pool, "30d", None, None).await.unwrap();
        assert_eq!(narrow.kpi.requests_count, 1);
        assert_eq!(narrow.kpi.stress_run_count, 1);
        assert_eq!(narrow.trend.granularity, "day");

        // 累计档：4 条请求全算
        let all = collect_usage_stats(&pool, "all", None, None).await.unwrap();
        assert_eq!(all.kpi.requests_count, 4);
        assert_eq!(all.trend.granularity, "week");
    }

    #[tokio::test]
    async fn stats_docs_scan_skips_missing_dirs() {
        // Review Focus 5：docs_output_dir 指向不存在的路径 → 计 0 不崩
        let pool = mem_db().await;
        sqlx::query("INSERT INTO projects (name, docs_output_dir) VALUES ('p', '/nonexistent/xyz')")
            .execute(&pool).await.unwrap();
        let s = collect_usage_stats(&pool, "30d", None, None).await.unwrap();
        assert_eq!(s.docs.file_count, 0);
    }

    #[tokio::test]
    async fn stats_docs_scan_counts_real_dir() {
        let pool = mem_db().await;
        let tmp = std::env::temp_dir().join(format!("apicat_stats_test_{}", std::process::id()));
        std::fs::create_dir_all(&tmp).unwrap();
        std::fs::write(tmp.join("a.md"), "# a".repeat(10)).unwrap();
        sqlx::query("INSERT INTO projects (name, docs_output_dir) VALUES ('p', ?)")
            .bind(tmp.to_string_lossy().as_ref())
            .execute(&pool).await.unwrap();
        let s = collect_usage_stats(&pool, "30d", None, None).await.unwrap();
        assert_eq!(s.docs.file_count, 1);
        assert!(s.docs.total_size > 0);
        assert!(s.docs.last_modified.is_some());
        std::fs::remove_dir_all(&tmp).ok();
    }

    #[tokio::test]
    async fn stats_scope_filter_project_and_request() {
        // Review Focus（1.0.9 级联）：项目/接口筛选只作用于所选范围；
        // 压测跟筛选走；接口总数项目级=该项目存量、接口级=1；字典存量跟随项目（含全局共享）
        let pool = mem_db().await;
        // 项目 A：接口 r1（2 条请求 + 1 次压测 + 1 个项目字典）；项目 B：接口 r2（1 条请求）
        let rid_a = seed_chain(&pool).await;
        let pa: i64 = sqlx::query_scalar("SELECT id FROM projects LIMIT 1")
            .fetch_one(&pool).await.unwrap();
        seed_history(&pool, rid_a, 0, Some(200), None, "GET").await;
        seed_history(&pool, rid_a, 1, Some(500), None, "POST").await;
        sqlx::query("INSERT INTO stress_runs (request_id, created_at) VALUES (?, datetime('now','-1 days'))")
            .bind(rid_a).execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO data_dictionaries (code, name, project_id) VALUES ('D1', 'd1', ?)")
            .bind(pa).execute(&pool).await.unwrap();

        let pb: i64 = sqlx::query_scalar("INSERT INTO projects (name) VALUES ('pb') RETURNING id")
            .fetch_one(&pool).await.unwrap();
        let cb: i64 = sqlx::query_scalar(
            "INSERT INTO collections (project_id, name) VALUES (?, 'cb') RETURNING id",
        ).bind(pb).fetch_one(&pool).await.unwrap();
        let rid_b: i64 = sqlx::query_scalar(
            "INSERT INTO api_requests (collection_id, name, method, url) \
             VALUES (?, 'r2', 'GET', 'http://x') RETURNING id",
        ).bind(cb).fetch_one(&pool).await.unwrap();
        seed_history(&pool, rid_b, 0, Some(200), None, "GET").await;

        // 全部：3 请求 / 2 接口 / 1 压测 / 1 字典
        let all = collect_usage_stats(&pool, "all", None, None).await.unwrap();
        assert_eq!(all.kpi.requests_count, 3);
        assert_eq!(all.kpi.total_interfaces, 2);
        assert_eq!(all.kpi.stress_run_count, 1);
        assert_eq!(all.dictionary.dict_count, 1);

        // 项目 A：2 请求 / 1 接口 / 1 压测 / 1 字典
        let proj_a = collect_usage_stats(&pool, "all", Some(pa), None).await.unwrap();
        assert_eq!(proj_a.kpi.requests_count, 2);
        assert_eq!(proj_a.kpi.total_interfaces, 1);
        assert_eq!(proj_a.kpi.stress_run_count, 1);
        assert_eq!(proj_a.dictionary.dict_count, 1);

        // 接口 r1：2 请求 / 接口总数 1 / 1 压测
        let req_a = collect_usage_stats(&pool, "all", None, Some(rid_a)).await.unwrap();
        assert_eq!(req_a.kpi.requests_count, 2);
        assert_eq!(req_a.kpi.total_interfaces, 1);
        assert_eq!(req_a.kpi.stress_run_count, 1);

        // 项目 B：1 请求 / 1 接口 / 0 压测 / 0 字典
        let proj_b = collect_usage_stats(&pool, "all", Some(pb), None).await.unwrap();
        assert_eq!(proj_b.kpi.requests_count, 1);
        assert_eq!(proj_b.kpi.total_interfaces, 1);
        assert_eq!(proj_b.kpi.stress_run_count, 0);
        assert_eq!(proj_b.dictionary.dict_count, 0);
    }
}
