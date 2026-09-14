//! 第 28 章配套代码：部署、监控与性能优化。
//!
//! 运行方式：`cargo run`（最后一章，输出接在第 27 章后面）。
//!
//! 这一章把「程序写完到线上跑稳」之间缺的那一段补上：
//!
//! - **健康检查**：存活探针与就绪探针语义不同，别用一个接口糊弄；
//! - **指标**：请求数、错误率、延迟百分位——没有数据就没有优化；
//! - **日志**：字段化输出，能被检索和告警；
//! - **性能**：先用测量找到热点，再决定改什么（第 23 章的 criterion 是同一套思路）。
//!
//! 只用标准库和仓库已有依赖，不引入新的第三方 crate。

use crate::rust26_database::{add_task, add_tasks_transactional, init};
use rusqlite::Connection;
use std::path::Path;
use std::time::{Duration, Instant};
use tracing::{info, warn};
use tracing_subscriber::fmt;

/// 健康状态。
///
/// 「存活」和「就绪」是两个问题：进程活着不代表能接流量，
/// 依赖挂了也不该被反复重启（重启治不了数据库故障）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Health {
    /// 进程存活：能跑到这里就说明事件循环还在转。
    Live,
    /// 就绪：依赖都可用，可以接流量。
    Ready,
    /// 依赖不可用：先别派流量，但不需要重启进程。
    Degraded,
}

impl Health {
    /// 探针要返回的 HTTP 状态码（第 22 章说过 5xx 表示服务端问题）。
    pub fn status_code(self) -> u16 {
        match self {
            Health::Live | Health::Ready => 200,
            Health::Degraded => 503,
        }
    }

    /// 是否可以把流量派过来。
    pub fn is_ready(self) -> bool {
        matches!(self, Health::Ready)
    }
}

/// 存活探针：不做任何 IO，也不查依赖——它只回答「进程还在不在」。
pub const fn liveness() -> Health {
    Health::Live
}

/// 就绪判定：数据库和依赖都可用才算就绪。
///
/// 真实服务里这两个输入来自「ping 数据库」和「配置/迁移是否完成」，
/// 而且必须有超时——探针自己卡死是最糟糕的故障形态。
pub fn health_status(database_ok: bool, dependency_ok: bool) -> Health {
    if database_ok && dependency_ok {
        Health::Ready
    } else {
        Health::Degraded
    }
}

/// 指标快照：适合打印、写日志或导出给监控系统。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MetricsSnapshot {
    pub requests: u64,
    pub errors: u64,
    /// 成功率，0.0 ~ 1.0；没有请求时约定为 0.0 而不是 NaN。
    pub success_rate: f64,
    /// 延迟 P95（毫秒）；没有样本时为 `None`。
    pub p95_ms: Option<u64>,
}

impl std::fmt::Display for MetricsSnapshot {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let p95 = match self.p95_ms {
            Some(value) => format!("{value}ms"),
            None => String::from("无样本"),
        };
        write!(
            formatter,
            "requests={} errors={} success={:.1}% p95={p95}",
            self.requests,
            self.errors,
            self.success_rate * 100.0
        )
    }
}

/// 最小可用的指标收集器：请求数、错误数、延迟样本。
///
/// 生产系统会用直方图（例如 Prometheus 的 histogram）避免保存每个样本，
/// 但「用固定内存换精度」的权衡要先理解清楚，再决定要不要换。
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Metrics {
    requests: u64,
    errors: u64,
    latencies_ms: Vec<u64>,
}

impl Metrics {
    /// 记录一次请求的结果与耗时。
    pub fn record(&mut self, success: bool, elapsed: Duration) {
        self.requests += 1;
        self.errors += u64::from(!success);
        self.latencies_ms.push(elapsed.as_millis() as u64);
    }

    pub fn requests(&self) -> u64 {
        self.requests
    }

    pub fn errors(&self) -> u64 {
        self.errors
    }

