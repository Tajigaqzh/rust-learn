# 第 30 章 · 异步进阶

> 前置：第 16 章（async/await）。本章假设你理解 `Future`、`poll`、
> `Waker` 和「不 poll 就不执行」这些第 16 章手工运行时里演示过的概念。

第 16 章用手写的 `block_on` 把 async 的底层机制拆开看了一遍，当时留了
两个悬而未决的问题：

1. `poll` 的参数为什么是 `Pin<&mut Self>`？`Pin` 到底是什么？
2. 真实的 tokio 长什么样？（当时环境下载不了 tokio，16.10 的代码
   只停留在纸上。）

本章还清这两笔债：先把 `Pin`/`Unpin` 讲透（第 29 章的自引用类型
正好是它的前置），再把 tokio 作为**真实依赖**引入，覆盖异步代码里
每天都在用的三块：同步原语（`tokio::sync`）、并发组合（`select!`/
`join!`）和取消语义。

本章代码在 `src/rust30_async_advanced/`，`cargo run` 看输出，
`cargo test rust30_async_advanced` 跑本章测试。tokio 已加入
`Cargo.toml`（`rt-multi-thread`、`macros`、`time`、`sync` 四个
feature，不开 `full`）。

## 30.1 Pin 与 Unpin：`poll` 签名里的那个悬案

先回忆第 16.3 章的签名：

```rust
fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output>
```

为什么不是 `&mut self`？答案是：**async 状态机是自引用结构**。

`async fn` 里的局部变量会被编译器切成状态机的字段。看这个函数：

```rust
async fn example() {
    let data = String::from("hello");     // 拥有
    let r = &data;                         // 借用同函数里的局部变量
    other().await;                         // 挂起点：data 和 r 都要活过这里
    println!("{}", r.len());
}
```

挂起时，`data` 和 `r` 都得存进状态机——于是状态机里有一个字段（`r`）
指向另一个字段（`data`）。**这就是第 29.4 章讲的自引用结构**。

自引用结构的死穴是移动：状态机被 move 到新地址后，`r` 还指着旧地址。
而 `poll` 需要拿到状态机的可变引用——如果直接给 `&mut self`，调用方
就能通过 `std::mem::swap`、`Box` 重组等手段移动它，悬垂就发生了。

`Pin` 的设计就是堵这个洞：

```text
Pin<&mut T> = 一个 &mut T，附带承诺：只要 T 还活着，它就不会被移动
```

- 对 `Pin<&mut T>` 你拿不到裸的 `&mut T`（除非 `T: Unpin` 或 unsafe）。
- 拿不到 `&mut T`，就不能 `swap`/`replace`/move 它。
- 不移动，自引用永远有效。

### `Unpin`：绝大多数类型的豁免

如果 `T` **没有**自引用（String、Vec、数字、几乎所有普通类型），
移动它毫无危险。为这类类型每次都走 `Pin` 的限制太烦，所以有个
自动 trait：

```rust
// String 实现了 Unpin——「我可以随便移动」
let mut s = String::from("hi");
let pinned = Pin::new(&mut s);
let back: &mut String = pinned.get_mut();   // 安全！因为 String: Unpin
```

`Unpin` 是 **auto trait**：所有字段都是 Unpin 的类型自动 Unpin。
`!Unpin`（不实现 Unpin）的类型只有两种：手写了 `PhantomPinned` 的，
和编译器生成的 async 状态机。配套代码用第一种构造了一个诚实的
自引用雏形：

```rust
struct SelfRef {
    data: String,
    self_ptr: *const String,        // 指向自己的 data 字段
    _pin: PhantomPinned,            // 零大小，唯一作用：关掉 Unpin
}
```

`SelfRef::new` 里 `Box::pin` 把它放到堆上固定；之后想拿回 `&mut SelfRef`
只能 `unsafe { pinned.get_unchecked_mut() }`——对 `!Unpin` 类型，
`Pin::get_mut` 直接编译不过。这就是类型系统在替你站岗。

