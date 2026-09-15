//! 第 30 章配套代码：异步进阶（Pin / Unpin、tokio 同步原语、select! 与取消）。
//!
//! 第 16 章手写了最小运行时来理解 `Future`；本章换上真实的 tokio，
//! 补上当时欠的两块：`Pin` 到底是什么，以及真实异步代码里的
//! 同步原语、并发组合和取消语义。
//!
//! 运行方式：`cargo run`；只跑本章的测试：`cargo test rust30_async_advanced`。

use std::marker::PhantomPinned;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::{Mutex as AsyncMutex, broadcast, mpsc, oneshot, watch};

// ---------------------------------------------------------------------------
// 30.1 Pin 与 Unpin
// ---------------------------------------------------------------------------

/// 一个「自引用雏形」：`self_ptr` 指向自身的另一个字段。
///
/// 真正的 async 状态机就是这种形状——`async fn` 里的局部变量被
/// `.await` 切成状态机字段，其中一个字段是引用，指向同一结构里的
/// 另一个字段。**一旦这个结构被 move，指向旧地址的引用全部悬垂**。
/// Rust 的解法是把「不允许再移动」编码进类型系统，这个类型就是 `Pin`。
struct SelfRef {
    data: String,
    /// 指向 `data` 的指针。构造见 [`SelfRef::new`]。
    self_ptr: *const String,
    /// `PhantomPinned` 让这个类型**不**实现 `Unpin`。
    ///
    /// 绝大多数类型自动实现 `Unpin`（含义是「可以随便移动」）；
    /// 只有自引用类型需要退出这个默认。`PhantomPinned` 零大小，
    /// 唯一作用是关掉 `Unpin` 的自动实现。
    _pin: PhantomPinned,
}

impl SelfRef {
    /// 安全构造：在堆上分配并固定，之后再也不移动，指针因此始终有效。
    fn new(data: String) -> Pin<Box<Self>> {
        let mut boxed = Box::pin(SelfRef {
            self_ptr: std::ptr::null(),
            _pin: PhantomPinned,
            data,
        });
        // SAFETY: 值在 Box::pin 之后地址固定（不会再被 move），
        // 所以先取 data 的地址、再写回 self_ptr 是安全的。
        unsafe {
            let ptr: *const String = &boxed.data;
            boxed.as_mut().get_unchecked_mut().self_ptr = ptr;
        }
        boxed
    }

    /// 通过自引用读 data——指针自构造起从未失效。
    fn data_via_self_ptr(pinned: &Pin<Box<Self>>) -> &str {
        // SAFETY: 构造时指针已指向有效地址，且 Pin 保证之后没被 move。
        unsafe { &*pinned.self_ptr }
    }
}

/// 演示 `Pin` 对 `Unpin` / `!Unpin` 两种类型的差别待遇。
fn pin_basics_demo() {
    // 对 Unpin 类型（绝大多数类型），Pin 只是薄包装：
    // 随时可以拿回 &mut T，移动语义不受影响。
    let mut normal = String::from("normal");
    let pinned = Pin::new(&mut normal);
    let back_to_mut = pinned.get_mut(); // 安全：String: Unpin
    back_to_mut.push_str(" (可以随便改)");
    println!("  Unpin 类型经 Pin::get_mut 还原: {normal}");

    // 对 !Unpin 类型（SelfRef），同一件事只能靠 unsafe：
    // let inner = pinned.get_mut();  // 编译不过：SelfRef: !Unpin
    let boxed = SelfRef::new("self-ref".to_string());
    println!(
        "  !Unpin 类型固定后经自引用读: {}",
        SelfRef::data_via_self_ptr(&boxed)
    );
    // async 状态机正是 !Unpin：poll 的参数 Pin<&mut Self> 在对编译器说
    // 「这个状态机不许整体移动」，async fn 的局部引用因此不会悬垂。
}

// ---------------------------------------------------------------------------
// 30.2 tokio::sync：按通信形状选原语
// ---------------------------------------------------------------------------

/// oneshot：一次性、单值、单接收者。
/// 典型场景：发起请求后留一个「回复将来会到」的句柄。
async fn oneshot_demo() {
    let (tx, rx) = oneshot::channel::<&'static str>();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(10)).await;
        // rx 若已被 drop，send 返回 Err；`.ok()` 表示「对方不关心就算了」。
        tx.send("回复到达").ok();
    });
    match rx.await {
        Ok(msg) => println!("  oneshot 收到: {msg}"),
        Err(_) => println!("  oneshot 发送端被 drop（对方放弃回复）"),
    }
}

/// broadcast：多接收者，各自维护游标，谁订阅谁收到。
/// 典型场景：事件扇出——一条日志同时给控制台和落盘两个消费者。
async fn broadcast_demo() {
    let (tx, _rx_keepalive) = broadcast::channel::<&'static str>(16);
    let mut console = tx.subscribe();
    let mut file = tx.subscribe();
    tx.send("事件 A").unwrap();
    tx.send("事件 B").unwrap();
    // 两个订阅者互不影响，各自按自己的节奏收。
    println!("  broadcast 订阅者1: {:?}", console.recv().await);
    println!("  broadcast 订阅者2: {:?}", file.recv().await);
}

