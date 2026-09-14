# 第 28 章 · 部署、监控与性能优化

代码写完、测试通过，只是「能跑」。上线之后要回答的是另外四个问题：

1. **它还在吗？** —— 存活探针；
2. **它能接流量吗？** —— 就绪探针（依赖挂了就不该接）；
3. **它快不快？** —— 延迟指标（均值会骗人，要看百分位）；
4. **它为什么慢？** —— 先测量，再优化。

这一章用标准库和仓库已有依赖把这几件事做出可运行的版本：

| 部分 | 位置 | 怎么跑 |
| --- | --- | --- |
| 健康检查、指标、日志、性能对比 | `src/rust28_ops.rs` | `cargo run`（最后一章） |
| 本章单元测试 | 同上文件末尾 | `cargo test --bin rust-learn-demo rust28_ops` |
| 基准（第 23 章） | `benches/benchmarks.rs` | `cargo bench` |
| 数据库查询计划（第 26 章） | `src/rust26_database/mod.rs` | 见 26.9 |

## 28.1 发布流程

```rust
/// 发布前的检查清单：和 CI 里跑的同一套命令。
pub const RELEASE_CHECKLIST: &[&str] = &[
    "cargo fmt --check",
    "cargo clippy --all-targets --all-features -- -D warnings",
    "cargo test --all-features --locked",
    "cargo build --release --locked",
    "cargo package --list",
];
```

把清单写成常量而不是散在文档里，有两个好处：演示会把它原样打印出来，
评审时也能直接对照 CI 配置（本仓库的 `.github/workflows/deploy.yml`）。

| 命令 | 拦住的错误 |
| --- | --- |
| `cargo fmt --check` | 格式没统一，评审里出现无意义的 diff |
| `cargo clippy … -D warnings` | 可疑写法：无用 `vec!`、`unwrap`、未标 `unsafe` 的裸指针解引用等 |
| `cargo test --all-features --locked` | 功能回归；`--locked` 保证依赖图没被悄悄改（第 24 章） |
| `cargo build --release --locked` | release profile 下才暴露的问题（优化、断言开关） |
| `cargo package --list` | 打进发布包的垃圾文件、缺失的 `license` 等元数据 |

> 现状说明：本仓库的三条命令现在都是通过的——早期章节留下的 lint 已逐条清理：
> 能改成惯用写法的就改（第 6、8、11、13、23 章），确实「故意这么写」的
> （第 15 章演示宏展开出 `Vec::new()` + `push`、第 18 章演示 regex 不支持的语法）
> 则用带说明的 `#[allow(...)]` **精确放行**，而不是把整条 lint 关掉。

本章的测试同样按契约写，实测输出：

```text
$ cargo test --bin rust-learn-demo rust28_ops
running 8 tests
test rust28_ops::tests::empty_metrics_do_not_divide_by_zero ... ok
test rust28_ops::tests::liveness_does_not_depend_on_dependencies ... ok
test rust28_ops::tests::percentile_uses_the_nearest_rank ... ok
test rust28_ops::tests::snapshot_aggregates_requests ... ok
test rust28_ops::tests::metrics_count_requests_and_errors ... ok
test rust28_ops::tests::readiness_requires_every_dependency ... ok
test rust28_ops::tests::batch_insert_and_loop_insert_write_the_same_rows ... ok
test rust28_ops::tests::both_insert_strategies_are_timed ... ok
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

整个仓库现在是「库目标 2 + 二进制目标 50 + 集成测试 6」，`cargo test` 全绿。
注意这里**没有对性能做断言**：CI 机器的负载不可控，耗时会波动；
能稳定断言的是「两条路径写进去的行数一致」这类事实。

## 28.2 健康检查：存活与就绪是两件事

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Health {
    Live,      // 进程活着
    Ready,     // 依赖都可用，可以接流量
    Degraded,  // 依赖不可用：别派流量，但不必重启
}

impl Health {
    pub fn status_code(self) -> u16 {
        match self {
            Health::Live | Health::Ready => 200,
            Health::Degraded => 503,
        }
    }
    pub fn is_ready(self) -> bool { matches!(self, Health::Ready) }
}

/// 存活探针：不做任何 IO，也不查依赖。
pub const fn liveness() -> Health { Health::Live }

/// 就绪判定：数据库和依赖都可用才算就绪。
pub fn health_status(database_ok: bool, dependency_ok: bool) -> Health {
    if database_ok && dependency_ok { Health::Ready } else { Health::Degraded }
}
```

