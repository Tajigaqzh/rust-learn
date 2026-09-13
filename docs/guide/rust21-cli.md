# 第 21 章 · 命令行工具

前面二十章写的都是「库式」代码：定义函数、被 `main` 调用。这一章写一个**真正的命令行程序**——它是用户直接操作的东西，所以除了业务逻辑，还要照顾好三件事：

| 关注点 | 约定 |
| --- | --- |
| 参数解析 | 用 `clap`，自动生成帮助和错误提示 |
| 输出分流 | **数据走 stdout，日志和错误走 stderr** |
| 退出码 | 0 成功、1 运行时错误、2 参数错误 |

配套代码在 `src/rust21_cli/mod.rs`，依赖：

```toml
[dependencies]
clap = { version = "4", default-features = false, features = ["std", "help", "usage", "error-context", "suggestions"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", default-features = false, features = ["fmt", "std", "ansi"] }
```

两点说明：配套代码用的是 clap 的 **builder API**（`Command::new(...)`），因为本机离线缓存里没有 `clap_derive`，装不了 `derive` feature；正文 21.3 给出 derive 写法的完整对照。日志用 `tracing` 而不是 `env_logger`，原因同样是本地缓存里没有后者——两者思路一致。

## 21.1 从 `env::args` 说起

标准库提供的最小工具是 `std::env::args`：

```rust
use std::env;

let args: Vec<String> = env::args().collect();
// args[0] 是程序名，后面才是用户传的参数
```

两个细节：

- **第一个元素是程序名**，不是用户参数，遍历时要 `skip(1)`。
- `args()` 要求参数是合法 UTF-8，遇到非 UTF-8 会 panic；要稳妥就用 `env::args_os()`（返回 `OsString`，第 19 章提过）。

手写解析很快会撞上一堆琐事：

| 需求 | 手写要处理的事 |
| --- | --- |
| `--help` / `-h` | 自己写帮助文本 |
| `-v` 与 `--verbose` | 长短选项都要认 |
| `--format json` 和 `--format=json` | 两种写法都得支持 |
| `--` 之后的参数 | 「后面都是位置参数」的约定 |
| 拼错参数 | 自己给出「你是不是想输入 xxx」的提示 |
| 参数错误 | 自己决定退出码（惯例是 2） |

所以除了几行的小脚本，**正式工具直接用 clap**：上面这些它全都自动处理。

## 21.2 clap：builder API

```rust
use clap::{Arg, ArgAction, Command};

fn build_cli() -> Command {
    Command::new("wc-lite")
        .version("1.0.0")
        .about("统计文本的行数 / 词数 / 字符数")
        .arg(Arg::new("file").help("要统计的文件；省略时从标准输入读").required(false))
        .arg(
            Arg::new("lines")
                .short('l')
                .long("lines")
                .action(ArgAction::SetTrue)      // 标志：不带值
                .help("只统计行数"),
        )
        .arg(
            Arg::new("format")
                .long("format")
                .value_parser(["plain", "json"]) // 取值范围
                .default_value("plain")          // 默认值
                .help("输出格式"),
        )
        .subcommand(Command::new("info").about("显示构建信息"))
}
```

解析结果：

```rust
let matches = build_cli()
    .try_get_matches_from(["wc-lite", "--lines", "--format", "json", "notes.txt"])
    .unwrap();
```

实测：

```text
    file    = Some("notes.txt")
    --lines = true
    --format= Some("json")
    --verbose = false
```

三种参数形态：

| 形态 | 定义方式 | 读取方式 | 例子 |
| --- | --- | --- | --- |
| 位置参数 | `Arg::new("file")` | `get_one::<String>("file")` | `wc-lite notes.txt` |
| 选项（带值） | `.long("format")` | `get_one::<String>("format")` | `--format json` |
| 标志（布尔） | `.action(ArgAction::SetTrue)` | `get_flag("lines")` | `--lines` |

**`ArgAction::SetTrue` 是关键**：不加它，`--lines` 会被当成「需要一个值的选项」，用户只写 `--lines` 反而报错（见 21.11 案例 1）。

## 21.3 derive API（更常见的写法）

clap 的 derive 写法把「命令行结构」直接写成结构体，样板代码更少：

```rust
use clap::{Parser, ValueEnum};

#[derive(Parser, Debug)]
#[command(name = "wc-lite", version, about = "统计文本的行数 / 词数 / 字符数")]
struct Cli {
    /// 要统计的文件；省略时从标准输入读
    file: Option<String>,

    /// 只统计行数
    #[arg(short, long)]
    lines: bool,

    /// 输出格式
    #[arg(long, value_enum, default_value_t = Format::Plain)]
    format: Format,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
enum Format { Plain, Json }

fn main() {
    let cli = Cli::parse();      // 解析失败会自动打印错误并以退出码 2 结束
    println!("{cli:?}");
}
```

