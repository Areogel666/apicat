//! 压测引擎
//!
//! 关键约束（§10.1 CRITICAL）：
//!   - 禁止逐请求 emit；每 200ms 聚合一次统计后推送
//!   - reqwest 连接池大小不超过 max_concurrent（§10.2）
//!   - Windows 跳过 fd 检测；macOS/Linux 检测 getrlimit

use crate::{error::CmdResult, http::client::{send, SendRequestParams}};
use serde::Serialize;
use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tauri::{AppHandle, Emitter};
use tokio::sync::broadcast;

/// 百分位滑动窗口大小：只保留最近 N 条耗时用于 P50/P95/P99 计算。
/// 内存上限 ≈ 10000 × 8B = 80KB，对长时压测 OOM 友好。
const DURATION_WINDOW: usize = 10_000;

// ── 统计数据（每 200ms 快照推送到前端）────────────────────

#[derive(Debug, Default, Clone, Serialize)]
pub struct StressStats {
    pub total: u64,
    pub success: u64,
    pub failed: u64,
    pub success_rate: f64,    // 0.0 ~ 100.0
    pub avg_ms: f64,
    pub p50_ms: u64,
    pub p95_ms: u64,
    pub p99_ms: u64,
    pub tps: f64,
    pub elapsed_sec: f64,
    pub done: bool,           // 压测是否已结束
}

/// 内部可变统计（由 worker tasks 写入）
#[derive(Default)]
struct RawStats {
    total: u64,
    success: u64,
    failed: u64,
    /// 滑动窗口：只保留最近 DURATION_WINDOW 条耗时，防止 OOM。
    /// 百分位计算基于此窗口，count/avg 基于全量计数器。
    durations_window: Vec<u64>,
    /// 全量 avg 用增量维护（sum / total），不依赖 window
    duration_sum: u64,
}

impl RawStats {
    fn record(&mut self, ok: bool, duration_ms: u64) {
        self.total += 1;
        if ok { self.success += 1; } else { self.failed += 1; }
        self.duration_sum = self.duration_sum.saturating_add(duration_ms);

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

        let success_rate = if self.total == 0 {
            0.0
        } else {
            self.success as f64 / self.total as f64 * 100.0
        };

        let tps = if elapsed_sec > 0.0 {
            self.total as f64 / elapsed_sec
        } else {
            0.0
        };

        StressStats {
            total: self.total,
            success: self.success,
            failed: self.failed,
            success_rate,
            avg_ms,
            p50_ms: percentile(50.0),
            p95_ms: percentile(95.0),
            p99_ms: percentile(99.0),
            tps,
            elapsed_sec,
            done,
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

// ── 主压测 Command ─────────────────────────────────────────

/// 启动压测
///
/// # 参数
/// - `params`：复用 SendRequestParams（与普通发请求相同的请求配置）
/// - `concurrent`：并发 worker 数（1-500）
/// - `mode`：`"count"` | `"duration"`
/// - `value`：mode=count 时为总请求数；mode=duration 时为持续秒数
#[tauri::command]
pub async fn start_stress(
    app: AppHandle,
    params: SendRequestParams,
    concurrent: u32,
    mode: String,
    value: u64,
) -> CmdResult<()> {
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
    let raw_stats = Arc::new(Mutex::new(RawStats::default()));

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
                    let ok = send(&client_clone, &params_clone).await.is_ok();
                    let dur = t.elapsed().as_millis() as u64;
                    raw_clone.lock().unwrap().record(ok, dur);
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
                    let ok = send(&client_clone, &params_clone).await.is_ok();
                    let dur = t.elapsed().as_millis() as u64;
                    raw_clone.lock().unwrap().record(ok, dur);
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

    Ok(())
}
