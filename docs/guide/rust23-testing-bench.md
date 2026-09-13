# 第 23 章 · 测试进阶与基准

第 14 章讲了「测试怎么写」：`#[cfg(test)]`、断言宏、集成测试、文档测试。这一章往前走三步：

1. **怎么组织**：多层测试、共享夹具、测试隔离；
2. **怎么让代码可测**：依赖注入——把「时间、随机、网络、文件」这类外部依赖抽出来；
3. **怎么测量性能**：用 `criterion` 写基准，读得懂结果。

配套代码分三部分，都在本仓库里真实可跑：

| 部分 | 位置 | 怎么跑 |
| --- | --- | --- |
| 单元测试 + 示范模块 | `src/rust23_testing/mod.rs` | `cargo test` |
| 共享夹具 + 集成测试 | `tests/common/mod.rs`、`tests/fixtures.rs` | `cargo test` |
| 基准 | `benches/benchmarks.rs` | `cargo bench` |

基准需要一个新的开发依赖：

```toml
[dev-dependencies]
criterion = "0.8"
zerocopy = "=0.8.55"      # 把 criterion 的传递依赖钉在确定版本，保证离线/CI 可复现

[[bench]]
name = "benchmarks"
harness = false           # 用 criterion 自己的入口，而不是内置的 bench 框架
```

## 23.1 三层测试各管什么

| 层次 | 位置 | 测什么 | 本仓库现状 |
| --- | --- | --- | --- |
| 单元测试 | 被测模块里的 `#[cfg(test)] mod tests` | 单个函数、私有逻辑、边界条件 | 13 个 |
| 集成测试 | `tests/*.rs` | 多个部分配合、对外行为 | 6 个（3 个夹具测试 + 3 个二进制冒烟测试） |
| 文档测试 | `///` 里的代码块 | 示例能不能跑（只对库目标生效） | 本项目是二进制 crate，暂无 |

执行 `cargo test` 的实测输出（节选）：

```text
running 13 tests
test rust14_modules_tests::geometry::tests::private_helper_is_reachable_inside_module ... ok
test rust23_testing::tests::uptime_label_covers_all_branches ... ok
test rust23_testing::tests::floats_compare_with_tolerance ... ok
test rust23_testing::tests::matches_macro_is_good_for_shapes ... ok
test rust23_testing::tests::division_by_zero_panics - should panic ... ok
test rust23_testing::tests::tests_can_return_result ... ok
test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

running 3 tests
test counts_chars_in_fixture ... ok
test fixtures_are_isolated_between_tests ... ok
test fixture_directories_are_cleaned_up ... ok
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

running 3 tests
test binary_prints_error_chapter_marker ... ok
test binary_reports_finish_markers ... ok
test binary_runs_and_prints_every_chapter ... ok
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

选择原则：**能写成单元测试的就写成单元测试**（快、定位准、能测私有函数）；只有需要验证「多个部分串起来的行为」时才写集成测试。

## 23.2 共享夹具：`tests/common/mod.rs`

集成测试之间常常要共用辅助代码。把公共部分放进 `tests/` 的**子目录**：

```text
tests/
├── common/
│   └── mod.rs        公共辅助代码（不会被执行器当成测试）
├── fixtures.rs       mod common; 引入上面那份
└── integration.rs
```

```rust
// tests/common/mod.rs
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use walkdir::WalkDir;

/// 建一个临时目录，把 (相对路径, 内容) 列表写进去。
pub fn make_fixture(files: &[(&str, &str)]) -> (TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().to_path_buf();
    for (relative, content) in files {
        let path = root.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&path, content).unwrap();
    }
    (dir, root)
}
```

**为什么是 `tests/common/mod.rs`，不是 `tests/common.rs`？** 因为 `tests/` 下每个 `.rs` 文件都会被 Cargo 当作**独立的测试 crate** 去编译运行。写成 `common.rs` 的话，它会变成一个「没有测试的测试目标」，还会浪费一次编译；放进子目录就不会。

（第 19 章学的 `tempfile` 在这里派上大用场：**夹具写在临时目录里，测试并行跑也互不干扰，跑完自动清理**。）

## 23.3 依赖注入：让「不可测」的代码变得可测

有些代码天生难测——**依赖当前时间、随机数、网络、文件系统**。直接调用它们，测试就不可复现：

```rust
// 难测：每次都依赖「现在」
fn uptime_label_system() -> String {
    let elapsed = std::time::SystemTime::now();
    // ... 直接读真实时间，结果不可复现
}
```

办法是把依赖**抽成 trait，从外面传进来**：

```rust
use std::time::Instant;

