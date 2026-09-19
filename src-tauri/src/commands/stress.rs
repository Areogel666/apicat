//! 压测引擎
//!
//! 关键约束（§10.1 CRITICAL）：
//!   - 禁止逐请求 emit；每 200ms 聚合一次统计后推送
//!   - reqwest 连接池大小不超过 max_concurrent（§10.2）
//!   - Windows 跳过 fd 检测；macOS/Linux 检测 getrlimit

use crate::{
    error::CmdResult,
    http::client::{send, SendRequestParams},
    sql_cols::REQUEST_COLS,
    types::{ApiRequest, StressRun},
};
use serde::Serialize;
use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tauri::{AppHandle, Emitter};
use tokio::sync::broadcast;
use crate::db::AppDb;
use sqlx::Sqlite;
use tauri::State;

/// 百分位滑动窗口大小：只保留最近 N 条耗时用于 P50/P95/P99 计算。
/// 内存上限 ≈ 10000 × 8B = 80KB，对长时压测 OOM 友好。
const DURATION_WINDOW: usize = 10_000;

// ── 统计数据（每 200ms 快照推送到前端）────────────────────

// 耗时直方图时间桶上界（毫秒）：[0,1) [1,2) [2,5) [5,10) [10,20) [20,50) [50,100) [100,200) [200,500) [500+)
const LATENCY_BUCKETS: [f64; 9] = [1.0, 2.0, 5.0, 10.0, 20.0, 50.0, 100.0, 200.0, 500.0];
const LATENCY_BUCKET_COUNT: usize = 10;

/// 耗时直方图的区间标签，与 LATENCY_BUCKETS 一一对应（10 桶）
pub const LATENCY_LABELS: [&str; 10] = [
    "<1", "1-2", "2-5", "5-10", "10-20", "20-50", "50-100", "100-200", "200-500", ">500",
];

/// 期望状态码默认值：2xx 视为业务成功
pub const DEFAULT_EXPECT_STATUS: &str = "2xx";

/// 判断状态码是否命中「期望状态码」表达式。
///
/// 表达式为逗号分隔的片段，每段可以是 `2xx` 这类百位通配，或 `200` 这类精确码。
/// 空表达式（含纯空白）按 `2xx` 处理，与前端默认值一致。
/// 无法解析的片段被忽略；全部片段都无法解析时不匹配任何状态码。
pub fn matches_expect(status: u16, spec: &str) -> bool {
    let spec = spec.trim();
    let spec = if spec.is_empty() { DEFAULT_EXPECT_STATUS } else { spec };
    spec.split(',')
        .map(str::trim)
        .filter(|seg| !seg.is_empty())
        .any(|seg| {
            let lower = seg.to_ascii_lowercase();
            if lower.len() == 3 && lower.ends_with("xx") {
                return lower[..1]
                    .parse::<u16>()
                    .map(|n| status / 100 == n)
                    .unwrap_or(false);
            }
            lower.parse::<u16>().map(|n| n == status).unwrap_or(false)
        })
}

#[derive(Debug, Default, Clone, Serialize)]
pub struct StressStats {
    pub total: u64,
    pub success: u64,
    pub failed: u64,
    pub success_rate: f64,    // 0.0 ~ 100.0（响应率：拿到响应的比例）
    // 1.0.5 新增：业务成功率（响应状态码命中 expect_status 的比例）
    pub biz_success: u64,
    pub biz_success_rate: f64, // 0.0 ~ 100.0
    pub avg_ms: f64,
    pub min_ms: u64,
    pub p50_ms: u64,
    pub p90_ms: u64,
    pub p95_ms: u64,
    pub p99_ms: u64,
    pub max_ms: u64,
    pub tps: f64,
    pub elapsed_sec: f64,
    pub done: bool,           // 压测是否已结束
    // 1.0.4 新增：耗时直方图（10 桶累计计数）+ 状态码分布（含 0 = 网络错误）
    pub latency_hist: Vec<u64>,
    pub status_counts: Vec<(u16, u64)>,
    /// 本轮使用的期望状态码表达式，写入快照供报告展示
    pub expect_status: String,
}

/// 内部可变统计（由 worker tasks 写入）
#[derive(Default)]
struct RawStats {
    total: u64,
    /// 拿到 HTTP 响应的请求数（对外称「响应率」的分子）
    success: u64,
    /// 传输失败数：超时 / 连接失败 / DNS 失败
    failed: u64,
    /// 1.0.5 新增：响应状态码命中 expect_status 的请求数
    biz_success: u64,
    /// 本轮的期望状态码表达式，原样写入快照供报告展示
    expect_status: String,
    /// 滑动窗口：只保留最近 DURATION_WINDOW 条耗时，防止 OOM。
    /// 百分位计算基于此窗口，count/avg 基于全量计数器。
    durations_window: Vec<u64>,
    /// 全量 avg 用增量维护（sum / total），不依赖 window
    duration_sum: u64,
    /// 1.0.4 新增：耗时直方图（10 桶累计）与状态码分布（0 = 网络错误）
    latency_hist: [u64; LATENCY_BUCKET_COUNT],
    status_counts: std::collections::HashMap<u16, u64>,
}

