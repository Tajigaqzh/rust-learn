//! 第 28 章：部署、监控与性能优化。
#[derive(Debug, PartialEq, Eq)]
pub enum HealthStatus {
    Healthy,
    Degraded,
}

pub fn health_status(database_ok: bool, dependency_ok: bool) -> HealthStatus {
    if database_ok && dependency_ok {
        HealthStatus::Healthy
    } else {
        HealthStatus::Degraded
    }
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Metrics {
    pub requests: u64,
    pub errors: u64,
}

impl Metrics {
    pub fn record(&mut self, success: bool) {
        self.requests += 1;
        self.errors += u64::from(!success);
    }
}

pub fn ops_demo() {
    println!("\n========== rust28_ops: 部署、监控与性能 ==========");
    println!("    发布：cargo build --release --locked");
    println!(
        "    检查：cargo test --all-features --locked && cargo clippy --all-targets -- -D warnings"
    );
    println!("    监控：结构化日志、健康检查、请求耗时与错误率指标");
    println!("    健康：{:?}", health_status(true, true));
    let mut metrics = Metrics::default();
    metrics.record(true);
    metrics.record(false);
    println!(
        "    指标快照：请求 {}，错误 {}",
        metrics.requests, metrics.errors
    );
    println!("    性能：先基准测试，再优化热点；避免无依据的微优化");
    println!("========== 部署、监控与性能演示结束 ==========");
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn health_distinguishes_dependency_failure() {
        assert_eq!(health_status(true, false), HealthStatus::Degraded);
    }
    #[test]
    fn metrics_count_requests_and_errors() {
        let mut metrics = Metrics::default();
        metrics.record(true);
        metrics.record(false);
        assert_eq!(
            metrics,
            Metrics {
                requests: 2,
                errors: 1
            }
        );
    }
}