trait ElapsedClock {
    fn elapsed_millis(&self) -> u64;
}

struct SystemClock {
    start: Instant,
}

impl ElapsedClock for SystemClock {
    fn elapsed_millis(&self) -> u64 { self.start.elapsed().as_millis() as u64 }
}

struct FakeClock { millis: u64 }                 // 测试用
impl ElapsedClock for FakeClock {
    fn elapsed_millis(&self) -> u64 { self.millis }
}

fn uptime_label(clock: &impl ElapsedClock) -> String {
    let millis = clock.elapsed_millis();
    if millis < 1_000 { format!("{millis} 毫秒") }
    else if millis < 60_000 { format!("{} 秒", millis / 1000) }
    else { format!("{} 分钟", millis / 60_000) }
}
```

实测演示输出：

```text
    真实时钟：算完 19999900000 用了 0 毫秒（每次运行可能不同）
    uptime_label(&FakeClock { millis: 5_000 }) = 5 秒
    uptime_label(&FakeClock { millis: 3_600_000 }) = 60 分钟
```

第一行随运行时间变化（可能是 0 毫秒、1 毫秒……）；后面两行**永远是 5 秒和 60 分钟**——这就是可测性。测试里可以一次覆盖所有分支：

```rust
#[test]
fn uptime_label_covers_all_branches() {
    let cases = [(500u64, "500 毫秒"), (1_000, "1 秒"), (59_999, "59 秒"), (60_000, "1 分钟")];
    for (millis, expected) in cases {
        assert_eq!(uptime_label(&FakeClock { millis }), expected, "输入 {millis} 毫秒时");
    }
}
```

这套思路叫**依赖注入**，也是「mock」的正统做法：**不需要 mock 框架，只需要一个 trait 和一个假实现**。真实的 `SystemClock` 只写一遍，测试里全用 `FakeClock`。

同样的套路适用于：随机数（注入 `Random` trait）、网络（注入 `HttpClient` trait）、文件系统（注入根目录路径）。

## 23.4 测试隔离：临时目录与「清理」也要测

测试最容易被忽视的要求是**隔离**：任何一个测试都不应该依赖别的测试留下的状态，也不该污染下一次运行。

```rust
#[test]
fn counts_chars_in_fixture() {
    let (_dir, root) = make_fixture(&[("a.txt", "abc"), ("sub/b.txt", "中文")]);
    assert_eq!(total_chars(&root), 5);      // 3 + 2
}

#[test]
fn fixtures_are_isolated_between_tests() {
    let (_dir, root) = make_fixture(&[("only.txt", "x")]);
    assert_eq!(total_chars(&root), 1);      // 不受上一个测试影响
}
```

`make_fixture` 返回的 `TempDir` 是**清理的关键**：它一被 `drop`，整个目录递归删除。所以测试里要把它绑定到一个变量（哪怕用 `_dir` 忽略名字），**只要别让它在语句结束时就被丢掉**。

顺手还能为「清理」本身写个测试：

```rust
#[test]
fn fixture_directories_are_cleaned_up() {
    let (dir, root) = make_fixture(&[("temp.txt", "内容")]);
    let path = root.clone();
    assert!(path.exists());

    drop(dir);                              // 模拟测试结束
    assert!(!path.exists(), "TempDir 被 drop 后目录应当被删除");
}
```

实测这个测试通过——**「资源最终会被清理」这件事本身也是可以断言的**。

## 23.5 断言的技巧：表达意图，而不是复述实现

**① 浮点必须用容差。** 第 2 章讲过 `0.1 + 0.2 != 0.3`：

```rust
#[test]
fn floats_compare_with_tolerance() {
    assert_ne!(0.1 + 0.2, 0.3);              // 直接比较确实不相等
    assert!(approx_eq(0.1 + 0.2, 0.3, 1e-9)); // 容差内算相等
    assert!(!approx_eq(0.1 + 0.2, 0.3, 1e-18));
}

