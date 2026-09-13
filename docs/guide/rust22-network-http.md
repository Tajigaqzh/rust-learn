# 第 22 章 · 网络与 HTTP

网络编程可以分成两层看：

| 层次 | 是什么 | 本章内容 |
| --- | --- | --- |
| 传输层 | TCP：可靠的双向**字节流** | `TcpListener` / `TcpStream` |
| 应用层 | HTTP：跑在 TCP 之上的**文本协议** | 手写一次请求、用 `reqwest` |

这一章有个特点：**所有演示都在本机回环地址（127.0.0.1）上完成**——自己起一个小服务器，自己发请求。这样既能看到真实的网络行为（连接、请求、响应、超时、连接被拒绝），又不受外网环境影响，输出还可复现。

配套代码在 `src/rust22_network/mod.rs`，依赖：

```toml
[dependencies]
reqwest = { version = "0.13", default-features = false, features = ["blocking", "json"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

`default-features = false` 省掉了 TLS（HTTPS）和系统代理，因为本章只访问 `http://127.0.0.1`。**要访问外网 HTTPS 时，需要打开 `default-tls` 或 `rustls-tls`**——这一点在 22.11 会再提。

## 22.1 从 TCP 说起

TCP 提供的抽象非常简单：**一条可靠、有序的双向字节流**。它保证：

- 你写进去的字节，对方会按顺序收到（不会乱序、不会丢）；
- 连不上时会明确报错（比如目标端口没有程序监听）。

它不保证的是「消息边界」——**TCP 不知道你一次发了多少「条」消息**，它只看到一串字节。所以协议必须自己定义边界（HTTP 用 `Content-Length` 或 `Connection: close`，其他协议常用「长度前缀」或「分隔符」）。

一个 TCP 服务端的基本动作就是：绑定地址 → 等待连接 → 读字节 → 写字节。

## 22.2 标准库的 TCP

```rust
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;

// 服务端
let listener = TcpListener::bind("127.0.0.1:0").unwrap();   // 0 = 让系统分配空闲端口
let addr = listener.local_addr().unwrap();
thread::spawn(move || {
    if let Ok((mut stream, _)) = listener.accept() {
        let clone = stream.try_clone().unwrap();
        let mut reader = BufReader::new(clone);
        let mut line = String::new();
        if reader.read_line(&mut line).is_ok() {
            writeln!(stream, "echo: {}", line.trim()).unwrap();
        }
    }
});

// 客户端
let mut stream = TcpStream::connect(addr).unwrap();
stream.write_all(b"hello network\n").unwrap();
```

实测服务器回应 `echo: hello network`。

几个值得记住的点：

- **端口写 `0` 表示「让操作系统挑一个空闲端口」**，然后从 `local_addr()` 读出来。测试里这样写就不会撞端口。
- `TcpStream` 是**双工的**：同一个连接既能读也能写。所以需要两个方向同时用时，用 `try_clone()` 复制一个句柄出来（内部共享同一个 socket）。
- `accept()`、`connect()`、`read()`、`write()` **全都是阻塞的**（默认情况下）。这就是第 16 章讲「异步 IO」的动机：一个线程一个连接，连接多了线程就爆了。
- 标准库的 `TcpStream` 也能设超时：`set_read_timeout` / `set_write_timeout`，不设的话会一直等。

## 22.3 HTTP 就是一段文本

HTTP 请求的本质是「按格式写几行 ASCII 文本」：

```text
GET / HTTP/1.1\r\n
Host: localhost\r\n
Connection: close\r\n
\r\n
```

四部分组成：**请求行**（方法 + 路径 + 版本）、**头部**、一个空行、可选的请求体。响应也是同样的结构：状态行 + 头部 + 空行 + 响应体。

配套代码真的手写了这段文本发过去，实测：

```text
    收到 116 字节
    状态行 = HTTP/1.1 200 OK
    响应体 = {"body":"","method":"GET"}
```

自己写一遍的价值在于：**再看到 `Content-Type`、`Content-Length`、`Connection: close` 这些头部时，你知道它们在字节流里的位置**。

那 116 字节里包含响应行、三个头部、空行和 JSON 响应体——HTTP 就是把这些文本按顺序写进 TCP 流里。

## 22.4 reqwest：blocking 客户端

