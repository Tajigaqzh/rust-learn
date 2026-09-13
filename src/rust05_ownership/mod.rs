//! 第 5 章配套代码：所有权与借用。
//!
//! 运行方式：`cargo run`，输出接在第 4 章后面。

/// 故意实现 `Drop`，用来看清值在什么时刻被释放。
struct Noisy(&'static str);

impl Drop for Noisy {
    fn drop(&mut self) {
        println!("    释放 {}", self.0);
    }
}

/// 只要 `&str`，不碰调用方的所有权。
fn count_chars(text: &str) -> usize {
    text.chars().count()
}

/// 接收所有权，函数结束时由它负责释放。
fn take_and_report(text: String) -> usize {
    println!("    take_and_report 拿到了「{text}」，长度 {}", text.len());
    text.len()
}

/// 原样交还所有权，调用方可以接着用返回值。
fn give_back(text: String) -> String {
    text
}

/// 需要修改内容时才要可变借用。
fn shout(text: &mut String) {
    text.push('!');
    text.make_ascii_uppercase();
}

/// 返回的是切片，不是新建的 `String`。
fn first_word(text: &str) -> &str {
    match text.find(' ') {
        Some(index) => &text[..index],
        None => text,
    }
}

/// 借用一组字符串算总长度，调用方的 `Vec` 不受影响。
fn total_len(words: &[String]) -> usize {
    let mut total = 0;
    for word in words {
        total += word.len();
    }
    total
}

/// 演示「先声明的后释放」：`_second` 先走，`_first` 后走。
fn show_drop_order() {
    let _first = Noisy("first");
    let _second = Noisy("second");
    println!("    （函数即将返回：second 先释放，first 后释放）");
}

/// 演示所有权、移动、`Copy`、`clone`、借用、切片与 `&str`。
pub fn ownership_demo() {
    println!("\n========== rust05_ownership: 所有权与借用 ==========");

    // 1. 作用域结束时自动释放
    println!("\n--- 1. 作用域结束就释放 ---");
    show_drop_order();
    {
        let _inner = Noisy("内层变量");
        println!("    内层块结束前");
    }
    println!("    内层块已经结束");
    let owned = Noisy("手动释放");
    drop(owned);
    println!("    drop(owned) 之后立刻释放，不用等作用域结束");

    // 2. 移动：所有权换主人
    println!("\n--- 2. 移动：所有权换主人 ---");
    let original = String::from("rust");
    let moved = original;
    println!("    移动后 moved = {moved}");
    // println!("{original}"); // 取消注释会报 E0382：borrow of moved value
    let returned = give_back(moved);
    println!("    传给函数再拿回来: {returned}");

    // 3. 传参即移动
    println!("\n--- 3. 传参会把所有权交出去 ---");
    let owned = String::from("ownership");
    println!("    调用前 owned 长度 = {}", owned.len());
    let len = take_and_report(owned);
    println!("    函数返回后只剩长度 {len}，owned 这个名字已经不能用了");

    // 4. Copy 类型是复制，不是移动
    println!("\n--- 4. Copy 类型是复制，不是移动 ---");
    let n = 42;
    let m = n;
    println!("    i32：n = {n}, m = {m}，两个都能用");
    let tuple = (1, 2.5, true, 'x');
    let tuple_copy = tuple;
    println!("    元组 {tuple:?} 复制后仍可用: {tuple_copy:?}");

    let list = vec![1, 2, 3];
    let list_moved = list;
    println!("    Vec 没有 Copy，移动之后只能用新名字: {list_moved:?}");

    // 5. clone：要两份就显式复制
    println!("\n--- 5. clone：要两份就显式复制 ---");
    let a = String::from("data");
    let mut b = a.clone();
    b.push_str("-modified");
    println!("    a = {a} 没被动过，b = {b}");
    println!("    clone 会在堆上再分配一份，循环里乱用是要花钱的");

    // 6. 借用：只看不拿
    println!("\n--- 6. 借用：只看不拿 ---");
    let text = String::from("所有权");
    let counted = count_chars(&text);
    println!("    count_chars(&text) = {counted}，借用之后 text 还能用: {text}");

    // 7. 可变借用：想改就要 &mut
    println!("\n--- 7. 可变借用：想改就要 &mut ---");
    let mut doc = String::from("rust");
    shout(&mut doc);
    println!("    shout(&mut doc) 之后: {doc}");

    // 8. 同一时间的借用规则：多个 & 可以共存
    println!("\n--- 8. 同一时间的借用规则 ---");
    let numbers = vec![1, 2, 3];
    let first = &numbers;
    let second = &numbers;
    println!("    多个不可变借用可以共存: {first:?} {second:?}");

    // 9. 借用到「最后一次使用」为止
    println!("\n--- 9. 借用到「最后一次使用」为止 ---");
    let mut data = vec![10, 20, 30];
    let head = &data[0];
    println!("    先看一眼 head = {head}");
    data.push(40);
    println!("    借用已经结束，可以改: {data:?}");

    // 10. 切片：借一部分出来
    println!("\n--- 10. 切片：借一部分出来 ---");
    let sentence = String::from("hello rusty world");
    println!(
        "    字符串切片: [{}] [{}]",
        &sentence[..5],
        &sentence[6..11]
    );
    println!("    first_word 返回切片而不是新 String: {}", first_word(&sentence));

    let scores = [80, 90, 75, 88];
    let middle = &scores[1..3];
    println!("    数组切片: {middle:?}，长度 {}", middle.len());

    let chinese = String::from("中文切片");
    let safe = chinese.chars().take(2).collect::<String>();
    println!("    按字符取前两个: {safe}");
    println!("    （直接写 &chinese[..1] 会 panic：字节下标落在字符中间）");

    // 11. 参数收 &str，String 和字面量都能传
    println!("\n--- 11. 参数收 &str，什么都能传 ---");
    let owned = String::from("owned");
    println!("    String 传 &owned: {}", count_chars(&owned));
    println!("    字面量直接传: {}", count_chars("literal"));

    // 12. 借进来算一算，原数据不动
    println!("\n--- 12. 借进来算一算，原数据不动 ---");
    let words = vec![
        String::from("ownership"),
        String::from("borrow"),
        String::from("slice"),
    ];
    println!("    total_len(&words) = {}", total_len(&words));
    println!("    调用之后 words 仍然完整: {words:?}");

    println!("\n========== 所有权与借用演示结束 ==========");
}
