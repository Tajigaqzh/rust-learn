# 第 20 章 · 序列化与配置

程序总要把数据写到外面去：存成文件、发给别的服务、读一份配置文件。手写解析器又长又容易出错，所以 Rust 生态里几乎统一用一个库：**`serde`**。

`serde` 的设计很关键：它只管「把类型转成某种通用数据模型」，**具体格式由后端的 crate 决定**。所以同一个结构体，加一行依赖就能在 JSON、TOML、YAML、MessagePack 之间自由转换，**业务代码一行都不用改**。

配套代码在 `src/rust20_serde/mod.rs`，依赖已经加进 `Cargo.toml`：

```toml
[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.9"
```

## 20.1 格式的分工

| 格式 | 特点 | 典型用途 |
| --- | --- | --- |
| JSON | 通用、紧凑、几乎所有语言都支持 | API 交互、日志、跨语言数据 |
| TOML | 为配置而生，可读性最好，支持注释 | `Cargo.toml`、应用配置文件 |
| YAML | 层级清晰，但缩进敏感、规范复杂 | K8s 清单、CI 配置 |
| MessagePack / bincode | 二进制，体积小、解析快 | 内部服务通信、缓存 |

选择原则：**给人看、给人改的用 TOML；给程序读写的用 JSON；内部高性能传输用二进制格式**。

## 20.2 派生 `Serialize` 和 `Deserialize`

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
struct Config {
    app_name: String,
    #[serde(default = "default_port")]
    port: u16,
    #[serde(default)]
    debug: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    log_path: Option<String>,
}
```

就这么几行，`Config` 就能和 JSON、TOML 互转。实测序列化结果：

```text
    紧凑：{"app-name":"rust-learn","port":8080,"debug":true}
    美化：
{
  "app-name": "rust-learn",
  "port": 8080,
  "debug": true
}
    （log-path 是 None，被 skip_serializing_if 跳过了）
```

注意两个细节：

- **字段名变成了 `app-name`**：`rename_all = "kebab-case"` 把 Rust 的 `snake_case` 转成了配置文件习惯的连字符风格。Rust 代码里仍然是 `app_name`。
- **`log_path` 没有出现在 JSON 里**：因为它是 `None`，而 `skip_serializing_if` 让「空值」直接省略——配置文件里不会出现一堆 `"log-path": null`。

反序列化回来：

```rust
let json = serde_json::to_string(&config).unwrap();
let back: Config = serde_json::from_str(&json).unwrap();
assert_eq!(back, config);
```

实测 `反序列化后与原值相等 = true`。**「往返一致」是序列化库最基本的正确性测试**，写自己的类型时可以先跑一遍它。

## 20.3 常用的 serde 属性

| 属性 | 作用 |
| --- | --- |
| `#[serde(rename = "x")]` | 单个字段改名 |
| `#[serde(rename_all = "kebab-case")]` | 整个结构体的命名风格（`camelCase` / `kebab-case` / `SCREAMING_SNAKE_CASE`…） |
| `#[serde(default)]` | 缺失时用 `Default::default()` |
| `#[serde(default = "path")]` | 缺失时用指定函数的值 |
| `#[serde(skip_serializing_if = "Option::is_none")]` | 满足条件时不输出该字段 |
| `#[serde(skip)]` | 完全不参与序列化和反序列化 |
| `#[serde(deny_unknown_fields)]` | 出现未知字段时报错 |
| `#[serde(alias = "old-name")]` | 反序列化时接受额外的别名（兼容旧配置） |
| `#[serde(flatten)]` | 把嵌套结构「摊平」到父级 |
| `#[serde(with = "...")]` | 自定义某个字段的编解码（比如时间格式） |

值得单独说的是 **`deny_unknown_fields`**。默认情况下，配置里多写了字段，serde 会**默默忽略**：

```toml
[server]
prot = 8080     # 拼错了 port，程序却用了默认端口
```

加上 `deny_unknown_fields`，这种拼写错误会立刻变成一条报错（见 20.9 的「未知字段」），**对配置文件来说，早报错远好过默默用错的值**。

## 20.4 TOML：配置文件的首选

同一个 `Config`，换成 TOML 完全不用改类型定义：

```rust
let config: Config = toml::from_str("app-name = \"rust-learn\"\nport = 3000\n").unwrap();
```

实测缺省值的效果：

```text
    只写 app-name 时：Config { app_name: "tiny", port: 8080, debug: false, log_path: None }
    配置文件里：port = 3000
```

只给了 `app_name`，`port` 自动补成 `8080`、`debug` 补成 `false`——**这正是「配置文件只写要改的部分」的基础**。

序列化回 TOML：

```text
app-name = "rust-learn"
port = 3000
debug = false
```

