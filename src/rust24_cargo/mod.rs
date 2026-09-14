//! 第 24 章配套代码：Cargo 深入与发布。
//!
//! 运行方式：
//!
//! - `cargo run`：默认关闭本章 feature；
//! - `cargo run --features chapter24-demo`：打开 feature，观察条件编译分支；
//! - `cargo run --release`：观察 `debug_assertions` 与 profile 的关系。
//!
//! 本章的 workspace 示例在 `examples/workspace-demo/`，用它自己的清单跑：
//! `cd examples/workspace-demo && cargo build && cargo test`。

use std::cmp::Ordering;

/// 版本号里「哪一位加一」由改动性质决定（SemVer）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Change {
    /// 破坏向后兼容：主版本 +1，次版本与补丁归零。
    Breaking,
    /// 新增功能但保持兼容：次版本 +1，补丁归零。
    Feature,
    /// 兼容的缺陷修复：补丁 +1。
    Fix,
}

/// 解析 `主.次.补丁` 三段纯数字版本；段数不对或含非数字时返回 `None`。
///
/// 这里只处理三个数字段，预发布后缀（如 `1.2.3-alpha.1`）不在范围内——
/// 真实项目请交给 `semver` 这类成熟 crate，示例函数只用来演示规则本身。
pub fn parse_version(text: &str) -> Option<(u64, u64, u64)> {
    let mut parts = text.trim().split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch = parts.next()?.parse().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some((major, minor, patch))
}

/// 比较两个版本号的大小；任意一边解析失败就返回 `None`。
pub fn compare_versions(left: &str, right: &str) -> Option<Ordering> {
    Some(parse_version(left)?.cmp(&parse_version(right)?))
}

/// 按 SemVer 规则算出下一个版本号。
///
/// 注意 `1.2.3` 遇到 `Change::Breaking` 会得到 `2.0.0`，而不是 `2.2.4`：
/// 主版本升级意味着 API 已经变了，次版本和补丁必须归零。
pub fn next_version(current: &str, change: Change) -> Option<String> {
    let (major, minor, patch) = parse_version(current)?;
    let next = match change {
        Change::Breaking => (major + 1, 0, 0),
        Change::Feature => (major, minor + 1, 0),
        Change::Fix => (major, minor, patch + 1),
    };
    Some(format!("{}.{}.{}", next.0, next.1, next.2))
}

/// 当前编译使用的 profile：靠 `debug_assertions` 是否开启来区分。
///
/// `cargo build` 与 `cargo run` 默认是 `dev`；`cargo build --release`
/// 打开优化并关掉调试断言，于是这里是 `release`。
pub fn build_profile() -> &'static str {
    if cfg!(debug_assertions) {
        "dev"
    } else {
        "release"
    }
}

/// 清单里声明的 `chapter24-demo` feature 是否开启。
pub fn demo_feature_enabled() -> bool {
    cfg!(feature = "chapter24-demo")
}

/// Cargo 在编译期注入的包元数据，取自 `Cargo.toml`。
pub fn package_metadata() -> [(&'static str, &'static str); 5] {
    [
        ("CARGO_PKG_NAME", env!("CARGO_PKG_NAME")),
        ("CARGO_PKG_VERSION", env!("CARGO_PKG_VERSION")),
        ("CARGO_PKG_VERSION_MAJOR", env!("CARGO_PKG_VERSION_MAJOR")),
        ("CARGO_PKG_VERSION_MINOR", env!("CARGO_PKG_VERSION_MINOR")),
        ("CARGO_MANIFEST_DIR", env!("CARGO_MANIFEST_DIR")),
    ]
}

/// 把 `Ordering` 换成中文，便于阅读演示输出。
fn order_label(order: Ordering) -> &'static str {
    match order {
        Ordering::Less => "小于",
        Ordering::Equal => "等于",
        Ordering::Greater => "大于",
    }
}