对照关系：

| builder | derive |
| --- | --- |
| `Arg::new("file")` | 结构体字段 `file: Option<String>` |
| `.short('l').long("lines")` | `#[arg(short, long)]` |
| `.action(ArgAction::SetTrue)` | 字段类型是 `bool` |
| `.default_value("plain")` | `#[arg(default_value_t = Format::Plain)]` |
| `.value_parser(["plain", "json"])` | `#[derive(ValueEnum)]` 的枚举 |
| `.help("...")` | 字段上的 `///` 文档注释 |

**注意**：上面这段 derive 代码需要 `clap` 的 `derive` feature（依赖 `clap_derive`），本机离线缓存里没有这个包，所以它没有参与本项目的编译验证；你在本地把依赖写成 `clap = { version = "4", features = ["derive"] }` 即可使用。两种写法生成的命令行行为是一致的。

## 21.4 默认值与取值范围

```rust
.arg(Arg::new("format")
    .long("format")
    .value_parser(["plain", "json"])   // 只接受这两个值
    .default_value("plain"))            // 不传就用它
```

实测什么都没传时：

```text
    什么都没传时：format = Some("plain")，file = None
```

`value_parser` 的检查发生在**解析阶段**，所以非法取值会被 clap 拦下来，业务代码永远拿不到非法值——这比「自己写 if 判断」可靠得多：

```text
error: invalid value 'xml' for '--format <format>'
  [possible values: plain, json]

For more information, try '--help'.
```

## 21.5 子命令

工具功能变多时，用子命令把功能分组（`git commit`、`cargo build`、`docker run` 都是这个模式）：

```rust
.subcommand(Command::new("info").about("显示构建信息"))
```

```rust
let with_sub = build_cli().try_get_matches_from(["wc-lite", "info"]).unwrap();
with_sub.subcommand_name();          // Some("info")
if let Some(("info", _)) = with_sub.subcommand() {
    // 执行 info 子命令
}
```

实测 `subcommand = Some("info")`。子命令各自可以有自己的参数、自己的帮助页面，`wc-lite info --help` 会显示 `info` 的选项。

## 21.6 自动生成的帮助与错误提示

这是用 clap 最大的收益：`--help` 的全部内容都由参数定义自动生成。

```text
统计文本的行数 / 词数 / 字符数

Usage: wc-lite [OPTIONS] [file] [COMMAND]

Commands:
  info  显示构建信息
  help  Print this message or the help of the given subcommand(s)

Arguments:
  [file]  要统计的文件；省略时从标准输入读

Options:
  -l, --lines            只统计行数
      --format <format>  输出格式 [default: plain] [possible values: plain, json]
  -v, --verbose          打开详细日志
  -h, --help             Print help
  -V, --version          Print version
```

注意几个自动生成的部分：`Usage` 行、`[default: ...]`、`[possible values: ...]`、以及 `-h` / `-V` 这两个内置选项。**帮助文档和实现永远不会不一致**——因为你只写了一遍。

参数写错时，给的提示也很有用：

```text
error: unexpected argument '--nope' found

  tip: to pass '--nope' as a value, use '-- --nope'

Usage: wc-lite [OPTIONS] [file] [COMMAND]

For more information, try '--help'.
```

`tip` 那行是 clap 的小心思：如果你想**把 `--nope` 当作普通值**传进去（而不是当选项），应该写成 `-- --nope`——这正是 21.1 提到的 `--` 约定。

## 21.7 输出分流：stdout 给数据，stderr 给日志

命令行工具最容易被忽略的一条约定：

| 流 | 放什么 | 理由 |
| --- | --- | --- |
| stdout | **程序的正常输出**（数据、结果） | 用户可能用管道接给别的程序 |
| stderr | 日志、进度、警告、错误 | 不污染数据，重定向 stdout 时仍然可见 |

举例：`wc-lite --format json data.txt > result.json` 时，**只有 JSON 会写进文件**，日志仍然打在终端上。如果日志走了 stdout，文件里就混进了人话，`jq` 之类的工具直接解析失败。

在 Rust 里对应的就是：

```rust
println!("{{\"lines\":2}}");        // stdout：数据
eprintln!("正在处理 data.txt");     // stderr：提示信息
tracing::warn!("文件很大");          // stderr：结构化日志（tracing 默认写 stderr）
```

第 1 章 1.7 讲的 `print!` / `eprintln!` 在这里变成了工程约定。

