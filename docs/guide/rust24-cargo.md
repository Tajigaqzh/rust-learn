# 第 24 章 · Cargo 深入与发布

Cargo 不只是「运行 `cargo run` 的那个命令」，它同时是依赖解析器、构建调度器、
测试与基准的执行器，以及打包发布工具。这一章把这些职责拆开看：

1. **清单与目标**：`Cargo.toml` 描述什么，Cargo 实际编译出哪些东西；
2. **依赖与锁文件**：版本范围、解析结果、可复现构建；
3. **feature 与 profile**：同一份代码怎样编译出不同形态；
4. **workspace**：一个清单管多个 crate；
5. **打包发布**：哪些文件会被上传，发布前要过哪些检查。

配套代码都在本仓库里真实可跑：

| 部分 | 位置 | 怎么跑 |
| --- | --- | --- |
| 本章演示模块 | `src/rust24_cargo/mod.rs` | `cargo run` |
| feature 分支 | 同上 + 根清单的 `chapter24-demo` | `cargo run --features chapter24-demo` |
| release 差异 | 同上 | `cargo run --release` |
| workspace 示例 | `examples/workspace-demo/` | `cd examples/workspace-demo && cargo build && cargo test` |

本章新增的 feature 声明就写在根清单里（没有引入任何新依赖）：

```toml
[features]
default = []
chapter24-demo = []
```

## 24.1 包、目标与清单

`Cargo.toml` 是包的**清单**（manifest）。`[package]` 描述名称、版本和 edition，
`[dependencies]` 描述运行时依赖，`[dev-dependencies]` 只用于测试、示例和基准。

一个包可以同时有多个**目标**（target）。本仓库的真实目标清单可以直接问 Cargo：

```bash
cargo metadata --no-deps --format-version 1
```

把这串 JSON 里的 `targets` 取出来看，就是下面这些（`kind` 来自清单里显式的
`[lib] crate-type`、`[[bin]]`、`[[bench]]` 和目录约定）：

```text
cdylib,rlib  rust_learn
bin          rust-learn-demo
test         fixtures
test         integration
bench        benchmarks
```

对应关系很清楚：

| 目标 | 来自哪里 | 说明 |
| --- | --- | --- |
| `rust_learn`（`rlib` + `cdylib`） | `src/lib.rs` + `[lib]` | 第 25 章要用它导出 C ABI 动态库 |
| `rust-learn-demo`（`bin`） | `src/main.rs` + `[[bin]]` | `cargo run` 跑的就是它 |
| `fixtures`、`integration`（`test`） | `tests/*.rs` | 每个文件都是一个独立的测试 crate |
| `benchmarks`（`bench`） | `benches/benchmarks.rs` + `[[bench]]` | 第 23 章的 criterion 基准 |

二进制目标为什么不叫 `rust-learn`？因为要和库目标 `rust_learn` 分开——
这正是本章 24.5 最后一小节要讲的真实踩坑记录。

**注意 `tests/common/mod.rs` 不在这个列表里**——`tests/` 下每个 `.rs` 文件都会被
当成独立测试目标，但子目录不会（第 23 章 23.2 解释过原因）。

### 编译期注入的元数据

Cargo 会把包信息以环境变量形式注入编译期，`env!` 在编译时就把值替换进代码：

```rust
pub fn package_metadata() -> [(&'static str, &'static str); 5] {
    [
        ("CARGO_PKG_NAME", env!("CARGO_PKG_NAME")),
        ("CARGO_PKG_VERSION", env!("CARGO_PKG_VERSION")),
        ("CARGO_PKG_VERSION_MAJOR", env!("CARGO_PKG_VERSION_MAJOR")),
        ("CARGO_PKG_VERSION_MINOR", env!("CARGO_PKG_VERSION_MINOR")),
        ("CARGO_MANIFEST_DIR", env!("CARGO_MANIFEST_DIR")),
    ]
}
```

`cargo run` 的实测输出：

