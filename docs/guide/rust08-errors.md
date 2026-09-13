# 第 8 章 · 错误处理

第 6 章的 `Option` 处理「可能没有值」，这一章处理「可能失败」。

Rust 把错误分成泾渭分明的两类：

| | 可恢复错误 | 不可恢复错误 |
| --- | --- | --- |
| 代表 | `Result<T, E>` | `panic!` |
| 典型场景 | 文件不存在、输入格式不对、网络超时 | 数组越界、断言失败、`unwrap()` 碰到 `None` |
| 谁来处理 | **调用方**，必须显式处理 | 程序直接结束（默认行为） |
| 适合谁 | 库和应用的主体逻辑 | 程序自身写错了、无法补救的情况 |

这套划分是 Rust 稳定性的一部分：**只要一个函数返回 `Result`，调用方就不可能「忘了」处理错误**——`Result` 带 `#[must_use]`，忽略它编译器会警告；想拿到里面的值就必须显式写出「失败时怎么办」。

本章的重点是 `Result`、`?` 运算符和自定义错误类型，最后会介绍 `thiserror` / `anyhow` 这两个生态里最常用的错误处理库。

本章配套代码在 `src/rust08_errors/mod.rs`，执行 `cargo run` 可以看到全部输出。

## 8.1 `panic!`：什么时候该用

`panic!` 会立刻结束当前线程，把栈上的值依次 `drop`（第 5 章的规则在这里生效），然后打印一条信息：

```rust
let v: Vec<i32> = Vec::new();
v.first().unwrap();
```

```text
thread 'main' panicked at src/main.rs:3:30:
called `Option::unwrap()` on a `None` value
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
```

`expect()` 是自己带消息的 `unwrap()`，消息会出现在 panic 输出里：

```rust
let n: i32 = "abc".parse().expect("端口号必须是数字");
```

```text
thread 'main' panicked at src/main.rs:2:32:
端口号必须是数字: ParseIntError { kind: InvalidDigit }
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
```

（新版本 rustc 的 panic 头里还会带线程编号，写成 `thread 'main' (12345) panicked at ...`，那串数字每次不一样。）

哪些情况 panic 是合理的？

- **程序有 bug**：数组越界、`unwrap()` 碰到 `None`、`assert!` 失败——这些说明代码本身的假设不成立，早崩早发现。
- **启动阶段的致命错误**：配置文件完全读不到、必需的端口被占用，继续跑也没意义。
- **原型和测试**：先 `unwrap()` 把逻辑跑通，之后再逐步换成 `Result`。

反过来，**库代码不应该因为可预期的失败而 panic**。如果函数是给别人调用的，调用方没法接住你的 panic（默认情况下），你能提供的只有「崩溃」这一个选项。返回 `Result` 则把选择权交出去。

一个对比：

```rust
// 不好：除数为 0 是可预期的输入问题，却直接 panic
fn divide_panic(a: i32, b: i32) -> i32 {
    a / b
}

// 好：把「失败」写进类型，调用方自己决定怎么办
fn divide(a: i32, b: i32) -> Result<i32, String> {
    if b == 0 {
        Err(String::from("除数不能为 0"))
    } else {
        Ok(a / b)
    }
}
```

## 8.2 `Result<T, E>` 就是普通枚举

第 6 章说过 `Option` 是标准库里的枚举，`Result` 也一样：

```rust
enum Result<T, E> {
    Ok(T),
    Err(E),
}
```

类型签名读法：`Result<u16, ParseIntError>` 表示「成功时得到一个 `u16`，失败时得到一个 `ParseIntError`」。两个类型参数各司其职——**错误类型也是类型**，调用方可以据此分别处理，而不只是拿到一句字符串。

标准库里常见的错误类型：

| 错误类型 | 来自哪里 |
| --- | --- |
| `std::io::Error` | 文件、网络、标准输入输出 |
| `std::num::ParseIntError` | `"abc".parse::<i32>()` |
| `std::fmt::Error` | 格式化输出（`write!` 到 writer 失败） |
| `std::env::VarError` | 环境变量不存在或不是 Unicode |