实测输出：

```text
--- 1. 健康检查：存活与就绪 ---
    /live  -> Live，HTTP 200（进程活着就返回 200，不查依赖）
    /ready -> 数据库可用=true 依赖可用=true => Ready，HTTP 200，可接流量=true
    /ready -> 数据库可用=true 依赖可用=false => Degraded，HTTP 503，可接流量=false
    /ready -> 数据库可用=false 依赖可用=false => Degraded，HTTP 503，可接流量=false
```

| 探针 | 问的问题 | 失败时的正确反应 | 检查什么 |
| --- | --- | --- | --- |
| 存活 `/live` | 进程还在转吗？ | **重启**容器 | 什么都不查（查了就说明语义错了） |
| 就绪 `/ready` | 能接流量吗？ | 从负载均衡里摘掉，**不重启** | 数据库连通、迁移完成、必要的下游 |

两条容易写反的地方：

- **存活探针里查数据库**：数据库抖一下，容器就被反复重启，故障反而扩散；
- **就绪探针没有超时**：依赖卡住时探针自己卡死，编排系统只能靠更长的超时兜底，
  恢复时间被拉长。探针必须快速返回（通常几百毫秒级）。

## 28.3 指标：请求数、错误率和延迟百分位

```rust
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Metrics {
    requests: u64,
    errors: u64,
    latencies_ms: Vec<u64>,
}

impl Metrics {
    pub fn record(&mut self, success: bool, elapsed: Duration) {
        self.requests += 1;
        self.errors += u64::from(!success);
        self.latencies_ms.push(elapsed.as_millis() as u64);
    }

    /// 成功率；没有任何请求时返回 0.0（而不是除零或 NaN）。
    pub fn success_rate(&self) -> f64 {
        if self.requests == 0 { return 0.0; }
        (self.requests - self.errors) as f64 / self.requests as f64
    }

    /// 最近秩（nearest-rank）百分位：排序后取第 ceil(p/100 × n) 个值。
    pub fn percentile_ms(&self, percent: u8) -> Option<u64> {
        if self.latencies_ms.is_empty() { return None; }
        let percent = usize::from(percent.clamp(1, 100));
        let mut sorted = self.latencies_ms.clone();
        sorted.sort_unstable();
        Some(sorted[(percent * sorted.len()).div_ceil(100).saturating_sub(1)])
    }
}
```

演示里记录 10 次请求（其中 1 次失败，耗时 830ms），实测输出：

```text
--- 2. 指标：成功率与 P95 延迟 ---
    计数：requests = 10，errors = 1，成功率 = 0.900
    快照：requests=10 errors=1 success=90.0% p95=830ms
    P50 = Some(41)ms，P95 = Some(830)ms，P100 = Some(830)ms
    （均值会被 830ms 那次失败拉高；P95 说的是「绝大多数请求有多快」）
    空指标：requests=0 errors=0 success=0.0% p95=无样本（没有请求时成功率约定为 0，不产生 NaN）
```

为什么盯百分位而不是均值：

| 指标 | 回答的问题 | 陷阱 |
| --- | --- | --- |
| 平均值 | 「整体大概多快」 | 少数极慢请求就能拉高；掩盖双峰分布 |
| P50 | 「典型用户体验」 | 看不到尾部 |
| P95 / P99 | 「最慢的那批有多慢」 | 需要足够样本，否则抖动大 |
| 最大值 | 「最坏情况」 | 单个异常值就代表整体，容易过拟合 |

「最近秩」是百分位算法里最简单的一种：不插值，结果**一定来自真实样本**。
上例中 `ceil(0.95 × 10) = 10`，所以 P95 就是最大值 830ms；
`ceil(0.5 × 10) = 5`，所以 P50 是排序后的第 5 个值 41ms。

几个实现上的坑：

- **零请求时不要产生 `NaN`**：`success_rate` 约定返回 0.0，否则告警规则会被
  `NaN` 的比较结果搞乱；
- **生产系统通常用直方图**（例如 Prometheus histogram）按桶累计，避免保存每个
  样本；代价是精度受桶边界限制，两者要权衡；
- **标签基数要可控**：把 URL、用户 ID 直接当标签会让时间序列爆炸（第 28.4 节同理）。

