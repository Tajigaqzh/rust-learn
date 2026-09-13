//! 第 16 章配套代码：async/await。
//!
//! 运行方式：`cargo run`，输出接在第 15 章后面。
//!
//! 本章**只用标准库**：异步的「运行时」由下面这个几十行的 `block_on` 充当。
//! 真实项目里会用 tokio / async-std 这样的运行时，原理和它是一样的。

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};

/// 极简运行时：反复 poll 这个 future，直到它返回 `Ready`。
///
/// 真实运行时（tokio 等）会在这里接上 IO 事件、定时器和任务调度；
/// 这里为了不引入依赖，`Pending` 时只是让出一次 CPU。
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

/// 一个永远立刻就绪的 future。
struct Ready(i32);

impl Future for Ready {
    type Output = i32;

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<i32> {
        Poll::Ready(self.0)
    }
}

/// 一个要 poll 好几次才就绪的 future：每 poll 一次倒计时减一。
struct Countdown {
    remaining: u32,
}

impl Countdown {
    fn new(remaining: u32) -> Self {
        Self { remaining }
    }
}

impl Future for Countdown {
    type Output = u32;

    fn poll(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<u32> {
        if self.remaining == 0 {
            Poll::Ready(0)
        } else {
            self.remaining -= 1;
            println!("        Countdown poll 一次，还剩 {}", self.remaining);
            Poll::Pending
        }
    }
}

/// 普通的 async 函数：返回的是一个实现了 `Future` 的值。
async fn compute() -> i32 {
    println!("        compute 真的开始执行了");
    42
}

async fn add(a: i32, b: i32) -> i32 {
    a + b
}

async fn double(value: i32) -> i32 {
    value * 2
}

/// `.await` 把多个异步步骤串起来。
async fn chain() -> i32 {
    let a = add(1, 2).await;
    let b = double(a).await;
    a + b
}

/// 演示 async/await、手写 Future 与最小运行时。
pub fn async_demo() {
    println!("\n========== rust16_async: async/await ==========");

    // 1. async fn 返回 Future，不 poll 就不执行
    println!("\n--- 1. async fn 返回的是 Future ---");
    println!("    创建 compute() 的 future（此时还没有任何输出）");
    let future = compute();
    println!("    future 已经拿到手，但它还没运行");
    println!("    block_on 之后：{}", block_on(future));

    // 2. 最小运行时
    println!("\n--- 2. 手写一个最小运行时 ---");
    println!("    block_on(add(1, 2)) = {}", block_on(add(1, 2)));
    println!("    block_on(double(21)) = {}", block_on(double(21)));
    println!("    （block_on 就是「反复 poll 直到 Ready」）");

    // 3. 手动实现 Future
    println!("\n--- 3. 手动实现 Future ---");
    println!("    block_on(Ready(7)) = {}", block_on(Ready(7)));
    println!("    block_on(Countdown::new(3)) = {}", block_on(Countdown::new(3)));
    println!("    （Countdown 每次 poll 返回 Pending，直到倒计时归零）");

    // 4. .await 串起来
    println!("\n--- 4. .await 串起多个异步步骤 ---");
    println!("    block_on(chain()) = {}", block_on(chain()));
    println!("    （chain 内部依次 await 了 add 和 double）");

    // 5. 交替 poll 两个 future
    println!("\n--- 5. 交替 poll：并发的雏形 ---");
    let waker = Waker::noop();
    let mut context = Context::from_waker(waker);
    let mut first = Box::pin(Countdown::new(2));
    let mut second = Box::pin(Countdown::new(1));
    let mut first_done = false;
    let mut second_done = false;

    while !first_done || !second_done {
        if !first_done {
            print!("    poll A -> ");
            if let Poll::Ready(value) = first.as_mut().poll(&mut context) {
                first_done = true;
                println!("Ready({value})");
            }
        }
        if !second_done {
            print!("    poll B -> ");
            if let Poll::Ready(value) = second.as_mut().poll(&mut context) {
                second_done = true;
                println!("Ready({value})");
            }
        }
    }
    println!("    两个 future 交替推进，谁都没阻塞谁");

    // 6. 现实世界：tokio
    println!("\n--- 6. 现实世界的异步运行时 ---");
    println!("    tokio / async-std / smol 提供真正的调度器和 IO、定时器");
    println!("    #[tokio::main] 把 async fn main 包装成「建 runtime + block_on」");
    println!("    tokio::spawn 派生任务，tokio::time::sleep 是非阻塞的睡眠");
    println!("    （本章配套代码保持零依赖，tokio 的用法见正文 16.10）");

    // 7. 异步里不要阻塞
    println!("\n--- 7. 异步里不能阻塞 ---");
    println!("    std::thread::sleep 会占住整个线程，让同一线程上所有任务一起卡住");
    println!("    要用运行时的异步睡眠（tokio::time::sleep），或者 spawn_blocking");

    println!("\n========== async/await 演示结束 ==========");
}
