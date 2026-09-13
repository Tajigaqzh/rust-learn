# 第 14 章 · 模块、包与测试

前面十三章的知识都写在**一个文件一个模块**里：`src/rustNN_主题/mod.rs`，`main.rs` 里用 `mod` 声明、调用。这一章把这件事讲清楚，并补上另一半——**测试**。

工程化的内容可以分成三个层次：

| 层次 | 回答的问题 | 关键词 |
| --- | --- | --- |
| 模块 | 代码怎么分组、哪些能对外看见 | `mod`、`pub`、`use` |
| 包与工作空间 | 一个项目怎么组织成多个 crate | `Cargo.toml`、`[lib]`、workspace |
| 测试 | 怎么保证代码是对的 | `#[test]`、`cargo test`、单元/集成/文档测试 |

本章配套代码在 `src/rust14_modules_tests/mod.rs`（含单元测试），集成测试在 `tests/integration.rs`。执行 `cargo run` 看演示，`cargo test` 跑测试。

## 14.1 模块：把代码分组

模块就是「命名空间 + 可见性边界」。声明一个模块有两种写法：

```rust
mod geometry {            // 内联模块：直接写在大括号里
    pub fn area_of_rect(width: f64, height: f64) -> f64 {
        width * height
    }
}
```

```rust
mod geometry;             // 外部模块：去 src/geometry.rs 或 src/geometry/mod.rs 找内容
```

本仓库就是这么组织的：`src/` 下每个 `rustNN_主题/` 目录里有一个 `mod.rs`，`main.rs` 顶部逐一声明：

```rust
mod rust01_print;
mod rust02_variables;
// ...
mod rust14_modules_tests;
```

**模块可以嵌套**。本章示例的模块树是：

```text
crate（最外层的包）
└── rust14_modules_tests
    ├── geometry
    │   └── tests（只在测试时编译）
    └── text
```

`mod` 声明本身就构成一棵树，**访问路径就是沿着树走**。

## 14.2 路径：`crate`、`self`、`super`

引用模块里的东西用路径，和文件系统很像：

| 前缀 | 含义 | 类比 |
| --- | --- | --- |
| `crate::` | 从 crate 根开始 | 绝对路径 |
| `self::` | 当前模块 | 当前目录 |
| `super::` | 父模块 | 上级目录 |
| 直接写名字 | 相对于当前作用域，或从 `use` 引入的名字 | 相对路径 |

```rust
fn demo() {
    // 在 crate 根（main.rs）里，这几行等价
    crate::rust14_modules_tests::geometry::area_of_rect(3.0, 4.0);
    rust14_modules_tests::geometry::area_of_rect(3.0, 4.0);
}

mod geometry {
    fn helper() {}

    pub fn exported() -> f64 {
        self::helper();          // 同模块
        super::text::word_count("a b");   // 兄弟模块
        0.0
    }
}
```

实测演示里 `geometry::area_of_rect(3.0, 4.0) = 12`、`geometry::area_text(3.0, 4.0) = 12.00`。

## 14.3 可见性：默认私有

Rust 的默认规则很干脆：**一切默认私有**。私有项只在「定义它的模块及其子模块」里可见。

```rust
pub mod geometry {
    pub fn area_of_rect(width: f64, height: f64) -> f64 { width * height }

    fn round_to(value: f64, digits: u32) -> f64 { /* ... */ }   // 私有

    pub fn area_text(width: f64, height: f64) -> String {
        format!("{:.2}", round_to(area_of_rect(width, height), 2))   // 同模块，能用
    }
}
```

外面想调用 `geometry::round_to` 会报 `E0603: function 'round_to' is private`（见 14.12）。这正是**封装**：`area_text` 是对外的接口，`round_to` 是实现细节，改它不会影响调用方。

可见性有几种粒度：

| 写法 | 可见范围 |
| --- | --- |
| （不写） | 当前模块及其子模块 |
| `pub` | 任何地方 |
| `pub(crate)` | 仅当前 crate 内（对使用者不可见，但对内部模块是公开的） |
| `pub(super)` | 仅父模块 |
| `pub(in crate::path)` | 指定某个祖先模块内 |