impl RawStats {
    fn record(&mut self, responded: bool, biz_ok: bool, duration_ms: u64, status: Option<u16>) {
        self.total += 1;
        if responded { self.success += 1; } else { self.failed += 1; }
        if biz_ok { self.biz_success += 1; }
        self.duration_sum = self.duration_sum.saturating_add(duration_ms);

        // 直方图分桶：定位 [0,1)..[500+)
        let d = duration_ms as f64;
        let mut bucket = LATENCY_BUCKET_COUNT - 1;
        for (i, upper) in LATENCY_BUCKETS.iter().enumerate() {
            if d < *upper { bucket = i; break; }
        }
        self.latency_hist[bucket] += 1;

        // 状态码分布：网络错误 status=None → 记 0（前端显示为「网络错误」）
        *self.status_counts.entry(status.unwrap_or(0)).or_insert(0) += 1;

        // 滑动窗口：超过上限时循环覆盖最旧条目
        if self.durations_window.len() < DURATION_WINDOW {
            self.durations_window.push(duration_ms);
        } else {
            let idx = (self.total as usize - 1) % DURATION_WINDOW;
            self.durations_window[idx] = duration_ms;
        }
    }

    fn snapshot(&self, elapsed_sec: f64, done: bool) -> StressStats {
        let mut sorted = self.durations_window.clone();
        sorted.sort_unstable();
        let n = sorted.len();

        let percentile = |pct: f64| -> u64 {
            if n == 0 { return 0; }
            let idx = ((n as f64 * pct / 100.0).ceil() as usize).saturating_sub(1).min(n - 1);
            sorted[idx]
        };

        let avg_ms = if self.total == 0 {
            0.0
        } else {
            self.duration_sum as f64 / self.total as f64
        };

        let min_ms = sorted.first().copied().unwrap_or(0);
        let max_ms = sorted.last().copied().unwrap_or(0);

        let success_rate = if self.total == 0 {
            0.0
        } else {
            self.success as f64 / self.total as f64 * 100.0
        };

        let biz_success_rate = if self.total == 0 {
            0.0
        } else {
            self.biz_success as f64 / self.total as f64 * 100.0
        };

        let tps = if elapsed_sec > 0.0 {
            self.total as f64 / elapsed_sec
        } else {
            0.0
        };

        // 状态码分布：按次数降序排序
        let mut status_vec: Vec<(u16, u64)> = self.status_counts.iter()
            .map(|(k, v)| (*k, *v)).collect();
        status_vec.sort_by(|a, b| b.1.cmp(&a.1));

        StressStats {
            total: self.total,
            success: self.success,
            failed: self.failed,
            success_rate,
            biz_success: self.biz_success,
            biz_success_rate,
            avg_ms,
            min_ms,
            p50_ms: percentile(50.0),
            p90_ms: percentile(90.0),
            p95_ms: percentile(95.0),
            p99_ms: percentile(99.0),
            max_ms,
            tps,
            elapsed_sec,
            done,
            latency_hist: self.latency_hist.to_vec(),
            status_counts: status_vec,
            expect_status: self.expect_status.clone(),
        }
    }
}

// ── fd 上限检测（macOS / Linux）────────────────────────────

#[cfg(unix)]
fn check_fd_limit(max_concurrent: u32) -> Result<(), String> {
    use std::mem::MaybeUninit;

    // SAFETY: getrlimit 是标准 POSIX syscall，参数结构体符合规范
    let mut rlim: libc::rlimit = unsafe { MaybeUninit::zeroed().assume_init() };
    let ret = unsafe { libc::getrlimit(libc::RLIMIT_NOFILE, &mut rlim) };
    if ret != 0 {
        return Ok(()); // 获取失败时不阻断，保守通过
    }
    let current_limit = rlim.rlim_cur;
    let needed = (max_concurrent as u64) * 4;
    if current_limit != libc::RLIM_INFINITY && current_limit < needed {
        return Err(format!(
            "系统文件描述符上限（{}）不足以支撑 {} 并发（需要约 {}）。\
             请运行 `ulimit -n {}` 提升上限后重试。",
            current_limit, max_concurrent, needed, needed
        ));
    }
    Ok(())
}

#[cfg(not(unix))]
fn check_fd_limit(_max_concurrent: u32) -> Result<(), String> {
    Ok(()) // Windows 无此限制，直接通过
}

// ── 压测报告生成（1.0.5）────────────────────────────────────
//
// 报告内容（说什么）的唯一真源：App 的预览/导出与 bridge 技能调用共用这一份。
// HTML 的「渲染」——把 Markdown 变成带样式的页面——由各消费方自己做
// （App 用 markdown-it，技能自带工具），那是渲染而非内容，不构成第二份真源。