## 28.4 结构化日志：字段化，而不是拼字符串

```rust
use tracing::{info, warn};
use tracing_subscriber::fmt;

fn log_request(request_id: u64, elapsed: Duration, ok: bool) {
    let elapsed_ms = elapsed.as_millis() as u64;
    if ok {
        info!(request_id, elapsed_ms, "请求完成");
    } else {
        warn!(request_id, elapsed_ms, "请求失败");
    }
}
```

演示里临时挂上一个 subscriber（`with_default` 只在闭包内生效，不影响测试）：

```rust
let subscriber = fmt()
    .with_writer(std::io::stdout)
    .with_ansi(false)
    .with_target(false)
    .finish();
tracing::subscriber::with_default(subscriber, || {
    log_request(1, Duration::from_millis(12), true);
    log_request(2, Duration::from_millis(830), false);
});
```

实测输出（时间戳每次运行都不同）：

```text
--- 3. 结构化日志：字段化比拼接字符串好用 ---
2026-09-14T00:44:21.932033Z  INFO 请求完成 request_id=1 elapsed_ms=12
2026-09-14T00:44:21.932062Z  WARN 请求失败 request_id=2 elapsed_ms=830
```

`request_id=1 elapsed_ms=12` 是**字段**，不是字符串的一部分：

| | 拼接字符串 | 结构化字段 |
| --- | --- | --- |
| 检索 | 正则或全文模糊匹配 | 按字段精确过滤 / 聚合 |
| 告警 | 很难写稳定的规则 | `elapsed_ms > 500` 直接可用 |
| 变更 | 改格式就悄悄破坏下游解析 | 字段名是契约，加成新字段不影响旧消费者 |

几条实践经验：

- **日志走 stderr，数据走 stdout**（第 21 章）：不然管道一接就混在一起；
- **必须有请求/任务 ID**：跨服务的排查全靠它把一条链路上的日志串起来；
- **写入前脱敏**：手机号、token、密码不要进日志，日志往往比数据库更"长寿"；
- **日志级别要能按需打开**：正常路径 `INFO`，排查细节用 `DEBUG`，
  不要把调试信息长期留在 `INFO` 里；
- **别在每行日志里打全量 payload**：日志成本和检索噪声都会失控。

## 28.5 性能：先用数据找到热点

「这个写法更快」不是结论，测出来才算。演示对比同一个任务的两条实现路径
——插入 500 行到**文件库**：

```rust
/// 逐条插入：每条 INSERT 都在自己的隐式事务里，各自提交一次。
fn insert_one_by_one(path: &Path, count: usize) -> Duration {
    let conn = Connection::open(path)?;
    init(&conn)?;
    let start = Instant::now();
    for index in 0..count {
        add_task(&conn, &format!("逐条-{index}"))?;
    }
    start.elapsed()
}

/// 一个事务插完：BEGIN ... COMMIT 只提交一次。
fn insert_in_transaction(path: &Path, count: usize) -> Duration {
    let mut conn = Connection::open(path)?;
    init(&conn)?;
    let titles: Vec<String> = (0..count).map(|index| format!("批量-{index}")).collect();
    let refs: Vec<&str> = titles.iter().map(String::as_str).collect();
    let start = Instant::now();
    add_tasks_transactional(&mut conn, &refs)?;   // 第 26 章的实现
    start.elapsed()
}
```

实测输出（同一个演示连跑两次，数字会跳）：

```text
--- 4. 性能：先用数据找热点 ---
    插入 500 行到文件库（实测一次）:
      逐条提交   = 1043.9 ms
      单事务提交 = 4.9 ms
      倍数 = 215x
    两边写进去的行数完全一样，差别只在提交次数：N 次落盘 vs 1 次落盘
    （绝对数字每次运行都会变，值得看的是量级；正式对比请用第 23 章的 criterion）
```

同一台机器上的另一次运行是「逐条 1051.8 ms / 单事务 25.8 ms = 41 倍」。
**绝对值和倍数都会变**（磁盘缓存、后台负载、杀毒软件都会影响），稳定的是
那个结论：逐条提交比单事务提交慢一到两个数量级。这也是为什么「跑一次就下结论」
不靠谱——正式对比要做多次采样、看置信区间（第 23 章）。