## 8.3 用 `match` 完整处理

最直接的处理方式还是第 6 章的 `match`：

```rust
match divide(9, 3) {
    Ok(value) => println!("9 / 3 = {value}"),
    Err(message) => println!("出错了：{message}"),
}
```

实测：`9 / 3 = 3`；换成 `divide(9, 0)` 则输出 `出错了：除数不能为 0`。

`Result` 也有和第 6 章 `Option` 类似的那套方法：

| 方法 | 含义 |
| --- | --- |
| `is_ok()` / `is_err()` | 只判断成败 |
| `ok()` / `err()` | 转成 `Option` |
| `map(f)` | 成功时对值做变换 |
| `map_err(f)` | **失败时对错误做变换** |
| `and_then(f)` | 成功时接着做另一个可能失败的操作 |
| `unwrap_or(v)` / `unwrap_or_else(f)` | 失败时给默认值 |

实测 `is_ok / is_err = true / true`（分别是 `Ok(7)` 和 `Err("坏掉了")` 两个值）。

## 8.4 `unwrap` 家族：什么时候可以用

| 写法 | 成功时 | 失败时 |
| --- | --- | --- |
| `.unwrap()` | 拿到值 | panic，信息里只有结构化的错误 |
| `.expect("msg")` | 拿到值 | panic，带上你写的消息 |
| `.unwrap_or(v)` | 拿到值 | 用 `v` 顶上 |
| `.unwrap_or_else(\|e\| ...)` | 拿到值 | 用闭包算出一个值顶上 |
| `.unwrap_or_default()` | 拿到值 | 用类型的默认值顶上 |

实测：

```text
    unwrap = 7
    unwrap_or 兜底 = -1
    unwrap_or_else 兜底 = 9
```

（最后一行是 `Err("坏掉了")` 的 `unwrap_or_else`，闭包返回错误的字符数 9。）

选择标准很简单：**失败时程序还能继续吗？**

- 能继续 → `unwrap_or` / `unwrap_or_else` / `unwrap_or_default`，或者 `match` / `?`。
- 不能继续，而且这是代码 bug → `expect("说清楚为什么不可能失败")`。用 `expect` 而不是 `unwrap`，因为半年后崩溃日志里的那句话能救你一次。
- 只是测试或临时验证 → `unwrap()` 无所谓。

## 8.5 `?` 运算符：把错误往上抛

`?` 是 Rust 错误处理的核心语法糖。写在 `Result` 后面时，它做两件事：

1. 如果是 `Ok(v)`，把 `v` 取出来继续执行；
2. 如果是 `Err(e)`，**立刻从当前函数返回 `Err(e)`**。

```rust
fn parse_port(text: &str) -> Result<u16, ConfigError> {
    let raw = read_field(text, "port")?;   // 找不到就直接返回 Err
    let port: u16 = raw.parse()?;          // 解析失败就直接返回 Err
    Ok(port)
}
```

上面两行 `?` 展开成 `match` 会是这样：

```rust
let raw = match read_field(text, "port") {
    Ok(value) => value,
    Err(e) => return Err(e),
};
```

所以 `?` 不只是「短」，它还保证了**错误一定会被传播，不会被忘掉**。

实测三种情况：

```text
    合法配置：Ok(8080)
    端口不是数字：配置项 `value` 不是数字：invalid digit found in string
    缺少端口：缺少配置项 `port`
```

第二、三行来自 `.unwrap_err()`——为了在演示里打印错误，测试和生产代码里不该这么写。

两个使用前提：

- **`?` 只能用在返回 `Result` 或 `Option` 的函数里**。写在返回 `()` 的函数里报 `E0277`（见 8.13 案例 1），`main` 也不例外——除非把 `main` 的签名改成 `Result`（见 8.8）。
- **错误类型要能转换过去**。`raw.parse()?` 产生的是 `ParseIntError`，函数声明的是 `ConfigError`，两者能接上是因为实现了 `From`（下一节）。