/// watch：只保留「最新值」，晚来的订阅者直接看当前状态，不回放历史。
/// 典型场景：配置热更新、健康状态、优雅关闭信号。
async fn watch_demo() {
    let (tx, mut rx) = watch::channel("running");
    tx.send("draining").ok();
    tx.send("stopped").ok(); // 中间值被覆盖，watch 只留最新
    println!(
        "  watch 最新值: {}（中间值 draining 被覆盖）",
        *rx.borrow_and_update()
    );
}

/// async Mutex vs std::sync::Mutex 的分界线：锁要不要**跨 .await 持有**。
async fn mutex_demo(counter: Arc<AsyncMutex<u64>>) {
    // 跨 await 持有 async Mutex：合法。guard 是 Send 的，
    // 挂起期间锁不物理释放也没关系，唤醒后继续持有。
    let mut guard = counter.lock().await;
    *guard += 1;
    tokio::time::sleep(Duration::from_millis(1)).await;
    let value = *guard;
    drop(guard);
    println!("  async Mutex 跨 await 持锁后读到: {value}");
    // std::sync::Mutex 的 guard 不是 Send：持有它跨 await 的 future
    // 不能被 spawn（正文 30.7 展示对应报错）。短临界区优先 std 锁，
    // 只有必须跨 await 才用 async 锁。
}

// ---------------------------------------------------------------------------
// 30.3 select! 与 join!
// ---------------------------------------------------------------------------

/// select!：多个分支同时等，**谁先就绪走谁**，其余分支被丢弃。
async fn select_demo() {
    let fast = tokio::time::sleep(Duration::from_millis(10));
    let slow = tokio::time::sleep(Duration::from_millis(50));
    tokio::select! {
        _ = fast => println!("  select!: 快分支先完成（10ms）"),
        _ = slow => println!("  select!: 慢分支先完成（50ms）"),
    }
}

/// join!：所有分支全等待，返回值保持书写顺序。
/// 注意它把分支放进**同一个任务**里交替 poll（并发但不并行）：
/// 想要跨工作线程并行，得 spawn 成多个任务再 await JoinHandle。
async fn join_demo() {
    let (a, b) = tokio::join!(async { 1 + 1 }, async { "two" },);
    println!("  join! 等齐所有分支: {a} 和 {b}");
}

// ---------------------------------------------------------------------------
// 30.4 取消语义
// ---------------------------------------------------------------------------

/// Drop 即取消：future 被提前 drop，`.await` 之后的代码永远不执行。
///
/// Rust 异步没有 abort 标志位、没有线程中断——取消就是「不再 poll + 析构」。
async fn drop_cancels_demo() {
    // 创建了 future 但从未 poll、直接 drop：
    // `cancels_me` 里 await 之后的「清理」从未运行。
    let fut = cancels_me();
    drop(fut);
    println!("  future 未 poll 就 drop：await 之后的「清理」从未运行");
}

async fn cancels_me() {
    tokio::time::sleep(Duration::from_secs(60)).await;
    println!("  （永远走不到这里）");
}

/// timeout：select! + 定时器的官方封装。
async fn timeout_demo() {
    let too_slow = tokio::time::sleep(Duration::from_secs(60));
    match tokio::time::timeout(Duration::from_millis(20), too_slow).await {
        Ok(()) => println!("  （不会发生：60s 睡不可能在 20ms 内完成）"),
        Err(_) => println!("  timeout 到点：内部 future 被 drop，即被取消"),
    }
}

/// abort：从外部取消一个已 spawn 的任务（请求取消 + drop 状态机）。
async fn abort_demo() {
    let handle = tokio::spawn(async {
        tokio::time::sleep(Duration::from_secs(60)).await;
        println!("  （永远走不到这里）");
    });
    handle.abort();
    match handle.await {
        Ok(()) => println!("  （不会发生：任务已被 abort）"),
        Err(e) if e.is_cancelled() => println!("  JoinHandle 返回 JoinError::is_cancelled()"),
        Err(e) => panic!("意外错误: {e}"),
    }
}

/// 取消安全性：select! 未命中的分支会被整体 drop，「干一半」的进度丢失。
///
/// 本例用 mpsc 演示正确写法：先入队 4 条消息再进 select!，
/// `recv()` 是取消安全的方法——未命中时消息仍留在队列里。
async fn cancellation_safety_demo() {
    let (tx, mut rx) = mpsc::channel::<u8>(8);
    for i in 0..4u8 {
        tx.send(i).await.unwrap();
    }
    drop(tx);

    // recv 已就绪（队列有 4 条），必胜过 1s 的 sleep，输出确定。
    let first = tokio::select! {
        v = rx.recv() => v,
        _ = tokio::time::sleep(Duration::from_secs(1)) => None,
    };
    println!("  取消安全的 recv: 第一条消息 = {first:?}（没被 select 丢掉）");
    let mut rest = Vec::new();
    while let Some(v) = rx.recv().await {
        rest.push(v);
    }
    println!("    队列里剩下的: {rest:?}");
}

