# 第 16 章 · async/await

第 13 章的线程是**操作系统级**的并发：一个线程一个栈，由内核调度。这一章讲**任务级**的并发：成千上万个轻量任务跑在少数几个线程上，等待 IO 时不占线程。

为什么需要它？设想一个服务器要同时处理一万个连接：

| 方案 | 一万个连接时 |
| --- | --- |
| 一个连接一个线程 | 一万个线程，每个栈默认 MB 级内存，调度开销也很大 |
| 异步任务 | 少量线程 + 一万个任务，等待网络时线程去跑别的任务 |

所以选择标准很清楚：**CPU 密集就用线程（或 rayon 这类并行库），IO 密集（网络、文件、数据库）用异步**。

还有一个重点：Rust 的 `async` 是**零成本**的——`async fn` 会被编译成一个状态机，不需要 GC、不额外分配堆内存。这也是「高级抽象、底层性能」的又一个例子。

本章配套代码在 `src/rust16_async/mod.rs`。为了保持整个项目零依赖，**本章只用标准库**，异步运行时由配套代码里几十行的 `block_on` 充当。

## 16.1 `async fn` 返回的是 `Future`，不 poll 就不执行

给函数加上 `async`，它的返回类型就变成了「一个还没开始跑的 `Future`」：

```rust
async fn compute() -> i32 {
    println!("compute 真的开始执行了");
    42
}

let future = compute();      // 这一步不会打印任何东西
```

实测输出：

```text
    创建 compute() 的 future（此时还没有任何输出）
    future 已经拿到手，但它还没运行
        compute 真的开始执行了
    block_on 之后：42
```

**`async fn` 调用时不执行函数体**，只是构造了一个状态机。如果拿到 future 却什么都不做，编译器会警告：

```text
warning: unused implementer of `Future` that must be used
 --> src/main.rs:6:5
  |
6 |     compute();
  |     ^^^^^^^^^
  |
  = note: futures do nothing unless you `.await` or poll them
  = note: `#[warn(unused_must_use)]` (part of `#[warn(unused)]`) on by default
```

`note` 那行就是全部要义：**Future 是惰性的，不 `.await` 或不 `poll` 就什么都不发生**。

## 16.2 `.await` 到底做了什么

`.await` 是语法糖，展开后大致是：

```rust
// let value = some_future.await;
match some_future.poll(cx) {
    Poll::Ready(value) => value,          // 好了，拿到值继续往下走
    Poll::Pending => return Poll::Pending, // 没就绪：把控制权交还给运行时
}
```

关键在 `Pending` 那一支：**当前任务把控制权交回去，运行时就趁机去跑别的任务**。等这个 future 真的就绪了，运行时再回来从断点继续。

这就是「等待时不占线程」的实现方式：一个线程上可以挂很多个 `Pending` 的任务。

两个限制：

- `.await` 只能出现在 `async fn` 或 `async` 块里，否则报 `E0728`（见 16.11）。
- **`main` 不能是 `async`**，直接写 `async fn main` 会报 `E0752`。真实项目里用 tokio 提供的 `#[tokio::main]` 宏来包装（16.10），本章的配套代码则用一个自己写的 `block_on`。

## 16.3 手动实现 `Future`

`Future` trait 只有一个必须实现的方法：

```rust
pub trait Future {
    type Output;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output>;
}
```

`Poll` 就两个分支：`Ready(值)` 或 `Pending`。先做一个永远就绪的：

```rust
struct Ready(i32);

impl Future for Ready {
    type Output = i32;

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<i32> {
        Poll::Ready(self.0)
    }
}
```

再做一个「要 poll 好几次才就绪」的，这样能看清 `Pending` 的作用：

```rust
struct Countdown {
    remaining: u32,
}

impl Future for Countdown {
    type Output = u32;

    fn poll(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<u32> {
        if self.remaining == 0 {
            Poll::Ready(0)
        } else {
            self.remaining -= 1;
            println!("Countdown poll 一次，还剩 {}", self.remaining);
            Poll::Pending
        }
    }
}
```