```text
--- 1. 编译期注入的包元数据 ---
    CARGO_PKG_NAME = rust-learn
    CARGO_PKG_VERSION = 0.1.0
    CARGO_PKG_VERSION_MAJOR = 0
    CARGO_PKG_VERSION_MINOR = 1
    CARGO_MANIFEST_DIR = <仓库根目录>（因机器而异）
```

这些值和清单是同一份来源：改 `Cargo.toml` 里的 `version`，输出立刻跟着变。
它们适合做版本号展示、定位随包资源，**不适合当运行时配置**——想改一个值就得
重新编译。运行时配置请用第 20 章的环境变量或配置文件。

## 24.2 依赖解析与 `Cargo.lock`

`Cargo.toml` 写的是**允许的版本范围**，`Cargo.lock` 记的是**这一次解析出的确切
版本**。以本仓库的清单为例：

```toml
[dependencies]
chrono = "0.4"
clap = { version = "4", default-features = false, features = ["std", "help", "usage", "error-context", "suggestions"] }
rusqlite = { version = "0.32", features = ["bundled"] }
```

`"0.4"` 是 `^0.4` 的简写，语义是「≥ 0.4.0 且 < 0.5.0」；`"1"` 是「≥ 1.0.0 且
< 2.0.0」。写范围而不是写死版本，是为了让依赖能收到兼容更新；而 `Cargo.lock`
保证**每个人、每次 CI 构建出来的依赖图完全相同**。

```bash
cargo tree                 # 依赖树
cargo tree -e features     # 连 feature 一起看（谁打开了什么，一目了然）
cargo update -p serde      # 只升一个包，并更新锁文件
cargo build --locked       # 锁文件过期就直接失败，绝不悄悄重新解析
```

`cargo tree -e features --depth 1` 的实测输出（节选）：

```text
rust-learn v0.1.0 (<仓库根目录>)
├── chrono feature "default"
├── clap feature "std"
├── regex feature "default"
├── reqwest feature "blocking"
├── reqwest feature "json"
├── rusqlite feature "bundled"
├── rusqlite feature "default"
├── tracing feature "default"
└── unicode-segmentation feature "default"
[dev-dependencies]
├── criterion feature "default"
└── zerocopy feature "default"
```

这张表解决的是「**feature 是从哪来的**」这类问题：`rusqlite` 的 `bundled`
不是我们手写进 `[dependencies]` 的普通依赖，而是打开它就会把 SQLite 源码一起
编进来（第 26 章用它保证离线可用）。

### 锁文件该不该提交

| 项目类型 | 是否提交 `Cargo.lock` | 原因 |
| --- | --- | --- |
| 应用、二进制、CLI | **提交** | 用户装到的就是同一条依赖链 |
| 库（发布到 crates.io） | 一般不提交 | 使用方按自己的锁文件解析；提交会让人误以为有约束力 |
| 本仓库（学习工程） | 提交 | 保证每章示例在任何机器上编出同样的结果 |

### 依赖下载走哪里

本仓库把镜像配置写在仓库内的 `.cargo/config.toml` 里，只对这个项目生效：

```toml
[source.crates-io]
replace-with = "rsproxy"

[source.rsproxy]
registry = "sparse+https://rsproxy.cn/index/"
```

好处是不动用户全局的 `~/.cargo/config.toml`；换环境时只需要覆盖
`CARGO_REGISTRIES_CRATES_IO_INDEX` 就能切回官方源。**注意**：
`cargo build --offline` 不是下载命令，只有在依赖已经缓存时才可用，这一点第 24.8 节
的发布流程里也会用到。

## 24.3 SemVer 与版本升级

发布出去的版本号是给使用者看的契约。规则只有三条，但很容易用错：

| 改动性质 | 哪一位 +1 | 例子 | 对使用者的含义 |
| --- | --- | --- | --- |
| 破坏兼容（改签名、删接口） | 主版本 | `1.2.3` → `2.0.0` | 升级必须看变更说明 |
| 向后兼容的新功能 | 次版本 | `1.2.3` → `1.3.0` | 可以直接升 |
| 向后兼容的修复 | 补丁 | `1.2.3` → `1.2.4` | 可以直接升 |

