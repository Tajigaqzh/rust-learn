//! 第 11 章配套代码：闭包与迭代器。
//!
//! 运行方式：`cargo run`，输出接在第 10 章后面。

use std::collections::HashMap;

/// 接收 `Fn`：可以被调用多次，且不改变捕获的环境。
fn apply_twice<F: Fn(i32) -> i32>(f: F, value: i32) -> i32 {
    f(f(value))
}

/// 接收 `FnMut`：会修改捕获的环境，所以闭包本身也要 `mut`。
fn apply_and_count<F: FnMut()>(mut f: F, times: usize) {
    for _ in 0..times {
        f();
    }
}

/// 接收 `FnOnce`：只能调用一次，通常因为闭包把捕获的值消费掉了。
fn consume_once<F: FnOnce() -> String>(f: F) -> String {
    f()
}

/// 自定义迭代器：斐波那契数列（无限序列）。
struct Fibonacci {
    a: u64,
    b: u64,
}

impl Fibonacci {
    fn new() -> Self {
        Self { a: 0, b: 1 }
    }
}

impl Iterator for Fibonacci {
    type Item = u64;

    fn next(&mut self) -> Option<u64> {
        let next = self.a + self.b;
        self.a = self.b;
        self.b = next;
        Some(self.a)
    }
}

/// 按单词长度分组，演示 `HashMap` 的 `entry` 与迭代器配合。
fn group_by_len(words: &[&str]) -> HashMap<usize, Vec<String>> {
    let mut groups: HashMap<usize, Vec<String>> = HashMap::new();
    for word in words {
        groups.entry(word.len()).or_default().push(word.to_string());
    }
    groups
}

