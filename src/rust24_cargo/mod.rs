//! 第 24 章配套代码：Cargo 深入与发布。
//! 运行方式：`cargo run`；启用本章 feature：`cargo run --features chapter24-demo`。

/// 展示 Cargo 在编译期注入的信息，以及 feature 如何选择代码路径。
pub fn cargo_demo() {
    println!("\n========== rust24_cargo: Cargo 深入与发布 ==========");

    println!("\n--- 1. 包与锁文件 ---");
    println!(
        "    package = {} v{}",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION")
    );
    println!("    manifest_dir = {}", env!("CARGO_MANIFEST_DIR"));
    println!("    Cargo.lock 应提交到应用项目，保证依赖图可复现");

    println!("\n--- 2. feature 条件编译 ---");
    #[cfg(feature = "chapter24-demo")]
    println!("    chapter24-demo = enabled");
    #[cfg(not(feature = "chapter24-demo"))]
    println!("    chapter24-demo = disabled（使用 --features chapter24-demo 开启）");

    println!("\n--- 3. 构建与发布检查 ---");
    println!("    cargo check       快速检查");
    println!("    cargo build --release  发布构建");
    println!("    cargo package --allow-dirty  检查待发布包内容");
    println!("    cargo publish --dry-run      发布前验证（不会上传）");
    println!("    cargo tree -e features       查看依赖与 feature 传播");

    println!("\n--- 4. 版本与可复现性 ---");
    println!("    Cargo.toml 声明意图，Cargo.lock 固定解析结果");
    println!("    CI 建议使用 cargo build --locked，避免锁文件被悄悄更新");
    println!("    发布前依次执行 fmt、clippy、test、package --list");

    println!("\n========== Cargo 深入与发布演示结束 ==========");
}

#[cfg(test)]
mod tests {
    #[test]
    fn cargo_metadata_is_available_at_compile_time() {
        assert!(!env!("CARGO_PKG_NAME").is_empty());
        assert!(!env!("CARGO_PKG_VERSION").is_empty());
    }

    #[test]
    fn feature_cfg_is_well_formed() {
        let enabled = cfg!(feature = "chapter24-demo");
        assert_eq!(enabled, cfg!(feature = "chapter24-demo"));
    }
}