## 21.8 退出码

退出码是命令行程序和外部世界（脚本、CI、`&&` 链）沟通的方式：

| 退出码 | 含义 | 怎么产生 |
| --- | --- | --- |
| 0 | 成功 | `main` 正常返回，或 `ExitCode::SUCCESS` |
| 1 | 运行时错误 | `fn main() -> Result<...>` 返回 `Err` |
| 2 | 参数用法错误 | clap 遇到参数问题（Unix 惯例） |
| 130 | 被 Ctrl-C 中断 | 128 + SIGINT(2)，由 shell 报告 |

两种写法：

```rust
// 1. main 返回 Result：出错自动打错误信息 + 退出码 1（第 8 章）
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let text = std::fs::read_to_string("data.txt")?;
    println!("{}", text.lines().count());
    Ok(())
}

// 2. 手动控制退出码
use std::process::ExitCode;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("错误：{error}");
            ExitCode::from(2)
        }
    }
}
```

推荐第一种（`Result`）作为默认：**错误信息走 stderr、退出码自动是 1**。只有需要区分「参数错误 = 2」这类具体码时才手写 `ExitCode`。

## 21.9 日志：`tracing` 入门

用 `println!` 打日志的问题：没有级别、没法过滤、没有时间戳、没法结构化。`tracing` 解决这些：

```rust
use tracing::{debug, info, warn};
use tracing_subscriber::fmt;

fmt().without_time().with_target(false)
     .with_max_level(tracing::Level::INFO)
     .init();

info!("开始统计文本");
debug!("这条 debug 日志默认不显示（级别不够）");
warn!(file = "notes.txt", "文件比较大，可能有点慢");
```

实测输出（注意日志走的是 stderr）：

```text
 INFO 开始统计文本
 WARN 文件比较大，可能有点慢 file="notes.txt"
```

三个要点：

1. **级别过滤**：`with_max_level(INFO)` 会丢掉 `debug!` 和 `trace!`。只写 `info!`/`warn!`/`error!` 属于「正常输出」，`debug!`/`trace!` 属于「排查时才开」。
2. **结构化字段**：`warn!(file = "notes.txt", "…")` 里的 `file` 是一个字段，不是拼进消息的字符串。日志系统可以按字段检索、过滤、聚合。
3. **`init()` 只能调一次**：它在进程里注册全局订阅者，重复调用会 panic。真实项目通常在 `main` 最开头调一次，并用环境变量控制级别（`EnvFilter` / `RUST_LOG`）。

`env_logger`（`log` crate 生态）用法类似，靠 `RUST_LOG=debug` 环境变量控制级别：

```rust
env_logger::init();        // 读取 RUST_LOG
log::info!("hello");
```

两者选择：**库用 `log`（门槛低、无依赖），应用用 `tracing`（结构化、异步友好）**。本项目用 `tracing` 是因为它是异步生态（第 16 章 tokio）的默认选择。

## 21.10 实战：一个命令行工具的骨架

把前面几章的东西拼起来，一个完整的 CLI 大致长这样：

```rust
use std::process::ExitCode;

fn main() -> ExitCode {
    // 1. 解析参数（clap）——参数错误时 clap 会自己打印并退出，码为 2
    let matches = build_cli().get_matches();
    let file = matches.get_one::<String>("file").cloned();
    let format = matches.get_one::<String>("format").cloned().unwrap_or_default();

    // 2. 初始化日志（tracing）
    tracing_subscriber::fmt()
        .with_max_level(if matches.get_flag("verbose") { Level::DEBUG } else { Level::INFO })
        .init();

    // 3. 干活：读输入（第 19 章）
    let text = match &file {
        Some(path) => match std::fs::read_to_string(path) {
            Ok(text) => text,
            Err(error) => {
                eprintln!("无法读取 {path}：{error}");
                return ExitCode::from(1);
            }
        },
        None => {
            let mut buffer = String::new();
            if let Err(error) = std::io::stdin().read_to_string(&mut buffer) {
                eprintln!("读取标准输入失败：{error}");
                return ExitCode::from(1);
            }
            buffer
        }
    };

    // 4. 输出结果：数据走 stdout
    println!("{}", render(&count(&text), &format));
    ExitCode::SUCCESS
}
```

分层很清楚：**参数 → 日志 → 输入 → 处理 → 输出**，每一步的错误都有明确去处：

| 出错的地方 | 怎么报 | 退出码 |
| --- | --- | --- |
| 参数写错 | clap 自动打印用法提示 | 2 |
| 文件读不到 | `eprintln!` 或 `tracing::error!` | 1 |
| 输入内容不符合预期 | 同上，最好带上上下文 | 1 |
| 一切正常 | 结果写 stdout | 0 |