本章的代码把这套规则写成了一个可以测试的小函数：

```rust
pub enum Change {
    Breaking, // 主版本 +1，次版本和补丁归零
    Feature,  // 次版本 +1，补丁归零
    Fix,      // 补丁 +1
}

pub fn next_version(current: &str, change: Change) -> Option<String> {
    let (major, minor, patch) = parse_version(current)?;
    let next = match change {
        Change::Breaking => (major + 1, 0, 0),
        Change::Feature => (major, minor + 1, 0),
        Change::Fix => (major, minor, patch + 1),
    };
    Some(format!("{}.{}.{}", next.0, next.1, next.2))
}
```

实测输出：

```text
--- 2. 版本号怎么比较（SemVer） ---
    0.1.0 vs 0.2.0 = 小于
    1.2.10 vs 1.2.9 = 大于
    1.0.0 vs 1.0.0 = 等于
    解析 "1.2"（少一段）= None
    解析 "1.2.x"（非数字）= None

--- 3. 改动性质决定哪一位加一 ---
    1.2.3 + Fix = 1.2.4
    1.2.3 + Feature = 1.3.0
    1.2.3 + Breaking = 2.0.0
```

三个容易忽略的点：

- **比较的是数字，不是字符串**：`1.2.10 > 1.2.9`。如果按字典序比，`10 < 9` 会
  得出相反结论——这正是几乎所有「手写版本比较」出 bug 的地方。
- **主版本升级要归零**：`1.2.3` 破坏兼容后是 `2.0.0`，不是 `2.2.4`。
- **`0.x` 是开发期**：Cargo 把 `^0.4` 解释成 `< 0.5.0`，因为 `0.x` 的次版本
  也被视为可能破坏兼容。

真实项目不要手写解析器：`semver` crate 支持预发布后缀（`1.2.3-alpha.1`）、
构建元数据（`+build.7`）和完整的大小比较。本章的函数只演示规则本身，
所以 `parse_version("1.2")` 和 `parse_version("1.2.x")` 都返回 `None`。

## 24.4 features 与条件编译

feature 是 Cargo 传给编译器的**命名开关**，本身不产生代码，只影响 `cfg`：

```toml
[features]
default = []
chapter24-demo = []
```

```rust
pub fn demo_feature_enabled() -> bool {
    cfg!(feature = "chapter24-demo")
}

#[cfg(feature = "chapter24-demo")]
println!("    chapter24-demo 分支：已编译（看到这行说明 feature 打开了）");
#[cfg(not(feature = "chapter24-demo"))]
println!("    chapter24-demo 分支：未编译（用 --features chapter24-demo 打开）");
```

两次运行的实测对比（同一个二进制，两份输出）：

```text
$ cargo run
--- 4. feature 条件编译 ---
    cfg!(feature = "chapter24-demo") = false
    chapter24-demo 分支：未编译（用 --features chapter24-demo 打开）

$ cargo run --features chapter24-demo
--- 4. feature 条件编译 ---
    cfg!(feature = "chapter24-demo") = true
    chapter24-demo 分支：已编译（看到这行说明 feature 打开了）
```

`cfg!` 和 `#[cfg]` 的区别值得记住：`cfg!(...)` 是**表达式**，两个分支都会被
编译，只是返回 `true`/`false`；`#[cfg(...)]` 是**属性**，不满足条件的代码
根本不会参与编译。所以「依赖某个可选 crate」的代码必须用 `#[cfg]` 包起来，
否则关掉 feature 时会因为找不到 crate 而编译失败。

### feature 统一（feature unification）

同一个 crate 在一次构建里只会被编译一次，feature 是所有依赖方需求的**并集**：

| 场景 | 结果 |
| --- | --- |
| 应用 A 要 `json`，应用 B 不要 | 一起构建 A+B 时，`json` 会被打开 |
| 默认 feature 里塞了重量级依赖 | 所有用户都得跟着编译 |
| 两个依赖想用不同版本的同名 crate | 各自的 feature 互不影响，除非它们指向同一个 crate 实例 |