fn approx_eq(left: f64, right: f64, epsilon: f64) -> bool {
    (left - right).abs() < epsilon
}
```

注意 `f64::NAN` 连自己都不等于自己，所以 `approx_eq(NAN, NAN, ...)` 也是 `false`——如果业务上要把 `NaN` 当相等，得单独处理。

**② 用 `matches!` 断言「形态」**，比比较整个值更聚焦、失败信息也更清楚：

```rust
let value = Some(42);
assert!(matches!(value, Some(n) if n > 40));
assert!(!matches!(value, None));
```

**③ 给 `assert_eq!` 加消息**，失败时能立刻知道是哪个输入出问题：

```rust
assert_eq!(uptime_label(&FakeClock { millis }), expected, "输入 {millis} 毫秒时");
```

在循环里跑多组用例时，这条消息能省掉一次「到底是哪组失败」的排查。

## 23.6 测试也可以返回 `Result`

测试体里要做几步可能失败的操作时，返回 `Result` 比到处 `unwrap` 更清楚：

```rust
#[test]
fn tests_can_return_result() -> Result<(), Box<dyn std::error::Error>> {
    let parsed: i32 = "42".parse()?;                 // 失败就是测试失败，并打印错误
    assert_eq!(parsed, 42);

    let date = chrono::NaiveDate::parse_from_str("2026-09-13", "%Y-%m-%d")?;
    assert_eq!(date.to_string(), "2026-09-13");
    Ok(())
}
```

实测通过。好处是**失败信息里带的是真正的错误原因**（比如 parse 失败的具体位置），而不是一句 `called Result::unwrap() on an Err value`。

## 23.7 测试的反模式

| 反模式 | 为什么不好 | 更好的做法 |
| --- | --- | --- |
| 测实现细节（私有函数被反复绕过） | 重构一下就一堆测试失败 | 测行为，私有逻辑用单元测试覆盖 |
| 断言写成「实现的一次翻译」 | 测试和代码一起错 | 断言业务上能解释的结果 |
| 依赖外部状态（真实网络、当前时间、固定文件路径） | 结果不可复现、CI 上随机挂 | 依赖注入 + 临时目录 |
| 测试之间共享可变状态 | 并行执行时互相干扰 | 每个测试自己建夹具 |
| 只测「顺利路径」 | 边界和错误分支没人管 | 空输入、越界、非法格式、重复调用都要有 |
| 用 `assert!(result.is_ok())` 而不看值 | 错了也不知道错在哪 | `assert_eq!(result, Ok(42))` |
| 一个测试断言几十件事 | 失败时定位困难 | 拆成多个聚焦的测试 |

最后一条在真实项目里特别常见：**一个测试只验证一件事**，失败了直接就知道哪里坏了。

## 23.8 基准测试：criterion 入门

「感觉这个写法更快」不算结论。Rust 里测量性能的标准工具是 [`criterion`](https://docs.rs/criterion)：它自动做预热、多次采样、统计置信区间，还能给出「和上次相比快了多少」。

基准放在 `benches/` 目录：

```toml
# Cargo.toml
[dev-dependencies]
criterion = "0.8"

[[bench]]
name = "benchmarks"
harness = false        # 关键：用 criterion 自己的入口
```

```rust
// benches/benchmarks.rs
use criterion::{criterion_group, criterion_main, Criterion};
use std::hint::black_box;

fn bench_sum(c: &mut Criterion) {
    let values: Vec<i64> = (0..1000).collect();
    let mut group = c.benchmark_group("求和 1000 个元素");

    group.bench_function("手写循环", |b| {
        b.iter(|| sum_with_loop(black_box(&values)))
    });
    group.bench_function("迭代器 sum", |b| {
        b.iter(|| sum_with_iter(black_box(&values)))
    });

    group.finish();
}