/// 展示编译期元数据、SemVer 算术、feature 条件编译与当前 profile。
pub fn cargo_demo() {
    println!("\n========== rust24_cargo: Cargo 深入与发布 ==========");

    println!("\n--- 1. 编译期注入的包元数据 ---");
    for (key, value) in package_metadata() {
        println!("    {key} = {value}");
    }
    println!("    （清单里改版本号，这里的值跟着变；CARGO_MANIFEST_DIR 因机器而异）");

    println!("\n--- 2. 版本号怎么比较（SemVer） ---");
    for (left, right) in [("0.1.0", "0.2.0"), ("1.2.10", "1.2.9"), ("1.0.0", "1.0.0")] {
        match compare_versions(left, right) {
            Some(order) => println!("    {left} vs {right} = {}", order_label(order)),
            None => println!("    {left} vs {right} = 解析失败"),
        }
    }
    println!("    解析 \"1.2\"（少一段）= {:?}", parse_version("1.2"));
    println!("    解析 \"1.2.x\"（非数字）= {:?}", parse_version("1.2.x"));

    println!("\n--- 3. 改动性质决定哪一位加一 ---");
    for change in [Change::Fix, Change::Feature, Change::Breaking] {
        println!(
            "    1.2.3 + {change:?} = {}",
            next_version("1.2.3", change).unwrap_or_else(|| "解析失败".into())
        );
    }
    println!("    （Breaking 会把次版本和补丁一起归零：1.2.3 -> 2.0.0）");

    println!("\n--- 4. feature 条件编译 ---");
    println!(
        "    cfg!(feature = \"chapter24-demo\") = {}",
        demo_feature_enabled()
    );
    #[cfg(feature = "chapter24-demo")]
    println!("    chapter24-demo 分支：已编译（看到这行说明 feature 打开了）");
    #[cfg(not(feature = "chapter24-demo"))]
    println!("    chapter24-demo 分支：未编译（用 --features chapter24-demo 打开）");

    println!("\n--- 5. 当前构建 profile ---");
    println!(
        "    profile = {}（debug_assertions = {}）",
        build_profile(),
        cfg!(debug_assertions)
    );
    println!("    cargo run             -> dev，编译快、带调试断言");
    println!("    cargo run --release   -> release，带优化、断言关闭");

    println!("\n--- 6. 依赖图与锁文件 ---");
    println!("    Cargo.toml 声明版本范围，Cargo.lock 固定解析结果");
    println!("    cargo tree -e features   查看依赖与 feature 传播");
    println!("    cargo build --locked     CI 里防止锁文件被悄悄更新");
    println!("    依赖走 .cargo/config.toml 里配置的 sparse 镜像，离线也能复现");

    println!("\n--- 7. 发布检查清单 ---");
    println!("    cargo fmt --check");
    println!("    cargo clippy --all-targets --all-features -- -D warnings");
    println!("    cargo test --all-features --locked");
    println!("    cargo package --list     看哪些文件会被打包");
    println!("    cargo publish --dry-run  演练发布（需要网络与登录，本仓库未执行）");

    println!("\n========== Cargo 深入与发布演示结束 ==========");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_three_numeric_segments_only() {
        assert_eq!(parse_version("1.2.3"), Some((1, 2, 3)));
        assert_eq!(parse_version(" 0.1.0 "), Some((0, 1, 0)));
        assert_eq!(parse_version("1.2"), None); // 少一段
        assert_eq!(parse_version("1.2.3.4"), None); // 多一段
        assert_eq!(parse_version("1.2.x"), None); // 非数字
        assert_eq!(parse_version(""), None);
    }

    #[test]
    fn compares_each_segment_in_order() {
        assert_eq!(compare_versions("0.1.0", "0.2.0"), Some(Ordering::Less));
        assert_eq!(compare_versions("1.2.10", "1.2.9"), Some(Ordering::Greater)); // 按数字比，不按字典序
        assert_eq!(compare_versions("1.0.0", "1.0.0"), Some(Ordering::Equal));
        assert_eq!(
            compare_versions("2.0.0", "1.99.99"),
            Some(Ordering::Greater)
        );
        assert_eq!(compare_versions("1.0", "1.0.0"), None); // 任一边非法就整体失败
    }

    #[test]
    fn bumps_the_right_segment_and_resets_lower_ones() {
        assert_eq!(next_version("1.2.3", Change::Fix), Some("1.2.4".into()));
        assert_eq!(next_version("1.2.3", Change::Feature), Some("1.3.0".into()));
        assert_eq!(
            next_version("1.2.3", Change::Breaking),
            Some("2.0.0".into())
        );
        assert_eq!(
            next_version("0.0.1", Change::Breaking),
            Some("1.0.0".into())
        );
        assert_eq!(next_version("1.2", Change::Fix), None);
    }

    #[test]
    fn package_metadata_comes_from_the_manifest() {
        let metadata = package_metadata();
        assert_eq!(metadata[0].0, "CARGO_PKG_NAME");
        assert_eq!(metadata[0].1, "rust-learn");
        assert!(!metadata[1].1.is_empty());
        assert!(!metadata[4].1.is_empty());
    }

    #[test]
    fn feature_and_profile_helpers_match_their_cfg_flags() {
        assert_eq!(demo_feature_enabled(), cfg!(feature = "chapter24-demo"));
        assert_eq!(
            build_profile(),
            if cfg!(debug_assertions) {
                "dev"
            } else {
                "release"
            }
        );
    }
}