实测：

```text
    block_on(Ready(7)) = 7
        Countdown poll 一次，还剩 2
        Countdown poll 一次，还剩 1
        Countdown poll 一次，还剩 0
    block_on(Countdown::new(3)) = 0
```

**每次被 poll，future 都要从上次中断的地方继续**——所以 `async fn` 编译出来的状态机会把「执行到哪一行」「局部变量是什么」都存起来。这也是为什么 `async fn` 的局部变量会进入 future 的内存布局。

### 为什么 `poll` 的参数是 `Pin<&mut Self>`

`async fn` 编译出的状态机可能**自己引用自己**（比如一个局部变量被后面的分支借用）。如果这时 future 被移动，内部引用就悬垂了。

`Pin<&mut T>` 是一个「保证不会移动 T」的包装：**只要实现了 `Future` 的自引用安全性，就必须用 Pin 把地址固定住**。日常写 `async fn` 不用管它；只有当你要手动实现 `Future`、或者把 future 存进结构体时才会遇到（那时候用 `Box::pin` 最常见）。

## 16.4 最小运行时：`block_on`

有了 `Future` 和 `poll`，一个「运行时」最小可以只有几行：

```rust
use std::future::Future;
use std::task::{Context, Poll, Waker};

fn block_on<F: Future>(future: F) -> F::Output {
    let waker = Waker::noop();                       // 一个「什么都不做」的唤醒器
    let mut context = Context::from_waker(waker);
    let mut future = std::pin::pin!(future);         // 把 future 固定在栈上

    loop {
        match future.as_mut().poll(&mut context) {
            Poll::Ready(value) => return value,      // 完成
            Poll::Pending => std::thread::yield_now(),// 没就绪就让出 CPU 再试
        }
    }
}
```

实测它能跑起来：

```text
    block_on(add(1, 2)) = 3
    block_on(double(21)) = 42
```

`.await` 串起来的异步函数同样能跑：

```rust
async fn chain() -> i32 {
    let a = add(1, 2).await;
    let b = double(a).await;
    a + b
}
```

实测 `block_on(chain()) = 9`（`a = 3`，`b = 6`，返回 `9`）。

这个 `block_on` 就是所有异步运行时的核心骨架。真实运行时在此基础上做了几件事：

| 职责 | 说明 |
| --- | --- |
| 调度器 | 管理成千上万个任务，决定谁先跑（tokio 用 work-stealing 多线程调度） |
| IO 事件 | 用操作系统的 epoll / kqueue / IOCP 等待网络和文件就绪 |
| 定时器 | 支持 `sleep`、超时，超时到了就唤醒对应任务 |
| 唤醒机制 | 真正实现 `Waker`：任务就绪时把任务重新排进队列 |

所以我们写的 `Waker::noop()` 其实是个「假的」唤醒器：它不会真的唤醒任何东西，`block_on` 只能靠 `yield_now()` 一遍遍轮询。**真实运行时的 Waker 会在 IO 就绪时主动叫醒任务，不需要空转。**

## 16.5 交替 poll：并发的雏形

两个 future 可以**交替推进**，谁都不用等谁跑完：

```rust
let mut first = Box::pin(Countdown::new(2));
let mut second = Box::pin(Countdown::new(1));

while !first_done || !second_done {
    if !first_done {
        match first.as_mut().poll(&mut context) { /* Ready 或 Pending */ }
    }
    if !second_done {
        match second.as_mut().poll(&mut context) { /* ... */ }
    }
}
```

实测：

```text
    poll A ->         Countdown poll 一次，还剩 1
    poll B ->         Countdown poll 一次，还剩 0
    poll A ->         Countdown poll 一次，还剩 0
    poll B -> Ready(0)
    poll A -> Ready(0)
```