    /// 成功率；没有任何请求时返回 0.0（而不是除零或 NaN）。
    pub fn success_rate(&self) -> f64 {
        if self.requests == 0 {
            return 0.0;
        }
        (self.requests - self.errors) as f64 / self.requests as f64
    }

    /// 最近秩（nearest-rank）百分位：排序后取第 `ceil(p/100 * n)` 个值。
    ///
    /// 这种算法不需要插值，结果一定来自真实样本，适合看「最慢的那批请求」。
    pub fn percentile_ms(&self, percent: u8) -> Option<u64> {
        if self.latencies_ms.is_empty() {
            return None;
        }
        let percent = usize::from(percent.clamp(1, 100));
        let mut sorted = self.latencies_ms.clone();
        sorted.sort_unstable();
        let rank = (percent * sorted.len()).div_ceil(100);
        Some(sorted[rank.saturating_sub(1)])
    }

    /// 当前快照。
    pub fn snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            requests: self.requests,
            errors: self.errors,
            success_rate: self.success_rate(),
            p95_ms: self.percentile_ms(95),
        }
    }
}

/// 打一条结构化日志：字段是键值对，日志系统可以按字段检索和告警。
fn log_request(request_id: u64, elapsed: Duration, ok: bool) {
    let elapsed_ms = elapsed.as_millis() as u64;
    if ok {
        info!(request_id, elapsed_ms, "请求完成");
    } else {
        warn!(request_id, elapsed_ms, "请求失败");
    }
}

/// 逐条插入：每条 `INSERT` 都在自己的隐式事务里，各自提交一次。
///
/// 用**文件库**而不是内存库：只有落到磁盘时，「提交一次」的代价
/// （写日志 + fsync）才显得出来，这正是事务在真实项目里的价值所在。
fn insert_one_by_one(path: &Path, count: usize) -> Duration {
    let conn = Connection::open(path).expect("打开数据库失败");
    init(&conn).expect("建表失败");
    let start = Instant::now();
    for index in 0..count {
        add_task(&conn, &format!("逐条-{index}")).expect("插入失败");
    }
    start.elapsed()
}

/// 一个事务插完：`BEGIN ... COMMIT` 只提交一次。
fn insert_in_transaction(path: &Path, count: usize) -> Duration {
    let mut conn = Connection::open(path).expect("打开数据库失败");
    init(&conn).expect("建表失败");
    let titles: Vec<String> = (0..count).map(|index| format!("批量-{index}")).collect();
    let refs: Vec<&str> = titles.iter().map(String::as_str).collect();
    let start = Instant::now();
    let ids = add_tasks_transactional(&mut conn, &refs).expect("批量插入失败");
    let elapsed = start.elapsed();
    assert_eq!(ids.len(), count);
    elapsed
}

/// 演示用的临时数据库路径（跑完删掉，不留垃圾文件）。
fn demo_database_path(tag: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("rust-learn-{tag}-{}.sqlite", std::process::id()))
}

/// 发布前的检查清单：和 CI 里跑的同一套命令。
pub const RELEASE_CHECKLIST: &[&str] = &[
    "cargo fmt --check",
    "cargo clippy --all-targets --all-features -- -D warnings",
    "cargo test --all-features --locked",
    "cargo build --release --locked",
    "cargo package --list",
];

