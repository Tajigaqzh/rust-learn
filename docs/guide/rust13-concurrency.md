# 第 13 章 · 并发编程

第 12 章末尾埋了个伏笔：`Rc` 不能跨线程，`Arc` 可以。这一章把并发讲完整。

Rust 的并发口号是「**无畏并发**」（fearless concurrency）：数据竞争这类在多线程里最难查的 bug，在 Rust 里大多是**编译错误**而不是运行时崩溃。靠的还是前面几章的东西——所有权（第 5 章）、trait bound（第 9 章）、`Arc` / `Mutex`（第 12 章），再加上这一章的两个标记 trait：`Send` 和 `Sync`。

本章配套代码在 `src/rust13_concurrency/mod.rs`，执行 `cargo run` 可以看到全部输出。

## 13.1 线程：`spawn` 与 `join`

`thread::spawn` 接收一个闭包，在新线程里运行它，返回一个 `JoinHandle`：

```rust
use std::thread;

let handle = thread::spawn(|| {
    let mut total = 0;
    for i in 1..=100 {
        total += i;
    }
    total
});

println!("主线程继续往下走");
let total = handle.join().unwrap();      // 等它结束并拿到返回值
```

实测输出顺序正好说明了「并发」：

```text
    主线程：spawn 之后立刻往下走，没有等它
    子线程：我在另一个线程里算东西
    子线程返回了 5050
```

`spawn` 立刻返回，主线程不等；`join()` 才是「等它跑完」，返回 `Result`——**如果子线程 panic 了，这里是 `Err`**，不是直接崩溃。多个线程时记得把 handle 收集起来一起 join，否则主线程结束后进程就退出了，子线程可能被直接掐断。

## 13.2 `move` 与 `'static`：线程拿不走别人的变量

`spawn` 的签名要求闭包满足 `F: Send + 'static`（第 9 章读 trait bound 的方法在这里用上了）：**闭包必须能安全送到别的线程，而且不能借用任何局部变量**——因为子线程可能比创建它的函数活得还长。

```rust
let data: Vec<i32> = (1..=3).collect();
let handle = thread::spawn(|| println!("{data:?}"));
```

```text
error[E0373]: closure may outlive the current function, but it borrows `data`, which is owned by the current function
 --> src/main.rs:5:32
  |
5 |     let handle = thread::spawn(|| println!("{data:?}"));
  |                                ^^            ---- `data` is borrowed here
  |                                |
  |                                may outlive borrowed value `data`
  |
note: function requires argument type to outlive `'static`
help: to force the closure to take ownership of `data` (and any other referenced variables), use the `move` keyword
  |
5 |     let handle = thread::spawn(move || println!("{data:?}"));
  |                                ++++
```

按 `help` 加 `move` 就行——把 `data` 的所有权交给线程（第 11 章讲过 `move` 闭包）：

```rust
let data: Vec<i32> = (1..=3).collect();
let handle = thread::spawn(move || data.iter().sum::<i32>());
println!("子线程求和 = {}", handle.join().unwrap());     // 6
```

这里没有写 `vec![1, 2, 3]`，是因为**数组是 `Copy` 类型**：`move` 进去的只是一份复制，
原变量照样能用，演示不出「所有权被移走」。`Vec` 不实现 `Copy`，`move` 之后
`data` 就真的没了——这正是要看的现象。

代价还是那个：`data` 之后不能再用。**如果只是想借一会儿、又不想 `clone`，用 `thread::scope`**（13.8）。

## 13.3 通道：`mpsc`

线程之间传数据的第一种方式：**消息传递**。标准库的 `mpsc` 是「多生产者、单消费者」通道：

```rust
use std::sync::mpsc;
use std::thread;

let (tx, rx) = mpsc::channel();

thread::spawn(move || {
    for i in 1..=3 {
        tx.send(i * 10).unwrap();
    }
});

for received in rx {          // 阻塞等待，直到所有发送端关闭
    println!("收到 {received}");
}
```

实测依次收到 `10`、`20`、`30`。

几个要点：

| 方法 | 行为 |
| --- | --- |
| `send(v)` | 返回 `Result`：接收端已经丢弃时是 `Err` |
| `recv()` | **阻塞**等待一条消息，通道关闭返回 `Err` |
| `try_recv()` | **不阻塞**：没消息立刻返回 `Err(Empty)` |
| `rx.iter()` | 反复 `recv`，所有发送端关闭后迭代结束 |

实测非阻塞的样子：

```text
    还没有消息：Empty
    发送 7 之后再试：Ok(7)