现在可以完整回答 16.3 的悬案了：**`poll` 拿 `Pin<&mut Self>` 是在向
编译器声明「这个 async 状态机不许整体移动」，它内部的自引用才合法。**

日常写 `async`/`.await` 时你不需要手碰 `Pin`——编译器和运行时包办。
需要直面的时刻只有两个：写自定义 `Future`（16.3 干过），和读
`Box::pin`/`pin!` 这类 API 时（30.4 会用到 `pin!`）。

## 30.2 从手写运行时到 tokio

第 16 章的手写 `block_on` 只能跑「轮询到自己 Ready」的简单 future。
真实运行时要做的多得多：epoll/kqueue/IOCP 驱动的 IO 事件循环、
工作线程池、定时器堆、任务调度。这就是 tokio 的本体。

本章 `Cargo.toml` 里的 tokio 是**真实依赖**了（不再是 16.10 的纸上
预告）：

```toml
tokio = { version = "1", features = ["rt-multi-thread", "macros", "time", "sync"] }
```

四个 feature 各管一件事：`rt-multi-thread`（多线程调度器）、`macros`
（`#[tokio::main]`/`#[tokio::test]`）、`time`（定时器）、`sync`
（30.3 的同步原语）。

`#[tokio::main]` 宏展开后就是「建运行时 + block_on」：

```rust
#[tokio::main]
async fn main() {
    // 展开为：
    // tokio::runtime::Builder::new_multi_thread()
    //     .enable_all().build().unwrap()
    //     .block_on(async { ... })
}
```

本项目 `main` 是同步的（28 个章节按顺序演示），所以配套代码
`async_advanced_demo` 里手动做了同样的事——`Builder::new_multi_thread()
.build()` + `block_on`。两种写法是一回事。

`tokio::spawn` 把 future 提交给调度器并发执行，返回 `JoinHandle`：

```rust
let handle = tokio::spawn(async {
    tokio::time::sleep(Duration::from_millis(10)).await;
    42
});
// ... 干别的 ...
let answer = handle.await.unwrap();   // 等它完成，取返回值
```

`spawn` 的 future 必须 `'static + Send`——第 16.8 章讲过 `Send` 约束
的原因（任务可能被调度到别的线程），`'static` 是因为任务的生命周期
不再由创建它的作用域管理。

## 30.3 tokio::sync：按通信形状选原语

std 的 `mpsc`（第 13 章）在 async 里有问题：`recv()` 是**阻塞**的——
在线程上睡等。异步任务睡住线程就浪费了它（一个线程上排着成千上万个
任务）。tokio 的 `sync` 模块提供感知 `.await` 的版本，外加几个 std
没有的形状：

| 原语 | 形状 | 典型场景 |
| --- | --- | --- |
| `oneshot` | 一次发送、一个值、一个接收者 | 请求-回复、结果句柄 |
| `mpsc` | 多发送者、单接收者、队列 | 任务间流水线 |
| `broadcast` | 多发送者、多接收者、各收各的 | 事件扇出（日志同时进控制台和文件） |
| `watch` | 单最新值，后来者只见当前 | 配置热更新、关闭信号 |
| `Mutex` | async 互斥锁 | **必须跨 .await 持锁**时 |

配套代码逐一演示。几个值得展开的点：

### watch：只有「最新值」的通道

```rust
let (tx, mut rx) = tokio::sync::watch::channel("running");
tx.send("draining").ok();
tx.send("stopped").ok();                  // 中间值被覆盖
println!("{}", *rx.borrow_and_update());   // stopped
```

`broadcast` 是回放历史（谁订阅谁从头收），`watch` 是**只留最新**。
这个差异决定了用途：事件日志用 broadcast（每条都不能丢），「当前
服务状态」用 watch（晚来的订阅者只关心现在，不关心历史）。优雅
关闭的经典实现就是一个装着 `bool` 的 watch：所有任务 `rx.changed()`
等着它变 true——第 28 章 28.2 节的「就绪探针」就是这种模式。