实践建议：

- `default = []`，把可选能力显式列出来（本仓库就是这样）；
- 库的 `[dependencies]` 尽量写 `default-features = false` 再逐项打开，
  例如 `clap` 在本仓库只启用了 `std`、`help`、`usage` 等必要项；
- 用 `cargo tree -e features` 检查「谁打开了什么」，而不是靠猜。

## 24.5 profile：debug、release 与构建产物

profile 决定优化等级、调试信息和断言开关。代码里能直接观察到这一点
（`debug_assertions` 在 `dev` 下是 `true`，在 `release` 下是 `false`）：

```rust
pub fn build_profile() -> &'static str {
    if cfg!(debug_assertions) { "dev" } else { "release" }
}
```

实测输出：

```text
$ cargo run
--- 5. 当前构建 profile ---
    profile = dev（debug_assertions = true）

$ cargo run --release
--- 5. 当前构建 profile ---
    profile = release（debug_assertions = false）
```

| profile | 谁在用 | 特点 |
| --- | --- | --- |
| `dev` | `cargo run`、`cargo test` | 不优化、带完整调试信息、`debug_assertions` 打开 |
| `release` | `cargo run --release`、`cargo build --release` | 优化、`overflow-checks` 默认关闭、编译更慢 |
| `bench` | `cargo bench` | 继承 `release`，第 23 章的 criterion 用的就是它 |

需要长期固定一组设置时，在清单里自定义 profile：

```toml
[profile.release]
opt-level = 3
lto = "thin"
codegen-units = 1
panic = "abort"      # 注意：abort 后 catch_unwind 不再拦截 panic（第 25 章）
```

`panic = "abort"` 这条在 FFI 项目里很常见（产物更小、崩溃行为更确定），但它和
第 25 章「用 `catch_unwind` 把 panic 换成错误码」是冲突的：abort 之后进程直接
结束，没有展开可言。改 profile 之前先想清楚异常语义。

### 一个真实踩过的坑：bin 与 lib 同名

本仓库早先的清单里，二进制目标用的是默认名（跟着包名走），于是 Windows 上执行
`cargo test` 会看到这样一条 Cargo 自己的提示：

```text
warning: output filename collision at <仓库根目录>\target\debug\deps\rust_learn.pdb
  = note: the bin target `rust-learn` in package `rust-learn v0.1.0` has the same
          output filename as the lib target `rust_learn` in package `rust-learn v0.1.0`
```

原因很简单：包名 `rust-learn` 的二进制目标在 Windows 上产出 `rust_learn.exe` /
`rust_learn.pdb`，而 `src/lib.rs` 的库目标名字也是 `rust_learn`，两者在
`target/debug/deps/` 里撞到了同一个文件名（`.exe` 与 `.dll` 扩展名不同并不冲突，
真正撞的是调试符号 `.pdb`）。这是**命名冲突**，不是代码错误：它不影响运行和
测试结果，但会让构建输出长期带噪音，项目一大还容易演变成真的产物覆盖。

修法是给二进制目标显式起个名字（库名保持不变——第 25 章的 Python / Go 示例
都按 `rust_learn.dll` 找它）：

```toml
[[bin]]
name = "rust-learn-demo"
path = "src/main.rs"
```

`path` 明确指向 `src/main.rs` 之后，Cargo 不会再额外自动发现一个同名二进制目标
（`cargo metadata --no-deps` 里仍然只有一个 `bin`）。改完的效果是可以验证的：

| | 改之前 | 改之后 |
| --- | --- | --- |
| 目标名 | `bin rust-learn` | `bin rust-learn-demo` |
| `cargo build` 输出 | 带 `output filename collision` 警告 | 干净 |
| 集成测试引用 | `CARGO_BIN_EXE_rust-learn` | `CARGO_BIN_EXE_rust-learn-demo` |
| 命令行 | `cargo test --bin rust-learn` | `cargo test --bin rust-learn-demo` |