## 8.6 `?` 的转换规则：`From`

`?` 在返回错误前，会先做一次转换：**`Err(e)` 会被换成 `Err(From::from(e))`**。也就是说，只要实现了 `From<源错误> for 目标错误`，`?` 就能自动把源错误「升级」成函数声明的错误类型。

```rust
impl From<ParseIntError> for ConfigError {
    fn from(source: ParseIntError) -> Self {
        ConfigError::NotANumber {
            key: String::from("value"),
            source,
        }
    }
}
```

有了这个实现，`raw.parse()?` 才能把 `ParseIntError` 变成 `ConfigError` 返回出去。**一个函数里可以混用多种错误源**，只要它们都能 `From` 到同一个错误类型：

```rust
fn load(text: &str) -> Result<Config, ConfigError> {
    let port = read_field(text, "port")?;   // ConfigError
    let port: u16 = port.parse()?;          // ParseIntError -> ConfigError
    let host = read_field(text, "host")?;   // ConfigError
    Ok(Config { host: host.to_string(), port })
}
```

没有 `From` 就会报 `E0277`（见 8.13 案例 2），`help` 里会写清楚「`From<X> for Y` 没实现」。

## 8.7 自定义错误类型

标准库的错误类型只能描述它自己的问题。应用里的错误，最好定义成自己的枚举——这样调用方能区分「缺少配置」和「配置格式不对」，而不是拿到一句无法判断的字符串。

一个完整的自定义错误需要三块：

```rust
use std::error::Error;
use std::fmt;

#[derive(Debug)]
enum ConfigError {
    MissingKey(String),
    NotANumber { key: String, source: std::num::ParseIntError },
}

// 1. Display：给人看的错误描述
impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::MissingKey(key) => write!(f, "缺少配置项 `{key}`"),
            ConfigError::NotANumber { key, source } => {
                write!(f, "配置项 `{key}` 不是数字：{source}")
            }
        }
    }
}

// 2. Error：接入标准库的错误体系，source() 说明「这个错误是谁引起的」
impl Error for ConfigError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            ConfigError::NotANumber { source, .. } => Some(source),
            ConfigError::MissingKey(_) => None,
        }
    }
}

// 3. Debug：derive 就够了，错误类型必须实现它
```

三者的分工：`Debug` 给开发者（`{:?}`），`Display` 给用户（`{}`），`Error` 让这个类型能被 `Box<dyn Error>` 装起来、能参与错误链。

实测一条错误信息：

```text
    错误描述 = 配置项 `value` 不是数字：invalid digit found in string
    Debug 输出 = NotANumber { key: "value", source: ParseIntError { kind: InvalidDigit } }
    底层错误 = invalid digit found in string
```

注意最后两行的区别：`Debug` 把整个结构打出来，而 `source()` 只返回那条被包裹的 `ParseIntError`。**错误链**就是这么一层层串起来的——上层错误说「哪个配置项有问题」，下层错误说「具体哪里不对」。真要看完整的链，用 `std::error::Error` 的 `chain()`，或者直接上 `anyhow`（8.12）。

经验上：**库**应该定义自己的错误枚举（调用方需要区分错误种类）；**应用**用 `Box<dyn Error>` 或 `anyhow::Error` 更省事。

## 8.8 `main` 也能返回 `Result`

`main` 可以声明成返回 `Result`，这样里面就能用 `?`：

```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let text = std::fs::read_to_string("config.txt")?;
    println!("读到 {} 字节", text.len());
    Ok(())
}
```

失败时进程返回非 0 退出码，并且会把错误打印到 stderr——对命令行工具来说正好是想要的行为。这也是编译器在 `?` 用错时的建议写法（见 8.13 案例 1）。

## 8.9 `Box<dyn Error>`：应用里的省事写法

