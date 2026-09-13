# 第 28 章 · 部署、监控与性能优化

## 28.1 发布流程

使用 `cargo fmt --check`、`cargo clippy --all-targets --all-features -- -D warnings`、`cargo test --all-features --locked`，最后执行 `cargo build --release --locked`。发布包前用 `cargo package --list` 检查内容。

## 28.2 监控

配套代码提供 `health_status` 健康状态判定和 `Metrics` 请求/错误计数器，实际服务可将它们接到 HTTP 健康端点和指标导出器。

日志应包含时间、级别、请求/任务 ID 和错误上下文；指标至少关注吞吐量、延迟（P95/P99）、错误率和数据库连接/锁等待。提供轻量健康检查，区分“进程存活”和“依赖可用”。

## 28.3 性能优化

先用 Criterion 或真实压测定位热点，再选择缓存、批量 SQL、索引或并发。数据库查询应检查 `EXPLAIN QUERY PLAN`，不要只凭直觉添加索引。优化后记录基线和回归结果。

## 28.4 部署清单

- 固定 Rust 工具链和 `Cargo.lock`。
- 配置通过环境变量或挂载文件注入，不写入二进制。
- 明确 SQLite 文件备份、权限和迁移回滚策略。
- 生产日志脱敏，崩溃和超时具备告警。

## 28.5 健康检查的两种语义

代码中的 `health_status(database_ok, dependency_ok)` 用两个输入区分进程存活和流量就绪。实际 HTTP 服务可以提供 `/live`（只要进程事件循环还在就返回 200）和 `/ready`（数据库、配置中心等依赖可用才返回 200）。就绪检查应设置超时，不能因为依赖故障把探针本身卡死。

## 28.6 指标快照与日志

`Metrics` 记录请求总数和错误数，适合作为最小测试替身：

```rust
let mut metrics = Metrics::default();
metrics.record(true);
metrics.record(false);
assert_eq!((metrics.requests, metrics.errors), (2, 1));
```

生产系统还需要延迟直方图、P95/P99、数据库锁等待和重试次数。指标标签不能直接使用无限基数的 URL 或用户 ID。日志应使用结构化字段记录请求 ID、耗时和错误上下文，并在写入前脱敏。

## 28.7 性能回归流程

先用 Criterion 或压测建立基线，再用 profiler 定位 CPU、分配、锁或 IO 热点。每次只改变一个因素，把吞吐量、延迟和内存结果记录到版本库。数据库查询用 `EXPLAIN QUERY PLAN` 确认索引是否生效；索引会增加写入成本，不应凭直觉添加。

## 28.8 发布检查与练习

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features --locked
cargo build --release --locked
```

练习：为 `Metrics` 增加成功率方法并处理零请求；为任务服务实现 `/live` 和 `/ready`；为 `list_tasks` 增加 Criterion 基准并比较索引前后的查询计划；写一份包含备份、迁移和回滚命令的发布 runbook。