/// 演示健康检查、指标、结构化日志与一次真实的性能对比。
pub fn ops_demo() {
    println!("\n========== rust28_ops: 部署、监控与性能 ==========");

    println!("\n--- 1. 健康检查：存活与就绪 ---");
    println!(
        "    /live  -> {:?}，HTTP {}（进程活着就返回 200，不查依赖）",
        liveness(),
        liveness().status_code()
    );
    for (database_ok, dependency_ok) in [(true, true), (true, false), (false, false)] {
        let health = health_status(database_ok, dependency_ok);
        println!(
            "    /ready -> 数据库可用={database_ok} 依赖可用={dependency_ok} => {:?}，HTTP {}，可接流量={}",
            health,
            health.status_code(),
            health.is_ready()
        );
    }
    println!("    （数据库挂了就返回 503，但不要重启进程——重启解决不了数据库问题）");

    println!("\n--- 2. 指标：成功率与 P95 延迟 ---");
    let mut metrics = Metrics::default();
    for (success, millis) in [
        (true, 12_u64),
        (true, 18),
        (true, 25),
        (true, 30),
        (true, 41),
        (true, 55),
        (true, 62),
        (true, 78),
        (true, 95),
        (false, 830),
    ] {
        metrics.record(success, Duration::from_millis(millis));
    }
    println!(
        "    计数：requests = {}，errors = {}，成功率 = {:.3}",
        metrics.requests(),
        metrics.errors(),
        metrics.success_rate()
    );
    println!("    快照：{}", metrics.snapshot());
    println!(
        "    P50 = {:?}ms，P95 = {:?}ms，P100 = {:?}ms",
        metrics.percentile_ms(50),
        metrics.percentile_ms(95),
        metrics.percentile_ms(100)
    );
    println!("    （均值会被 830ms 那次失败拉高；P95 说的是「绝大多数请求有多快」）");
    println!(
        "    空指标：{}（没有请求时成功率约定为 0，不产生 NaN）",
        Metrics::default().snapshot()
    );

    println!("\n--- 3. 结构化日志：字段化比拼接字符串好用 ---");
    let subscriber = fmt()
        .with_writer(std::io::stdout)
        .with_ansi(false)
        .with_target(false)
        .finish();
    tracing::subscriber::with_default(subscriber, || {
        log_request(1, Duration::from_millis(12), true);
        log_request(2, Duration::from_millis(830), false);
    });
    println!(
        "    （时间戳每次运行都不同；request_id / elapsed_ms 是字段，日志系统可以直接按它们检索）"
    );

    println!("\n--- 4. 性能：先用数据找热点 ---");
    const ROWS: usize = 500;
    let once_path = demo_database_path("ch28-once");
    let batch_path = demo_database_path("ch28-batch");
    let _ = std::fs::remove_file(&once_path);
    let _ = std::fs::remove_file(&batch_path);
    let one_by_one = insert_one_by_one(&once_path, ROWS);
    let batched = insert_in_transaction(&batch_path, ROWS);
    println!("    插入 {ROWS} 行到文件库（实测一次）:");
    println!(
        "      逐条提交   = {:.1} ms",
        one_by_one.as_secs_f64() * 1000.0
    );
    println!(
        "      单事务提交 = {:.1} ms",
        batched.as_secs_f64() * 1000.0
    );
    println!(
        "      倍数 = {:.0}x",
        one_by_one.as_secs_f64() / batched.as_secs_f64()
    );
    println!("    两边写进去的行数完全一样，差别只在提交次数：N 次落盘 vs 1 次落盘");
    println!("    （绝对数字每次运行都会变，值得看的是量级；正式对比请用第 23 章的 criterion）");
    let _ = std::fs::remove_file(&once_path);
    let _ = std::fs::remove_file(&batch_path);

    println!("\n--- 5. 发布检查清单 ---");
    for command in RELEASE_CHECKLIST {
        println!("    {command}");
    }

    println!("\n========== 部署、监控与性能演示结束 ==========");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn liveness_does_not_depend_on_dependencies() {
        assert_eq!(liveness(), Health::Live);
        assert_eq!(liveness().status_code(), 200);
        assert!(!liveness().is_ready(), "存活不等于就绪");
    }

    #[test]
    fn readiness_requires_every_dependency() {
        assert_eq!(health_status(true, true), Health::Ready);
        assert_eq!(health_status(true, true).status_code(), 200);
        assert!(health_status(true, true).is_ready());

        assert_eq!(health_status(true, false), Health::Degraded);
        assert_eq!(health_status(false, true), Health::Degraded);
        assert_eq!(health_status(false, false).status_code(), 503);
        assert!(!health_status(true, false).is_ready());
    }

    #[test]
    fn metrics_count_requests_and_errors() {
        let mut metrics = Metrics::default();
        metrics.record(true, Duration::from_millis(10));
        metrics.record(false, Duration::from_millis(20));

        assert_eq!(metrics.requests(), 2);
        assert_eq!(metrics.errors(), 1);
        assert_eq!(metrics.success_rate(), 0.5);
    }

    #[test]
    fn empty_metrics_do_not_divide_by_zero() {
        let metrics = Metrics::default();
        assert_eq!(metrics.success_rate(), 0.0);
        assert_eq!(metrics.percentile_ms(95), None);
        assert_eq!(
            metrics.snapshot(),
            MetricsSnapshot {
                requests: 0,
                errors: 0,
                success_rate: 0.0,
                p95_ms: None,
            }
        );
    }

    #[test]
    fn percentile_uses_the_nearest_rank() {
        let mut metrics = Metrics::default();
        for millis in [10_u64, 20, 30, 40, 50, 60, 70, 80, 90, 100] {
            metrics.record(true, Duration::from_millis(millis));
        }
        assert_eq!(metrics.percentile_ms(50), Some(50)); // ceil(0.5 × 10) = 5 -> 第 5 个
        assert_eq!(metrics.percentile_ms(95), Some(100)); // ceil(9.5) = 10 -> 最大值
        assert_eq!(metrics.percentile_ms(100), Some(100));
        assert_eq!(metrics.percentile_ms(1), Some(10));
        assert_eq!(metrics.percentile_ms(0), Some(10), "越界输入被夹到 1");
    }

    #[test]
    fn snapshot_aggregates_requests() {
        let mut metrics = Metrics::default();
        for millis in [10_u64, 20, 30, 40] {
            metrics.record(true, Duration::from_millis(millis));
        }
        metrics.record(false, Duration::from_millis(500));

        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.requests, 5);
        assert_eq!(snapshot.errors, 1);
        assert_eq!(snapshot.success_rate, 0.8);
        assert_eq!(snapshot.p95_ms, Some(500));
        assert_eq!(
            snapshot.to_string(),
            "requests=5 errors=1 success=80.0% p95=500ms"
        );
    }

    #[test]
    fn batch_insert_and_loop_insert_write_the_same_rows() {
        const COUNT: usize = 20;

        let looped = Connection::open_in_memory().unwrap();
        init(&looped).unwrap();
        for index in 0..COUNT {
            add_task(&looped, &format!("逐条-{index}")).unwrap();
        }

        let mut batched = Connection::open_in_memory().unwrap();
        init(&batched).unwrap();
        let titles: Vec<String> = (0..COUNT).map(|index| format!("批量-{index}")).collect();
        let refs: Vec<&str> = titles.iter().map(String::as_str).collect();
        add_tasks_transactional(&mut batched, &refs).unwrap();

        let looped_total = crate::rust26_database::stats(&looped).unwrap().total;
        let batched_total = crate::rust26_database::stats(&batched).unwrap().total;
        assert_eq!(looped_total, COUNT as i64);
        assert_eq!(batched_total, COUNT as i64);
    }

    #[test]
    fn both_insert_strategies_are_timed() {
        // 只为确认两条路径都能跑通并返回耗时（不做性能断言：CI 机器不可控）。
        let dir = tempfile::tempdir().unwrap();
        let once = dir.path().join("once.sqlite");
        let batch = dir.path().join("batch.sqlite");
        assert!(insert_one_by_one(&once, 5) > Duration::ZERO);
        assert!(insert_in_transaction(&batch, 5) > Duration::ZERO);

        // 两条路径写进去的行数必须一致
        let once_count = crate::rust26_database::stats(&Connection::open(&once).unwrap())
            .unwrap()
            .total;
        let batch_count = crate::rust26_database::stats(&Connection::open(&batch).unwrap())
            .unwrap()
            .total;
        assert_eq!((once_count, batch_count), (5, 5));
    }
}