### async Mutex vs std Mutex：就看要不要跨 await

第 13 章的 `std::sync::Mutex` 在异步代码里**依然合法**——事实上短
临界区用它更快。分界线只有一条：

```rust
// std 锁：guard 不是 Send。持有 guard 跨 await 的 future
// 就不是 Send → 不能 spawn。编译器直接拒绝。
let g = std_mutex.lock().unwrap();
do_io().await;        // ← E0277: future cannot be sent between threads safely
drop(g);

// async 锁：guard 是 Send 的，跨 await 持有合法。
let mut g = async_mutex.lock().await;
*g += 1;
tokio::time::sleep(Duration::from_millis(1)).await;   // OK
```

配套代码 `mutex_demo` 演示了跨 await 持有 async Mutex。经验法则：
**锁的临界区里没有 `.await` → 用 std 锁；有 → 用 async 锁**（或重构
把 await 挪出临界区，通常更好）。

## 30.4 select! 与 join!

两个组合子是异步控制流的主干：

**`tokio::join!`——全等待。** 所有分支都完成才继续，返回值按书写
顺序排列：

```rust
let (a, b) = tokio::join!(fetch_a(), fetch_b());
```

**`tokio::select!`——赛跑。** 同时 poll 所有分支，**谁先就绪走谁，
其余分支整体丢弃**：

```rust
tokio::select! {
    msg = rx.recv() => println!("收到 {msg:?}"),
    _ = tokio::time::sleep(Duration::from_secs(1)) => println!("超时"),
}
```

配套代码 `select_demo` 用 10ms vs 50ms 两个 sleep 演示。理解
`select!` 的关键是它编译成什么：把每个分支的 future 收集起来，每次
poll 时问所有分支「你好了吗」——第一个 Ready 的胜出，**没 Ready 的
future 全部被 drop**。最后半句是 30.5 全部内容的伏笔。

## 30.5 取消：Drop 即取消

Rust 异步最独特也最容易误解的机制：**没有 abort 标志位，没有线程
中断，取消 = 不再 poll + 析构（drop）**。

一个 future 被 drop 后：

- 它内部的所有局部变量（状态机字段）正常析构——`Drop` 会跑；
- `.await` 之后的代码**永远不执行**；
- 如果它正持有锁/占用连接，随析构释放。

配套代码四个演示，从直接到间接：

**1. 直接 drop：**

```rust
let fut = cancels_me();   // 里面 sleep(60s) 之后 println
drop(fut);                 // 睡眠和 println 全部消失
```

**2. `timeout`——官方封装的「select + 定时器」：**

```rust
match tokio::time::timeout(Duration::from_millis(20), slow_op()).await {
    Ok(result) => { /* 20ms 内完成 */ }
    Err(_)    => { /* 到点，slow_op 的 future 已被 drop 取消 */ }
}
```

**3. `JoinHandle::abort`——从外部取消已 spawn 的任务：**

```rust
let handle = tokio::spawn(long_task());
handle.abort();
let err = handle.await.unwrap_err();
assert!(err.is_cancelled());   // JoinError::is_cancelled()
```

abort 是**协作式的**：运行时在下一个挂起点把任务 drop，不是抢占式
杀线程。所以任务里没有挂起点（纯计算循环）时，abort 要等它让出。

**4. `select!` 的隐式取消**——最重要也最隐蔽。回看 30.4 的结尾：
未命中的分支**整体丢弃**。如果某个分支「干了一半」，进度就没了：

```rust
// 危险模式：先把数据取出来，再等别的条件
tokio::select! {
    _ = other_signal() => { ... },
    _ = async {
        let data = rx.recv().await.unwrap();   // 取出了数据
        process(data).await;                    // ← 这里没等到 select 结束就被 drop
    } => { ... },
}
```

`select!` 被另一分支胜出时，内层 async 块可能在 `process(data)` 中途
被 drop——**data 丢了**。这就是「取消安全性」（cancellation safety）：
一个 future 在任意挂起点被取消都不丢状态，它就是取消安全的。

