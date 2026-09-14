//! 第 23 章配套代码：测试进阶。
//!
//! 运行方式：`cargo run` 看演示，`cargo test` 跑本章的单元测试与集成测试。

use std::time::Instant;

/// 计时抽象：只要能回答「从起点到现在过了多少毫秒」就够了。
trait ElapsedClock {
    fn elapsed_millis(&self) -> u64;
}

/// 真实时钟：从创建它的那一刻开始计时。
struct SystemClock {
    start: Instant,
}

impl SystemClock {
    fn new() -> Self {
        Self {
            start: Instant::now(),
        }
    }
}

impl ElapsedClock for SystemClock {
    fn elapsed_millis(&self) -> u64 {
        self.start.elapsed().as_millis() as u64
    }
}

/// 假时钟：测试里用它换取「结果可预期」。
struct FakeClock {
    millis: u64,
}

impl ElapsedClock for FakeClock {
    fn elapsed_millis(&self) -> u64 {
        self.millis
    }
}

/// 把毫秒数渲染成人类可读的「运行时长」。
fn uptime_label(clock: &impl ElapsedClock) -> String {
    let millis = clock.elapsed_millis();
    if millis < 1_000 {
        format!("{millis} 毫秒")
    } else if millis < 60_000 {
        format!("{} 秒", millis / 1000)
    } else {
        format!("{} 分钟", millis / 60_000)
    }
}

/// 浮点比较：不能用 `==`，要用容差。
fn approx_eq(left: f64, right: f64, epsilon: f64) -> bool {
    (left - right).abs() < epsilon
}

/// 演示「可测试的设计」：依赖注入、容差比较与测试的几种形态。
pub fn testing_demo() {
    println!("\n========== rust23_testing: 测试进阶 ==========");

    println!("\n--- 1. 依赖注入让代码可测试 ---");
    let clock = SystemClock::new();
    let mut total: u64 = 0;
    for i in 0..200_000u64 {
        total += i;
    }
    println!(
        "    真实时钟：算完 {total} 用了 {}（每次运行可能不同）",
        uptime_label(&clock)
    );
    println!(
        "    uptime_label(&FakeClock {{ millis: 5_000 }}) = {}",
        uptime_label(&FakeClock { millis: 5_000 })
    );
    println!(
        "    uptime_label(&FakeClock {{ millis: 3_600_000 }}) = {}",
        uptime_label(&FakeClock { millis: 3_600_000 })
    );
    println!("    （真实时钟每次结果都不同；假时钟让测试可复现）");

    println!("\n--- 2. 浮点比较要用容差 ---");
    println!("    0.1 + 0.2 == 0.3 ？{}", 0.1 + 0.2 == 0.3);
    println!(
        "    approx_eq(0.1 + 0.2, 0.3, 1e-9) ？{}",
        approx_eq(0.1 + 0.2, 0.3, 1e-9)
    );
    println!("    （第 2 章说过：浮点是二进制近似值）");

    println!("\n--- 3. 本章的测试 ---");
    println!("    单元测试在本文件末尾（#[cfg(test)] mod tests）");
    println!("    共享夹具在 tests/common/mod.rs，集成测试在 tests/fixtures.rs");
    println!("    基准测试在 benches/benchmarks.rs，用 cargo bench 跑");

    println!("\n========== 测试进阶演示结束 ==========");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    /// 用假时钟把「时间相关」的逻辑变成确定性测试。
    #[test]
    fn uptime_label_covers_all_branches() {
        let cases = [
            (500u64, "500 毫秒"),
            (1_000, "1 秒"),
            (59_999, "59 秒"),
            (60_000, "1 分钟"),
            (185_000, "3 分钟"),
        ];
        for (millis, expected) in cases {
            assert_eq!(
                uptime_label(&FakeClock { millis }),
                expected,
                "输入 {millis} 毫秒时"
            );
        }
    }

    /// 浮点比较：容差内的差值应当被判定为相等。
    #[test]
    fn floats_compare_with_tolerance() {
        assert_ne!(0.1 + 0.2, 0.3); // 直接比较会失败
        assert!(approx_eq(0.1 + 0.2, 0.3, 1e-9));
        assert!(!approx_eq(0.1 + 0.2, 0.3, 1e-18));
        // NaN 不等于任何值，包括它自己
        assert!(!approx_eq(f64::NAN, f64::NAN, 1e-9));
    }

    /// 用 `matches!` 断言枚举形态，比比较整个值更聚焦。
    #[test]
    fn matches_macro_is_good_for_shapes() {
        let value = Some(42);
        // 带 guard 的形态匹配：既看变体，也看里面的值
        assert!(matches!(value, Some(n) if n > 40));
        assert!(!matches!(value, Some(n) if n > 100));

        // 同一套写法也能用在 Result 上，并且能把错误内容绑出来判断
        let outcome: Result<i32, String> = Err(String::from("boom"));
        assert!(matches!(outcome, Err(message) if message.contains("boom")));
    }

    /// 应该 panic 的测试：`expected` 是子串匹配，必须写。
    #[test]
    #[should_panic(expected = "除数不能为 0")]
    fn division_by_zero_panics() {
        fn divide(a: i32, b: i32) -> i32 {
            assert!(b != 0, "除数不能为 0");
            a / b
        }
        let _ = divide(1, 0);
    }

    /// 测试也可以返回 `Result`，用 `?` 处理中间步骤的失败。
    #[test]
    fn tests_can_return_result() -> Result<(), Box<dyn Error>> {
        let parsed: i32 = "42".parse()?;
        assert_eq!(parsed, 42);
        let date = chrono::NaiveDate::parse_from_str("2026-09-13", "%Y-%m-%d")?;
        assert_eq!(date.to_string(), "2026-09-13");
        Ok(())
    }
}