/// 报告生成时对 `stats_json` 的宽松解析视图。
/// 旧记录缺字段一律 None，报告里显示「未采集」，不编造数值。
#[derive(Default, serde::Deserialize)]
struct ReportStats {
    #[serde(default)] total: u64,
    #[serde(default)] success: u64,
    #[serde(default)] failed: u64,
    #[serde(default)] success_rate: f64,
    #[serde(default)] avg_ms: f64,
    #[serde(default)] min_ms: u64,
    #[serde(default)] p50_ms: u64,
    #[serde(default)] p90_ms: u64,
    #[serde(default)] p95_ms: u64,
    #[serde(default)] p99_ms: u64,
    #[serde(default)] max_ms: u64,
    #[serde(default)] tps: f64,
    #[serde(default)] elapsed_sec: f64,
    // 1.0.5 新增字段：1.0.4 及以前的记录没有 → None → 报告标「未采集」
    #[serde(default)] biz_success: Option<u64>,
    #[serde(default)] biz_success_rate: Option<f64>,
    #[serde(default)] expect_status: Option<String>,
    #[serde(default)] latency_hist: Option<Vec<u64>>,
    #[serde(default)] status_counts: Option<Vec<(u16, u64)>>,
}

/// 报告生成时对 `config_json` 的宽松解析视图。
/// 期望状态码不在这里取——统一走 `ReportStats.expect_status`，
/// 因为只有它能区分「旧记录没采集」和「采集到了」。
#[derive(Default, serde::Deserialize)]
struct ReportConfig {
    #[serde(default)] concurrent: Option<u32>,
    #[serde(default)] mode: Option<String>,
    #[serde(default)] value: Option<u64>,
    // 1.0.6：压测参考线阈值（旧记录没有 → 用默认值）
    #[serde(default)] p95_threshold_ms: Option<u64>,
    #[serde(default)] p99_threshold_ms: Option<u64>,
}

const NA_UNCOLLECTED: &str = "—（旧版本未采集）";

/// 状态码语义说明（报告的「说明」列）。
/// 注意：前端 `stressReport.ts` 有一份同名的 TS 实现，仅供 App 状态码分布条使用；
/// 报告里的这一列由本函数生成。两处都是 5 个分支的稳定映射，刻意不强行统一。
fn status_hint(code: u16) -> &'static str {
    match code {
        0 => "网络错误（未拿到响应）",
        200..=299 => "成功",
        300..=399 => "重定向",
        400..=499 => "客户端错误",
        500..=599 => "服务端错误",
        _ => "未知",
    }
}

/// 结论摘要。按固定顺序产出，保证同一份数据每次生成结果一致。
fn conclusions(st: &ReportStats, p95_threshold: u64, p99_threshold: u64) -> Vec<String> {
    let mut out = Vec::new();

    if st.failed > 0 {
        out.push(format!(
            "⚠️ **{}** 次请求未拿到响应（超时 / 连接失败 / DNS 失败），响应率 **{:.1}%**",
            st.failed, st.success_rate
        ));
    }

    match st.biz_success_rate {
        None => out.push("ℹ️ 该记录由旧版本产生，未采集「业务成功率」与「期望状态码」".to_string()),
        Some(rate) if rate < 100.0 => {
            let bad = st.total.saturating_sub(st.biz_success.unwrap_or(0));
            let spec = st.expect_status.as_deref().unwrap_or(DEFAULT_EXPECT_STATUS);
            out.push(format!(
                "⚠️ **{}** 次响应的状态码不在期望范围（`{}`），业务成功率 **{:.1}%**",
                bad, spec, rate
            ));
        }
        Some(_) => {}
    }

    if st.p95_ms > p95_threshold {
        out.push(format!("⚠️ P95 耗时 **{}ms**，超过 {}ms 参考线", st.p95_ms, p95_threshold));
    }
    if st.p99_ms > p99_threshold {
        out.push(format!("❌ P99 耗时 **{}ms**，超过 {}ms，长尾明显", st.p99_ms, p99_threshold));
    }

    if out.is_empty() {
        out.push(format!("✅ 全部请求均拿到响应，状态码全部符合预期，P95 在 {}ms 以内", p95_threshold));
    }
    out
}

/// 生成一张 Markdown 表格（含表头分隔行），末尾补一个空行
fn md_table(headers: &[&str], rows: &[Vec<String>]) -> String {
    let mut s = String::new();
    s.push_str(&format!("| {} |\n", headers.join(" | ")));
    let sep: Vec<&str> = headers.iter().map(|_| "---").collect();
    s.push_str(&format!("| {} |\n", sep.join(" | ")));
    for r in rows {
        s.push_str(&format!("| {} |\n", r.join(" | ")));
    }
    s.push('\n');
    s
}

/// 生成压测报告的 Markdown。App 预览/导出与 bridge 技能共用这一份内容。
/// 默认 P95 参考线（毫秒）
pub const DEFAULT_P95_THRESHOLD_MS: u64 = 500;
/// 默认 P99 参考线（毫秒）
pub const DEFAULT_P99_THRESHOLD_MS: u64 = 1000;