```

`SendError` / `RecvError` 都是第 8 章那种普通错误类型——**通道的另一端没了，是一种可恢复的错误**，不是异常。

## 13.4 多个生产者与发送端关闭

`mpsc` 的「多生产者」靠 clone 发送端实现：

```rust
let (tx, rx) = mpsc::channel();

for id in 0..3 {
    let tx = tx.clone();                 // 每个线程一份
    thread::spawn(move || {
        tx.send(format!("来自线程 {id}")).unwrap();
    });
}

drop(tx);                                // 关键：丢掉主线程手里那一份
let messages: Vec<String> = rx.iter().collect();
```

实测收集到 `["来自线程 0", "来自线程 1", "来自线程 2"]`（配套代码里排过序；**消息到达顺序本身不确定**，这取决于线程调度）。

`drop(tx)` 那行是新手最容易漏的一步：`rx.iter()` 只在**所有**发送端都被丢弃后才结束。主线程手里的 `tx` 一直活着的话，接收端就会永远阻塞——程序看起来「卡住了」。

## 13.5 `Mutex<T>`：同一时刻只有一个线程能访问

第二种共享方式：**共享内存 + 加锁**。

```rust
use std::sync::Mutex;

let mutex = Mutex::new(0);
{
    let mut guard = mutex.lock().unwrap();   // 拿到锁
    *guard += 1;
}                                            // guard 在这里被 drop，自动解锁
```

两个关键点：

- **`lock()` 返回一个守卫**（`MutexGuard`），它实现了 `DerefMut`，所以能像 `&mut T` 一样用；它同时是 RAII 的——**离开作用域自动解锁**，不会忘记。
- **`lock()` 返回 `Result`**，因为锁可能「中毒」（poisoned）：持有锁的线程 panic 了，数据可能处于不一致状态，Rust 就把锁标记为中毒，后续 `lock()` 返回 `Err`。所以到处能见到 `.lock().unwrap()`——它的意思是「中毒了就直接 panic」。

**临界区要尽量小**：锁住的时间越长，线程等待越久，并发的收益就越小。常见做法是把计算结果放到局部变量里，只在真正读写共享数据时持锁。

## 13.6 `Arc<Mutex<T>>`：多线程共享可改

单靠 `Arc` 只能共享**只读**数据，因为它不实现 `DerefMut`：

```rust
let counter = Arc::new(5);
*counter += 1;
```

```text
error[E0594]: cannot assign to data in an `Arc`
 --> src/main.rs:5:5
  |
5 |     *counter += 1;
  |     ^^^^^^^^^^^^^ cannot assign
  |
  = help: trait `DerefMut` is required to modify through a dereference, but it is not implemented for `Arc<i32>`
```

加上 `Mutex` 就有了「多线程共享 + 可改」：

```rust
use std::sync::{Arc, Mutex};
use std::thread;

let counter = Arc::new(Mutex::new(0));
let mut handles = Vec::new();

for _ in 0..4 {
    let counter = Arc::clone(&counter);
    handles.push(thread::spawn(move || {
        for _ in 0..1000 {
            *counter.lock().unwrap() += 1;      // 每次加锁、加一、解锁
        }
    }));
}

for handle in handles {
    handle.join().unwrap();
}
println!("{}", *counter.lock().unwrap());
```

实测 `4 个线程各加 1000 次 = 4000`——**一次都没丢**。如果换成不加锁的共享计数（比如 C 里的 `int`），这种竞态几乎必然出错；而在 Rust 里，不加锁根本编译不过。

记法：**`Arc` 管「谁能拿到」，`Mutex` 管「谁能改」**。这两个职责分开，组合起来才完整。

## 13.7 `try_lock` 与死锁

标准库的 `Mutex` **不可重入**：同一个线程对同一把锁 `lock()` 两次会直接死锁——第二次调用会永远等自己释放。

```rust
let mutex = Mutex::new(0);
let first = mutex.lock().unwrap();
let second = mutex.lock().unwrap();     // 永远卡在这里
```

调试这种问题时，`try_lock()` 更好用：拿不到锁时立刻返回，不阻塞。

```rust
match mutex.try_lock() {
    Ok(guard) => println!("拿到了"),
    Err(error) => println!("拿不到：{error:?}"),
}
```

实测输出 `拿不到锁："WouldBlock"`。`WouldBlock` 就是「锁现在被占着」的意思。

死锁的几个常见来源：

| 来源 | 例子 | 预防 |
| --- | --- | --- |
| 同一线程重复加锁 | 上面的 `first` / `second` | 别在持锁时再锁同一把 |
| 两把锁顺序相反 | 线程 A 先锁 X 再锁 Y，线程 B 先锁 Y 再锁 X | 全局统一加锁顺序 |
| 守卫忘记释放 | 在 `if` 里拿了锁，代码路径长 | 缩小作用域，尽早 `drop(guard)` |
| 跨 `await` 持锁 | 异步任务里锁住再 `.await` | 用异步锁，或先算完再锁（第 16 章） |

死锁在 Rust 里**不是编译错误**——类型系统挡的是数据竞争，不是逻辑上的互相等待。所以这部分要靠设计习惯。

## 13.8 `thread::scope`：借用局部变量

`spawn` 要求 `'static`，所以每次都得 `move` 或者 `clone`。Rust 1.63 起有了 `thread::scope`，它保证**所有派生线程在 scope 结束时都被 join**，因此允许借用外部数据：

