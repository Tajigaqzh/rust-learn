//! 第 7 章配套代码：常用集合。
//!
//! 运行方式：`cargo run`，输出接在第 6 章后面。

use std::collections::HashMap;

/// 统计每个单词出现的次数：`HashMap` + `entry` 的经典用法。
fn word_count(text: &str) -> HashMap<&str, usize> {
    let mut counts = HashMap::new();
    for word in text.split_whitespace() {
        *counts.entry(word).or_insert(0) += 1;
    }
    counts
}

/// 把统计结果按键排序后返回：`HashMap` 本身不保证顺序，想稳定输出就要自己排。
///
/// 这里必须手写 `<'a>`：返回值里的 `&str` 借的是 `counts` 里那些键，
/// 编译器无法从「一个是 `&HashMap`、一个是键里的 `&str`」这两条生命里自己挑（第 5 章 5.11 案例 9）。
fn sorted_counts<'a>(counts: &HashMap<&'a str, usize>) -> Vec<(&'a str, usize)> {
    let mut items: Vec<(&str, usize)> = Vec::new();
    for (word, count) in counts {
        items.push((*word, *count));
    }
    items.sort();
    items
}

/// 参数收 `&str`，`String` 和字面量都能传进来（第 5 章 5.10）。
fn shout(text: &str) -> String {
    format!("{}!", text.to_uppercase())
}

/// 自定义类型要当 `HashMap` 的键，必须实现 `Hash` + `Eq`。
#[derive(Debug, Hash, PartialEq, Eq)]
struct Coord {
    x: i32,
    y: i32,
}

