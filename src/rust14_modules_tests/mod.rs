//! 第 14 章配套代码：模块、包与测试。
//!
//! 运行方式：`cargo run` 看演示，`cargo test` 跑本章的单元测试与集成测试。

/// 几何计算：演示「模块 + 可见性」。
pub mod geometry {
    /// 矩形面积：`pub` 之后外部才能调用。
    pub fn area_of_rect(width: f64, height: f64) -> f64 {
        width * height
    }

    /// 私有函数：只有本模块（和它的子模块）能调用，外部调用报 `E0603`。
    fn round_to(value: f64, digits: u32) -> f64 {
        let factor = 10f64.powi(digits as i32);
        (value * factor).round() / factor
    }

    /// 对外暴露的「带格式」版本，内部调用私有函数。
    pub fn area_text(width: f64, height: f64) -> String {
        format!("{:.2}", round_to(area_of_rect(width, height), 2))
    }

    // 子模块可以访问父模块的私有项，所以测试就写在这里。
    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn private_helper_is_reachable_inside_module() {
            assert_eq!(round_to(1.2345, 2), 1.23);
        }

        #[test]
        fn area_of_rect_handles_zero() {
            assert_eq!(area_of_rect(0.0, 5.0), 0.0);
        }
    }
}

/// 文本处理：演示 `pub use` 重导出。
pub mod text {
    /// 清洗：去掉首尾空白，把连续空白压成一个空格。
    pub fn normalize(input: &str) -> String {
        input.split_whitespace().collect::<Vec<_>>().join(" ")
    }

    /// 统计单词数。
    pub fn word_count(input: &str) -> usize {
        input.split_whitespace().count()
    }
}

/// 在 crate 根重导出，外部就能写更短的路径。
pub use text::normalize;

/// 演示模块树、可见性与重导出。
pub fn modules_tests_demo() {
    println!("\n========== rust14_modules_tests: 模块、包与测试 ==========");

    println!("\n--- 1. 模块与可见性 ---");
    println!("    模块树：crate -> rust14_modules_tests -> geometry / text");
    println!(
        "    geometry::area_of_rect(3.0, 4.0) = {}",
        geometry::area_of_rect(3.0, 4.0)
    );
    println!(
        "    geometry::area_text(3.0, 4.0) = {}",
        geometry::area_text(3.0, 4.0)
    );
    println!("    （round_to 是私有的，只能被同模块的 area_text 调用）");

    println!("\n--- 2. pub use 重导出 ---");
    println!(
        "    normalize(\"  rust   is  fun \") = [{}]",
        normalize("  rust   is  fun ")
    );
    println!(
        "    text::word_count(\"rust is fun\") = {}",
        text::word_count("rust is fun")
    );

    println!("\n--- 3. 测试 ---");
    println!("    单元测试写在本文件末尾：#[cfg(test)] mod tests");
    println!("    集成测试在 tests/integration.rs：它会运行本程序并检查输出");
    println!("    执行 cargo test 可以看到全部结果");

    println!("\n========== 模块、包与测试演示结束 ==========");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn area_of_rect_works() {
        assert_eq!(geometry::area_of_rect(3.0, 4.0), 12.0);
    }

    #[test]
    fn area_text_rounds_to_two_digits() {
        assert_eq!(geometry::area_text(1.0, 2.0 / 3.0), "0.67");
    }

    #[test]
    fn normalize_collapses_spaces() {
        assert_eq!(normalize("  rust   is   fun "), "rust is fun");
    }

    #[test]
    fn normalize_keeps_single_word() {
        assert_eq!(normalize("rust"), "rust");
        assert_ne!(normalize("rust"), "rust ");
    }

    #[test]
    fn word_count_counts_words() {
        assert_eq!(text::word_count("rust is fun"), 3);
        assert_eq!(text::word_count("   "), 0);
    }

    #[test]
    #[should_panic(expected = "除数不能为 0")]
    fn division_by_zero_panics() {
        fn divide(a: i32, b: i32) -> i32 {
            assert!(b != 0, "除数不能为 0");
            a / b
        }
        let _ = divide(1, 0);
    }
}