```rust
let numbers = vec![1, 2, 3, 4];

thread::scope(|scope| {
    for chunk in numbers.chunks(2) {
        let handle = scope.spawn(move || chunk.iter().sum::<i32>());
        println!("分块求和 = {}", handle.join().unwrap());
    }
});

println!("scope 结束之后 numbers 还能用 = {numbers:?}");
```

实测分块求和 `3`、`7`，`scope` 之后 `numbers` 仍然是 `[1, 2, 3, 4]`——**没有任何 `clone`，也没有 `Arc`**。

`scope` 里派生线程的 handle 类型是 `ScopedJoinHandle`，它的生命周期绑在 `scope` 上。把这种 handle 存到 scope 外面的容器里会报 `E0521`（这是我写这个示例时踩到的真实错误）：

```text
error[E0521]: borrowed data escapes outside of closure
 --> src/main.rs:8:13
  |
5 |     let mut sums = Vec::new();
  |         -------- `sums` declared here, outside of the closure body
6 |     thread::scope(|scope| {
  |                    ----- `scope` is a reference that is only valid in the closure body
8 |             sums.push(scope.spawn(move || chunk.iter().sum::<i32>()));
  |             ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ `scope` escapes the closure body here
```

修法很简单：**把 handle 收集在 scope 闭包内部**，在闭包里 join 完再往外传结果（第 13.13 练习第 4 题就是这么写的）。

## 13.9 `Send` 与 `Sync`

这两个标记 trait 是 Rust 并发安全的地基：

| trait | 含义 | 直觉 |
| --- | --- | --- |
| `Send` | 值可以**移动**到别的线程 | 「这东西能寄给别的线程」 |
| `Sync` | 值的**引用**可以同时被多个线程用（`&T` 是 `Send`） | 「多个线程同时看它没问题」 |

常见类型的对照：

| 类型 | `Send` | `Sync` | 说明 |
| --- | --- | --- | --- |
| `i32`、`String`、`Vec<T>` | ✅ | ✅ | 普通拥有型数据 |
| `&T`（`T: Sync`） | ✅ | ✅ | 只读引用可以共享 |
| `Rc<T>` / `RefCell<T>` | ❌ | ❌ | 计数/借用不是原子的 |
| `Arc<T>`（`T: Send + Sync`） | ✅ | ✅ | 原子计数 |
| `Mutex<T>`（`T: Send`） | ✅ | ✅ | 锁保证互斥 |

`Send` / `Sync` 是**自动推导的**（auto trait）：一个类型的所有字段都 `Send`，它才 `Send`。所以 `Rc<Cell<i32>>` 这种嵌套结构会被自动判定为不安全，你不需要手动标注——当然也标不了（只有 `unsafe` 才允许手写，那是第 25 章的话题）。

把不满足 `Sync` 的类型塞进线程会直接报错：

```text
error[E0277]: `Cell<i32>` cannot be shared between threads safely
   --> src/main.rs:7:32
    |
7 |     let handle = thread::spawn(move || counter.set(1));
    |                  ------------- ^^^^^^^^^^^^^^^^^^^^^^ `Cell<i32>` cannot be shared between threads safely
    |
    = help: the trait `Sync` is not implemented for `Cell<i32>`
    = note: if you want to do aliasing and mutation between multiple threads, use `std::sync::RwLock` or `std::sync::atomic::AtomicI32` instead
    = note: required for `Arc<Cell<i32>>` to implement `Send`
```

`note` 里甚至给出了替代方案。这类报错的读法是**从下往上**：`spawn` 要求 `F: Send + 'static` → 闭包捕获了 `Arc<Cell<i32>>` → `Arc<T>: Send` 要求 `T: Send + Sync` → `Cell<i32>` 不是 `Sync`。链条很长，但每一环都写清楚了。

## 13.10 通道还是共享状态？

两种并发风格各有适用场景：