`toml::to_string_pretty` 输出的就是人类可读的配置文件，可以直接写回磁盘（配合第 19 章的 `fs::write`）。注意 `log_path` 是 `None`，同样被跳过了。

## 20.5 配置分层：默认值 < 文件 < 环境变量

真实的配置来源往往有三个，优先级从低到高：

```text
代码里的默认值  →  配置文件  →  环境变量  →  命令行参数
（兜底）           （常规来源）   （部署时覆盖）  （临时覆盖）
```

serde 负责其中两段：**类型定义里的 `default` 提供兜底值，反序列化负责读文件**。环境变量这一层要自己写：

```rust
fn apply_env(mut config: Config, env_port: Option<String>) -> Result<Config, String> {
    if let Some(raw) = env_port {
        config.port = raw
            .trim()
            .parse()
            .map_err(|error| format!("PORT 不是合法端口：{error}"))?;
    }
    Ok(config)
}
```

实测：

```text
    环境变量覆盖后 port = 9000
    非法环境变量 = Err("PORT 不是合法端口：invalid digit found in string")
```

这个函数把「环境变量」当成参数传进来（`Option<String>`），而不是直接读环境——**这样测试才可控**，否则测试结果会随运行环境变化。

真实代码里是这样读的：

```rust
use std::env;

let port = match env::var("APP_PORT") {
    Ok(value) => Some(value),
    Err(env::VarError::NotPresent) => None,          // 没设置：用配置文件的值
    Err(env::VarError::NotUnicode(_)) => {
        return Err(String::from("APP_PORT 不是合法的 UTF-8"));
    }
};
```

`std::env::var` 返回 `Result<String, VarError>`——**「没设置」和「不是 UTF-8」是两种不同的情况**（第 8 章的思路）。需要非 UTF-8 的环境变量时用 `env::var_os`。

顺带一提，`Cargo.toml` 里的 `[dependencies]`、`env!("CARGO_PKG_VERSION")`、`option_env!` 也是这个体系的一部分：前者是配置文件，后者是「编译期读环境变量」。

## 20.6 动态 JSON：`json!` 宏与 `Value`

有时候结构体不适合表达数据——比如要拼一个结构不固定的日志事件，或者从接口拿到一份还不知道字段的返回。这时用 `serde_json::Value`：

```rust
let value = serde_json::json!({
    "name": "ada",
    "age": 36,
    "tags": ["rust", "systems"],
});
```

实测：

```text
    json! = {"age":36,"name":"ada","tags":["rust","systems"]}
    value["name"] = "ada"
    value["tags"][1] = "systems"
    不存在的键返回 Null，不会 panic = true
```

`Value` 是一个枚举（`Null` / `Bool` / `Number` / `String` / `Array` / `Object`），用 `value["key"]` 索引**永远不会 panic**——取不到就返回 `Null`（对比第 7 章 `HashMap` 的 `map[k]` 会 panic）。

两种用法各有位置：

| 场景 | 用什么 |
| --- | --- |
| 结构固定、字段明确（配置、API 请求体） | 定义结构体 + derive |
| 结构不固定，或者只转发不解析 | `Value` / `json!` |
| 大部分字段固定，个别字段动态 | 结构体里放一个 `Value` 字段 |
| 只需要读一两个字段 | `Value` 更快写，但失去了类型检查 |

经验：**能用结构体就用结构体**。`Value` 把类型错误从编译期推迟到了运行期，只有在「结构真的不确定」时才值得。

## 20.7 枚举怎么表示

枚举在 JSON / TOML 里有几种表示方式，serde 用属性来控制。默认是「外部标签」，而**内部标签**更符合配置文件的习惯：

```rust
#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "lowercase")]
enum Backend {
    Memory,
    File { path: String },
}
```

实测：

```text
    Memory -> {"kind":"memory"}
    File   -> {"kind":"file","path":"/var/data.db"}
    解析回来 = File { path: "/tmp/x.db" }
```

四种表示方式对照：

| 方式 | 属性 | JSON 形态 |
| --- | --- | --- |
| 外部标签（默认） | 无 | `{"File":{"path":"x"}}` |
| 内部标签 | `#[serde(tag = "kind")]` | `{"kind":"file","path":"x"}` |
| 相邻标签 | `#[serde(tag = "t", content = "c")]` | `{"t":"file","c":{"path":"x"}}` |
| 无标签 | `#[serde(untagged)]` | `{"path":"x"}`（靠字段猜） |

内部标签在实践中最好用：**配置里写 `kind = "file"` 一眼就能看懂**，解析时也不用额外嵌套。

一个限制：**内部标签对「元组变体」和「新类型变体」不友好**（因为要塞进一个对象里），所以配置类枚举尽量写成结构体变体（`File { path }`）而不是 `File(String)`。