/// 演示闭包捕获、`Fn` 家族、迭代器适配器、消费器与自定义迭代器。
pub fn closures_iterators_demo() {
    println!("\n========== rust11_closures_iterators: 闭包与迭代器 ==========");

    // 1. 闭包基础
    println!("\n--- 1. 闭包：能捕获环境的匿名函数 ---");
    let double = |n: i32| n * 2;
    println!("    double(21) = {}", double(21));

    let threshold = 10;
    let above = |n: i32| n > threshold; // 捕获 threshold（不可变借用）
    println!("    above(20)（阈值 {threshold}）= {}", above(20));

    // 2. 三种捕获方式
    println!("\n--- 2. 三种捕获方式 ---");
    let data = vec![1, 2, 3];
    let sum_ref = || data.iter().sum::<i32>(); // 不可变借用
    println!("    不可变借用求和 = {}", sum_ref());
    println!("    借用之后 data 还能用 = {data:?}");

    let mut counter = 0;
    let mut bump = || counter += 1; // 可变借用
    bump();
    bump();
    println!("    可变借用：调用两次之后 counter = {counter}");

    let owned = String::from("move 进去");
    let take = move || owned.len(); // 移动进闭包
    println!(
        "    move 闭包：长度 = {}（owned 已经不能在外面用了）",
        take()
    );

    // 3. 闭包作为参数
    println!("\n--- 3. 闭包作为参数 ---");
    println!(
        "    apply_twice(|n| n + 3, 1) = {}",
        apply_twice(|n| n + 3, 1)
    );
    let mut ticks = 0;
    apply_and_count(|| ticks += 1, 3);
    println!("    apply_and_count 三次之后 ticks = {ticks}");
    let text = String::from("只调用一次");
    let taken = consume_once(move || text);
    println!("    FnOnce 拿走的值 = {taken}");

    // 4. 迭代器的三种起点
    println!("\n--- 4. iter / iter_mut / into_iter ---");
    let mut numbers = vec![1, 2, 3];
    print!("    iter() 只读: ");
    for n in numbers.iter() {
        print!("{n} ");
    }
    println!();
    for n in numbers.iter_mut() {
        *n *= 10;
    }
    println!("    iter_mut() 改过之后 = {numbers:?}");
    let moved: Vec<i32> = numbers.into_iter().collect();
    println!("    into_iter() 拿走所有权 = {moved:?}");

    // 5. 惰性求值
    println!("\n--- 5. 惰性求值：不消费就不干活 ---");
    let values = vec![1, 2, 3];
    let iter = values.iter().map(|n| {
        println!("        正在计算 {n}");
        n * 2
    });
    println!("    迭代器已经建好，但还没有任何计算发生");
    let doubled: Vec<i32> = iter.collect();
    println!("    collect 之后 = {doubled:?}");

    // 6. 常用适配器
    println!("\n--- 6. 常用适配器 ---");
    let scores = vec![55, 92, 68, 77, 45, 88];
    let passed: Vec<i32> = scores.iter().copied().filter(|s| *s >= 60).collect();
    println!("    及格分数 = {passed:?}");
    let bumped: Vec<i32> = passed.iter().map(|s| s + 5).collect();
    println!("    各加 5 分 = {bumped:?}");
    let top_two: Vec<i32> = bumped.iter().take(2).copied().collect();
    println!("    前两名 = {top_two:?}");
    let after_first: Vec<i32> = bumped.iter().skip(1).copied().collect();
    println!("    跳过第一名 = {after_first:?}");

    let left = [1, 2];
    let right = [3, 4];
    let chained: Vec<i32> = left.iter().chain(right.iter()).copied().collect();
    println!("    chain = {chained:?}");

    let names = ["ada", "linus"];
    let ages = [36, 54];
    let zipped: Vec<String> = names
        .iter()
        .zip(ages.iter())
        .map(|(name, age)| format!("{name}:{age}"))
        .collect();
    println!("    zip = {zipped:?}");

    print!("    enumerate: ");
    for (index, score) in scores.iter().enumerate() {
        print!("{index}={score} ");
    }
    println!();

    let nested = vec![vec![1, 2], vec![3]];
    let flat: Vec<i32> = nested.iter().flatten().copied().collect();
    println!("    flatten = {flat:?}");

    // 7. 常用消费器
    println!("\n--- 7. 常用消费器 ---");
    let nums = vec![3, 1, 4, 1, 5, 9, 2, 6];
    println!("    sum = {}", nums.iter().sum::<i32>());
    println!("    count = {}", nums.iter().count());
    println!(
        "    max / min = {:?} / {:?}",
        nums.iter().max(),
        nums.iter().min()
    );
    println!(
        "    fold 求积（前四个）= {}",
        nums.iter().take(4).fold(1, |acc, n| acc * n)
    );
    println!("    有大于 8 的数吗 = {}", nums.iter().any(|n| *n > 8));
    println!("    全都大于 0 吗 = {}", nums.iter().all(|n| *n > 0));
    println!("    第一个偶数 = {:?}", nums.iter().find(|n| **n % 2 == 0));
    println!("    5 的下标 = {:?}", nums.iter().position(|n| *n == 5));

    // 8. collect 到不同容器
    println!("\n--- 8. collect 的目标可以有很多种 ---");
    let chars: Vec<char> = "hello".chars().collect();
    println!("    Vec<char> = {chars:?}");
    let reversed: String = chars.iter().rev().collect();
    println!("    倒序拼成 String = {reversed}");

    let sentence = "the quick the lazy the";
    let mut counts: HashMap<&str, usize> = HashMap::new();
    for word in sentence.split_whitespace() {
        *counts.entry(word).or_insert(0) += 1;
    }
    let mut sorted: Vec<(&str, usize)> = counts.into_iter().collect();
    sorted.sort();
    println!("    词频（排序后）= {sorted:?}");

    let groups = group_by_len(&["rust", "is", "fun", "safe", "fast"]);
    let mut lengths: Vec<usize> = groups.keys().copied().collect();
    lengths.sort();
    for length in lengths {
        println!("    {length} 个字母: {:?}", groups[&length]);
    }

    // 9. 自定义迭代器
    println!("\n--- 9. 自定义迭代器 ---");
    let fibs: Vec<u64> = Fibonacci::new().take(8).collect();
    println!("    斐波那契前 8 项 = {fibs:?}");
    println!("    第 10 项 = {:?}", Fibonacci::new().nth(9));

    // 10. 与手写循环对比
    println!("\n--- 10. 迭代器 vs 手写循环 ---");
    let data = vec![1, 2, 3, 4, 5, 6];
    let mut manual = Vec::new();
    for n in &data {
        if n % 2 == 0 {
            manual.push(n * n);
        }
    }
    let chained: Vec<i32> = data.iter().filter(|n| *n % 2 == 0).map(|n| n * n).collect();
    println!("    手写循环 = {manual:?}");
    println!("    迭代器链 = {chained:?}");
    println!("    结果相同，迭代器版本没有中间变量，性能也几乎一样");

    println!("\n========== 闭包与迭代器演示结束 ==========");
}