代价就是最后两行：**凡是引用二进制名的地方都得跟着改**（本仓库的
`tests/integration.rs` 和好几章正文里的命令都改了一遍）。所以另一种同样合法的
做法是反过来改库名，例如 `[lib] name = "rust_learn_ffi"`，让动态库产物改叫
`rust_learn_ffi.dll`——哪一种改动小，取决于哪个名字被更多地方引用。

### 从「有 lint」到 `-D warnings` 通过

上面那条命令本仓库一度是**过不去**的：较早的章节累计留下了十几条 clippy 提示，
其中第 18 章为了演示 regex 不支持的语法，还产生了两条 error 级别的
`invalid_regex`。后来是这么清掉的：

| 情况 | 处理方式 | 涉及的章节 |
| --- | --- | --- |
| 写法确实可以更地道（`manual_find`、`collapsible_if`、`useless_vec`、`useless_format`、`redundant_pattern_matching` 等） | 改成 lint 建议的惯用写法，并同步正文 | 6、8、11、13、23、benches |
| 代码**故意**写成这样（宏展开成 `Vec::new()` + `push`；用非法正则演示报错） | 用带注释的 `#[allow(...)]` **精确放行**，不关整条 lint | 15、18 |

现在整仓 `cargo clippy --all-targets --all-features -- -D warnings` 是通过的，
本章与第 24–28 章新增的代码也没有任何诊断。这里有两个可以带走的经验：

1. **lint 是建议的起点，不是圣旨**：该改的改，该解释的用 `#[allow]` 加理由；
2. **放行的范围要小**：`#[allow]` 尽量写在那一行/那个块上，写进 crate 级别的
   `#![allow(...)]` 等于把整类问题从检查里删掉。

顺带记一个宏相关的坑：`#[allow]` 加在 `macro_rules!` 定义上、或者挂在宏调用
语句上都不生效，得写在**调用处展开的语句**上（本章第 15 章那段就是这么处理的）。

## 24.6 workspace：一个清单管多个 crate

项目一大，常见做法是把「业务规则」和「可执行入口」拆成两个 crate。workspace
就是让它们共享一份锁文件、一个 `target/` 目录和一条命令：

```text
examples/workspace-demo/
├── Cargo.toml          虚拟清单：只声明成员，不自己产出产物
├── Cargo.lock          工作区共享一份锁文件
├── core-lib/           业务规则（库）+ 单元测试
└── cli-bin/            命令行入口（二进制），依赖 core-lib
```

```toml
# examples/workspace-demo/Cargo.toml
[workspace]
resolver = "3"
members = ["core-lib", "cli-bin"]
```

成员之间用**路径依赖**相连，这比版本号更适合同一个仓库里的协作：

```toml
# examples/workspace-demo/cli-bin/Cargo.toml
[dependencies]
core-lib = { path = "../core-lib" }
```

在 `examples/workspace-demo/` 下依次执行（实测输出）：

```text
$ cargo build
   Compiling core-lib v0.1.0 (<...>/examples/workspace-demo/core-lib)
   Compiling cli-bin v0.1.0 (<...>/examples/workspace-demo/cli-bin)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.43s

$ cargo test
running 0 tests   (cli-bin 的 main.rs 没有测试)
test result: ok. 0 passed; 0 failed; 0 ignored; ...

running 1 test
test tests::trims_and_collapses_whitespace ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; ...

$ cargo run -q -p cli-bin
写 Rust

$ cargo tree
cli-bin v0.1.0 (<...>/examples/workspace-demo/cli-bin)
└── core-lib v0.1.0 (<...>/examples/workspace-demo/core-lib)

core-lib v0.1.0 (<...>/examples/workspace-demo/core-lib)
```

几个高频命令的区别：

| 命令 | 作用 |
| --- | --- |
| `cargo build` / `cargo test` | 对**所有成员**执行 |
| `cargo build -p cli-bin` | 只构建指定包（`-p` = `--package`） |
| `cargo test -p core-lib` | 只测指定包，改一个小库时能省很多时间 |
| `cargo run -p cli-bin` | 运行指定包的二进制 |
| `cargo metadata` | 列出成员、依赖和解析结果，写脚本时很好用 |

