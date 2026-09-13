//! 第 13 章配套代码：并发编程。
//!
//! 运行方式：`cargo run`，输出接在第 12 章后面。

use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::Duration;

/// 演示线程、通道、`Mutex`、`thread::scope` 与 `Send` / `Sync`。
pub fn concurrency_demo() {
    println!("\n========== rust13_concurrency: 并发编程 ==========");

    // 1. 创建线程与 join
    println!("\n--- 1. thread::spawn 与 join ---");
    let handle = thread::spawn(|| {
        println!("    子线程：我在另一个线程里算东西");
        let mut total = 0;
        for i in 1..=100 {
            total += i;
        }
        total
    });
    println!("    主线程：spawn 之后立刻往下走，没有等它");
    let total = handle.join().unwrap();
    println!("    子线程返回了 {total}");

    // 2. move 与所有权
    println!("\n--- 2. move：把数据交给线程 ---");
    let data = vec![1, 2, 3];
    let handle = thread::spawn(move || data.iter().sum::<i32>());
    println!("    子线程求和 = {}", handle.join().unwrap());
    // println!("{data:?}");   // 取消注释会报 E0382：data 已经被移进线程

    // 3. 通道
    println!("\n--- 3. mpsc 通道：线程之间传消息 ---");
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        for i in 1..=3 {
            tx.send(i * 10).unwrap();
        }
    });
    for received in rx {
        println!("    收到 {received}");
    }

    // 4. 多个生产者
    println!("\n--- 4. 多个生产者：clone 发送端 ---");
    let (tx, rx) = mpsc::channel();
    for id in 0..3 {
        let tx = tx.clone();
        thread::spawn(move || {
            tx.send(format!("来自线程 {id}")).unwrap();
        });
    }
    drop(tx); // 丢掉主线程手里的这一份，否则 rx 永远等不到「所有发送端都关闭」
    let mut messages: Vec<String> = rx.iter().collect();
    messages.sort();
    println!("    {messages:?}");

    // 5. try_recv 非阻塞
    println!("\n--- 5. try_recv：不阻塞地看一眼 ---");
    let (tx, rx) = mpsc::channel::<i32>();
    match rx.try_recv() {
        Ok(value) => println!("    收到了 {value}"),
        Err(error) => println!("    还没有消息：{error:?}"),
    }
    let _ = tx.send(7);
    thread::sleep(Duration::from_millis(20));
    println!("    发送 7 之后再试：{:?}", rx.try_recv());

    // 6. Mutex
    println!("\n--- 6. Mutex：同一时刻只有一个线程能改 ---");
    let counter = Arc::new(Mutex::new(0));
    let mut handles = Vec::new();
    for _ in 0..4 {
        let counter = Arc::clone(&counter);
        handles.push(thread::spawn(move || {
            for _ in 0..1000 {
                *counter.lock().unwrap() += 1;
            }
        }));
    }
    for handle in handles {
        handle.join().unwrap();
    }
    println!("    4 个线程各加 1000 次 = {}", *counter.lock().unwrap());

    // 7. try_lock 与死锁
    println!("\n--- 7. try_lock：避免自己把自己锁死 ---");
    let mutex = Mutex::new(0);
    let first = mutex.lock().unwrap();
    match mutex.try_lock() {
        Ok(_) => println!("    又拿到了一次锁"),
        Err(error) => println!("    拿不到锁：{error:?}（同一线程重复加锁会死锁）"),
    }
    println!("    第一次锁里的值 = {first}");

    // 8. thread::scope
    println!("\n--- 8. thread::scope：并行借用局部变量 ---");
    let numbers = vec![1, 2, 3, 4];
    thread::scope(|scope| {
        for chunk in numbers.chunks(2) {
            let handle = scope.spawn(move || chunk.iter().sum::<i32>());
            println!("    分块求和 = {}", handle.join().unwrap());
        }
    });
    println!("    scope 结束之后 numbers 还能用 = {numbers:?}");

    // 9. Send 与 Sync
    println!("\n--- 9. Send 与 Sync ---");
    println!("    Send：值可以「移动」到另一个线程（String、Vec、Arc<Mutex<T>>）");
    println!("    Sync：值的「引用」可以同时被多个线程使用（i32、Mutex<T>）");
    println!("    Rc / RefCell 两者都不满足，跨线程要换成 Arc / Mutex");
    println!("    thread::spawn 要求闭包满足 Send + 'static，编译器会替你检查");

    println!("\n========== 并发编程演示结束 ==========");
}