`pub(crate)` 在真实项目里用得很多：**「整个项目内部共享，但不作为公开 API」**——既方便内部复用，又不会在版本升级时背上兼容性负担（第 24 章讲发布时会更清楚）。

## 14.4 `use` 与 `pub use`

路径太长时用 `use` 引入作用域：

```rust
use crate::rust14_modules_tests::geometry::area_of_rect;

println!("{}", area_of_rect(3.0, 4.0));
```

还有两个常用写法：

```rust
use crate::geometry::{area_text, area_of_rect};   // 一次引入多个
use std::collections::HashMap as Map;              // 起别名
use super::*;                                      // 测试模块里常见的「全引入」
```

**`pub use` 是重导出**：不仅引入，还把它变成当前模块公开 API 的一部分。本章示例在 crate 根做了一次：

```rust
pub use text::normalize;      // 外部可以直接用短路径，不必写 text::normalize
```

实测 `normalize("  rust   is  fun ") = [rust is fun]`、`text::word_count("rust is fun") = 3`。

这在库设计里很常见：内部按功能分层（`text::normalize`），对外提供一个扁平的、好用的路径（`normalize`）。
## 14.5 模块和文件怎么对应

一个模块的内容可以写在三个地方，效果一样：

| 写法 | 位置 | 说明 |
| --- | --- | --- |
| 内联 | `mod foo { ... }` | 就地写在父文件里 |
| 同名文件 | `src/foo.rs` | 现在推荐的写法 |
| 目录 | `src/foo/mod.rs` | 旧写法，仍然合法 |

子模块则是「目录 + 文件」：

```text
src/
├── main.rs            声明 mod foo;
├── foo.rs             模块 foo
└── foo/
    └── bar.rs         模块 foo::bar（在 foo.rs 里写 mod bar;）
```

本仓库用的是目录形式：`src/rust14_modules_tests/mod.rs`。之所以选它，是因为**每章一个目录**，将来想给某一章拆分文件时，直接在目录里加 `helpers.rs` 之类即可，不用改 `main.rs`。

`mod xxx;` 却找不到文件时，编译器会明确告诉你该建哪个文件：

```text
error[E0583]: file not found for module `missing_module`
 --> src/main.rs:1:1
  |
1 | mod missing_module;
  | ^^^^^^^^^^^^^^^^^^^
  |
  = help: to create the module `missing_module`, create file "missing_module.rs" or "missing_module\mod.rs"
  = note: if there is a `mod missing_module` elsewhere in the crate already, import it with `use crate::...` instead
```

## 14.6 一个 crate 的结构

一个「包」（package）由 `Cargo.toml` 描述，可以包含一个库和多个二进制目标：

```text
rust-learn/
├── Cargo.toml         包的清单：名字、版本、依赖、目标
├── src/
│   ├── main.rs        二进制入口（我们的项目）
│   └── lib.rs         库入口（也可以同时有）
├── tests/             集成测试，每个文件是一个独立 crate
├── examples/          cargo run --example xxx
└── benches/           cargo bench（第 23 章）
```

几个对应关系值得记：

| 文件 | 编译成什么 | 测试怎么跑 |
| --- | --- | --- |
| `src/main.rs` | 可执行文件 | 文件里的 `#[cfg(test)]` 会被当成单元测试 |
| `src/lib.rs` | 库 | 单元测试 + 文档测试 |
| `tests/*.rs` | 独立的测试 crate | `cargo test` 时每个文件单独编译运行 |

**我们的项目是纯二进制 crate，没有 `lib.rs`**，所以文档注释里的代码块不会作为文档测试执行（文档测试只对库目标跑）；集成测试也没法 `use rust_learn::...`，只能运行编译出来的程序——`tests/integration.rs` 就是这么做的。

`Cargo.toml` 里最简单的形态：

```toml
[package]
name = "rust-learn"
version = "0.1.0"
edition = "2024"

[dependencies]
```

## 14.7 workspace：多个 crate 一起管理