| | 通道（消息传递） | 共享状态（`Arc<Mutex<T>>`） |
| --- | --- | --- |
| 数据流向 | 明确的「谁发给谁」 | 多方读写同一份数据 |
| 同步方式 | 队列天然串行化 | 靠锁互斥 |
| 典型场景 | 任务分发、事件流、worker 结果回收 | 计数器、缓存、共享配置 |
| 风险 | 发送端没关导致接收端卡住 | 死锁、锁粒度过大 |
| Rust 的说法 | 「不要通过共享内存来通信」 | 「必要时才共享内存」 |

Go 社区有句名言：**不要通过共享内存来通信，而要通过通信来共享内存**。Rust 两种都支持，选择标准是「数据的流向是否清晰」：有明确的生产者-消费者关系就用通道；确实是多方共享一份状态（而且需要修改）就用 `Arc<Mutex<T>>`。

## 13.11 并发相关的报错速查

这一章的报错都来自同一套规则，汇总成表：

| 错误 | 触发场景 | 修法 |
| --- | --- | --- |
| `E0373: closure may outlive the current function` | `spawn` 的闭包借用了局部变量 | 加 `move`，或改用 `thread::scope` |
| `E0594: cannot assign to data in an Arc` | 直接修改 `Arc` 里的数据 | 用 `Arc<Mutex<T>>` |
| `E0277: Rc<T> cannot be sent between threads safely` | 把 `Rc` 送进线程 | 换 `Arc` |
| `E0277: Cell<i32> cannot be shared between threads safely` | 把非 `Sync` 类型放进 `Arc` 共享 | 换 `Mutex` / `RwLock` / `Atomic*` |
| `E0521: borrowed data escapes outside of closure` | 把 `ScopedJoinHandle` 存到 `scope` 外 | 在 `scope` 闭包内部收集并 join |
| 程序不报错但「卡住」 | 死锁，或接收端等不到发送端关闭 | 检查重复加锁 / 加锁顺序 / 有没有 `drop(tx)` |

最后一行很关键：**死锁和「忘记关闭发送端」都不是编译错误**。类型系统能挡住数据竞争，但挡不住逻辑上的互相等待。写并发代码时，这两点是必须靠自查的部分。

## 13.12 常见坑速查

| 现象 | 原因 | 解法 |
| --- | --- | --- |
| 子线程的打印有时不出现 | 主线程结束、进程退出 | 用 `join()` 等所有线程 |
| `spawn` 报闭包要 `'static` | 借用了局部变量 | `move` + `clone`，或 `thread::scope` |
| 通道接收端一直阻塞 | 还有发送端没被 drop | `drop(tx)`，或让发送端离开作用域 |
| 消息顺序和发送顺序不同 | 线程调度不确定 | 需要顺序就带序号，或汇总后排序 |
| 线程 panic 了却「没事」 | `join()` 返回 `Err`，你没处理 | `handle.join().unwrap()` 或显式处理 |
| 报「锁中毒」 | 持锁线程 panic 了 | 处理 `PoisonError`，或让临界区里的代码不 panic |
| 程序卡死不动 | 死锁（重复加锁 / 加锁顺序相反） | `try_lock` 排查，缩小临界区，统一顺序 |
| 多个线程改计数结果偏小 | 用了不加锁的共享状态 | `Arc<Mutex<T>>` 或原子类型 |
| `Arc<RefCell<T>>` 编译不过 | `RefCell` 不是 `Sync` | 换 `Mutex` / `RwLock` |
| 加了锁反而更慢 | 临界区太大，或者锁竞争激烈 | 缩小临界区，减少共享状态 |
| `scope` 里的 handle 存到外面报 `E0521` | handle 的生命周期绑在 scope 上 | 在 scope 内部 join |

## 13.13 练习

1. 把 `vec![1, 2, 3, 4, 5, 6, 7, 8]` 分成若干块，每块交给一个线程求和，最后汇总出总数。
2. 用 `mpsc` 起 3 个 worker 线程，每个把「自己算出的平方数」发回主线程，主线程收集成 `Vec`。
3. 用 `Arc<Mutex<Vec<i32>>>` 让 3 个线程各自往同一个 `Vec` 里 `push` 自己的编号，最后打印排序后的结果。
4. 用 `thread::scope` 把 `vec![1, 2, 3, 4, 5, 6]` 分块并行翻倍，返回 `[2, 4, 6, 8, 10, 12]`（要求不使用 `Arc` 和 `clone` 整个数组）。
5. 用自己的话回答：(a) `Rc` 为什么不能跨线程而 `Arc` 可以？(b) 什么情况下 `Arc<T>` 就够了，什么时候必须 `Arc<Mutex<T>>`？

