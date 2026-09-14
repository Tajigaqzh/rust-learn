//! 集成测试：把编译出来的程序当成黑盒来用。
//!
//! 集成测试只能访问 crate 的公开 API。对二进制 crate 来说，最常见的做法
//! 就是运行这个二进制、检查它的输出——`CARGO_BIN_EXE_<名字>` 是 Cargo
//! 在运行测试时提供的环境变量，指向刚编译好的可执行文件。

use std::process::Command;

#[test]
fn binary_runs_and_prints_every_chapter() {
    let output = Command::new(env!("CARGO_BIN_EXE_rust-learn-demo"))
        .output()
        .expect("运行二进制失败");

    assert!(output.status.success(), "程序退出码不是 0");

    let stdout = String::from_utf8_lossy(&output.stdout);
    for marker in [
        "rust01_print",
        "rust05_ownership",
        "rust10_lifetimes",
        "rust13_concurrency",
        "rust14_modules_tests",
    ] {
        assert!(stdout.contains(marker), "输出里找不到章节标记 {marker}");
    }
}

#[test]
fn binary_reports_finish_markers() {
    let output = Command::new(env!("CARGO_BIN_EXE_rust-learn-demo"))
        .output()
        .expect("运行二进制失败");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("模块、包与测试演示结束"));
}

#[test]
fn binary_prints_error_chapter_marker() {
    let output = Command::new(env!("CARGO_BIN_EXE_rust-learn-demo"))
        .output()
        .expect("运行二进制失败");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("错误处理演示结束"));
}