真实项目不会手写 HTTP，用 `reqwest`：

```rust
let client = reqwest::blocking::Client::builder()
    .timeout(Duration::from_millis(500))
    .build()?;

let response = client.get("http://127.0.0.1:8080/").send()?;
response.status();        // 200 OK
response.text()?;         // 响应体字符串
```

实测：

```text
    状态 = 200 OK
    响应体 = {"body":"","method":"GET"}
```

为什么要用 `Client` 而不是每次 `reqwest::blocking::get(...)`？

| 写法 | 说明 |
| --- | --- |
| `reqwest::blocking::get(url)` | 快捷函数，内部每次建一个客户端 |
| `Client::builder().build()` | **推荐**：可以设超时、默认头部、连接池，并且复用连接 |

`Client` 内部有**连接池**：对同一个主机发 100 次请求，不会真的建立 100 次 TCP 连接（HTTP/1.1 默认 `keep-alive`）。所以**把 `Client` 建一次、反复用**是基本要求。

## 22.5 POST JSON：和 serde 接上

第 20 章的 serde 在这里直接派上用场：

```rust
let payload = serde_json::json!({ "name": "ada", "age": 36 });

let echoed: serde_json::Value = client
    .post(&url)
    .json(&payload)        // 自动设置 Content-Type: application/json 并序列化请求体
    .send()?
    .json()?;              // 自动检查状态码、反序列化响应体
```

实测（服务器把我们发的内容原样回显）：

```text
    服务器回显 = {"body":"{\"age\":36,\"name\":\"ada\"}","method":"POST"}
```

注意响应体里的 `body` 字段：**引号被转义了**。这说明请求体是被当作「一段 JSON 文本」原样放进 HTTP body 里的，服务端再把它塞进自己的 JSON 字段——两层 JSON 嵌套，转义是必然的。

如果响应结构是固定的，直接反序列化成自己的类型更省事：

```rust
#[derive(Debug, serde::Deserialize)]
struct ApiResponse {
    method: String,
    body: String,
}

let parsed: ApiResponse = client.get(&url).send()?.json()?;
```

`.json()` 内部做两件事：**检查 HTTP 状态码**（4xx/5xx 会返回错误，除非调用过 `error_for_status` 的相反设置）和**用 serde 反序列化**。任何一步失败都会得到带上下文的 `reqwest::Error`（见 22.10）。

## 22.6 超时：网络代码的必选项

**不设超时的网络请求不是「慢」，而是「可能永远不返回」**。所以 `Client` 一定要设超时：

```rust
let client = reqwest::blocking::Client::builder()
    .timeout(Duration::from_millis(500))
    .build()?;
```

配套代码让服务器故意慢 1200 毫秒，实测：

```text
    请求失败：is_timeout = true，error sending request for url (http://127.0.0.1:58684/)
```

**这里有个关键细节：错误消息里根本没有 "timeout" 这个词。** 想判断「是不是超时」，必须用 `error.is_timeout()`：

```rust
match client.get(&url).send() {
    Ok(response) => println!("{}", response.status()),
    Err(error) if error.is_timeout() => println!("超时，可以重试"),
    Err(error) => println!("其他错误：{error}"),
}
```

按消息文本匹配（`error.to_string().contains("timeout")`）在不同的 reqwest 版本、不同的底层平台上都会失效——**认方法，不认消息**，这和 19.10 的 `ErrorKind` 是同一条原则。

`reqwest::Error` 提供的判断方法：

| 方法 | 含义 |
| --- | --- |
| `is_timeout()` | 超时 |
| `is_connect()` | 建立连接失败（DNS、端口不通） |
| `is_request()` | 请求构造/发送阶段出错 |
| `is_decode()` | 响应体解析失败（比如 JSON 格式不对） |
| `is_status()` | 服务器返回了错误状态码（配合 `error_for_status()`） |

## 22.7 连接失败与重试

如果目标端口没人监听，TCP 会明确报错：

```text
    连接不存在的主机端口：kind = ConnectionRefused
```

网络请求失败太常见了（服务在重启、网络抖动、偶发超时），所以**重试是标配**：