## 20.8 五个真实报错怎么读

**案例 1：忘了 `derive`（`E0277`）**

```rust
#[derive(Debug)]
struct Config { app_name: String, port: u16 }

serde_json::to_string(&config)     // 报错
```

```text
error[E0277]: the trait bound `Config: serde::Serialize` is not satisfied
    --> src/main.rs:11:42
     |
  11 |     println!("{}", serde_json::to_string(&config).unwrap());
     |                    --------------------- ^^^^^^^ unsatisfied trait bound
     |
help: the trait `Serialize` is not implemented for `Config`
    --> src/main.rs:4:1
     |
   4 | struct Config {
     | ^^^^^^^^^^^^^
     = note: for local types consider adding `#[derive(serde::Serialize)]` to your `Config` type
     = note: for types from other crates check whether the crate offers a `serde` feature flag
```

`note` 里两句话覆盖了 90% 的场景：**自己的类型就加 `derive`；第三方类型就去它的文档里找 `serde` feature**（很多 crate 的 serde 支持是可选的，要显式开）。

**案例 2：JSON 语法错误**

```text
    缺少逗号：expected `,` or `}` at line 1 column 19
```

**案例 3：类型不匹配**

```text
    类型不对：invalid type: string "abc", expected u16 at line 1 column 32
```

注意它把「实际是什么」和「期望什么」都说了——比「解析失败」有用得多。

**案例 4：未知字段（配了 `deny_unknown_fields`）**

```text
    未知字段：unknown field `extra`, expected one of `app-name`, `port`, `debug`, `log-path` at line 1 column 37
```

**它把允许的字段全列出来了**，拼错配置键时一眼就能对照。

**案例 5：缺少必填字段**

```text
    缺少必填字段：missing field `app-name` at line 1 column 13
```

注意带 `#[serde(default)]` 的字段不会出现在这种错误里——这正是「默认值」的意义。

**TOML 的报错更直观**，会把出错的那一行画出来：

```text
TOML parse error at line 2, column 8
  |
2 | port = "abc"
  |        ^^^^^
invalid type: string "abc", expected u16
```

**所有解析错误都带行列号**，这也是「配置文件格式错误」相对「代码 bug」更好修的原因——用户能直接定位到出错的字符。

## 20.9 常见坑速查

| 现象 | 原因 | 解法 |
| --- | --- | --- |
| 报 `trait bound Serialize is not satisfied` | 类型没加 `derive` | `#[derive(Serialize, Deserialize)]` |
| 第三方类型序列化不了 | 它的 serde 支持是可选 feature | 打开对应的 feature |
| 配置里拼错的字段被静默忽略 | 默认忽略未知字段 | 加 `deny_unknown_fields` |
| 配置文件里一堆 `null` | `Option` 字段也参与序列化 | `skip_serializing_if = "Option::is_none"` |
| 字段名风格和配置文件不一致 | 没有命名转换 | `rename_all = "kebab-case"` |
| 缺少字段就报错 | 没有默认值 | `#[serde(default)]` 或 `#[serde(default = "fn")]` |
| 改字段名后旧配置读不了 | 没有兼容处理 | `#[serde(alias = "old-name")]` |
| 枚举解析出来结构很别扭 | 用了默认的「外部标签」 | `#[serde(tag = "kind")]` |
| `Value` 里取字段要写一堆 `as_str()` | 动态类型没有编译期检查 | 结构固定就定义结构体 |
| 测试结果随环境变化 | 直接读了真实环境变量 | 把环境变量作为参数注入 |
| 时间字段序列化格式不对 | serde 默认用 RFC3339，但类型要支持 | `chrono` 开 `serde` feature，或用 `#[serde(with = "...")]` |
| 数字精度丢失 | JSON 的数字按 `f64` 解析 | 大整数用字符串传输 |

## 20.10 练习

1. 定义 `struct Server { host: String, port: u16 }` 和 `struct AppConfig { server: Server, #[serde(default)] debug: bool }`，从一段 TOML 解析出配置，再序列化成 JSON。
2. 给配置结构体加上 `deny_unknown_fields`，然后故意在 TOML 里写一个拼错的键，观察报错信息里列出了哪些合法字段。
3. 用 `json!` 构造一个嵌套的 JSON（含对象、数组），然后用 `as_str()` / `as_array()` 取出字段并打印；说明为什么这些方法返回 `Option`。
4. 定义带 `#[serde(tag = "type")]` 的枚举 `Shape { Circle { radius: f64 }, Rect { width: f64, height: f64 } }`，把一组形状序列化成 JSON 再解析回来。
5. 回答两个问题：(a) 为什么「往返一致」是值得写的测试？(b) 把环境变量直接读进配置函数，会带来什么测试上的问题？

