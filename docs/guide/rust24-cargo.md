# 第 24 章 · Cargo 深入与发布

Cargo 不只是“运行 `cargo run` 的命令”。它负责解析依赖、编译多个目标、管理 feature、生成发布包，并把这些步骤串成可复现的构建流程。本章的代码在 `src/rust24_cargo/mod.rs`，可直接运行：

```bash
cargo run
cargo run --features chapter24-demo
```

## 24.1 包、目标与清单

`Cargo.toml` 是包的清单（manifest）。`[package]` 描述名称、版本和 edition；`[dependencies]` 描述运行时依赖；`[dev-dependencies]` 只用于测试和基准。一个包可以同时包含库目标、二进制目标、示例、测试和 benchmark。默认的 `src/main.rs` 是本项目的二进制目标。

Cargo 会把 `CARGO_PKG_NAME`、`CARGO_PKG_VERSION`、`CARGO_MANIFEST_DIR` 等变量在编译期注入，示例模块打印了这些值。它们适合记录版本和定位资源，但不应被当成运行时配置。

## 24.2 依赖解析与 Cargo.lock

`Cargo.toml` 表达允许的版本范围，Cargo 解析后把确切版本、校验和及依赖关系写入 `Cargo.lock`。应用和可执行程序通常应提交 lock 文件；库是否提交取决于发布策略。CI 中使用 `cargo build --locked` 或 `cargo test --locked`，可以在锁文件过期时立即失败，而不是重新解析出另一棵依赖树。

常用检查命令：

```bash
cargo tree
cargo tree -e features
cargo outdated                 # 需要额外安装 cargo-outdated
cargo update -p serde --precise 1.0.219
```

升级依赖应当小步进行，并检查变更日志与完整测试结果。

## 24.3 features 与条件编译

feature 是 Cargo 传给编译器的命名开关。在清单中声明：

```toml
[features]
default = []
chapter24-demo = []
```

代码使用 `#[cfg(feature = "chapter24-demo")]` 选择实现，命令行使用 `--features chapter24-demo` 开启。本章示例会显示 feature 的状态。默认 feature 尽量保持少，避免库用户无意中引入重量级依赖；依赖 feature 也要明确列出，减少 feature 统一（feature unification）带来的意外。

## 24.4 构建配置与工作区

`cargo check` 只做类型和借用检查，反馈最快；`cargo build --release` 使用 release profile 生成优化产物。项目变大后可以用 workspace 在根清单中统一管理多个 crate，并用 `cargo check --workspace`、`cargo test --workspace` 一次验证全部成员。workspace 共享锁文件和 `target` 目录，但每个 crate 仍有独立的公开 API 与版本。

## 24.5 发布前检查

发布到 crates.io 前建议按顺序执行：

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features --locked
cargo package --list
cargo publish --dry-run
cargo publish
```

`cargo package` 会按清单打包并检查缺失文件；`--dry-run` 验证发布流程但不会上传。版本号遵循 SemVer：破坏 API 的改动升主版本，向后兼容的功能升次版本，修复升补丁版本。发布后不要复用同一个版本号。

## 24.6 小结

- 清单描述意图，锁文件固定结果；应用的 CI 使用 `--locked`。
- feature 让同一 crate 支持可选能力，`cfg` 决定编译哪些代码。
- `check`、`build --release`、`tree`、`package` 分别服务于反馈、产物、依赖审计和发布验证。
- 发布前让格式化、clippy、测试和 dry-run 全部通过，再上传不可变的版本。