criterion_group!(benches, bench_sum);
criterion_main!(benches);
```

几个要点：

- **`harness = false` 必须写**，否则 Cargo 会用内置的 bench 框架，criterion 跑不起来。
- `benchmark_group` 把相关的基准分到一起，报告里显示成 `组名/用例名`。
- `b.iter(|| ...)` 是「反复执行这段代码」，criterion 自己控制执行多少次（预热 → 采样 → 分析）。
- **基准也是「测试目标」，只能看到 crate 的公开接口**——所以本章的基准函数直接写在 bench 文件里，不依赖二进制 crate 内部。

## 23.9 `black_box`：别让编译器把你要测的东西优化掉

基准里到处都写着 `black_box`，它的作用是**告诉编译器「这个值的内容不可预测，别优化」**：

```rust
b.iter(|| sum_with_loop(black_box(&values)))
```

没有它，编译器可能发现「这个循环的结果没被使用」，直接把整段代码删掉——于是基准测的是「什么都不做」，得到几十皮秒的荒谬结果。

两个使用习惯：

- **输入要包**：`black_box(&values)`，防止编译器提前算出结果。
- **输出最好也包**：`black_box(sum_with_loop(...))`，防止结果被判定为无用。

顺带一提：`criterion::black_box` 已经被标记为废弃，官方建议用标准库的 `std::hint::black_box`（本章实测时编译器给了 `use of deprecated function` 警告，改过来就没警告了）——**看到废弃警告就顺手换掉，这类警告迟早会变成编译错误**。

## 23.10 跑基准与读结果

```bash
cargo bench                              # 跑所有基准（默认完整采样，比较慢）
cargo bench --bench benchmarks           # 只跑指定目标
cargo bench --bench benchmarks -- --quick  # 快速跑一遍（采样少、结果略粗糙）
```

**最后一条的写法值得记住**：`--quick` 要传给 criterion，所以得先指定 `--bench 目标名`。如果直接写 `cargo bench -- --quick`，Cargo 会先把 `--quick` 传给**所有** bench 目标，而二进制目标自带的内置 harness 不认识这个参数，于是报：

```text
error: Unrecognized option: 'quick'
error: bench failed, to rerun pass `--bin rust-learn`
```

输出长这样：

```text
     Running benches\benchmarks.rs (target\release\deps\benchmarks-....exe)
求和 1000 个元素/手写循环
                        time:   [68.865 ns 68.966 ns 69.368 ns]
求和 1000 个元素/迭代器 sum
                        time:   [82.958 ns 82.963 ns 82.984 ns]