两个 future 交叉推进，各自推进了 2 次和 1 次——**这就是并发的本质**：不是「同时执行」，而是「在等待时切换去做别的事」。

标准库没有提供组合子，社区库（`futures`、`tokio`）里的常用工具就是这套机制：

| 组合方式 | 语义 | 对应工具 |
| --- | --- | --- |
| 都完成才继续 | 并行等待多个结果 | `join!` / `try_join!` |
| 任一完成就继续 | 竞速、超时 | `select!` / `timeout` |
| 独立推进、互不等待 | 派生后台任务 | `tokio::spawn` |

## 16.6 `async` 块与 `async move`

除了 `async fn`，还可以写 `async` 块——它同样返回一个 future：

```rust
let future = async {
    let value = add(1, 2).await;
    value * 10
};
```

需要把外部变量交给 future 时用 `async move`，和闭包的 `move` 是同一个道理（第 11 章）：

```rust
let data = String::from("hello");
let future = async move {
    data.len()          // data 被移动进 future
};
// println!("{data}");  // 这里已经不能用了
```

`async move` 与 `move` 闭包的对照：

| | 闭包 | async 块 |
| --- | --- | --- |
| 立即执行 | 调用时才执行 | **创建后不执行**，要 poll |
| 捕获方式 | `Fn` / `FnMut` / `FnOnce` | 类似，`async move` 表示按值捕获 |
| 调用方式 | `f()` | `.await` 或 `poll` |

`async move` 最常见的用途是把任务交给运行时：

```rust
tokio::spawn(async move {
    // 这里拥有所有权的数据可以安全地跨线程
});
```

## 16.7 异步里不能阻塞

这是新手最容易踩的坑，也是异步性能事故的头号原因：

```rust
// 千万别在 async 函数里这么写
async fn bad() {
    std::thread::sleep(std::time::Duration::from_secs(1));   // 阻塞整个线程
}
```

`std::thread::sleep` 会让**当前线程**睡 1 秒。可是一个运行时线程上可能挂着成千上万个任务——这一睡，**所有任务一起卡住**。

同样的道理适用于：

| 阻塞操作 | 后果 | 正确做法 |
| --- | --- | --- |
| `thread::sleep` | 占住线程 | `tokio::time::sleep(...).await` |
| 标准库的文件 / 网络 IO | 占住线程 | 用异步版本（`tokio::fs`、`tokio::net`） |
| 大量 CPU 计算 | 占住线程 | 拆小、或者交给 `spawn_blocking` / 线程池 |
| 同步锁（`Mutex`）跨 `.await` 持有 | 可能死锁 | 用异步锁，或把锁的范围缩到 `.await` 之前 |

判断标准很简单：**任何「会让线程停下来等」的操作，都不能直接出现在异步任务里**。要么用它的异步版本，要么用 `spawn_blocking` 把它丢到专门的阻塞线程池。

## 16.8 Future 的 `Send` 约束

多线程运行时（比如默认配置的 tokio）会把任务在线程之间搬来搬去，所以 `spawn` 要求：

```text
F: Future + Send + 'static
```

和第 13 章 `thread::spawn` 的要求一模一样（第 9 章的 trait bound 又出现了）。最常见的报错是「把非 `Send` 的东西跨 `.await` 持有」：

```rust
// 典型的编译错误来源（示意）
let guard = rc_value.borrow_mut();     // Rc<RefCell<T>> 不是 Send
do_something().await;                  // 跨越了 .await，guard 还在作用域里
```

编译器会告诉你「future 不是 `Send`」，而**修法通常是缩短跨 `.await` 的借用**：先把需要的数据取出来、把锁释放掉，再去 `.await`。

如果确实要用单线程模型，运行时也提供了出路：tokio 的 `current_thread` 运行时和 `LocalSet` 允许任务不满足 `Send`——代价是所有任务挤在一个线程上。

## 16.9 常用组合子与工具