工作区里的成员各自有自己的 `Cargo.toml`，所以**根目录的 `rust-learn` 包不会
把它一起编译**：在仓库根执行 `cargo run` 时，`examples/workspace-demo/` 完全
不参与。这一点在打包时也能验证（见下一节）。

## 24.7 交叉编译与目标平台

`cargo build --target <三元组>` 可以为别的平台产出二进制，前提是本机装了对应的
标准库和链接器：

```bash
rustup target add aarch64-unknown-linux-gnu
cargo build --release --target aarch64-unknown-linux-gnu
```

常见目标三元组：`x86_64-pc-windows-msvc`（Windows）、`x86_64-unknown-linux-gnu`
（Linux）、`aarch64-apple-darwin`（Apple Silicon）、`wasm32-unknown-unknown`
（WebAssembly）。

交叉编译最容易踩的三个坑：

1. **只有 Rust 标准库不够**：链接阶段需要目标平台的 C 工具链或系统库，
   纯 Rust 且不依赖系统库的项目最省事。
2. **带 C 依赖的项目更麻烦**：本仓库的 `rusqlite` 用了 `bundled`，需要为目标
   平台准备 C 编译器；这也是选 `bundled` 时要提前确认的一点。
3. **目标专属配置写在清单里**：

```toml
[target.x86_64-pc-windows-msvc]
rustflags = ["-C", "target-feature=+crt-static"]
```

> 本机实测：`rustup target list --installed` 只列出了
> `x86_64-pc-windows-msvc`，所以本章**没有实际执行过跨平台构建**。
> 上面的命令来自 Cargo/rustup 的常规用法，请在自己的环境里验证后再用于生产。

## 24.8 打包与发布

发布到 crates.io 之前，先看清「包里到底装了哪些文件」：

```bash
cargo package --list
```

本仓库的实测结果一共 82 个文件，节选如下：

```text
.cargo/config.toml
.cargo_vcs_info.json
Cargo.lock
Cargo.toml
README.md
benches/benchmarks.rs
docs/guide/rust24-cargo.md
examples/ffi/caller.py
src/lib.rs
src/main.rs
src/rust24_cargo/mod.rs
tests/integration.rs
```

两个和本章新内容直接相关的观察：

- `examples/ffi/` 会被打包进去——普通目录的内容默认跟着包走；
- `examples/workspace-demo/` **不在**列表里：它自带 `Cargo.toml`，是一个嵌套的
  包，Cargo 不会把它塞进父包里（这也是它能在仓库里独立构建的原因）。

同一份清单里还能看到一条真实警告：

```text
warning: manifest has no description, license, license-file, documentation,
homepage or repository
```

发布到 crates.io 时这些字段不是「锦上添花」而是硬要求：没有 `license` 或
`license-file` 会被直接拒绝。需要精确控制打包内容时用 `include` / `exclude`：

```toml
[package]
exclude = ["docs/", "benches/"]      # 文档站和基准不必上传
include = ["src/**", "Cargo.toml", "README.md"]
```