```

**怎么读这一行**：

| 部分 | 含义 |
| --- | --- |
| `time:` | 每次迭代的耗时估计 |
| 三个数字 | 置信区间的下界、**估计值**、上界 |
| 单位 | `ns`（纳秒）、`µs`（微秒）、`ms`（毫秒） |

比较两个实现时，**看区间有没有重叠**：完全分开说明差异可信；重叠严重说明「差不多」。只看中位数容易被噪声骗。

其他常用参数：

| 命令 | 作用 |
| --- | --- |
| `--save-baseline before` | 保存一份基线 |
| `--baseline before` | 和基线比较，直接显示「快了/慢了多少 %」 |
| `--sample-size 10` | 减少采样次数，跑得更快 |
| HTML 报告 | 生成在 `target/criterion/report/index.html` |

## 23.11 三次实测：什么差距值得优化

本仓库的 `benches/benchmarks.rs` 对比了三组写法，实测结果（`--quick`，本机数据，仅供对比参考）：

| 对比 | 耗时 | 差距 |
| --- | --- | --- |
| 求和 1000 个元素：手写循环 | 68.9 ns | — |
| 求和 1000 个元素：迭代器 `sum` | 83.0 ns | 慢约 20% |
| 拼接 100 段字符串：`format!` 反复重建 | 21.6 µs | — |
| 拼接 100 段字符串：`push_str` 复用缓冲 | 654.6 ns | **快 33 倍** |
| 查找：`Vec::contains`（1000 个元素） | 145.2 ns | — |
| 查找：`HashSet::contains` | 8.64 ns | **快 17 倍** |

三条结论，正好代表三种情况：

**① 迭代器和手写循环基本一样快。** 20% 的差距在小到几十纳秒的尺度上很容易受噪声影响，而且**这种差距通常不值得为它牺牲可读性**——这正是第 11 章说的「零成本抽象」：用 `sum()` 写更清楚，性能几乎不变。

**② 字符串拼接差了 33 倍，值得改。** 原因是算法复杂度的区别：`format!("{result},{part}")` **每次都新建一个 String 并把已有内容全部复制一遍**，100 次拼接就是 O(n²) 的复制；`push_str` 复用同一块缓冲区，是均摊 O(1)。**遇到「循环里反复拼接」就改用 `push_str` / `push`。**

**③ 查找差了 17 倍，但要看场景。** `Vec::contains` 是线性扫描（O(n)），`HashSet` 是哈希查找（O(1)）；元素越多差距越大。但**如果只在 10 个元素里查一次，145ns 和 9ns 都没人在意**——真正的问题是「在循环里、对大量数据反复查找」，那时才该换成 `HashSet` / `HashMap`（第 7 章 7.1 的复杂度表在这里有了实测数据）。

## 23.12 什么时候该做基准

基准不是「没事跑一跑」，它有两个明确用途：

| 场景 | 怎么做 |
| --- | --- |
| 优化前，确认瓶颈 | 先测（profile / bench），别凭感觉猜 |
| 优化后，确认真的变快了 | 用同一套基准对比（`--baseline`） |
| 选实现方案 | 两个候选写法各写一个基准 |
| 防性能回归 | 把关键基准纳入 CI（阈值告警） |

几条经验：

- **不要过早优化**：先把功能写对、写清楚，再谈性能。第 11 章的迭代器例子就是证明——清晰的写法往往已经够快。
- **看数量级，不看小数点**：快 10 倍值得改，快 5% 通常不值得让代码变复杂。
- **微基准 ≠ 真实场景**：微基准里没有 IO、没有分配模式、没有缓存局部性差异。微基准用来比较「同一种任务的不同写法」，别拿它预测整个程序的性能。
- **优化要有终点**：达到目标（比如「单次请求 < 10ms」）就停下来。

## 23.13 常见坑速查

| 现象 | 原因 | 解法 |
| --- | --- | --- |
| criterion 跑了但输出是内置格式 | 忘了 `harness = false` | 在 `[[bench]]` 里加上 |
| 基准结果是「几十皮秒」 | 代码被编译器优化掉了 | 输入和输出都套 `black_box` |
| 报 `Unrecognized option: 'quick'` | 参数传给了所有 bench 目标 | `cargo bench --bench 目标名 -- --quick` |
| `use of deprecated function criterion::black_box` | API 已迁移到标准库 | 改用 `std::hint::black_box` |
| bench 里 `use crate::...` 失败 | 二进制 crate 没有库目标 | 把被测代码放进 bench 文件，或加 `src/lib.rs` |
| 集成测试里 `common.rs` 被当成测试跑 | 文件名在 `tests/` 下就是测试目标 | 改成 `tests/common/mod.rs` |
| 临时目录「还没用就没了」 | `TempDir` 没绑定到变量 | 绑定成 `_dir` 之类的变量名 |
| 测试偶尔失败、重跑又好了 | 依赖时间 / 端口 / 固定路径 | 注入时钟、端口用 `0`、路径用 `tempfile` |
| 浮点断言偶尔失败 | 用 `==` 比较 | 用容差比较 |
| 测试跑得越来越慢 | 集成测试里有真实等待（网络、sleep） | 缩小等待时间，或标记 `#[ignore]` 手动跑 |
| 一个测试失败却看不出哪步坏了 | 一个测试里断言太多 | 拆成多个聚焦的测试 |

## 23.14 练习

1. 给 `uptime_label` 加一个「小时」分支（`>= 3_600_000` 毫秒时输出 `N 小时`），并用 `FakeClock` 补上覆盖边界（59 分钟 59 秒、正好 1 小时、2 小时）的测试。
2. 写 `fn parse_port(text: &str) -> Result<u16, String>`：解析失败返回带原文的错误，端口为 0 也返回错误；然后为它写「合法值」和「非法值」两组测试。
3. 用本章的 `make_fixture` 写两个测试：一个验证「空文件算 0 个字符」，一个验证「目录不存在时 `total_chars` 返回 0」。
4. 用 criterion 加一个基准，对比 `Vec::sort` 与 `Vec::sort_unstable`（用一段固定的乱序数据），并解释为什么两者会不同。
5. 回答两个问题：(a) 为什么测试里要避免依赖「当前时间」和「真实网络」？(b) 为什么 `black_box` 在基准里是必需的？

（第 4、5 题是 23.8–23.11 讨论过的内容，这里不再单独给答案。）

### 参考答案

::: details 第 1 题

```rust
fn uptime_label(clock: &impl ElapsedClock) -> String {
    let millis = clock.elapsed_millis();
    if millis < 1_000 {
        format!("{millis} 毫秒")
    } else if millis < 60_000 {
        format!("{} 秒", millis / 1000)
    } else if millis < 3_600_000 {
        format!("{} 分钟", millis / 60_000)
    } else {
        format!("{} 小时", millis / 3_600_000)
    }
}

#[test]
fn uptime_label_covers_hours() {
    assert_eq!(uptime_label(&FakeClock { millis: 3_599_999 }), "59 分钟");
    assert_eq!(uptime_label(&FakeClock { millis: 3_600_000 }), "1 小时");
    assert_eq!(uptime_label(&FakeClock { millis: 7_200_000 }), "2 小时");
}
```