为什么差这么多：SQLite 的默认日志模式（`journal_mode=DELETE`、`synchronous=FULL`）
下，**每次提交都要把数据刷到磁盘并等待**。逐条插入 = 500 次落盘等待；
单事务 = 1 次。差距来自系统调用和磁盘延迟，不是「Rust 写得快不快」。

> 本节最初用**内存库**做同样的对比，结果只有 1.7 倍（实测 4.04ms vs 2.41ms）——
> 因为内存库根本没有落盘，提交几乎是白送的。换成文件库之后差距才真实显现。
> 这说明一件事：**基准环境选错了，结论就会错**；优化线上代码时，基准要尽量
> 贴近真实部署形态（文件、网络、并发、数据量）。

优化的正确顺序：

| 步骤 | 做什么 | 本章对应 |
| --- | --- | --- |
| 1. 测量 | 找到「哪一步慢」，别猜 | 本节 `Instant` 计时；正式项目用 criterion + profiler |
| 2. 定位 | 是 CPU、分配、锁、IO 还是数据库提交次数？ | 本例是提交次数 |
| 3. 改动 | 一次只改一个因素 | 把 N 次提交压成 1 次 |
| 4. 复测 | 用同一套基准对比，记录基线 | `--save-baseline` / `--baseline`（第 23 章） |
| 5. 守住 | 关键路径进 CI 防回归 | 阈值告警 |

几条经验：**先写对再写快**；**看数量级，不看小数点**（快 41 倍值得改，快 5% 通常不值得
让代码变复杂）；**优化要有终点**（达到目标就停）；**别拿微基准预测整个系统**。

## 28.6 数据库相关的运维动作

第 26 章讲了 SQL 层面的事情，这里补上「线上怎么操作」。备份与恢复（命令形态，
按自己的路径替换）：

```bash
sqlite3 tasks.sqlite ".backup 'backup-2026-09-14.sqlite'"   # 热备份，比直接复制文件安全
sqlite3 backup-2026-09-14.sqlite "PRAGMA integrity_check;"   # 校验备份可读
```

上线前的几条约定：

| 事项 | 做法 |
| --- | --- |
| 迁移何时跑 | 启动时执行（本仓库 `TaskService::open_file` 就是），跑完再打开就绪 |
| 迁移失败 | 进程启动失败并报错，不要带着旧 schema 接流量 |
| 回滚 | 迁移只追加、不改历史；数据结构变更要能和新旧两版代码共存 |
| 并发 | 每线程一条连接 + `busy_timeout`，写操作串行化，事务保持短小（第 26 章 26.10） |
| 备份 | 用 `.backup` 或 `VACUUM INTO`，别在写入过程中直接拷文件 |
| 磁盘 | 监控数据库文件与 WAL 文件大小，磁盘满是最常见的"神秘故障" |

## 28.7 部署清单

| 类别 | 检查项 |
| --- | --- |
| 构建 | 固定工具链版本；`Cargo.lock` 提交并 `--locked` 构建；release 产物在 CI 里产出 |
| 配置 | 全部通过环境变量或挂载文件注入（第 27 章）；配置错误启动即失败 |
| 数据 | 启动时迁移；备份与恢复演练过；磁盘空间有告警 |
| 可观测性 | `/live` 与 `/ready`；请求数、错误率、P95/P99；结构化日志带请求 ID |
| 安全 | 日志脱敏；数据库文件权限最小化；不对公网暴露管理端口 |
| 发布 | 灰度或滚动发布；有明确回滚步骤；版本号不可复用（第 24 章） |
| 故障 | 明确「重启能解决什么」与「重启解决不了什么」（比如数据库故障） |

## 28.8 常见坑速查