项目变大之后，通常会拆成多个 crate，用一个 **workspace** 把它们放在同一棵树下，共享 `Cargo.lock` 和 `target/`：

```toml
[workspace]
members = ["core", "cli", "web"]
resolver = "3"
```

```text
my-project/
├── Cargo.toml         只有 [workspace]
├── Cargo.lock         整个工作空间共用一份
├── core/              一个库 crate
├── cli/               依赖 core：core = { path = "../core" }
└── web/
```

什么时候值得拆 workspace？

| 情况 | 说明 |
| --- | --- |
| 库 + 命令行工具 | 逻辑放库里（可测试、可复用），CLI 只负责参数和输出 |
| 多个二进制 | 比如 `server` 和 `admin` 共享同一套模型 |
| 需要分开发布 | 每个 crate 独立版本号（第 24 章） |
| 编译太慢 | 改一个 crate 不必重编全部 |

反过来，**小项目一个 crate 就够了**。拆分的成本是路径变长、依赖关系要维护——本项目目前就只有一个包，单个 `main.rs` 加若干章节模块。
## 14.8 单元测试：和代码写在一起

单元测试放在**被测代码所在文件的末尾**，用 `#[cfg(test)]` 包起来：

```rust
#[cfg(test)]
mod tests {
    use super::*;                 // 引入父模块里的东西

    #[test]
    fn area_of_rect_works() {
        assert_eq!(geometry::area_of_rect(3.0, 4.0), 12.0);
    }
}
```

两个要点：

1. **`#[cfg(test)]` 表示「只有 `cargo test` 时才编译这个模块」**，正常构建里它不存在，不占体积、不影响运行。
2. **测试模块是被测模块的子模块，所以能访问私有项**——这是它和集成测试最大的区别。本章示例就在 `geometry` 里放了一个测试，专门验证私有函数 `round_to`：

```rust
pub mod geometry {
    fn round_to(value: f64, digits: u32) -> f64 { /* 私有 */ }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn private_helper_is_reachable_inside_module() {
            assert_eq!(round_to(1.2345, 2), 1.23);   // 子模块能访问父模块的私有项
        }
    }
}
```

**测试私有函数是单元测试的独特价值**：公开接口往往只覆盖几条主路径，内部函数的边界条件（空输入、溢出、舍入）才是 bug 高发区。

## 14.9 断言宏与 `#[should_panic]`

常用的断言：

```rust
assert!(condition);                       // 条件为假就失败
assert_eq!(left, right);                  // 相等，失败时打印两边的值
assert_ne!(left, right);                  // 不相等
assert!(value > 0, "值必须为正，实际是 {value}");   // 自定义消息
```

**`assert_eq!` 比 `assert!(a == b)` 好**：失败时它会打印两个值，一眼就能看出差在哪。

「应该 panic」的测试用属性标注：

```rust
#[test]
#[should_panic(expected = "除数不能为 0")]
fn division_by_zero_panics() {
    let _ = divide(1, 0);
}
```

`expected` 是子串匹配：panic 消息里包含这段文字才算通过。**一定要写 `expected`**，否则只要因为任何原因 panic 就算通过——包括你自己断言写错导致的 panic。

另外两个实用属性：

- `#[ignore]`：默认跳过（比如很慢的测试），需要时用 `cargo test -- --ignored` 单独跑。
- 测试函数**不能带参数**，带了会直接编译失败：

```text
error: functions used as tests can not have any arguments
 --> src/main.rs:2:1
  |
2 | fn bad(arg: i32) {}
  | ^^^^^^^^^^^^^^^^^^^
```

## 14.10 集成测试与文档测试

**集成测试**放在 `tests/` 目录，每个文件会被编译成一个**独立的 crate**，只能使用你提供的**公开 API**——这种限制正是它的价值：它验证「从外部看，这个东西好不好用」。

我们的项目是二进制 crate，没有库 API 可用，于是换个角度：**把编译出来的程序当黑盒**。Cargo 在跑测试时会提供环境变量 `CARGO_BIN_EXE_<二进制名>`，指向刚编译好的可执行文件：