如果配置也要读，把第 20 章的 `Config` 接在第 1 步和第 3 步之间：**默认值 → 配置文件 → 环境变量 → 命令行参数**，优先级依次升高。

## 21.11 四个典型的运行时错误

clap 的问题**不是编译错误，而是运行时**——命令行是外部输入，只有解析时才知道对错。

**案例 1：忘了 `ArgAction::SetTrue`，标志变成了「需要值的选项」**

```text
error: a value is required for '--lines <lines>' but none was supplied

For more information, try '--help'.
```

用户只写了 `--lines`，clap 却以为他在等一个值。修法：给标志加 `.action(ArgAction::SetTrue)`（derive 写法里字段类型是 `bool` 就自动是这个行为）。

**案例 2：必填参数没提供**

```text
error: the following required arguments were not provided:
  <file>

Usage: wc-lite <file>

For more information, try '--help'.
```

**案例 3：未知参数**

```text
error: unexpected argument '--nope' found

  tip: to pass '--nope' as a value, use '-- --nope'
```

**案例 4：取值不在允许范围内**

```text
error: invalid value 'xml' for '--format <format>'
  [possible values: plain, json]
```

自定义 `value_parser` 的错误也会原样带出来。实测把 `--limit` 的解析函数写成 `parse_size` 之后：

```text
error: invalid value '10XB' for '--limit <limit>': 无效大小 `10XB`：invalid digit found in string
```

**自定义解析函数的签名是 `fn(&str) -> Result<T, E>`**，`E` 只要能转成 `Box<dyn Error + Send + Sync>` 就行——`String` 和绝大多数错误类型都满足。这让「参数校验」和「参数解析」合在一个地方：

```rust
fn parse_size(text: &str) -> Result<u64, String> {
    let text = text.trim();
    let (digits, multiplier) = if let Some(value) = text.strip_suffix("GB") {
        (value, 1024 * 1024 * 1024)
    } else if let Some(value) = text.strip_suffix("MB") {
        (value, 1024 * 1024)
    } else if let Some(value) = text.strip_suffix("KB") {
        (value, 1024)
    } else {
        (text, 1)
    };
    let number: u64 = digits.trim().parse()
        .map_err(|error| format!("无效大小 `{text}`：{error}"))?;
    Ok(number * multiplier)
}
```

实测 `--limit 10MB` 得到 `10485760` 字节。

## 21.12 常见坑速查

| 现象 | 原因 | 解法 |
| --- | --- | --- |
| `--flag` 报「a value is required」 | 忘了 `ArgAction::SetTrue` | 加 action，或 derive 里用 `bool` |
| 参数明明传了却读不到 | 名字写错（`new("file")` 与 `get_one("file")` 不一致） | 两边用同一个字符串或常量 |
| 取值非法却进了业务代码 | 没写 `value_parser` | 用 `value_parser([...])` 或自定义函数 |
| 日志混进了数据文件 | 用 `println!` 打日志 | 日志一律走 `eprintln!` / `tracing` |
| 管道下游解析失败 | 输出里混了人类可读的提示 | stdout 只放数据，且用稳定的机器格式 |
| 脚本判断不了成功失败 | 退出码永远 0 | `main` 返回 `Result` / `ExitCode` |
| `tracing` 日志全都不显示 | 没初始化订阅者，或级别过滤太低 | 调一次 `fmt().with_max_level(...).init()` |
| 程序 panic「已设置全局订阅者」 | `init()` 被调了两次 | 只在 `main` 开头调一次 |
| 中文参数乱码 | 非 UTF-8 的 `OsString` 被强行转换 | 用 `args_os` / `OsStr` 处理 |
| 用户在主目录下运行，相对路径失效 | 相对路径基于「当前工作目录」 | 用绝对路径或 `canonicalize`（第 19 章） |

## 21.13 练习

1. 给本章的 `wc-lite` 加一个 `--chars` 标志：加上它时只输出字符数。
2. 写 `fn parse_size(text: &str) -> Result<u64, String>`，支持 `1KB` / `10MB` / `2GB` 和后缀省略的纯数字；把它接到 clap 的 `--limit` 上，并验证非法输入会得到带自定义消息的报错。
3. 写一个迷你程序：数据用 `println!` 输出、日志用 `eprintln!` 输出，然后用 `> out.txt 2> err.txt` 重定向，验证两个文件里分别只有数据和日志。
4. 把 `main` 改成返回 `Result<(), Box<dyn std::error::Error>>` 的版本，说明出错时退出码是多少、错误信息去了哪里。
5. 回答两个问题：(a) 为什么「帮助信息由参数定义生成」比手写更好？(b) 为什么日志不能走 stdout？