pub fn build_stress_report_markdown(run: &StressRun, request: Option<&ApiRequest>) -> String {
    let cfg: ReportConfig = serde_json::from_str(&run.config_json).unwrap_or_default();
    let st: Option<ReportStats> = serde_json::from_str(&run.stats_json).ok();

    // 阈值取值链：config_json（压测时写入）→ 默认值
    let p95_threshold = cfg.p95_threshold_ms.unwrap_or(DEFAULT_P95_THRESHOLD_MS);
    let p99_threshold = cfg.p99_threshold_ms.unwrap_or(DEFAULT_P99_THRESHOLD_MS);

    let mut out = String::new();
    out.push_str("# ApiCat 压测报告\n\n");

    match request {
        Some(r) => {
            let name = if r.name.is_empty() { "(未命名)" } else { r.name.as_str() };
            out.push_str(&format!("> **接口**：`{}` {}\n", r.method, name));
            out.push_str(&format!("> **URL**：`{}`\n", r.url));
        }
        None => out.push_str(&format!("> **接口**：request_id = {}\n", run.request_id)),
    }
    out.push_str(&format!("> **压测时间**：{}\n\n", run.created_at));

    let Some(st) = st else {
        out.push_str("（无统计数据）\n");
        return out;
    };

    let has_biz = st.biz_success_rate.is_some();
    let expect_shown = if has_biz {
        st.expect_status.as_deref().unwrap_or(DEFAULT_EXPECT_STATUS).to_string()
    } else {
        NA_UNCOLLECTED.to_string()
    };

    // ── 一、结论摘要 ──
    out.push_str("## 一、结论摘要\n\n");
    for c in conclusions(&st, p95_threshold, p99_threshold) {
        out.push_str(&format!("- {c}\n"));
    }
    out.push('\n');

    // ── 二、核心指标 ──
    out.push_str("## 二、核心指标\n\n");
    let biz_cell = if has_biz {
        format!(
            "{:.1}%（{} / {}）",
            st.biz_success_rate.unwrap_or(0.0),
            st.biz_success.unwrap_or(0),
            st.total
        )
    } else {
        NA_UNCOLLECTED.to_string()
    };
    out.push_str(&md_table(&["指标", "值"], &[
        vec!["总请求数".into(), st.total.to_string()],
        vec!["响应率".into(), format!("{:.1}%（{} / {}）", st.success_rate, st.success, st.total)],
        vec!["业务成功率".into(), biz_cell],
        vec!["期望状态码".into(), expect_shown.clone()],
        vec!["传输失败".into(), st.failed.to_string()],
        vec!["TPS".into(), format!("{:.1}", st.tps)],
        vec!["平均耗时".into(), format!("{:.1} ms", st.avg_ms)],
        vec!["最小耗时".into(), format!("{} ms", st.min_ms)],
        vec!["P50".into(), format!("{} ms", st.p50_ms)],
        vec!["P90".into(), format!("{} ms", st.p90_ms)],
        vec!["P95".into(), format!("{} ms", st.p95_ms)],
        vec!["P99".into(), format!("{} ms", st.p99_ms)],
        vec!["最大耗时".into(), format!("{} ms", st.max_ms)],
        vec!["压测时长".into(), format!("{:.2} s", st.elapsed_sec)],
    ]));

    // ── 三、耗时分布 ──
    out.push_str("## 三、耗时分布\n\n");
    match &st.latency_hist {
        None => out.push_str(&format!("{NA_UNCOLLECTED}\n\n")),
        Some(hist) => {
            let total: u64 = hist.iter().sum::<u64>().max(1);
            let rows: Vec<Vec<String>> = LATENCY_LABELS
                .iter()
                .enumerate()
                .map(|(i, label)| {
                    let n = hist.get(i).copied().unwrap_or(0);
                    vec![
                        label.to_string(),
                        n.to_string(),
                        format!("{:.1}%", n as f64 / total as f64 * 100.0),
                    ]
                })
                .collect();
            out.push_str(&md_table(&["区间(ms)", "次数", "占比"], &rows));
        }
    }

    // ── 四、状态码分布 ──
    out.push_str("## 四、状态码分布\n\n");
    match &st.status_counts {
        None => out.push_str(&format!("{NA_UNCOLLECTED}\n\n")),
        Some(sc) => {
            let total: u64 = sc.iter().map(|(_, n)| n).sum::<u64>().max(1);
            let rows: Vec<Vec<String>> = sc
                .iter()
                .map(|(code, n)| {
                    vec![
                        if *code == 0 { "网络错误".to_string() } else { code.to_string() },
                        n.to_string(),
                        format!("{:.0}%", *n as f64 / total as f64 * 100.0),
                        status_hint(*code).to_string(),
                    ]
                })
                .collect();
            out.push_str(&md_table(&["状态码", "次数", "占比", "说明"], &rows));
        }
    }

    // ── 五、压测配置 ──
    out.push_str("## 五、压测配置\n\n");
    let mode_cell = match cfg.mode.as_deref() {
        Some("count") => "总请求数".to_string(),
        Some("duration") => "持续时间".to_string(),
        Some(other) => other.to_string(),
        None => "—".to_string(),
    };
    out.push_str(&md_table(&["项", "值"], &[
        vec!["并发数".into(), cfg.concurrent.map(|v| v.to_string()).unwrap_or_else(|| "—".into())],
        vec!["模式".into(), mode_cell],
        vec!["请求数 / 持续秒数".into(), cfg.value.map(|v| v.to_string()).unwrap_or_else(|| "—".into())],
        vec!["期望状态码".into(), expect_shown],
        vec!["P95 参考线".into(), format!("{} ms", p95_threshold)],
        vec!["P99 参考线".into(), format!("{} ms", p99_threshold)],
    ]));

    out
}