```rust
use std::process::Command;

#[test]
fn binary_runs_and_prints_every_chapter() {
    let output = Command::new(env!("CARGO_BIN_EXE_rust-learn"))
        .output()
        .expect("运行二进制失败");

    assert!(output.status.success(), "程序退出码不是 0");

    let stdout = String::from_utf8_lossy(&output.stdout);
    for marker in ["rust01_print", "rust13_concurrency", "rust14_modules_tests"] {
        assert!(stdout.contains(marker), "输出里找不到章节标记 {marker}");
    }
}
```

实测这个集成测试通过——顺带验证了**全部十四章的演示代码能一起跑通**，比手工敲 `cargo run` 更可靠。

**文档测试**是 Rust 的特色：文档注释里的代码块会被 `cargo test` 编译并运行（只对库目标生效）：

```rust
/// 两数相加。
///
/// # Examples
///
/// ```
/// use demo::add;
/// assert_eq!(add(2, 3), 5);
/// ```
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}
```

好处是**文档永远不会过期**：示例写错了，`cargo test` 会失败。本项目目前没有库目标，所以文档测试暂时用不上；等以后加了 `lib.rs`，章节里的示例就能直接变成文档测试。
## 14.11 `cargo test` 的输出

本仓库执行 `cargo test` 的真实输出（节选）：

```text
     Running unittests src\main.rs (target\debug\deps\rust_learn-....exe)

running 8 tests
test rust14_modules_tests::tests::area_of_rect_works ... ok
test rust14_modules_tests::tests::area_text_rounds_to_two_digits ... ok
test rust14_modules_tests::tests::normalize_collapses_spaces ... ok
test rust14_modules_tests::tests::normalize_keeps_single_word ... ok
test rust14_modules_tests::tests::word_count_counts_words ... ok
test rust14_modules_tests::tests::division_by_zero_panics - should panic ... ok
test rust14_modules_tests::geometry::tests::private_helper_is_reachable_inside_module ... ok
test rust14_modules_tests::geometry::tests::area_of_rect_handles_zero ... ok

test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests\integration.rs (target\debug\deps\integration-....exe)

running 3 tests
test binary_prints_error_chapter_marker ... ok
test binary_reports_finish_markers ... ok
test binary_runs_and_prints_every_chapter ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
```

输出分成几块，对应不同的测试目标：先是 `src/main.rs` 里的单元测试，然后是 `tests/integration.rs`。如果项目有 `lib.rs`，还会多一块 **Doc-tests**——下面是另一个小型库 crate 的输出，能看到三种测试一起跑的样子：

```text
running 4 tests
test tests::slow_test ... ignored                                     ← #[ignore]
test tests::adds_two_numbers ... ok
test tests::divides_by_zero_returns_none ... ok
test tests::panics_with_message - should panic ... ok                 ← #[should_panic]

test result: ok. 3 passed; 0 failed; 1 ignored; ...

running 2 tests                                                        ← tests/integration.rs
test divides ... ok
test adds_across_crates ... ok

test result: ok. 2 passed; ...

   Doc-tests demo
running 1 test
test src\lib.rs - add (line 7) ... ok                                  ← /// 里的示例

test result: ok. 1 passed; ...
```

常用参数：

| 命令 | 作用 |
| --- | --- |
| `cargo test 名字片段` | 只跑名字里含该片段的测试（比如 `cargo test normalize`） |
| `cargo test -- --nocapture` | 显示测试里的 `println!` 输出（默认会被捕获） |
| `cargo test -- --ignored` | 只跑被 `#[ignore]` 标记的测试 |
| `cargo test -- --test-threads=1` | 串行执行（测试之间有共享状态时用） |
| `cargo test --test integration` | 只跑 `tests/integration.rs` |
| `cargo test --lib` / `--bins` | 只跑库 / 二进制目标里的测试 |

`--` 之后的内容会传给测试运行器本身，这是容易搞混的地方：**`cargo test` 的参数给 Cargo，`--` 之后的给测试程序**。

## 14.12 四个真实报错怎么读

**案例 1：访问私有项（`E0603`）**