```rust
fn retry_get(url: &str, attempts: u32) -> Result<u16, reqwest::Error> {
    let mut last_error = None;
    for attempt in 1..=attempts {
        match reqwest::blocking::get(url) {
            Ok(response) => return Ok(response.status().as_u16()),
            Err(error) => {
                println!("第 {attempt} 次失败：{error}");
                last_error = Some(error);
                thread::sleep(Duration::from_millis(20));
            }
        }
    }
    Err(last_error.unwrap())
}
```

实测对一个必然失败的地址重试 3 次，三次都打印了失败原因。

重试有几个必须想清楚的点：

| 问题 | 建议 |
| --- | --- |
| 什么错误该重试？ | 连接失败、超时、5xx（服务端问题）——**4xx 不该重试**（重试多少次都是同样的客户端错误） |
| 间隔多久？ | 指数退避（20ms → 40ms → 80ms）加一点随机抖动，避免所有客户端同时重试 |
| 重试几次？ | 通常 2–3 次，再失败就上报；无限重试会把故障放大 |
| 请求能重复执行吗？ | **POST / 支付这类操作要小心**——重试可能造成重复下单，需要幂等键 |
| 有没有现成的？ | 生产里常用 `reqwest-retry` / `tower` 中间件，而不是手写 |

最后一条值得强调：**「重试」不是无脑循环，而是一个业务决策**。

## 22.8 并发请求

阻塞客户端 + 线程可以并发（第 13 章）：

```rust
let handle = thread::spawn(move || {
    reqwest::blocking::get(&url).unwrap().status().as_u16()
});
println!("{}", handle.join().unwrap());
```

实测子线程拿到 `200`。

但「一个线程一个请求」不能扩展：几百个并发连接就要几百个线程，每个线程的栈和调度都是开销。所以**高并发场景要用异步客户端**：

```rust
// 需要 tokio 运行时（第 16 章）和 reqwest 的 async API
let client = reqwest::Client::new();
let futures = urls.iter().map(|url| client.get(url).send());
let responses = futures::future::join_all(futures).await;
```

上面的 `join_all` 会让这些请求**真正并发**（复用少量线程、等待时不占线程）。这正是第 16 章说的「IO 密集用异步」在真实项目里的样子。

## 22.9 现实中的异步客户端

生产项目的典型配置是这样（**本仓库为了离线可编译只用了 blocking，这段没有参与本仓库的编译验证**，外网请求也没法在这个环境里验证）：

```toml
[dependencies]
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
reqwest = { version = "0.13", features = ["json", "rustls-tls"] }
```

```rust
#[tokio::main]
async fn main() -> Result<(), reqwest::Error> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()?;

    let response = client
        .get("https://example.com/api/items")
        .header("Authorization", "Bearer <token>")
        .send()
        .await?;

    let status = response.status();
    let items: Vec<Item> = response.json().await?;
    println!("{status}，拿到 {} 条", items.len());
    Ok(())
}
```

和 blocking 版本的区别只有两点：**多了个运行时**（`#[tokio::main]`）和**每个操作后加 `.await`**。超时、连接池、JSON 序列化这些能力完全一样。

选哪个？

| 场景 | 选择 |
| --- | --- |
| 写命令行工具、脚本，请求很少 | blocking（简单、不用运行时） |
| 服务端、要处理成百上千并发 | async |
| 已经在用 tokio | async（别混用 blocking，会卡住运行时） |
| 必须在异步里做阻塞 IO | `tokio::task::spawn_blocking`（第 16 章 16.7） |

## 22.10 四个真实报错怎么读

**案例 1：忘了打开 feature（`E0433`）**

```rust
// Cargo.toml 里只写了 features = ["json"]，没写 "blocking"
let response = reqwest::blocking::get("http://127.0.0.1:1/");
```

```text
error[E0433]: cannot find `blocking` in `reqwest`
   --> src/main.rs:2:29
    |
  2 |     let response = reqwest::blocking::get("http://127.0.0.1:1/");
    |                             ^^^^^^^^ could not find `blocking` in `reqwest`
    |
note: found an item that was configured out
   --> .../reqwest-0.13.4/src/lib.rs:374:13
    |
373 |     #[cfg(feature = "blocking")]
    |           -------------------- the item is gated behind the `blocking` feature
374 |     pub mod blocking;
    |             ^^^^^^^^
```