如果一个函数可能返回好几种错误，又不需要调用方区分，可以让它们统一装进 `Box<dyn Error>`：

```rust
use std::error::Error;

fn double(text: &str) -> Result<i32, Box<dyn Error>> {
    let n: i32 = text.trim().parse()?;   // ParseIntError 自动装箱
    Ok(n * 2)
}
```

实测 `double(" 21 ") = Ok(42)`、`double("abc")` 的错误消息是 `invalid digit found in string`。`?` 能自动转换是因为标准库为 `Box<dyn Error>` 实现了 `From<E>`（只要 `E: Error`）。

代价是：**错误类型被擦除了**。调用方拿到的是 `Box<dyn Error>`，没法用 `match` 区分到底是哪种错误，只能看消息字符串。所以这套写法适合「错误一路打印出来给人看」的程序入口，不适合需要按错误类型分支的库代码。

## 8.10 `Option` 与 `Result` 互转

`Option` 表示「没有」，`Result` 表示「失败」，两者经常需要转换：

| 方向 | 方法 | 说明 |
| --- | --- | --- |
| `Option<T>` → `Result<T, E>` | `.ok_or(e)` | 没有值时给出错误（`e` 马上求值） |
| | `.ok_or_else(\|\| e)` | 同上，错误靠闭包现算（推荐，省一次构造） |
| `Result<T, E>` → `Option<T>` | `.ok()` | 失败信息丢掉 |
| `Result<T, E>` → `Option<E>` | `.err()` | 只留错误 |

实测：

```text
    find_user(ada) = Some(0)
    ok_or 之后 = Err("没有这个人")
    ok_or_else 之后 = Err("名单 2 里没有 grace")
    Ok(3).ok() = Some(3)
```

`ok_or_else` 里的闭包**只在需要的时候才执行**，所以格式化字符串这类有成本的操作应该放进去：

```rust
let found = find_user(&users, "grace")
    .ok_or_else(|| format!("名单 {} 里没有 grace", users.len()))?;
```

反过来，`?` 用在返回 `Result` 的函数里碰到 `Option` 会报错（见 8.13 案例 3），因为 `Option` 里没有「错误是什么」这个信息——必须先用 `ok_or` 补上。

## 8.11 被忽略的 `Result` 会被警告

`Result` 标了 `#[must_use]`，直接丢掉会有警告：

```rust
fn main() {
    "42".parse::<i32>();
}
```

```text
warning: unused `Result` that must be used
 --> src/main.rs:2:5
  |
2 |     "42".parse::<i32>();
  |     ^^^^^^^^^^^^^^^^^^^
  |
  = note: this `Result` may be an `Err` variant, which should be handled
  = note: `#[warn(unused_must_use)]` (part of `#[warn(unused)]`) on by default
help: use `let _ = ...` to ignore the resulting value
  |
2 |     let _ = "42".parse::<i32>();
  |     +++++++
```

这条警告是 Rust 错误处理体验的关键一环：**忘记处理错误，编译器会提醒你**。如果确实想忽略（比如「失败了我也不在乎」），显式写 `let _ = ...`，把「我知道我在忽略」写进代码。

## 8.12 生态：`thiserror` 和 `anyhow`

手写 `Display` + `Error` + `From` 有点啰嗦，社区的两个库把它们变成了声明式的：

```rust
// thiserror：给库作者用，定义错误类型
use thiserror::Error;

#[derive(Error, Debug)]
enum ConfigError {
    #[error("缺少配置项 `{0}`")]
    MissingKey(String),
    #[error("配置项 `{key}` 不是数字")]
    NotANumber { key: String, #[source] source: std::num::ParseIntError },
}
```

```rust
// anyhow：给应用作者用，统一错误类型
use anyhow::{Context, Result};

fn load(path: &str) -> Result<Config> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("读取配置 {path} 失败"))?;
    // ...
    Ok(config)
}
```

两个库的分工是社区共识：**库用 `thiserror`（错误类型明确、可匹配），应用用 `anyhow`（省事、错误链清晰）**。它们不是变魔术，只是把本章手写的东西自动化了——所以先理解 `From`、`Display`、`source()` 这套机制，用起它们来才不迷糊。

## 8.13 五个真实报错怎么读

**案例 1：在返回 `()` 的函数里用 `?`（`E0277`）**

```rust
use std::fs::File;