| 现象 | 原因 | 解法 |
| --- | --- | --- |
| 数据库抖一下容器就被重启 | 存活探针里查了数据库 | 存活只回答「进程在不在」，依赖检查放就绪探针 |
| 就绪探针把编排系统拖死 | 探针没有超时 | 探针内部设几百毫秒级的超时，失败立即返回 |
| 报警一会儿好一会儿坏 | 用平均值或最大值判断 | 用 P95/P99，并规定最小样本量 |
| 零请求时告警乱跳 | `success_rate` 是 `NaN` | 约定「无请求 = 0」，或对 `NaN` 单独处理 |
| 时间序列数量爆炸 | 把 URL / 用户 ID 当指标标签 | 标签只放低基数的维度（方法、状态码、路由模板） |
| 日志搜不到关键信息 | 用 `format!` 拼字符串 | 用结构化字段，按字段检索与告警 |
| 日志里出现密码或 token | 没有脱敏 | 在写入点统一脱敏，别指望事后清理 |
| 批量导入慢得离谱 | 每行一次提交 | 放进一个事务里提交一次 |
| 基准结果和线上差别巨大 | 用了内存库或空表做基准 | 基准环境贴近生产（文件、数据量、并发） |
| 优化了半天没有效果 | 没找到真正的瓶颈 | 先 profile 再改；一次只改一个因素 |
| `cargo build --release` 行为和调试版不同 | 断言与溢出检查的开关变了 | 关键不变量用 `Result` 表达，不靠 `debug_assert!` 兜底 |

## 28.9 练习

1. 给 `Metrics` 加一个 `error_rate()`，并处理「零请求」；再为它写两条测试。
2. 把 `liveness` / `health_status` 包成两个 HTTP 端点（提示：第 22 章的
   `TcpListener` + 手写响应），用 `curl` 或 `reqwest` 验证 200 / 503。
3. 给 `insert_in_transaction` 加一个参数化的 `batch_size`：每 N 行提交一次，
   测量 N = 1 / 100 / 1000 的耗时曲线，并解释为什么「越大越快」有上限。
4. 为 `list_tasks` 写一个 criterion 基准（第 23 章），比较「有索引」和
   「删掉索引」两种情况下的查询耗时，再用 `EXPLAIN QUERY PLAN` 印证结论。
5. 写一份发布 runbook：包含构建命令、配置注入、迁移与回滚、备份恢复、
   健康检查验证、失败时的处置步骤。
6. 回答两个问题：(a) 为什么存活探针不该检查依赖？
   (b) 为什么性能对比要用文件库而不是内存库？

（第 6 题是 28.2 与 28.5 讨论过的内容，这里不再单独给答案。）

### 参考答案

::: details 第 1 题

```rust
/// 错误率；没有请求时返回 0.0。
pub fn error_rate(&self) -> f64 {
    if self.requests == 0 {
        return 0.0;
    }
    self.errors as f64 / self.requests as f64
}

#[test]
fn error_rate_handles_empty_and_mixed_traffic() {
    assert_eq!(Metrics::default().error_rate(), 0.0);

    let mut metrics = Metrics::default();
    metrics.record(true, Duration::from_millis(1));
    metrics.record(true, Duration::from_millis(1));
    metrics.record(false, Duration::from_millis(1));
    metrics.record(false, Duration::from_millis(1));
    assert_eq!(metrics.error_rate(), 0.5);
    // 成功率与错误率互补
    assert!((metrics.success_rate() + metrics.error_rate() - 1.0).abs() < f64::EPSILON);
}
```

注意最后一行用的是容差比较而不是 `==`：浮点相加可能有微小误差（第 23 章
23.5 讲过），测试里踩这个坑非常常见。

:::

::: details 第 2 题

```rust
fn handle(path: &str, database_ok: bool) -> (&'static str, u16) {
    match path {
        "/live" => ("ok", liveness().status_code()),
        "/ready" => {
            let health = health_status(database_ok, true);
            (if health.is_ready() { "ready" } else { "not ready" }, health.status_code())
        }
        _ => ("not found", 404),
    }
}

#[test]
fn probes_map_to_status_codes() {
    assert_eq!(handle("/live", false), ("ok", 200));   // 依赖挂了也照样 200
    assert_eq!(handle("/ready", true), ("ready", 200));
    assert_eq!(handle("/ready", false), ("not ready", 503));
}
```

真正的 HTTP 端点可以照第 22 章的做法：`TcpListener::bind("127.0.0.1:0")`、
读请求行、回一条带 `Content-Length` 的响应。注意**健康端点也要限流/限来源**，
不要把它们暴露给公网当"探测免费接口"。

:::

::: details 第 3 题

```rust
fn insert_in_batches(path: &Path, count: usize, batch_size: usize) -> Duration {
    let mut conn = Connection::open(path).expect("打开数据库失败");
    init(&conn).expect("建表失败");
    let start = Instant::now();
    for chunk_start in (0..count).step_by(batch_size) {
        let end = (chunk_start + batch_size).min(count);
        let titles: Vec<String> = (chunk_start..end).map(|i| format!("批量-{i}")).collect();
        let refs: Vec<&str> = titles.iter().map(String::as_str).collect();
        add_tasks_transactional(&mut conn, &refs).expect("批量插入失败");
    }
    start.elapsed()
}
```