报错里那句 **`the item is gated behind the 'blocking' feature`** 是重点：不是「没这个方法」，而是「feature 没开」。第三方 crate 的很多能力都是可选的，遇到「找不到某个模块/方法」时，先去它的文档里确认 feature 名字。

**案例 2：连接被拒绝**

```text
    连接不存在的主机端口：kind = ConnectionRefused
```

标准的 `io::ErrorKind`（第 19 章），常见原因：服务没启动、端口写错、防火墙拦截。

**案例 3：超时**

```text
    请求失败：is_timeout = true，error sending request for url (http://127.0.0.1:58684/)
```

注意**消息里没有 "timeout" 字样**——所以判断超时必须用 `error.is_timeout()`。

**案例 4：响应体解析失败（错误链）**

如果服务端返回的不是合法 JSON，`.json()` 会报：

```text
reqwest::Error { kind: Decode, url: "http://127.0.0.1:53989/", source: Error("expected `,` or `}`", line: 1, column: 28) }
```

这是一个**错误链**的典型样子（第 8 章的 `source()`）：

| 层次 | 内容 |
| --- | --- |
| 最外层 | `reqwest::Error`，`kind = Decode` |
| 附加上下文 | `url` —— 哪个请求出的错 |
| 根因 | serde_json 的 `expected ',' or '}'`，带行列号 |

排错时**要一路看到最里面那层**：「请求失败」没有信息量，「URL x 的响应体第 1 行第 28 列少了个逗号」才能定位问题。这也是 22.7 那个重试函数要保留 `last_error` 而不是只打印「失败了」的原因。

## 22.11 常见坑速查

| 现象 | 原因 | 解法 |
| --- | --- | --- |
| 报「cannot find `blocking`」 | feature 没开 | `features = ["blocking"]` |
| HTTPS 请求报 TLS 相关错误 | 关了默认 feature 又没开 tls | 加 `rustls-tls` 或 `default-tls` |
| 请求「卡住」很久 | 没设超时 | `Client::builder().timeout(...)` |
| 判断超时失败 | 按消息文本匹配 | 用 `error.is_timeout()` |
| 每个请求都新建连接，很慢 | 每次调 `reqwest::get` | 建一次 `Client` 复用 |
| 重试把订单下了两次 | 对非幂等操作重试 | 用幂等键，或只对 GET / 幂等接口重试 |
| 重试风暴把服务打垮 | 固定间隔、无上限重试 | 指数退避 + 抖动 + 次数上限 |
| 4xx 也一直重试 | 没区分错误类型 | 只重试连接失败 / 超时 / 5xx |
| TCP 收到的数据「粘在一起」或「断成两半」 | 把字节流当成了一条条消息 | 用协议定义边界（长度前缀 / 分隔符 / HTTP 头） |
| 服务端起在固定端口，测试偶尔失败 | 端口被占用 | 绑 `127.0.0.1:0` 让系统分配 |
| 异步任务里调 blocking 客户端 | 阻塞会卡住整个运行时线程 | 用 async 客户端，或 `spawn_blocking` |
| 响应体太大，内存爆掉 | `text()` / `json()` 会一次性读进内存 | 用 `bytes_stream()` 流式处理（async） |

## 22.12 练习

1. 用 `TcpStream::connect` 写一个 `fn probe(addr: &str) -> Result<(), std::io::Error>`，连不上时把 `ErrorKind` 一起返回。
2. 用 reqwest 请求本机的一个 HTTP 服务，把响应的 JSON **反序列化成结构体** `struct ApiResponse { method: String, body: String }`（提示：`#[derive(Deserialize)]` + `.json()`）。
3. 写 `fn retry_with_backoff(url: &str, max_attempts: u32) -> Result<u16, reqwest::Error>`，使用指数退避（20ms → 40ms → 80ms），并测量总耗时。
4. 并发发起 3 个请求（每个请求打到一个独立的本地服务器），统计成功的数量。
5. 回答两个问题：(a) 为什么判断超时要用 `error.is_timeout()` 而不是看错误消息？(b) TCP 是「字节流」，这句话对写协议有什么影响？

（第 1、5 题是 22.2、22.6 和 22.1 讨论过的内容，这里不再单独给答案。）

### 参考答案

::: details 第 2 题