fn main() {
    let f = File::open("x.txt")?;
    println!("{f:?}");
}
```

```text
error[E0277]: the `?` operator can only be used in a function that returns `Result` or `Option` (or another type that implements `FromResidual`)
 --> src/main.rs:4:32
  |
3 | fn main() {
  | --------- this function should return `Result` or `Option` to accept `?`
4 |     let f = File::open("x.txt")?;
  |                                ^ cannot use the `?` operator in a function that returns `()`
  |
help: consider adding return type
  |
3 ~ fn main() -> Result<(), Box<dyn std::error::Error>> {
4 |     let f = File::open("x.txt")?;
5 |     println!("{f:?}");
6 +     Ok(())
  |
```

`help` 直接把 `main` 该改成什么样写好了。`?` 的本质是「提前 return」，所以当前函数的返回类型必须能装下这个错误。

**案例 2：错误类型转换不过去（`E0277`）**

```rust
use std::fs::File;

fn read() -> Result<(), String> {
    let f = File::open("x.txt")?;
    println!("{f:?}");
    Ok(())
}
```

```text
error[E0277]: `?` couldn't convert the error to `String`
 --> src/main.rs:4:32
  |
3 | fn read() -> Result<(), String> {
  |              ------------------ expected `String` because of this
4 |     let f = File::open("x.txt")?;
  |             -------------------^ the trait `From<std::io::Error>` is not implemented for `String`
  |             |
  |             this can't be annotated with `?` because it has type `Result<_, std::io::Error>`
  |
  = note: the question mark operation (`?`) implicitly performs a conversion on the error value using the `From` trait
```

错误信息把机制说得很清楚：`?` 会隐式调用 `From`。要么实现 `From<io::Error> for String`（不推荐，`String` 是外部类型，孤儿规则不允许），要么把返回类型改成 `Box<dyn Error>` 或自定义错误枚举。

**案例 3：返回 `Result` 的函数里对 `Option` 用 `?`（`E0277`）**

```text
error[E0277]: the `?` operator can only be used on `Result`s, not `Option`s, in a function that returns `Result`
 --> src/main.rs:6:35
  |
5 | fn total(values: &[i32]) -> Result<i32, String> {
  | ----------------------------------------------- this function returns a `Result`
6 |     let first = first_even(values)?;
  |                                   ^ use `.ok_or(...)?` to provide an error compatible with `Result<i32, String>`
```

`Option` 里没有错误信息，编译器直接给出了补法：`first_even(values).ok_or("没有偶数")?`。

**案例 4：反过来，返回 `Option` 的函数里对 `Result` 用 `?`（`E0277`）**

```text
error[E0277]: the `?` operator can only be used on `Option`s, not `Result`s, in a function that returns `Option`
 --> src/main.rs:3:30
  |
1 | fn first_even(values: &[i32]) -> Option<i32> {
  | -------------------------------------------- this function returns an `Option`
3 |     let n: i32 = text.parse()?;
  |                              ^ use `.ok()?` if you want to discard the `Result<Infallible, _>` error information
```

两种转换方向都能靠 `ok_or` / `ok` 补上，编译器也都会直接告诉你用哪个。

**案例 5：实现 `Error` 却忘了 `Display` / `Debug`（`E0277`）**

```rust
struct MyError;

impl std::error::Error for MyError {}
```

```text
error[E0277]: `MyError` doesn't implement `std::fmt::Display`
 --> src/main.rs:3:28
  |
3 | impl std::error::Error for MyError {}
  |                            ^^^^^^^ unsatisfied trait bound
  |
help: the trait `std::fmt::Display` is not implemented for `MyError`
note: required by a bound in `std::error::Error`
  |
59 | pub trait Error: Debug + Display {
  |                          ^^^^^^^ required by this bound in `Error`
error[E0277]: `MyError` doesn't implement `Debug`
  |
help: consider annotating `MyError` with `#[derive(Debug)]`
  |
1 + #[derive(Debug)]
```

报错里那行 `pub trait Error: Debug + Display` 就是答案：标准库的错误 trait 要求实现者同时具备 `Debug` 和 `Display`。（`Debug + Display` 这种写法叫 **supertrait**，第 9 章讲 trait 时会展开。）

## 8.14 常见坑速查

| 现象 | 原因 | 解法 |
| --- | --- | --- |
| `?` 报 `can only be used in a function that returns Result` | 当前函数返回 `()` | 让函数返回 `Result`，或改用 `match` / `unwrap_or` |
| `?` 报 `couldn't convert the error to ...` | 缺 `From` 实现 | 实现 `From<源错误>`，或统一用 `Box<dyn Error>` |
| `返回 Result 的函数里对 Option 用 ?` | `Option` 没有错误信息 | `.ok_or("原因")?` |
| `返回 Option 的函数里对 Result 用 ?` | 错误信息无处安放 | `.ok()?` |
| 实现 `Error` 报缺 `Display`/`Debug` | `Error: Debug + Display` | `#[derive(Debug)]` + 手写 `Display` |
| 忽略 `Result` 有警告 | `Result` 标了 `#[must_use]` | 处理它，或显式 `let _ = ...` |
| 生产代码里到处都是 `unwrap()` | 把可恢复错误当成了崩溃 | 换成 `?`、`unwrap_or`、`match` |
| `unwrap()` 的 panic 信息看不出上下文 | 没有自定义消息 | 改用 `expect("这里为什么不该失败")` |
| 库里返回 `Box<dyn Error>`，调用方没法分支处理 | 错误类型被擦除 | 库里定义枚举错误，应用层再装箱 |
| 错误信息只有一句字符串，定位不到根因 | 没保留 `source` | 用枚举包裹底层错误 + 实现 `source()` |

## 8.15 练习

1. 写 `fn parse_positive(text: &str) -> Result<u32, String>`：解析失败返回带原文的错误消息，解析出 0 也返回错误，其余返回 `Ok`。
2. 写 `fn sum_pair(a: &str, b: &str) -> Result<i32, Box<dyn std::error::Error>>`，用 `?` 把两个字符串解析成整数并相加。
3. 定义 `enum LoginError { EmptyName, WrongPassword }`，实现 `Display` 和 `Error`，写 `fn login(name: &str, password: &str) -> Result<(), LoginError>`：用户名为空报第一种错，密码不等于 `"secret"` 报第二种错。
4. 给第 1 题的错误类型做个升级：定义 `enum NumberError { NotANumber { input: String, source: ParseIntError }, Zero }`，实现 `From<ParseIntError>`，让 `parse_positive` 改用 `?` 传播解析错误。
5. 写 `fn must_find<'a>(users: &'a [String], name: &str) -> Result<&'a String, String>`：用 `iter().find()` 找，找不到返回带名字的错误（提示：`find` 返回 `Option`，接 `ok_or_else`）。

（第 2、4 题分别是 8.9 和 8.7 示例的直接变体，这里不再单独给答案。）

### 参考答案

::: details 第 1 题

```rust
fn parse_positive(text: &str) -> Result<u32, String> {
    let n: u32 = text
        .trim()
        .parse()
        .map_err(|e| format!("`{text}` 不是数字：{e}"))?;
    if n == 0 {
        return Err(String::from("必须是正整数"));
    }
    Ok(n)
}

fn main() {
    println!("{:?}", parse_positive(" 42 "));   // Ok(42)
    println!("{:?}", parse_positive("0"));      // Err("必须是正整数")
    println!("{:?}", parse_positive("abc"));    // Err("`abc` 不是数字：invalid digit found in string")
}
```

两个细节：`map_err` 把 `ParseIntError` 换成人类可读的消息（这里用 `String` 当错误类型，简单但会丢失错误种类）；`u32` 的解析本身就会拒绝负数，所以不用额外判断。

实测输出是 `Ok(42)`、`Err("必须是正整数")`、`Err("`abc` 不是数字：invalid digit found in string")`。

:::

::: details 第 3 题

```rust
use std::fmt;

#[derive(Debug)]
enum LoginError {
    EmptyName,
    WrongPassword,
}

impl fmt::Display for LoginError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LoginError::EmptyName => write!(f, "用户名不能为空"),
            LoginError::WrongPassword => write!(f, "密码不正确"),
        }
    }
}

impl std::error::Error for LoginError {}

fn login(name: &str, password: &str) -> Result<(), LoginError> {
    if name.trim().is_empty() {
        return Err(LoginError::EmptyName);
    }
    if password != "secret" {
        return Err(LoginError::WrongPassword);
    }
    Ok(())
}

fn main() {
    println!("{:?}", login("ada", "secret"));            // Ok(())
    println!("{}", login("  ", "secret").unwrap_err());  // 用户名不能为空
    println!("{}", login("ada", "123456").unwrap_err()); // 密码不正确
}
```

这个错误类型没有包裹底层错误，所以 `impl Error` 可以空着——`source()` 的默认实现返回 `None`。`Error` 只要求 `Debug + Display`，其余都有默认实现。

:::

::: details 第 5 题

```rust
fn must_find<'a>(users: &'a [String], name: &str) -> Result<&'a String, String> {
    users
        .iter()
        .find(|user| user.as_str() == name)
        .ok_or_else(|| format!("没有找到 {name}"))
}

fn main() {
    let users = vec![String::from("ada"), String::from("linus")];
    println!("{:?}", must_find(&users, "ada"));    // Ok("ada")
    println!("{:?}", must_find(&users, "grace"));  // Err("没有找到 grace")
}
```

这条链把第 7 章和第 8 章串起来了：`find` 返回 `Option`，`ok_or_else` 把它升级成 `Result`，返回的引用借自 `users`（所以要写 `<'a>`）。错误消息用 `ok_or_else` 现算，避免在成功路径上白白构造字符串。

:::

## 8.16 小结

- 错误分两类：**可恢复**的用 `Result<T, E>` 交给调用方，**不可恢复**的用 `panic!` 直接结束；库代码不该因为可预期的失败而 panic。

- `Result<T, E>` 就是一个普通枚举；`match` 是最完整的处理方式，`is_ok` / `map` / `map_err` / `unwrap_or` 是常用的简化手段。

- `unwrap` 家族按「失败时程序还能不能继续」来选；`expect("原因")` 比 `unwrap()` 多一句能救命的上下文。

- `?` 遇到 `Err` 就提前返回，遇到 `Ok` 就取值继续；它只能用在返回 `Result` / `Option` 的函数里。

- `?` 会用 `From` 自动转换错误类型，所以一个函数可以混用多种错误源，前提是它们都能转成同一个错误类型。

- 自定义错误 = `#[derive(Debug)]` + 手写 `Display` + `impl Error`（需要错误链就实现 `source()`）+ 按需实现 `From`。

- 应用层图省事可以用 `Box<dyn Error>` 或 `anyhow`；库层应该定义明确的错误枚举，让调用方能 `match`。`thiserror` 是手写这些代码的自动化版本。

- `Option` 与 `Result` 互转：`ok_or` / `ok_or_else` 升级成错误，`ok()` / `err()` 退化成 `Option`。

- `Result` 带 `#[must_use]`，忽略它会有编译警告——这是「不会忘记处理错误」的保证。

下一章讲**泛型与 trait**：把「同样的逻辑适用于多种类型」写成代码，并理解本章里反复出现的 `Display`、`Error`、`From` 到底是什么。