```text
error[E0603]: function `secret` is private
 --> src/main.rs:8:27
  |
8 |     println!("{}", inner::secret());
  |                           ^^^^^^ private function
  |
note: the function `secret` is defined here
 --> src/main.rs:2:5
  |
2 |     fn secret() -> i32 {
  |     ^^^^^^^^^^^^^^^^^^
```

`note` 会指出定义的位置，方便你决定「到底是该加 `pub`，还是我不该调用它」。

**案例 2：路径写错（`E0432`）**

```text
error[E0432]: unresolved import `crate::does_not_exist`
 --> src/main.rs:1:12
  |
1 | use crate::does_not_exist::Thing;
  |            ^^^^^^^^^^^^^^ could not find `does_not_exist` in the crate root
```

多半是模块名拼错，或者**忘了在 `main.rs` / 父模块里写 `mod does_not_exist;`**。

**案例 3：`mod` 找不到文件（`E0583`）**

```text
error[E0583]: file not found for module `missing_module`
  |
  = help: to create the module `missing_module`, create file "missing_module.rs" or "missing_module\mod.rs"
```

按 `help` 建文件就行。

**案例 4：测试函数带参数**

```text
error: functions used as tests can not have any arguments
  |
2 | fn bad(arg: i32) {}
  | ^^^^^^^^^^^^^^^^^^^
```

测试函数必须是零参数的；需要参数化的场景用「循环调用同一个断言函数」或参数化测试库（比如 `rstest`）。

## 14.13 常见坑速查

| 现象 | 原因 | 解法 |
| --- | --- | --- |
| 外部模块访问不了内部函数 | 默认私有 | 加 `pub`，或改成 `pub(crate)`（内部共享） |
| `use` 报 `unresolved import` | 路径错，或忘了声明 `mod` | 检查模块树，先 `mod xxx;` 再 `use` |
| `mod` 报「file not found」 | 文件放错位置 | `src/xxx.rs` 或 `src/xxx/mod.rs` |
| 测试模块里访问不到被测函数 | 测试写在了子模块外面 | `use super::*;` 或把测试放进对应模块 |
| 集成测试里 `use crate::...` 失败 | 集成测试是独立 crate | 只能访问公开 API，二进制则用 `CARGO_BIN_EXE_*` |
| `#[should_panic]` 明明没 panic 却通过 | 忘了写 `expected`，被别的 panic 骗过 | 写上 `expected = "..."` |
| 测试里的 `println!` 看不到 | 测试输出默认被捕获 | `cargo test -- --nocapture` |
| 文档里的示例从来不被执行 | 项目没有 `lib.rs`（文档测试只跑库） | 加库目标，或把示例放进单元测试 |
| `cargo test` 很慢 | 集成测试反复启动二进制 | 合并断言到同一个测试，或标记 `#[ignore]` |
| 模块越拆越多、路径越来越长 | 缺少重导出 | 用 `pub use` 在上级模块露出短路径 |

## 14.14 练习

1. 给本章模块加一个 `stats` 子模块：`pub fn mean(values: &[f64]) -> Option<f64>`，内部用私有的 `fn sum(&[f64]) -> f64`；写三个单元测试覆盖「空切片返回 `None`」「正常求平均」「私有 `sum` 正确」。
2. 用 `pub(crate)` 暴露一个内部函数，并说明它和 `pub` 的区别：什么情况下内部模块能用、外部使用者能不能用。
3. 写一个 `#[should_panic(expected = "...")]` 的测试，验证「除以 0 会 panic」。
4. 在 `tests/` 里加一个集成测试，断言程序输出里包含「错误处理演示结束」。
5. 回答两个问题：(a) 集成测试为什么不能测试私有函数？(b) 你想测私有函数时有哪些办法？

（第 2、3 题是 14.3 和 14.9 示例的直接变体，这里不再单独给答案。）

### 参考答案

::: details 第 1 题