好消息：tokio 官方通道的 `recv()` 都是取消安全的——没命中时消息
留在队列里，下次还能收到。配套代码 `cancellation_safety_demo` 验证
了这一点：select 胜出后队列里的消息一条不少。经验法则：

- 通道 `recv`/`send`、`sleep`、简单 IO：取消安全，放心进 select!；
- 「多步事务」（读出来→处理→写回）包在一个分支里：不安全——要么
  拆成独立任务用消息驱动，要么用 `tokio_util::sync::CancellationToken`
  在事务边界主动检查取消。

`CancellationToken` 来自 `tokio-util` crate（本章没引入依赖，知道
形状即可）：一个可 `.clone()` 扩散、`.cancel()` 广播、`.cancelled()`
可 await 的「取消令牌」，是 watch 关闭信号的工程化封装。

## 30.6 真实报错怎么读

**例 1：spawn 了借用局部变量的任务**

```text
error: future cannot be sent between threads safely
note: ... has a lifetime `'a` ... borrowed value does not live long enough
```

`spawn` 要求 `'static`。`async move` 把所有权搬进去，或用
`tokio::scope` 式的受限借用（scoped spawn 还在演进，目前主流是 move
或用通道传引用数据）。

**例 2：std Mutex guard 跨 await**

```text
error: future cannot be sent between threads safely
note: ... within the lifetime of `MutexGuard<'_, ...>`
note: await occurs here, with the guard maybe used later
```

报错直接把现场拍脸上了。修法：把 `.await` 挪出临界区，或换
`tokio::sync::Mutex`（30.3）。

**例 3：对 `!Send` 的 future 用 `#[tokio::main]`**

```text
error: future cannot be sent between threads safely
note: `Rc<...>` cannot be sent between threads safely
```

`Rc`/`RefCell` 进了 async 块。换 `Arc`/`Mutex`。

**例 4：abort 后 await 返回 Err 没处理**

```text
error[E0308]: mismatched types  // handle.await 是 Result，不是 ()
```

`JoinHandle::await` 返回 `Result<T, JoinError>`——任务可能被 abort，
也可能 panic。用 `if let Err(e) if e.is_cancelled()` 处理。

## 30.7 常见坑速查

| 坑 | 症状 | 解法 |
| --- | --- | --- |
| 以为取消是「标记后继续跑」 | await 之后的清理代码没执行 | 取消 = drop；清理放进 `Drop` 或 RAII 守卫 |
| 多步事务塞进 select! 分支 | 分支被取消时数据丢失 | 拆任务/消息驱动，或 CancellationToken |
| 短临界区也用 async Mutex | 白付跨线程开销 | 无 await 用 std 锁 |
| std 锁 guard 跨 await | future not Send | 挪 await 或换 async 锁 |
| watch 期望收全历史 | 只看到最新值 | 要历史用 broadcast |
| broadcast 消费者落后被踢 | `RecvError::Lagged` | 加大容量或处理 Lagged |
| 忘了 `handle.await` | 任务结果/panic 被静默丢弃 | await JoinHandle，至少 `.await.ok()` |
| 纯计算任务 abort 不动 | 任务迟迟不死 | 计算里定期 `.await` 让出（`yield_now`） |

## 30.8 练习

1. 写一个「带超时的一揽子请求」：用 `join!` 同时发 3 个模拟请求
   （sleep 10ms/30ms/50ms 各返回一个值），整体 40ms 超时——预期
   只有两个成功。分别用 `tokio::time::timeout` 包住整个 `join!`
   和分别包住每个请求两种方式实现，比较语义差别。
2. 用 `watch` 实现优雅关闭：一个任务循环干活（每 50ms 一次），
   主任务 200ms 后发出关闭信号；干活任务收到信号后完成当前这轮
   再退出（不是立刻中断）。