（第 4、5 题是 21.7、21.8 讨论过的内容，这里不再单独给答案。）

### 参考答案

::: details 第 1 题

```rust
.arg(
    Arg::new("chars")
        .short('c')
        .long("chars")
        .action(ArgAction::SetTrue)
        .help("只输出字符数"),
)
```

读取时：

```rust
if matches.get_flag("chars") {
    println!("字符数 = {}", text.chars().count());
}
```

实测传入 `--chars` 时输出 `字符数 = 5`。注意**标志的读取方法必须是 `get_flag`**，用 `get_one::<bool>` 取不到值（clap 会 panic，因为类型的存取方式必须与定义一致）。

:::

::: details 第 2 题

```rust
use clap::{Arg, Command};

fn parse_size(text: &str) -> Result<u64, String> {
    let text = text.trim();
    let (digits, multiplier) = if let Some(value) = text.strip_suffix("GB") {
        (value, 1024 * 1024 * 1024)
    } else if let Some(value) = text.strip_suffix("MB") {
        (value, 1024 * 1024)
    } else if let Some(value) = text.strip_suffix("KB") {
        (value, 1024)
    } else {
        (text, 1)
    };
    let number: u64 = digits
        .trim()
        .parse()
        .map_err(|error| format!("无效大小 `{text}`：{error}"))?;
    Ok(number * multiplier)
}

fn main() {
    let cli = Command::new("mini").arg(
        Arg::new("limit")
            .long("limit")
            .value_parser(parse_size)
            .default_value("1KB"),
    );

    let matches = cli.try_get_matches_from(["mini", "--limit", "10MB"]).unwrap();
    println!("{}", matches.get_one::<u64>("limit").unwrap());   // 10485760
}
```

实测 `10MB` 得到 `10485760`；传入 `10XB` 时 clap 报：

```text
error: invalid value '10XB' for '--limit <limit>': 无效大小 `10XB`：invalid digit found in string
```

**自定义 parser 让「非法输入」在解析阶段就被拦住**，业务代码里拿到的保证是合法的 `u64`——这正是第 6 章「让非法状态无法表示」的思想用在命令行参数上。

:::

::: details 第 3 题

```rust
fn main() {
    println!("DATA:42");        // stdout：给下游程序读
    eprintln!("LOG: 处理完成");   // stderr：给人看
}
```

实测重定向之后：

```text
stdout 文件内容：DATA:42
stderr 文件内容：LOG: 处理完成
```

**两个流彻底分开**，所以 `mini > result.txt` 拿到的文件可以直接喂给 `jq` 或写进 CSV，而日志仍然打在终端上。这是所有成熟命令行工具的默认行为（`grep`、`cargo`、`docker` 都这样）。

:::

## 21.14 小结

- 命令行程序要照顾三件事：**参数解析、输出分流、退出码**。

- `env::args()` 够简单场景用，但 `--help`、长短选项、`=` 形式、错误提示这些琐事会迅速堆积——正式工具用 `clap`。

- clap 有 builder 和 derive 两种写法，行为一致：builder 用 `Command::new(...).arg(...)`，derive 用 `#[derive(Parser)]` 结构体 + `#[arg(...)]`，后者样板更少。

- 标志（布尔）必须用 `ArgAction::SetTrue` / `bool` 字段；取值受限的参数用 `value_parser`，需要自定义解析就传一个 `fn(&str) -> Result<T, E>`。

- `--help` 和错误提示都由参数定义自动生成，**帮助文档和实现不会不一致**；参数错误的退出码约定是 2。

- 子命令（`git commit` 那种）用 `.subcommand(...)` 分组，每个子命令有自己的参数和帮助。

- **数据走 stdout，日志和错误走 stderr**；这样才能安全地重定向和接管道。

- 退出码约定：0 成功、1 运行时错误、2 参数错误；`main` 返回 `Result` 是最省心的写法，需要精细控制时用 `ExitCode`。

- 日志用 `tracing`（应用）/ `log` + `env_logger`（库）；级别过滤、结构化字段、`init()` 只调一次。

- 完整骨架的分层是：**参数 → 日志 → 输入 → 处理 → 输出**，每类错误都有明确的去处和退出码。

到这里，标准库与生态篇的主体（日期时间、文本与正则、文件 IO、序列化与配置、命令行工具）就完成了。下一章是**第 22 章 网络与 HTTP**：TCP 基础、`reqwest` 发请求、超时与重试。