完整的发布流程：

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features --locked
cargo package --list                 # 检查文件清单
cargo package                        # 真正打包并在 target/package 里做一次验证构建
cargo publish --dry-run              # 演练：走完上传前的所有检查，但不真的上传
cargo publish                        # 上传，版本号自此不可复用
```

几条铁律：

- **版本号不可复用**：crates.io 不允许覆盖已发布的版本，发现错了只能发 `0.1.1`
  或 `yank` 掉旧版本（`cargo yank --version 0.1.0`，只是阻止新项目依赖它）；
- **发布前用干净工作区**：`cargo package` 对未提交改动会警告，必要时显式加
  `--allow-dirty`，但这说明你还没准备好；
- **CI 用 `--locked`**：锁文件过期就失败，避免发布出一个「和本地不一样」的包。

> 本机实测：`cargo publish --dry-run` 需要登录 crates.io 并访问其接口，
> 本章**没有执行**这一步。上面的顺序来自官方发布流程，请在有网络和 token 的
> 环境里执行。

## 24.9 常见坑速查

| 现象 | 原因 | 解法 |
| --- | --- | --- |
| `cargo build` 能过，CI 上依赖版本却不一样 | CI 重新解析了依赖 | 提交 `Cargo.lock`，CI 用 `--locked` |
| 关掉 feature 后编译失败 | 可选依赖的代码没被 `#[cfg]` 包住 | 用 `#[cfg(feature = "...")]` 圈住相关代码 |
| 只想开一个小功能，却拖进来一堆依赖 | 依赖用了默认 feature | `default-features = false` + 显式列表 |
| `cargo tree` 里出现两份同名 crate | 版本范围不兼容，Cargo 保留了两份 | `cargo tree -d` 查重复，统一版本或升级依赖 |
| 本地改了依赖却没生效 | Cargo 用了缓存的解析结果 | `cargo update -p <名字>`，再检查 `Cargo.lock` |
| 测试通过但 release 下行为不同 | 断言的开关不同（`debug_assertions`、`overflow-checks`） | 关键不变量用返回 `Result` 而不是断言 |
| `panic = "abort"` 后 `catch_unwind` 失效 | abort 不展开栈 | 要么保留 unwind，要么改成错误码契约 |
| Windows 上出现 `output filename collision` | bin 与 lib 归一化后同名 | 改 `[[bin]] name` 或 `[lib] name` |
| `cargo package` 里混进不该上传的文件 | 默认打包目录下的所有内容 | 用 `include` / `exclude` 精确控制 |
| `cargo publish` 报缺少 `license` | crates.io 的硬性要求 | 补 `license` 或 `license-file` |

## 24.10 练习

1. 给 `next_version` 加一条规则：`0.x` 项目里 `Change::Breaking` 只升次版本
   （`0.4.3` → `0.5.0`），并补上覆盖 `0.x` 与 `1.x` 两边行为的测试。
2. 把本章的 feature 从「空开关」改成「可选依赖」：加一个 `chapter24-extras`
   feature，打开时才启用 `chrono`（提示：`chrono` 设成可选依赖
   `optional = true`，feature 写成 `chapter24-extras = ["dep:chrono"]`），
   并各跑一次 `cargo run` 与 `cargo run --features chapter24-extras` 对比。
3. 在 `examples/workspace-demo/` 里加第三个成员 `core-lib-tests`（一个只依赖
   `core-lib` 的测试 crate，或直接给 `core-lib` 加集成测试 `tests/`），
   然后回答：`cargo test` 和 `cargo test -p core-lib` 的输出有什么不同？
4. 用 `cargo package --list` 对比「把 `exclude = ["docs/"]` 加进 `[package]`」
   前后的文件数差异，并解释为什么 `examples/workspace-demo/` 一直不在列表里。
5. 回答两个问题：(a) 为什么应用的 `Cargo.lock` 要提交，而库通常不提交？
   (b) `cfg!(feature = "x")` 和 `#[cfg(feature = "x")]` 有什么区别，
   什么时候必须用后者？

（第 4、5 题是 24.2 与 24.4 讨论过的内容，这里不再单独给答案。）

### 参考答案

::: details 第 1 题

```rust
pub fn next_version(current: &str, change: Change) -> Option<String> {
    let (major, minor, patch) = parse_version(current)?;
    // 0.x 还在开发期：SemVer 允许次版本升级带破坏性改动，
    // 所以 0.4.3 破坏兼容时得到 0.5.0，而不是 1.0.0。
    let next = match (change, major) {
        (Change::Breaking, 0) => (0, minor + 1, 0),
        (Change::Breaking, _) => (major + 1, 0, 0),
        (Change::Feature, _) => (major, minor + 1, 0),
        (Change::Fix, _) => (major, minor, patch + 1),
    };
    Some(format!("{}.{}.{}", next.0, next.1, next.2))
}

#[test]
fn zero_major_breaking_change_bumps_minor() {
    assert_eq!(next_version("0.4.3", Change::Breaking), Some("0.5.0".into()));
    assert_eq!(next_version("0.4.3", Change::Feature), Some("0.5.0".into()));
    assert_eq!(next_version("1.4.3", Change::Breaking), Some("2.0.0".into()));
}
```