// ── 主压测 Command ─────────────────────────────────────────

/// 核心压测逻辑（供 Tauri command 和 HTTP bridge 共用），阻塞到压测结束，返回最终统计
pub async fn start_stress_impl(
    app: &AppHandle,
    pool: &sqlx::SqlitePool,
    request_id: i64,
    params: SendRequestParams,
    concurrent: u32,
    mode: &str,
    value: u64,
    expect_status: &str,
    p95_threshold_ms: Option<u64>,
    p99_threshold_ms: Option<u64>,
) -> Result<StressStats, crate::error::AppError> {
    let app = app.clone();
    let mode = mode.to_string();
    // ── 参数校验 ──────────────────────────────────────────
    if concurrent == 0 || concurrent > 500 {
        return Err(crate::error::AppError::Custom(
            "并发数必须在 1 ~ 500 之间".to_string(),
        ));
    }
    if value == 0 {
        return Err(crate::error::AppError::Custom(
            "请求数/持续时间不能为 0".to_string(),
        ));
    }
    // count 模式上限：防止一次性积累过多 JoinHandle 占用大量内存
    if mode == "count" && value > 10_000 {
        return Err(crate::error::AppError::Custom(
            "单次压测请求数不能超过 10000，请使用「持续时间」模式进行大规模压测".to_string(),
        ));
    }

    // ── fd 上限检测（§10.2）──────────────────────────────
    check_fd_limit(concurrent).map_err(crate::error::AppError::Custom)?;

    // ── 构建专用 reqwest Client（连接池不超过 max_concurrent）
    let client = reqwest::Client::builder()
        .pool_max_idle_per_host(concurrent as usize)
        .timeout(Duration::from_secs(30))
        .danger_accept_invalid_certs(true)
        .build()
        .map_err(|e| crate::error::AppError::Custom(format!("构建 HTTP 客户端失败: {e}")))?;

    // ── 共享统计（Arc<Mutex<RawStats>>）──────────────────
    let raw_stats = Arc::new(Mutex::new(RawStats {
        expect_status: expect_status.to_string(),
        ..Default::default()
    }));

    // ── 停止信号（broadcast channel）──────────────────────
    // 容量 1 足够，所有 worker 都会监听
    let (stop_tx, _) = broadcast::channel::<()>(1);

    // ── timer 完成确认（oneshot）—— 替代 sleep(300ms) 脆弱等待
    // main task 发送 stop 后阻塞等待 timer 确认已 emit done 事件
    let (timer_done_tx, timer_done_rx) = tokio::sync::oneshot::channel::<()>();

    let start_time = Instant::now();

    // ── 定时推送任务（每 200ms emit 一次，§10.1 聚合方案）─
    {
        let raw_stats_clone = Arc::clone(&raw_stats);
        let app_clone = app.clone();
        let mut stop_rx = stop_tx.subscribe();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(200));
            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        let snapshot = {
                            let guard = raw_stats_clone.lock().unwrap();
                            guard.snapshot(start_time.elapsed().as_secs_f64(), false)
                        };
                        // 推送进度事件（聚合后，非逐请求）
                        let _ = app_clone.emit("stress://progress", &snapshot);
                    }
                    _ = stop_rx.recv() => {
                        // 收到停止信号，发最终 done 快照后退出
                        let snapshot = {
                            let guard = raw_stats_clone.lock().unwrap();
                            guard.snapshot(start_time.elapsed().as_secs_f64(), true)
                        };
                        let _ = app_clone.emit("stress://progress", &snapshot);
                        let _ = app_clone.emit("stress://done", &snapshot);
                        // 通知 main task：done 已 emit，可安全返回
                        let _ = timer_done_tx.send(());
                        break;
                    }
                }
            }
        });
    }

    // ── 压测 Worker 池 ────────────────────────────────────
    match mode.as_str() {
        "count" => {
            // 固定总请求数模式：用 semaphore 限并发，dispatch value 个任务
            let semaphore = Arc::new(tokio::sync::Semaphore::new(concurrent as usize));
            let mut handles = Vec::with_capacity(value as usize);

            for _ in 0..value {
                let permit = Arc::clone(&semaphore)
                    .acquire_owned()
                    .await
                    .map_err(|e| crate::error::AppError::Custom(format!("semaphore 错误: {e}")))?;
                let client_clone = client.clone();
                let params_clone = params.clone();
                let raw_clone = Arc::clone(&raw_stats);

                let h = tokio::spawn(async move {
                    let t = Instant::now();
                    match send(&client_clone, &params_clone).await {
                        Ok(resp) => {
                            let dur = t.elapsed().as_millis() as u64;
                            // 先取出期望表达式再释放锁，避免在已持锁时二次取锁导致死锁
                            let spec = raw_clone.lock().unwrap().expect_status.clone();
                            let biz_ok = matches_expect(resp.status_code, &spec);
                            raw_clone.lock().unwrap().record(true, biz_ok, dur, Some(resp.status_code));
                        }
                        Err(_) => {
                            let dur = t.elapsed().as_millis() as u64;
                            raw_clone.lock().unwrap().record(false, false, dur, None);
                        }
                    }
                    drop(permit);
                });
                handles.push(h);
            }

            // 等待所有 worker 完成
            for h in handles {
                let _ = h.await;
            }
        }
        "duration" => {
            // 持续时间模式：在 value 秒内不断发请求
            // 用 JoinSet 替代 Vec<JoinHandle>：abort_all() 可强制中止超时任务，无需积累句柄
            let deadline = start_time + Duration::from_secs(value);
            let semaphore = Arc::new(tokio::sync::Semaphore::new(concurrent as usize));
            let mut join_set = tokio::task::JoinSet::new();

            loop {
                if Instant::now() >= deadline {
                    break;
                }
                // 清理已完成的任务，避免 JoinSet 无限增长
                while let Some(Ok(_)) = join_set.try_join_next() {}

                let permit = match Arc::clone(&semaphore).try_acquire_owned() {
                    Ok(p) => p,
                    Err(_) => {
                        // 并发槽满，yield 一次让其他任务运行（避免忙等耗 CPU）
                        tokio::task::yield_now().await;
                        continue;
                    }
                };
                let client_clone = client.clone();
                let params_clone = params.clone();
                let raw_clone = Arc::clone(&raw_stats);

                join_set.spawn(async move {
                    let t = Instant::now();
                    match send(&client_clone, &params_clone).await {
                        Ok(resp) => {
                            let dur = t.elapsed().as_millis() as u64;
                            // 先取出期望表达式再释放锁，避免在已持锁时二次取锁导致死锁
                            let spec = raw_clone.lock().unwrap().expect_status.clone();
                            let biz_ok = matches_expect(resp.status_code, &spec);
                            raw_clone.lock().unwrap().record(true, biz_ok, dur, Some(resp.status_code));
                        }
                        Err(_) => {
                            let dur = t.elapsed().as_millis() as u64;
                            raw_clone.lock().unwrap().record(false, false, dur, None);
                        }
                    }
                    drop(permit);
                });
            }

            // 等待已派发的请求完成（不强制 abort，给进行中请求机会完成）
            join_set.join_all().await;
        }
        _ => {
            return Err(crate::error::AppError::Custom(
                "mode 必须为 'count' 或 'duration'".to_string(),
            ));
        }
    }

    // 通知定时推送任务结束
    let _ = stop_tx.send(());
    // 等待 timer 任务确认已 emit done 事件（代替 sleep(300ms)）
    // 超时 1s 保底，防止 timer 异常时主任务卡死
    let _ = tokio::time::timeout(Duration::from_secs(1), timer_done_rx).await;

    // 1.0.4：压测结果落库（最终快照 + 配置）
    let final_snapshot = raw_stats.lock().unwrap().snapshot(start_time.elapsed().as_secs_f64(), true);
    let stats_json = serde_json::to_string(&final_snapshot).unwrap_or_else(|_| "{}".to_string());
    let config_json = serde_json::json!({
        "concurrent": concurrent,
        "mode": mode,
        "value": value,
        "expect_status": expect_status,
        "p95_threshold_ms": p95_threshold_ms,
        "p99_threshold_ms": p99_threshold_ms,
    }).to_string();
    let _ = sqlx::query(
        "INSERT INTO stress_runs (request_id, config_json, stats_json) VALUES (?1, ?2, ?3)",
    )
    .bind(request_id)
    .bind(&config_json)
    .bind(&stats_json)
    .execute(pool)
    .await;

    Ok(final_snapshot)
}