本机实测的大致规律是「batch_size 越大越快，但边际收益递减」：50 → 500 行的
提升明显，500 → 5000 行就接近平了，因为提交次数已经少到不再是瓶颈，
剩下的是真正的写入开销。**上限在于用户可接受的延迟和内存占用**：
一个事务持锁越久，其他写者越容易超时（第 26 章 26.10 的 `busy_timeout`）。

:::

::: details 第 5 题

一份能直接用的 runbook 骨架：

```text
1. 构建
   cargo fmt --check && cargo clippy --all-targets --all-features -- -D warnings
   cargo test --all-features --locked
   cargo build --release --locked
2. 备份
   sqlite3 <db> ".backup '<db>-$(date +%F-%H%M).sqlite'"
   sqlite3 <backup> "PRAGMA integrity_check;"
3. 部署
   上线新版本 → 启动时自动迁移 → 等 /ready 返回 200 → 再切流量
4. 验证
   curl -s -o /dev/null -w '%{http_code}' http://<host>/live    # 期望 200
   curl -s -o /dev/null -w '%{http_code}' http://<host>/ready   # 期望 200
   观察错误率与 P95（至少一个完整业务周期）
5. 回滚
   切回上一个版本 → 若迁移不兼容，用第 2 步的备份恢复 → 再次确认 /ready
6. 事后
   记录时间线、影响范围、触发原因、修复项，并补一条回归测试
```

第 6 步经常被忽略，但它才是让同类故障不再重复的关键。

:::

## 28.10 小结

- 发布流程固定成五条命令（`fmt` / `clippy` / `test --locked` /
  `build --release --locked` / `package --list`），并原样写进 CI。
- 健康检查分两种：`/live` 只回答「进程在不在」（失败就重启），
  `/ready` 回答「能不能接流量」（失败就摘流量，不重启），前者绝不能查依赖。
- 指标盯四样：请求数、错误率、P50、P95/P99。均值会被少数慢请求带偏；
  零请求时不要产生 `NaN`。
- 百分位用「最近秩」实现最简单，结果一定来自真实样本；生产上常用直方图，
  代价是精度受桶边界限制。
- 日志要字段化（`request_id=1 elapsed_ms=12`），写 stderr、脱敏、控制基数；
  这样才谈得上检索和告警。
- 性能优化只有一条正确顺序：测量 → 定位 → 改一个因素 → 复测 → 防回归。
  本仓库实测「500 行逐条提交 vs 单事务提交」是 1043.9ms vs 4.9ms（同一演示另一次
  是 1051.8ms vs 25.8ms）——**数字会跳，但「差一到两个数量级、差距全来自提交与
  落盘次数」这个结论稳定**。
- 同样的对比在内存库上只有 1.7 倍——**基准环境选错，结论就会错**。
- 数据库运维的底线是：迁移在启动时跑、只追加不改历史、备份用 `.backup`、
  写事务尽量短、磁盘与 WAL 有监控。

### 这本书到这里就结束了

从第 1 章的 `println!` 一路走到这里，你已经覆盖了：语言基础（1–13）、工程结构
与测试（14、23）、标准库与生态（16–22）、交付相关的 Cargo / FFI / 数据库
（24–26），以及一个串起全部的综合项目与上线实践（27–28）。

接下来值得投入的方向：

| 方向 | 建议的下一步 |
| --- | --- |
| 把知识变成手感 | 挑一个自己的小需求，从 `cargo new` 开始做完整项目 |
| 读源码 | 从 `std` 的 `Vec` / `HashMap` 文档与源码看起，再看一个你常用的 crate |
| 异步与网络 | 用 `tokio` + `axum` 把第 27 章的服务改成 HTTP 版 |
| 工程质量 | 给项目补 CI（fmt / clippy / test / release）、发布流程与变更日志 |
| 性能 | 用 criterion 建立基线，用 `perf` / `flamegraph` 找热点 |
| 更深的安全 | 跑 Miri、写 fuzz 测试、给 FFI 补 C 头文件与跨语言测试 |

遇到问题时，回到这本书的做法：**先跑一遍，再看输出，最后才下结论**。