/// 演示 `Vec`、`String`、`HashMap` 的常用操作与坑。
pub fn collections_demo() {
    println!("\n========== rust07_collections: 常用集合 ==========");

    // 1. Vec 的增删改查
    println!("\n--- 1. Vec：创建与增删改查 ---");
    let mut scores = vec![88, 92, 75];
    scores.push(100); // 末尾追加，均摊 O(1)
    println!("    push(100) 之后 = {scores:?}");
    scores.insert(0, 60); // 指定位置插入，后面的元素都要挪，O(n)
    println!("    insert(0, 60) 之后 = {scores:?}");
    let removed = scores.remove(0); // 同样要挪元素
    println!("    remove(0) 移除了 {removed}，剩下 = {scores:?}");
    let popped = scores.pop();
    println!("    pop() 拿走 {popped:?}，剩下 = {scores:?}");
    println!(
        "    len = {}，第一个 = {}，最后一个 = {}",
        scores.len(),
        scores[0],
        scores[scores.len() - 1]
    );

    // 2. 下标 vs get
    println!("\n--- 2. 下标 vs get ---");
    println!("    scores.get(1) = {:?}", scores.get(1));
    println!("    scores.get(99) = {:?}", scores.get(99));
    println!("    （写 scores[99] 会 panic：index out of bounds）");
    let first = scores.first().copied().unwrap_or_default();
    let last = scores.last().copied().unwrap_or_default();
    println!("    first / last 安全取值 = {first} / {last}");

    // 3. 遍历与借用规则
    println!("\n--- 3. 遍历与借用规则 ---");
    print!("    只读遍历: ");
    for score in &scores {
        print!("{score} ");
    }
    println!();

    for score in &mut scores {
        if *score < 90 {
            *score += 5; // 改自己这一项是允许的
        }
    }
    println!("    小于 90 的加 5 分 = {scores:?}");
    println!("    （遍历时 v.push(...) 会报 E0502：遍历是借用，扩容要可变借用）");

    // 4. len 与 capacity
    println!("\n--- 4. len 与 capacity ---");
    let mut numbers = Vec::with_capacity(10);
    println!(
        "    with_capacity(10) 之后: len = {}, capacity = {}",
        numbers.len(),
        numbers.capacity()
    );
    numbers.push(1);
    numbers.push(2);
    numbers.push(3);
    println!(
        "    推入 3 个之后: len = {}, capacity = {}",
        numbers.len(),
        numbers.capacity()
    );
    numbers.shrink_to_fit();
    println!(
        "    shrink_to_fit 之后: len = {}, capacity = {}",
        numbers.len(),
        numbers.capacity()
    );

    // 5. Vec 常用方法
    println!("\n--- 5. Vec 常用方法 ---");
    let mut data = vec![3, 1, 4, 1, 5, 9, 2, 6];
    data.sort();
    println!("    sort = {data:?}");
    data.dedup();
    println!("    dedup = {data:?}");
    data.retain(|n| n % 2 == 0);
    println!("    retain(只留偶数) = {data:?}");
    println!("    contains(&4) = {}", data.contains(&4));

    print!("    windows(2): ");
    for window in data.windows(2) {
        print!("{window:?} ");
    }
    println!();
    print!("    chunks(2): ");
    for chunk in data.chunks(2) {
        print!("{chunk:?} ");
    }
    println!();

    let mut left = vec![1, 2, 3, 4, 5];
    let right = left.split_off(3);
    println!("    split_off(3): 左 = {left:?}，右 = {right:?}");

    // 6. String 的本质
    println!("\n--- 6. String 的本质：UTF-8 字节 ---");
    let zh = String::from("中文abc");
    println!(
        "    「{zh}」len()（字节）= {}，chars().count()（字符）= {}",
        zh.len(),
        zh.chars().count()
    );
    print!("    bytes(): ");
    for byte in zh.bytes() {
        print!("{byte} ");
    }
    println!();
    print!("    chars(): ");
    for c in zh.chars() {
        print!("{c} ");
    }
    println!();
    println!("    （&zh[..1] 会 panic：字节下标落在字符中间，见第 5 章 5.9）");

    // 7. String 常用操作
    println!("\n--- 7. String 常用操作 ---");
    let mut text = String::from("  rust  ");
    println!("    trim = [{}]", text.trim());
    text.push_str("learn");
    println!("    push_str(\"learn\") = [{text}]");
    text.push('!');
    println!("    push('!') = [{text}]");

    let hello = String::from("hello ");
    let world = String::from("world");
    let joined = hello + &world; // hello 被移动，world 只是借用
    println!("    + 拼接 = [{joined}]，world 还能用 = [{world}]");

    let parts = ["rust", "learn", "note"];
    println!("    join(\"-\") = {}", parts.join("-"));
    println!("    replace = {}", joined.replace("world", "rust"));
    println!(
        "    starts_with(\"hello\") = {}，contains(\"world\") = {}",
        joined.starts_with("hello"),
        joined.contains("world")
    );
    println!("    \"42\".trim().parse::<i32>() = {:?}", "42".trim().parse::<i32>());
    println!(
        "    解析失败用 unwrap_or 兜底 = {}",
        "abc".parse::<i32>().unwrap_or(-1)
    );

    print!("    split(','): ");
    for part in "alpha,beta,gamma".split(',') {
        print!("{part} ");
    }
    println!();

    // 8. String 与 &str 的转换
    println!("\n--- 8. String 与 &str 之间的转换 ---");
    let owned = String::from("owned");
    let borrowed: &str = &owned; // &String 自动转 &str
    let same: &str = owned.as_str();
    let from_literal = "literal".to_string();
    println!("    借用 = [{borrowed}] / [{same}]");
    println!("    字面量转 String = [{from_literal}]");
    println!("    shout(&owned) = {}", shout(&owned));
    println!("    shout(\"literal\") = {}", shout("literal"));

    // 9. HashMap 插入与查询
    println!("\n--- 9. HashMap：插入与查询 ---");
    let mut ages: HashMap<String, u32> = HashMap::new();
    ages.insert(String::from("ada"), 36);
    ages.insert(String::from("linus"), 54);
    ages.insert(String::from("ada"), 37); // 同一个键：覆盖旧值
    println!("    len = {}", ages.len());
    println!("    get(\"ada\") = {:?}（按 &str 查 String 键也可以）", ages.get("ada"));
    println!("    get(\"grace\") = {:?}", ages.get("grace"));
    println!("    contains_key(\"linus\") = {}", ages.contains_key("linus"));
    println!(
        "    查不到的兜底 = {}",
        ages.get("nobody").copied().unwrap_or(0)
    );

    // 10. entry API
    println!("\n--- 10. entry API：没有就插入，有就更新 ---");
    let mut counters: HashMap<&str, usize> = HashMap::new();
    for word in ["rust", "book", "rust", "rust"] {
        *counters.entry(word).or_insert(0) += 1;
    }
    for (word, count) in sorted_counts(&counters) {
        println!("    {word} -> {count}");
    }

    let sentence = "the quick brown the lazy the";
    let counts = word_count(sentence);
    println!("    「{sentence}」里有 {} 个不同的词", counts.len());
    for (word, count) in sorted_counts(&counts) {
        println!("    {word} -> {count}");
    }

    // 11. 遍历 HashMap
    println!("\n--- 11. 遍历 HashMap ---");
    let mut langs: HashMap<&str, i32> = HashMap::new();
    langs.insert("rust", 2015);
    langs.insert("go", 2009);
    langs.insert("python", 1991);
    let mut entries: Vec<(&str, i32)> = Vec::new();
    for (name, year) in &langs {
        entries.push((*name, *year));
    }
    entries.sort_by_key(|(_, year)| *year);
    for (name, year) in entries {
        println!("    {name} 发布于 {year}");
    }
    println!("    （HashMap 不保证遍历顺序，要固定顺序就先排序）");

    // 12. 自定义类型当键
    println!("\n--- 12. 自定义类型当键 ---");
    let mut grid: HashMap<Coord, &str> = HashMap::new();
    grid.insert(Coord { x: 0, y: 0 }, "起点");
    grid.insert(Coord { x: 1, y: 2 }, "目标");
    println!("    表里有 {} 项", grid.len());
    println!(
        "    get(1, 2) = {:?}",
        grid.get(&Coord { x: 1, y: 2 })
    );
    println!("    （键必须实现 Hash + Eq，derive 一行就够）");

    println!("\n========== 常用集合演示结束 ==========");
}