// ---------------------------------------------------------------------------
// 演示入口
// ---------------------------------------------------------------------------

/// 第 30 章演示。`main` 保持同步调用约定，内部构建 tokio 运行时驱动。
pub fn async_advanced_demo() {
    println!("\n========== rust30_async_advanced: 异步进阶 ==========");

    println!("\n--- 1. Pin 与 Unpin ---");
    pin_basics_demo();

    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("构建 tokio 运行时");

    rt.block_on(async {
        println!("\n--- 2. tokio::sync：按通信形状选原语 ---");
        oneshot_demo().await;
        broadcast_demo().await;
        watch_demo().await;
        let counter = Arc::new(AsyncMutex::new(0u64));
        mutex_demo(counter).await;

        println!("\n--- 3. select! 与 join! ---");
        select_demo().await;
        join_demo().await;

        println!("\n--- 4. 取消：Drop 即取消 ---");
        drop_cancels_demo().await;
        timeout_demo().await;
        abort_demo().await;
        cancellation_safety_demo().await;
    });

    println!("\n========== rust30_async_advanced 演示结束 ==========");
}

// ---------------------------------------------------------------------------
// 测试
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pin_keeps_self_reference_valid() {
        let pinned = SelfRef::new("data".to_string());
        assert_eq!(SelfRef::data_via_self_ptr(&pinned), "data");
    }

    // SelfRef 是 !Unpin 的编译期证明：如果给 SelfRef 实现了 Unpin，
    // 下面这个函数体（Pin::get_mut，要求 T: Unpin）就会通过编译；
    // 正因为它编译不过，我们只能把它放在注释里。
    //
    // fn get_mut_would_compile_if_unpin(pinned: Pin<&mut SelfRef>) {
    //     let _ = pinned.get_mut();
    // }

    #[tokio::test]
    async fn oneshot_delivers_single_value() {
        let (tx, rx) = oneshot::channel();
        tx.send(42).unwrap();
        assert_eq!(rx.await.unwrap(), 42);
    }

    #[tokio::test]
    async fn oneshot_reports_dropped_sender() {
        let (tx, rx) = oneshot::channel::<u8>();
        drop(tx);
        assert!(rx.await.is_err());
    }

    #[tokio::test]
    async fn broadcast_reaches_all_subscribers() {
        let (tx, _keep) = broadcast::channel(8);
        let mut a = tx.subscribe();
        let mut b = tx.subscribe();
        tx.send("msg").unwrap();
        assert_eq!(a.recv().await.unwrap(), "msg");
        assert_eq!(b.recv().await.unwrap(), "msg");
    }

    #[tokio::test]
    async fn watch_keeps_only_latest_value() {
        let (tx, mut rx) = watch::channel(0);
        tx.send(1).ok();
        tx.send(2).ok();
        assert_eq!(*rx.borrow_and_update(), 2);
    }

    #[tokio::test]
    async fn async_mutex_guards_across_await() {
        let counter = Arc::new(AsyncMutex::new(0u64));
        let mut handles = Vec::new();
        for _ in 0..10 {
            let counter = Arc::clone(&counter);
            handles.push(tokio::spawn(async move {
                for _ in 0..100 {
                    let mut g = counter.lock().await;
                    *g += 1;
                }
            }));
        }
        for h in handles {
            h.await.unwrap();
        }
        assert_eq!(*counter.lock().await, 1000);
    }

    #[tokio::test]
    async fn timeout_cancels_slow_future() {
        let slow = tokio::time::sleep(Duration::from_secs(60));
        let r = tokio::time::timeout(Duration::from_millis(5), slow).await;
        assert!(r.is_err());
    }

    #[tokio::test]
    async fn abort_cancels_spawned_task() {
        let h = tokio::spawn(async {
            tokio::time::sleep(Duration::from_secs(60)).await;
        });
        h.abort();
        let err = h.await.unwrap_err();
        assert!(err.is_cancelled());
    }

    #[tokio::test]
    async fn select_prefers_ready_branch() {
        // recv 分支已就绪（队列非空），sleep 分支 60s 内不可能就绪：
        // 结果确定，不依赖调度时序。
        let (tx, mut rx) = mpsc::channel(4);
        tx.send(7u8).await.unwrap();
        drop(tx);
        let r = tokio::select! {
            v = rx.recv() => v,
            _ = tokio::time::sleep(Duration::from_secs(60)) => None,
        };
        assert_eq!(r, Some(7));
    }

    #[tokio::test]
    async fn cancellation_safe_recv_does_not_lose_messages() {
        let (tx, mut rx) = mpsc::channel(8);
        for i in 0..4u8 {
            tx.send(i).await.unwrap();
        }
        drop(tx);
        let mut got = Vec::new();
        while let Some(v) = rx.recv().await {
            got.push(v);
        }
        assert_eq!(got, vec![0, 1, 2, 3]);
    }
}