实测三个断言全部通过。**边界值的取法是有讲究的**：`3_599_999`（差一毫秒）和 `3_600_000`（正好一小时）分别测的是「上界之下」和「正好取上界」，这正是最容易写错 `<=` / `<` 的地方。

:::

::: details 第 2 题

```rust
fn parse_port(text: &str) -> Result<u16, String> {
    let port: u16 = text
        .trim()
        .parse()
        .map_err(|error| format!("`{text}` 不是合法端口：{error}"))?;
    if port == 0 {
        return Err(String::from("端口不能为 0"));
    }
    Ok(port)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_valid_values() {
        assert_eq!(parse_port("80"), Ok(80));
        assert_eq!(parse_port(" 8080 "), Ok(8080));
        assert_eq!(parse_port("65535"), Ok(65535));   // u16 的上界
    }

    #[test]
    fn rejects_invalid_values() {
        assert!(parse_port("0").is_err());
        assert!(parse_port("65536").is_err());        // 上界 + 1，溢出
        assert!(parse_port("abc").is_err());
        assert!(parse_port("-1").is_err());           // 负数
    }
}
```

实测两组测试都通过。这里利用了 `u16` 的类型边界：**`65536` 由 `parse` 自动拒绝，不需要自己判断范围**——把校验交给类型系统，代码更短也更不容易漏（第 6 章的思路）。

:::

::: details 第 3 题

```rust
mod common;

use common::{make_fixture, total_chars};

#[test]
fn empty_files_count_as_zero() {
    let (_dir, root) = make_fixture(&[("empty.txt", ""), ("note.txt", "内容")]);
    assert_eq!(total_chars(&root), 2);
}

#[test]
fn missing_directory_returns_zero() {
    let (_dir, root) = make_fixture(&[("only.txt", "x")]);
    let missing = root.join("nope");
    assert_eq!(total_chars(&missing), 0);
}
```

实测两个测试都通过。

第二个测试揭示了一个设计细节：`total_chars` 内部用 `WalkDir` + `filter_map(Result::ok)`，**目录不存在时遍历会直接结束**，于是返回 0。这种「静默返回默认值」的行为在测试里要**明确写出来**——它是特性还是 bug，取决于业务约定；写进测试就相当于把它固化成了契约。

:::

## 23.15 小结

- 测试分三层：单元（测内部、能碰私有项）、集成（测配合、只看公开接口）、文档（示例即测试，需要库目标）。

- 共享夹具放 `tests/common/mod.rs`（子目录），别放 `tests/common.rs`——后者会被当成独立的测试目标。

- **依赖注入是让代码可测的关键**：把时间、随机、网络、文件抽成 trait，测试里换成假实现，「mock 框架」往往并不需要。

- 测试必须隔离：用 `tempfile` 建独立的临时目录，`TempDir` 的生命周期绑定到变量上，drop 时自动清理——清理行为本身也可以写测试。

- 浮点比较用容差；枚举形态用 `matches!`；断言加消息能让失败立刻定位；测试可以返回 `Result` 用 `?` 串联。

- 别测实现细节、别依赖外部状态、别一个测试断言一堆东西——**一个测试验证一件事**。

- 基准用 `criterion`：`[[bench]]` 加 `harness = false`，`benchmark_group` 分组，`b.iter` 驱动，`black_box` 防止被优化掉。

- 读结果看**置信区间**：区间分开了才说明有差异；`--quick` 适合快速看，正式对比用完整采样 + `--save-baseline`。

- 本仓库实测：迭代器与手写循环几乎一样快（不值得为 20% 牺牲可读性）；`format!` 循环拼接比 `push_str` 慢 33 倍（O(n²) 的复制）；`Vec::contains` 比 `HashSet::contains` 慢 17 倍（线性 vs 哈希）——**性能差异要看数量级，优化要有终点**。

- 微基准只用来比较「同一任务的不同写法」，别拿它预测整个程序的性能；真实瓶颈要靠 profile 找。

下一章是**第 24 章 Cargo 深入与发布**：features、workspace、依赖管理、交叉编译，以及把 crate 发布到 crates.io 的完整流程。