```rust
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct ApiResponse {
    method: String,
    body: String,
}

fn main() {
    let url = spawn_server();          // 22.3 里那个迷你 HTTP 服务器
    let parsed: ApiResponse = reqwest::blocking::get(&url).unwrap().json().unwrap();
    println!("method = {}, body = {:?}", parsed.method, parsed.body);
}
```

实测输出 `method = GET, body = ""`（这一节演示的是 GET，没有请求体）。

注意这里**没有手动调 `serde_json::from_str`**：`.json()` 已经帮你检查状态码 + 读取 body + 反序列化。**失败时会返回带 URL 和行列号的错误链**（22.10 案例 4），比自己拼字符串再解析省事得多。

:::

::: details 第 3 题

```rust
use std::thread;
use std::time::Duration;

fn retry_with_backoff(url: &str, max_attempts: u32) -> Result<u16, reqwest::Error> {
    let mut delay = Duration::from_millis(20);
    let mut last_error = None;

    for attempt in 1..=max_attempts {
        match reqwest::blocking::get(url) {
            Ok(response) => return Ok(response.status().as_u16()),
            Err(error) => {
                last_error = Some(error);
                if attempt < max_attempts {
                    thread::sleep(delay);
                    delay *= 2;          // 20ms → 40ms → 80ms
                }
            }
        }
    }
    Err(last_error.unwrap())
}
```

实测对一个必然失败的地址调用（3 次尝试）：返回 `Err`，**总耗时 ≥ 60ms**（两次退避 20 + 40），说明退避真的生效了。

生产代码里还会加两个改进：**抖动**（把 delay 乘一个 0.5–1.5 的随机系数，避免所有客户端同时重试）和**可重试判定**（只重试 `is_timeout()` / `is_connect()` / 5xx，4xx 直接放弃）。

:::

::: details 第 4 题

```rust
use std::thread;

fn main() {
    let handles: Vec<_> = (0..3)
        .map(|_| {
            let url = spawn_server();   // 每个线程一个独立的服务器
            thread::spawn(move || {
                reqwest::blocking::get(&url).unwrap().status().as_u16()
            })
        })
        .collect();

    let ok = handles
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .filter(|status| *status == 200)
        .count();

    println!("3 个并发请求中成功 {ok} 个");
}
```

实测输出 `3 个并发请求中成功 3 个`。

这里的并发是「线程并发」：每个线程一个阻塞请求。它简单直接，但**一个连接一个线程**——上到几百个就吃不消了。要真正扩展，就用 22.9 的 async 版本 + `join_all`。

:::

## 22.13 小结

- TCP 提供的是**可靠的字节流**：有序、不丢，但**没有消息边界**——协议必须自己定义边界（HTTP 用 `Content-Length`）。

- 标准库的 `TcpListener` / `TcpStream` 足以理解网络本质：绑定 → 连接 → 读写字节；端口写 `0` 让系统分配，测试里很有用。

- HTTP 就是 TCP 上的一段文本：请求行 + 头部 + 空行 + 请求体；自己手写一次，再看头部字段就清楚了。

- 用 `reqwest` 时**建一次 `Client` 反复用**（连接池），并**一定要设超时**。

- POST JSON 用 `.json(&payload)` 发送、`.json::<T>()` 接收，和第 20 章的 serde 无缝衔接。

- 判断错误类型要用 `is_timeout()` / `is_connect()` / `is_decode()`，**不要匹配错误消息**；超时的消息里通常没有 "timeout" 字样。

- 重试是标配但要有策略：只重试可重试的错误（连接失败、超时、5xx）、指数退避 + 抖动、有次数上限；**非幂等操作要慎重重试**。

- 高并发用异步客户端（`reqwest` + tokio + `join_all`），别用「一个连接一个线程」硬扛。

- 生产代码里要保留完整的错误链：`reqwest::Error { kind, url, source }` 里那层 `source` 往往才是真正的根因。

- 本章所有结论都在本机回环上实测（GET/POST/超时/重试/并发/原始 TCP）；涉及外网 HTTPS 的片段如实标注为未验证。

下一章是**第 23 章 测试进阶与基准**：单元 / 集成 / 文档测试的组织方式、`assert` 家族与测试夹具，以及用 `criterion` 做性能基准。