3. 把 30.5 的危险模式写出来观察丢数据：一个分支里 `recv` 两个消息
   相加，另一个分支 5ms 就绪。多跑几次，体会「半途被取消」。

### 参考答案

<details>
<summary>点开前先自己写</summary>

```rust
// 1. 两种超时的语义差别：包住整个 join!——只要最慢的没赶上，
//    三个全丢（Result 整体是 Err）；分别包——快的两个保得住。
async fn fake(id: u8, ms: u64) -> u8 {
    tokio::time::sleep(Duration::from_millis(ms)).await;
    id
}

// 方式 A：整体超时
let all = tokio::time::timeout(
    Duration::from_millis(40),
    tokio::join!(fake(1, 10), fake(2, 30), fake(3, 50)),
).await;
assert!(all.is_err());            // 50ms 那个拖垮了全队

// 方式 B：逐个超时，快的保住
let (a, b, c) = tokio::join!(
    tokio::time::timeout(Duration::from_millis(40), fake(1, 10)),
    tokio::time::timeout(Duration::from_millis(40), fake(2, 30)),
    tokio::time::timeout(Duration::from_millis(40), fake(3, 50)),
);
assert_eq!(a.unwrap(), 1);
assert_eq!(b.unwrap(), 2);
assert!(c.is_err());              // 只牺牲最慢的
```

```rust
// 2. watch 关闭信号：收到信号后干完当前轮再退。
let (shutdown_tx, mut shutdown_rx) = tokio::sync::watch::channel(false);
let worker = tokio::spawn(async move {
    let mut round = 0;
    loop {
        // 先干活后检查：保证「完成当前这轮」。
        round += 1;
        tokio::time::sleep(Duration::from_millis(50)).await;
        if *shutdown_rx.borrow() {
            println!("完成第 {round} 轮后退出");
            break;
        }
    }
    round
});
tokio::time::sleep(Duration::from_millis(200)).await;
shutdown_tx.send(true).ok();
let rounds = worker.await.unwrap();
assert!(rounds >= 3);
```

```rust
// 3. 半途取消丢数据：recv 两个消息相加的分支被 5ms 分支抢先。
let (tx, mut rx) = tokio::sync::mpsc::channel(8);
for i in 0..10u8 { tx.send(i).await.unwrap(); }
drop(tx);

tokio::select! {
    sum = async {
        let a = rx.recv().await.unwrap_or(0);
        let b = rx.recv().await.unwrap_or(0);   // ← 可能在这被 drop
        a + b
    } => println!("完整拿到，和 = {sum}"),
    _ = tokio::time::sleep(Duration::from_millis(5)) => {
        println!("被抢先：第一条 a 已从队列取出，b 还没取——a 丢了");
    }
}
// 关键：a 取出后分支若被取消，a 不会回队。这就是为什么
// 官方文档强调「多步操作不是取消安全的」。
```

</details>

## 30.9 小结

- async 状态机是自引用结构（第 29.4 的概念在 async 世界的化身），
  移动会让内部引用悬垂；`Pin<&mut Self>` 把「不许移动」写进类型。
- `Unpin` 是 auto trait，普通类型全部豁免；`PhantomPinned` 是手动
  退出豁免的开关。日常 async 不手碰 Pin，但必须能读懂它。
- tokio 四件套 feature 就够本章用：rt、macros、time、sync。
- 同步原语按**通信形状**选：oneshot 单值一次性、mpsc 流水线、
  broadcast 扇出回放、watch 只留最新。锁的分界线是「跨不跨 await」。
- `join!` 全等待，`select!` 赛跑并丢弃未就绪分支。
- 取消 = drop：没有中断，只有析构。多步事务不取消安全——拆任务或
  用 CancellationToken。

至此，第 16 章埋下的两条线索（Pin 悬案、真实 tokio）都收回了。
异步部分还值得继续的方向：`tokio::net` 写真实网络服务（和第 22 章
的 TCP 知识结合）、`tower` 中间件生态、`tracing` 的 async 感知 span
——它们都建立在本章的原语之上。