真实项目里很少手写 `poll`，绝大多数需求都能用现成的组合子表达：

| 工具 | 作用 |
| --- | --- |
| `join!` | 并行等待多个 future，全部完成才继续 |
| `try_join!` | 同上，但任一返回 `Err` 就提前结束 |
| `select!` | 谁先完成就用谁，常用于超时和竞速 |
| `tokio::time::timeout` | 给 future 加超时 |
| `tokio::spawn` | 派生独立任务，返回 `JoinHandle` |
| `FuturesUnordered` | 动态收集一批 future，谁就绪处理谁 |

**超时**是最常用的一个：

```rust
match tokio::time::timeout(Duration::from_secs(3), request()).await {
    Ok(Ok(response)) => println!("拿到响应"),
    Ok(Err(error)) => println!("请求失败：{error}"),
    Err(_) => println!("超时了"),          // 注意是三层结构
}
```

注意返回值的层次：`timeout` 外面套了一层 `Result`，里面才是原 future 的结果——**「超时」和「请求失败」是两种不同的错误**，需要分别处理（第 8 章的错误处理在这里又用上了）。

## 16.10 真实世界的 tokio

标准库只提供了 `Future` 这套接口，**没有提供运行时**。真实项目要加一个依赖：

```toml
[dependencies]
tokio = { version = "1", features = ["full"] }
```

然后就能写：

```rust
use std::time::Duration;

#[tokio::main]
async fn main() {
    // 派生一个独立任务
    let handle = tokio::spawn(async {
        tokio::time::sleep(Duration::from_millis(10)).await;
        "子任务完成"
    });

    // 非阻塞的等待
    tokio::time::sleep(Duration::from_millis(5)).await;
    println!("主任务等待中...");

    // 等子任务结束
    println!("{}", handle.await.unwrap());
}
```

三个「魔法」其实都有对应的机制：

| tokio 的写法 | 底层做了什么 |
| --- | --- |
| `#[tokio::main]` | 属性宏：把 `async fn main` 改写成 `fn main() { runtime.block_on(async { ... }) }`（第 15 章的过程宏） |
| `tokio::spawn` | 把 future 交给调度器，要求 `Send + 'static`，返回 `JoinHandle` |
| `tokio::time::sleep(...).await` | 向运行时注册一个定时器，`Pending` 之后由 Waker 在超时点唤醒 |

**关于验证的说明**：本章配套代码**只用标准库**（`block_on` 是手写的），因为这台机器访问不了 crates.io，没法下载 tokio 依赖。上面这段 tokio 代码是 tokio 1.x 的标准写法，但没有参与本仓库的编译与运行验证——你在本地新建一个项目、按上面的 `Cargo.toml` 加依赖即可运行。其余所有结论（`async fn` 的惰性、`poll` 的 `Pending`/`Ready`、`block_on` 的原理、阻塞的危害）都由配套代码实测过。

## 16.11 三个真实报错与警告怎么读

**案例 1：在非 async 环境里 `.await`（`E0728`）**

```rust
fn main() {
    let value = async { 1 }.await;
}
```

```text
error[E0728]: `await` is only allowed inside `async` functions and blocks
 --> src/main.rs:2:29
  |
1 | fn main() {
  | --------- this is not `async`
2 |     let value = async { 1 }.await;
  |                             ^^^^^ only allowed inside `async` functions and blocks
```

`.await` 需要运行时来驱动，所以它只能在 `async fn` 或 `async` 块里出现。

**案例 2：把 `main` 写成 `async`（`E0752`）**

```text
error[E0752]: `main` function is not allowed to be `async`
 --> src/main.rs:1:1
  |
1 | async fn main() {
  | ^^^^^^^^^^^^^^^ `main` function is not allowed to be `async`
```

原因和上一条一样：`main` 不是 `async` 环境，而且语言层面没有规定用哪个运行时。真实项目用 `#[tokio::main]` 之类的宏来包装；本章配套代码用的是自己写的 `block_on`。