/// Tauri command 包装：启动压测（阻塞到结束，前端靠事件收进度）
#[tauri::command]
pub async fn start_stress(
    app: AppHandle,
    db: State<'_, AppDb>,
    request_id: i64,
    params: SendRequestParams,
    concurrent: u32,
    mode: String,
    value: u64,
    expect_status: Option<String>,
    p95_threshold_ms: Option<u64>,
    p99_threshold_ms: Option<u64>,
) -> CmdResult<()> {
    let expect = expect_status.unwrap_or_else(|| DEFAULT_EXPECT_STATUS.to_string());
    start_stress_impl(&app, &db.0, request_id, params, concurrent, &mode, value, &expect, p95_threshold_ms, p99_threshold_ms).await?;
    Ok(())
}

/// 获取某接口的压测历史（按时间倒序，最近 50 条）
#[tauri::command]
pub async fn list_stress_runs(
    db: State<'_, AppDb>,
    request_id: i64,
) -> CmdResult<Vec<StressRun>> {
    let rows = sqlx::query_as::<Sqlite, StressRun>(
        "SELECT id, request_id, config_json, stats_json, created_at
         FROM stress_runs WHERE request_id = ?1 ORDER BY created_at DESC LIMIT 50",
    )
    .bind(request_id)
    .fetch_all(&db.0)
    .await?;
    Ok(rows)
}