```rust
pub mod stats {
    pub fn mean(values: &[f64]) -> Option<f64> {
        if values.is_empty() {
            return None;
        }
        Some(sum(values) / values.len() as f64)
    }

    fn sum(values: &[f64]) -> f64 {
        let mut total = 0.0;
        for value in values {
            total += value;
        }
        total
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn mean_of_empty_is_none() {
            assert_eq!(mean(&[]), None);
        }

        #[test]
        fn mean_works() {
            assert_eq!(mean(&[1.0, 2.0, 3.0]), Some(2.0));
        }

        #[test]
        fn private_sum_works() {
            assert_eq!(sum(&[1.0, 2.0]), 3.0);   // 子模块能访问父模块的私有项
        }
    }
}
```

实测 `cargo test` 输出 `3 passed; 0 failed`。注意 `mean(&[])` 返回 `None` 而不是 `NaN`——空集合的平均值没有定义，用 `Option` 表达比返回 `0.0` 或 `NaN` 都更诚实（第 6 章和第 8 章的选择）。

:::

::: details 第 4 题

```rust
use std::process::Command;

#[test]
fn binary_prints_error_chapter_marker() {
    let output = Command::new(env!("CARGO_BIN_EXE_rust-learn"))
        .output()
        .expect("运行二进制失败");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("错误处理演示结束"));
}
```

实测通过（本仓库的 `tests/integration.rs` 里就有这个测试）。写这类集成测试的要点：断言**稳定的标记**（章节名、结束语），而不是具体的行数或输出顺序——后者会随着章节更新频繁失效。

:::

::: details 第 5 题

(a) 集成测试放在 `tests/` 目录，每个文件会被编译成**独立的 crate**，它只能看到你的**公开 API**。私有函数没有出现在公开 API 里，所以集成测试看不到——这不是限制，而是设计意图：**集成测试就应该从「外部使用者」的视角验证行为**。

(b) 想测私有函数有三种办法：

1. **单元测试**：在被测模块内部写 `#[cfg(test)] mod tests`，用 `use super::*;` 拿到私有项（本章 `geometry::tests` 的写法）。
2. **间接测试**：通过公开函数覆盖私有函数的各种输入路径——如果某条路径无论如何都覆盖不到，可能说明它该被删掉或重构。
3. **改动可见性**：真的需要外部测试时，把它改成 `pub(crate)`——但先想清楚这是「测试需要」还是「设计需要」。

:::

## 14.15 小结

- `mod` 既是命名空间也是可见性边界；默认**一切私有**，`pub` / `pub(crate)` / `pub(super)` 按需放开。

- 路径用 `crate::`（绝对）、`self::`（当前）、`super::`（父级）；`use` 引入作用域，`pub use` 重导出、缩短外部路径。

- 模块可以内联、放同名 `.rs` 文件，或放 `模块名/mod.rs`；子模块靠目录组织。本项目用「一章一个目录」的形式。

- 一个包由 `Cargo.toml` 描述，可以同时有 `src/main.rs`（二进制）和 `src/lib.rs`（库）；`tests/`、`examples/`、`benches/` 各有约定用途。

- workspace 用一个顶层 `Cargo.toml` 管理多个 crate，共享 `Cargo.lock` 和 `target/`；小项目不必拆。

- 单元测试写在 `#[cfg(test)] mod tests` 里，是子模块**能访问私有项**，适合测内部边界条件。

- 断言优先用 `assert_eq!` / `assert_ne!`（失败时打印两边的值）；`#[should_panic(expected = "...")]` 一定要写 `expected`。

- 集成测试放 `tests/`，每个文件是独立 crate，只能访问公开 API；二进制 crate 可以用 `CARGO_BIN_EXE_<名字>` 把它当黑盒跑。

- 文档测试让 `///` 里的示例跟着 `cargo test` 一起跑，保证文档不过期（只对库目标生效）。

- `cargo test` 的常用参数：按名字过滤、`-- --nocapture`、`-- --ignored`、`-- --test-threads=1`；`--` 后面才是给测试运行器的参数。

下一章讲**宏**：`macro_rules!` 怎么定义、`println!` 这类宏为什么带感叹号、重复匹配（`$(...)`）怎么写，以及过程宏能做什么。