版本号规则里 `0.x` 是唯一一个「次版本可以破坏兼容」的区间，这也是为什么
`^0.4` 被 Cargo 解释成 `< 0.5.0`。

:::

::: details 第 2 题

清单（关键是 `optional = true` 与 `dep:` 前缀）：

```toml
[features]
default = []
chapter24-demo = []
chapter24-extras = ["dep:chrono"]

[dependencies]
chrono = { version = "0.4", optional = true }
```

代码里用属性而不是 `cfg!` 把依赖圈住，否则关掉 feature 时会因为 `chrono` 不在
依赖图里而编译失败：

```rust
#[cfg(feature = "chapter24-extras")]
pub fn extras_report() -> String {
    format!("extras 已启用，构建时间戳 {}", chrono::Utc::now())
}

#[cfg(not(feature = "chapter24-extras"))]
pub fn extras_report() -> String {
    String::from("extras 未启用（cargo run --features chapter24-extras 打开）")
}
```

`dep:chrono` 的写法让 feature 直接控制依赖，而不是再暴露一个同名的隐式 feature
——项目里的 feature 名字和依赖名字混在一起时，这是最容易踩的坑。

:::

::: details 第 3 题

实测对比（本仓库的 workspace 示例）：

```text
$ cargo test -p core-lib
running 1 test
test tests::trims_and_collapses_whitespace ... ok
test result: ok. 1 passed; ...

$ cargo test
running 0 tests      (cli-bin)
test result: ok. ...
running 1 test       (core-lib)
test result: ok. ...
```

差别就是**范围**：不带 `-p` 会遍历所有成员（包括没有测试的 `cli-bin`，
所以会看到一条 `running 0 tests`）；带 `-p` 只编译和运行指定成员。
工作区变大以后，开发时用 `-p` 加快反馈，提交前用完整命令做一次全量验证。

:::

## 24.11 小结

- 清单描述**意图**（范围、feature、profile），锁文件固定**结果**；应用提交
  `Cargo.lock` 并在 CI 用 `--locked`，库通常不提交。
- 一个包可以有多个目标（库、二进制、测试、基准）；用
  `cargo metadata --no-deps` 能一次性看清本仓库都编译出什么。
- `env!` 注入的 `CARGO_PKG_*` 来自清单，适合版本展示，不适合运行时配置。
- SemVer 的三条规则要记牢：破坏兼容升主版本并归零、兼容功能升次版本、
  修复升补丁；`0.x` 是特例，次版本可以带破坏性改动。
- feature 是编译期开关：`cfg!` 是表达式（两边都编译），`#[cfg]` 是属性
  （不满足就整段不编译），可选依赖必须用后者。
- profile 决定优化与断言；`panic = "abort"` 会让 `catch_unwind` 失效，和 FFI
  的错误码契约冲突，改之前先想清楚。
- workspace 用一份虚拟清单管多个 crate，共享锁文件和 `target/`；成员用路径
  依赖相连，`-p` 用来把操作限定到单个成员。
- 发布前先 `cargo package --list` 看文件清单，再 `cargo package` /
  `publish --dry-run`；版本号不可复用，`yank` 只能止损。
- 本仓库真实踩过的三个细节：bin 与 lib 同名会在 Windows 上触发
  `output filename collision`（已通过 `[[bin]]` 显式命名修好）；把 `-D warnings`
  跑通需要逐条处理历史 lint，而不是关掉规则；`cargo package` 会要求 `license`
  等元数据（这条仍然存在，真要发布就得补）。

下一章是**第 25 章 unsafe 与 FFI**：裸指针与不变量、把 `unsafe` 封在安全函数
里、`extern "C"` 的调用约定与结构体布局、panic 不穿过 FFI 边界，以及用 Python
`ctypes` 调用本仓库编译出的动态库。