/// 删除一条压测历史
#[tauri::command]
pub async fn delete_stress_run(db: State<'_, AppDb>, id: i64) -> CmdResult<()> {
    sqlx::query("DELETE FROM stress_runs WHERE id = ?1")
        .bind(id)
        .execute(&db.0)
        .await?;
    Ok(())
}

/// 按记录 id 生成报告（Markdown）。IPC 与 bridge 共用。
pub async fn build_stress_report_by_id(
    pool: &sqlx::SqlitePool,
    run_id: i64,
) -> Result<String, crate::error::AppError> {
    let run = sqlx::query_as::<Sqlite, StressRun>(
        "SELECT id, request_id, config_json, stats_json, created_at
         FROM stress_runs WHERE id = ?1",
    )
    .bind(run_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| crate::error::AppError::Custom(format!("压测记录 {run_id} 不存在")))?;

    // 接口可能已取不到，那就降级成只显示 request_id，不报错
    let request = sqlx::query_as::<Sqlite, ApiRequest>(&format!(
        "SELECT {REQUEST_COLS} FROM api_requests WHERE id = ?1"
    ))
    .bind(run.request_id)
    .fetch_optional(pool)
    .await?;

    Ok(build_stress_report_markdown(&run, request.as_ref()))
}

/// 生成某条压测历史的报告（Markdown）
#[tauri::command]
pub async fn get_stress_report(db: State<'_, AppDb>, run_id: i64) -> CmdResult<String> {
    build_stress_report_by_id(&db.0, run_id).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_matches_expect_class_wildcard() {
        assert!(matches_expect(200, "2xx"));
        assert!(matches_expect(204, "2xx"));
        assert!(!matches_expect(301, "2xx"));
        assert!(!matches_expect(404, "2xx"));
        assert!(!matches_expect(500, "2xx"));
    }

    #[test]
    fn test_matches_expect_explicit_code() {
        assert!(matches_expect(200, "200"));
        assert!(!matches_expect(201, "200"));
    }

    #[test]
    fn test_matches_expect_multi_segment() {
        assert!(matches_expect(201, "200,201"));
        assert!(matches_expect(404, "2xx,404"));
        assert!(matches_expect(302, "3xx"));
        assert!(!matches_expect(500, "2xx,404"));
    }

    #[test]
    fn test_matches_expect_blank_defaults_to_2xx() {
        assert!(matches_expect(200, ""));
        assert!(matches_expect(200, "   "));
        assert!(!matches_expect(500, ""));
    }

    #[test]
    fn test_matches_expect_garbage_never_matches() {
        assert!(!matches_expect(200, "abc"));
        assert!(!matches_expect(200, "2x"));
        assert!(!matches_expect(200, ",,,"));
    }

    #[test]
    fn test_record_keeps_respond_and_business_apart() {
        let mut s = RawStats::default();
        s.record(true, true, 10, Some(200));   // 拿到响应 + 命中期望
        s.record(true, false, 20, Some(500));  // 拿到响应但业务失败
        s.record(false, false, 30, None);      // 传输失败

        assert_eq!(s.total, 3);
        assert_eq!(s.success, 2);      // 响应 2 次
        assert_eq!(s.failed, 1);       // 传输失败 1 次
        assert_eq!(s.biz_success, 1);  // 业务成功 1 次

        let snap = s.snapshot(1.0, true);
        assert!((snap.success_rate - 200.0 / 3.0).abs() < 1e-9);
        assert!((snap.biz_success_rate - 100.0 / 3.0).abs() < 1e-9);
    }

    #[test]
    fn test_snapshot_carries_expect_status() {
        let mut s = RawStats { expect_status: "200".to_string(), ..Default::default() };
        s.record(true, true, 5, Some(200));
        assert_eq!(s.snapshot(1.0, true).expect_status, "200");
    }

    // ── 报告生成器 ────────────────────────────────────────

    /// 测试夹具：只填报告生成器真正读到的字段
    fn sample_run(stats_json: &str, config_json: &str) -> StressRun {
        StressRun {
            id: 1,
            request_id: 7,
            config_json: config_json.to_string(),
            stats_json: stats_json.to_string(),
            created_at: "2026-09-18 14:03:22".to_string(),
        }
    }

    /// 新记录：100 次请求全部拿到响应，但期望 2xx、实际全是 500
    #[test]
    fn test_report_flags_business_failure() {
        let run = sample_run(
            r#"{"total":100,"success":100,"failed":0,"success_rate":100.0,"avg_ms":12.5,
                "min_ms":1,"p50_ms":10,"p90_ms":20,"p95_ms":30,"p99_ms":40,"max_ms":50,
                "tps":50.0,"elapsed_sec":2.0,"biz_success":0,"biz_success_rate":0.0,
                "expect_status":"2xx","latency_hist":[0,0,0,0,0,0,0,0,0,100],
                "status_counts":[[500,100]]}"#,
            r#"{"concurrent":10,"mode":"count","value":100,"expect_status":"2xx"}"#,
        );
        let md = build_stress_report_markdown(&run, None);
        // 传输没问题，所以不该出现「未拿到响应」的告警
        assert!(!md.contains("次请求未拿到响应"), "不该报传输失败:\n{md}");
        // 但业务全错，必须报出来
        assert!(md.contains("业务成功率 **0.0%**"), "缺业务失败告警:\n{md}");
        assert!(md.contains("| 500 | 100 |"), "缺状态码明细:\n{md}");
        assert!(md.contains("服务端错误"), "缺状态码语义说明:\n{md}");
    }

    /// 旧记录：没有 biz_success_rate / latency_hist / status_counts，
    /// 必须显示「未采集」而不是编造 0%
    #[test]
    fn test_report_marks_legacy_record_as_uncollected() {
        let run = sample_run(
            r#"{"total":10,"success":10,"failed":0,"success_rate":100.0,"avg_ms":1.0,
                "min_ms":1,"p50_ms":1,"p90_ms":1,"p95_ms":1,"p99_ms":1,"max_ms":1,
                "tps":5.0,"elapsed_sec":2.0}"#,
            r#"{"concurrent":5,"mode":"count","value":10}"#,
        );
        let md = build_stress_report_markdown(&run, None);
        assert!(md.contains("旧版本未采集"), "旧记录应标未采集:\n{md}");
        assert!(md.contains("旧版本产生"), "旧记录应有说明行:\n{md}");
        assert!(!md.contains("业务成功率 **0.0%**"), "不该给旧记录编造 0%:\n{md}");
    }

    /// 健康记录：全绿，结论是 ✅
    #[test]
    fn test_report_healthy_record_concludes_ok() {
        let run = sample_run(
            r#"{"total":100,"success":100,"failed":0,"success_rate":100.0,"avg_ms":12.5,
                "min_ms":1,"p50_ms":10,"p90_ms":20,"p95_ms":30,"p99_ms":40,"max_ms":50,
                "tps":50.0,"elapsed_sec":2.0,"biz_success":100,"biz_success_rate":100.0,
                "expect_status":"2xx","latency_hist":[0,0,0,0,0,0,0,0,0,100],
                "status_counts":[[200,100]]}"#,
            r#"{"concurrent":10,"mode":"count","value":100,"expect_status":"2xx"}"#,
        );
        let md = build_stress_report_markdown(&run, None);
        assert!(md.contains('✅'), "健康记录应给出通过结论:\n{md}");
        assert!(!md.contains('⚠'));
        assert!(!md.contains('❌'));
    }

    /// 带接口信息时，报告头部应显示 method / name / url
    #[test]
    fn test_report_uses_request_brief_when_present() {
        let run = sample_run(
            r#"{"total":1,"success":1,"failed":0,"success_rate":100.0,"avg_ms":1.0,
                "min_ms":1,"p50_ms":1,"p90_ms":1,"p95_ms":1,"p99_ms":1,"max_ms":1,
                "tps":1.0,"elapsed_sec":1.0,"biz_success":1,"biz_success_rate":100.0,
                "expect_status":"2xx","latency_hist":[1,0,0,0,0,0,0,0,0,0],
                "status_counts":[[200,1]]}"#,
            r#"{"concurrent":1,"mode":"count","value":1,"expect_status":"2xx"}"#,
        );
        let req = ApiRequest {
            id: 7, collection_id: 1,
            name: "登录".to_string(), method: "POST".to_string(),
            url: "https://example.com/api/login".to_string(),
            params: "[]".to_string(), headers: "[]".to_string(),
            body_type: "json".to_string(), body: "{}".to_string(),
            auth_type: "none".to_string(), auth_config: "{}".to_string(),
            description: String::new(), sort_order: 0,
            created_at: String::new(), updated_at: String::new(),
        };
        let md = build_stress_report_markdown(&run, Some(&req));
        assert!(md.contains("`POST` 登录"), "缺接口名:\n{md}");
        assert!(md.contains("https://example.com/api/login"), "缺 URL:\n{md}");
    }

    /// stats_json 是坏的时候不能 panic
    #[test]
    fn test_report_survives_garbage_stats() {
        let run = sample_run("not json at all", "{}");
        let md = build_stress_report_markdown(&run, None);
        assert!(md.contains("（无统计数据）"), "坏数据应优雅降级:\n{md}");
    }
}