（第 2、5 题是 13.3 和 13.9 讨论过的内容，这里不再单独给答案。）

### 参考答案

::: details 第 1 题

```rust
use std::thread;

fn main() {
    let data = vec![1, 2, 3, 4, 5, 6, 7, 8];
    let mut handles = Vec::new();

    for chunk in data.chunks(2) {
        let chunk = chunk.to_vec();       // 每块单独持有，才能 move 进线程
        handles.push(thread::spawn(move || chunk.iter().sum::<i32>()));
    }

    let total: i32 = handles.into_iter().map(|handle| handle.join().unwrap()).sum();
    println!("{total}");                   // 36
}
```

实测 `36`。`chunk.to_vec()` 是为了满足 `'static`：切片的借用活不过 `spawn`，必须让它拥有一份自己的数据。如果不想复制，用第 4 题的 `thread::scope` 就行。

:::

::: details 第 3 题

```rust
use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    let shared = Arc::new(Mutex::new(Vec::new()));
    let mut handles = Vec::new();

    for id in 0..3 {
        let shared = Arc::clone(&shared);
        handles.push(thread::spawn(move || {
            shared.lock().unwrap().push(id);
        }));
    }

    for handle in handles {
        handle.join().unwrap();
    }

    let mut values = shared.lock().unwrap().clone();
    values.sort();
    println!("{values:?}");                // [0, 1, 2]
}
```

实测 `[0, 1, 2]`。注意 `shared.lock().unwrap()` 这个临时 `MutexGuard` 在**语句结束时**就释放了——所以 `for` 循环里连续加锁不会死锁，这也是「临界区尽量小」的自然写法。

最后 `clone()` 出来的是一份快照：如果直接打印 `shared.lock().unwrap()`，守卫会在打印语句期间一直持有，虽然这里没问题，但**尽量让锁的作用域短**是个好习惯。

:::

::: details 第 4 题

```rust
use std::thread;

fn main() {
    let data = vec![1, 2, 3, 4, 5, 6];

    let doubled: Vec<i32> = thread::scope(|scope| {
        let handles: Vec<_> = data
            .chunks(3)
            .map(|chunk| scope.spawn(move || chunk.iter().map(|n| n * 2).collect::<Vec<i32>>()))
            .collect();

        handles
            .into_iter()
            .flat_map(|handle| handle.join().unwrap())
            .collect()
    });

    println!("{doubled:?}");               // [2, 4, 6, 8, 10, 12]
}
```

实测 `[2, 4, 6, 8, 10, 12]`。关键在于 `handles` 是**在 scope 闭包内部**声明的（对比 13.8 那个 `E0521` 的错误版本）；scope 结束前所有线程都 join 完，而 `data` 从头到尾只被借用、没有复制。

:::

## 13.14 小结

- `thread::spawn` 立刻返回，`join()` 才等待；`join()` 返回 `Result`，子线程 panic 会体现在这里。

- `spawn` 要求闭包 `Send + 'static`：要么 `move` 把数据交给线程，要么用 `thread::scope` 借用局部变量。

- `mpsc` 通道适合有明确流向的数据传递：`send` 返回 `Result`，`recv` 阻塞，`try_recv` 不阻塞，`rx.iter()` 在所有发送端关闭后结束。

- 多生产者靠 `clone` 发送端；**别忘了丢掉主线程手里的 `tx`**，否则接收端永远不会结束。

- `Mutex<T>` 保证同一时刻只有一个访问，`lock()` 返回 RAII 守卫（离开作用域自动解锁），锁可能「中毒」所以返回 `Result`。

- `Arc` 管共享所有权、`Mutex` 管可变访问，`Arc<Mutex<T>>` 是多线程共享可改的标准组合；`Arc` 单独只能共享只读数据。

- 标准库 `Mutex` 不可重入，同一线程重复加锁会死锁；`try_lock` 可以非阻塞地检查，加锁顺序要全局统一。

- `thread::scope` 允许线程借用外部数据，结束时自动 join；`ScopedJoinHandle` 不能逃出 scope。

- `Send` = 值能移动到别的线程，`Sync` = 引用能同时被多个线程用；两者都是自动推导的，`Rc` / `RefCell` 都不满足，`Arc` / `Mutex` 满足。

- 类型系统能挡住数据竞争，但**挡不住死锁和逻辑卡死**——那部分要靠设计（缩小临界区、统一加锁顺序）和 `try_lock` 之类的排查手段。

下一章讲**模块、包与测试**：`mod` / `pub` 的作用域规则、crate 与工作空间的结构，以及单元测试、集成测试和文档测试怎么写。