**案例 3：调用 `async fn` 却忘了 `.await`（警告）**

```text
warning: unused implementer of `Future` that must be used
 --> src/main.rs:6:5
  |
6 |     compute();
  |     ^^^^^^^^^
  |
  = note: futures do nothing unless you `.await` or poll them
```

这条警告是「代码没生效」这类 bug 的主要防线——**看到它就意味着某个异步调用根本没执行**。

## 16.12 常见坑速查

| 现象 | 原因 | 解法 |
| --- | --- | --- |
| `async fn` 调用了却没效果 | future 是惰性的，没被 poll | 加上 `.await`，或交给 `spawn` |
| 报 `await is only allowed inside async` | 在非 async 环境里 `.await` | 把代码放进 `async fn` / `async` 块 |
| 报 `main function is not allowed to be async` | 直接写了 `async fn main` | 用运行时的宏（`#[tokio::main]`）或自己 `block_on` |
| 程序「卡住不动」 | 异步里调用了阻塞操作 | 换异步版本 / `spawn_blocking` |
| 并发度上不去，像串行 | 循环里逐个 `.await`，没有并行 | `join!` / `FuturesUnordered` / `spawn` |
| 报 `future is not Send` | 跨 `.await` 持有了非 `Send` 的值 | 缩短作用域，先把数据处理完再 await |
| 报 `future cannot be sent between threads`（`spawn`） | 任务不是 `Send + 'static` | 用 `Arc` 共享、避免借用局部变量，或改单线程运行时 |
| 锁在 `.await` 前后表现异常 | 持有同步锁跨越了 `.await` | 缩小锁的范围，或用异步锁 |
| 手动实现 `Future` 时编译报 Pin 相关错误 | 自引用类型不能随便移动 | 用 `Box::pin` / `pin!` 固定住 |
| 忘了给 future 类型加 `mut` | `poll` 需要 `&mut self` | 绑定写成 `let mut future = ...` 或 `pin!` |

## 16.13 练习

1. 写一个 `Delayed` future：内部用 `polls_left: u32` 计数，每次 `poll` 返回 `Pending` 并减一，归零后返回 `Ready(42)`；用本章的 `block_on` 跑它。
2. 用 `block_on` 运行一个 `async fn factorial(n: u64) -> u64`，算 `factorial(5)`。
3. 用 `Box::pin` 把两个 `Delayed`（分别 2 次和 1 次）交替 poll，统计总 poll 次数，并说明为什么它比串行执行「更并发」。
4. 解释三件事：(a) 为什么 `async fn` 调用后不执行？(b) 为什么 `main` 不能是 `async`？(c) `.await` 遇到 `Pending` 时发生了什么？
5. 解释为什么异步任务里不能用 `std::thread::sleep`，并给出两种正确的替代方案。

（第 4、5 题是 16.1、16.2 和 16.7 讨论过的内容，这里不再单独给答案。）

### 参考答案

::: details 第 1 题

```rust
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};

struct Delayed {
    polls_left: u32,
}

impl Future for Delayed {
    type Output = u32;

    fn poll(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<u32> {
        if self.polls_left == 0 {
            Poll::Ready(42)
        } else {
            self.polls_left -= 1;
            Poll::Pending
        }
    }
}

fn block_on<F: Future>(future: F) -> F::Output {
    let waker = Waker::noop();
    let mut context = Context::from_waker(waker);
    let mut future = std::pin::pin!(future);
    loop {
        match future.as_mut().poll(&mut context) {
            Poll::Ready(value) => return value,
            Poll::Pending => std::thread::yield_now(),
        }
    }
}

fn main() {
    println!("{:?}", block_on(Delayed { polls_left: 3 }));   // 42
}
```

实测输出 `42`。`poll` 会依次看到 `Pending`、`Pending`、`Pending`、`Ready(42)`——**future 每次被 poll 都从上次的状态继续**，这就是它内部必须保存状态的原因。