（第 2、5 题是 20.3、20.5 讨论过的内容，这里不再单独给答案。）

### 参考答案

::: details 第 1 题

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct Server {
    host: String,
    port: u16,
}

#[derive(Debug, Serialize, Deserialize)]
struct AppConfig {
    server: Server,
    #[serde(default)]
    debug: bool,
}

fn main() {
    let toml_text = "[server]\nhost = \"127.0.0.1\"\nport = 8080\n";
    let config: AppConfig = toml::from_str(toml_text).unwrap();
    println!("{config:?}");
    println!("{}", serde_json::to_string_pretty(&config).unwrap());
}
```

实测：

```text
AppConfig { server: Server { host: "127.0.0.1", port: 8080 }, debug: false }
{
  "server": {
    "host": "127.0.0.1",
    "port": 8080
  },
  "debug": false
}
```

TOML 用 `[server]` 表示嵌套表，JSON 用嵌套对象——**同一份数据，两种格式，类型定义完全不用改**。这就是 serde 抽象层的价值。

:::

::: details 第 3 题

```rust
use serde_json::json;

fn main() {
    let value = json!({
        "user": { "name": "ada", "roles": ["admin", "dev"] },
        "active": true,
    });

    let name = value["user"]["name"].as_str().unwrap_or("未知");
    let roles = value["user"]["roles"]
        .as_array()
        .map(|items| items.len())
        .unwrap_or(0);

    println!("name = {name}, roles = {roles}");
}
```

实测输出 `name = ada, roles = 2`。

`as_str()` / `as_array()` 返回 `Option`，因为 **`Value` 是运行时的动态类型**：编译器不知道 `value["user"]["name"]` 到底是不是字符串，只有运行时才知道。这也正是「能用结构体就用结构体」的原因——结构体把这些检查搬到了编译期。

:::

::: details 第 4 题

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
enum Shape {
    Circle { radius: f64 },
    Rect { width: f64, height: f64 },
}

fn main() {
    let shapes = vec![
        Shape::Circle { radius: 1.5 },
        Shape::Rect { width: 3.0, height: 4.0 },
    ];

    let json_text = serde_json::to_string(&shapes).unwrap();
    println!("{json_text}");

    let back: Vec<Shape> = serde_json::from_str(&json_text).unwrap();
    println!("{} 个形状，第二个 = {:?}", back.len(), back[1]);
}
```

实测：

```text
[{"type":"circle","radius":1.5},{"type":"rect","width":3.0,"height":4.0}]
2 个形状，第二个 = Rect { width: 3.0, height: 4.0 }
```

内部标签让每种形状都带上 `"type"` 字段，**接收方只看 `type` 就知道该怎么解析**——这正是「多态数据」在 JSON 里的标准做法，也是第 6 章「枚举建模」在网络协议上的自然延伸。

:::

## 20.11 小结

- `serde` 把「类型」和「数据格式」解耦：类型实现 `Serialize` / `Deserialize`，具体格式由 `serde_json` / `toml` 这类后端决定，换格式不改业务代码。

- `#[derive(Serialize, Deserialize)]` 几乎零成本；忘了 `derive` 会报 `E0277`，`note` 里会提示加哪个 derive 或开哪个 feature。

- 常用属性：`rename` / `rename_all`（命名风格）、`default`（缺省值）、`skip_serializing_if`（省略空值）、`deny_unknown_fields`（拒绝拼错的键）、`alias`（兼容旧名）、`flatten`（摊平嵌套）。

- JSON 适合跨语言传输，TOML 适合人类编辑的配置；`to_string` / `from_str` 是最常用的两个入口。

- 配置分层的优先级是「默认值 < 文件 < 环境变量 < 命令行」，`serde` 负责前两层，环境变量要自己接；`env::var` 的 `NotPresent` 和 `NotUnicode` 是两种不同情况。

- **把环境变量作为参数注入**，而不是在函数里直接读——这样测试才可复现。

- `serde_json::Value` / `json!` 用于结构不固定的数据；索引取不到返回 `Null` 不会 panic，但类型检查从编译期推迟到了运行期。

- 枚举用 `#[serde(tag = "kind")]` 内部标签最适合配置；避免用元组 / 新类型变体。

- 解析错误都带行列号，并且会说明「实际类型 vs 期望类型」「允许哪些字段」——**报错本身就是最好的文档**。

- 「往返一致」（序列化再反序列化得到相等值）是最基本的正确性测试，自定义类型时值得写一个。

下一章讲**命令行工具**：`env::args` 与 `clap` 解析参数、退出码、错误输出，以及 `tracing` / `env_logger` 做日志。