:::

::: details 第 2 题

```rust
async fn factorial(n: u64) -> u64 {
    let mut result = 1;
    for i in 1..=n {
        result *= i;
    }
    result
}

fn main() {
    println!("{}", block_on(factorial(5)));   // 120
}
```

实测输出 `120`。（沿用第 1 题的 `block_on`。）

注意这里的 `factorial` 其实没有任何「等待」：它一口气跑完就返回 `Ready`。**异步不等于并发**——把纯计算包进 `async fn` 并不会让它变快，只会让代码变复杂；`async` 的价值在于「等待 IO 时不占线程」。

:::

::: details 第 3 题

```rust
fn main() {
    let waker = Waker::noop();
    let mut context = Context::from_waker(waker);
    let mut a = Box::pin(Delayed { polls_left: 2 });
    let mut b = Box::pin(Delayed { polls_left: 1 });
    let mut polls = 0;
    let (mut a_done, mut b_done) = (false, false);

    while !a_done || !b_done {
        if !a_done {
            polls += 1;
            if let Poll::Ready(value) = a.as_mut().poll(&mut context) {
                a_done = true;
                println!("A -> {value}");
            }
        }
        if !b_done {
            polls += 1;
            if let Poll::Ready(value) = b.as_mut().poll(&mut context) {
                b_done = true;
                println!("B -> {value}");
            }
        }
    }
    println!("总 poll 次数 = {polls}");
}
```

实测：

```text
B -> 42
A -> 42
总 poll 次数 = 5
```

总共 5 次 poll（A 用了 3 次、B 用了 2 次），而**串行执行需要 3 + 2 = 5 次「等待」**。差别不在次数，而在**等待的方式**：交替 poll 时，B 的等待时间被用来推进 A，两个任务的等待区间重叠了。

换成真实的 IO 场景会更直观：两个网络请求各要 100ms，串行是 200ms，并发是 100ms——**这正是 `join!` 和 `spawn` 的价值**。

:::

## 16.14 小结

- 异步解决的是 **IO 密集**场景：用少量线程跑大量任务，等待时不占线程；CPU 密集还是用线程 / 并行库。

- `async fn` 调用时**不执行**，只返回一个 `Future`；不 `.await`、不 `poll` 就什么都不发生（编译器会警告）。

- `.await` 是语法糖：`poll` 得到 `Ready` 就取值继续，得到 `Pending` 就把控制权交还运行时去跑别的任务。

- `Future` 只有一个方法 `poll(self: Pin<&mut Self>, cx) -> Poll<Output>`；`Pin` 的意义是保证自引用的状态机不被移动。

- 运行时最小可以只有几十行：**循环 poll + Waker**；tokio 在此基础上加了调度器、IO 事件、定时器和真正的唤醒机制。

- `async` 块和 `async fn` 一样返回 future；`async move` 把变量按值捕获，常用于交给 `spawn`。

- **异步里绝不能阻塞**：`thread::sleep`、同步 IO、长时间计算都会卡住同一线程上的所有任务；要用异步版本或 `spawn_blocking`。

- 多线程运行时要求 `Future: Send + 'static`；跨 `.await` 持有非 `Send` 值是最常见的报错来源，修法是缩短作用域。

- 组合 future 用 `join!`（都完成）、`select!`（任一完成）、`timeout`（超时）；注意 `timeout` 返回的是嵌套的 `Result`。

- 标准库只有接口、没有运行时；真实项目加 tokio / async-std / smol 这样的依赖，本章为保持零依赖只用了手写的 `block_on`。

到这里，第 1–16 章的主线就走完了：从打印输出、类型、函数、所有权，一路到错误处理、泛型、生命周期、并发与异步。接下来第 17 章开始进入标准库与生态篇：日期与时间、文本与正则、文件 IO、序列化、命令行工具、网络，以及工程与发布。
