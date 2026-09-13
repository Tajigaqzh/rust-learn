//! 第 10 章配套代码：生命周期。
//!
//! 运行方式：`cargo run`，输出接在第 9 章后面。

use std::fmt;

/// 返回两个字符串里较长的那个。
///
/// 两个输入引用用同一个 `'a`，返回值也只能活到 `'a`——也就是「跟着活得短的那个」。
fn longest<'a>(a: &'a str, b: &'a str) -> &'a str {
    if a.len() >= b.len() { a } else { b }
}

/// 只有一个输入引用时可以省略标注：编译器按省略规则自动补上。
fn first_word(text: &str) -> &str {
    match text.find(' ') {
        Some(index) => &text[..index],
        None => text,
    }
}

/// 结构体里存引用：必须写生命周期参数，表示「借的东西必须比我活得久」。
#[derive(Debug)]
struct Excerpt<'a> {
    part: &'a str,
}

impl<'a> Excerpt<'a> {
    /// 省略规则：返回值的生命周期跟着 `&self`。
    fn part(&self) -> &str {
        self.part
    }

    /// 参数里也有引用，但它不参与返回值：返回值仍然跟着 `self`。
    fn announce(&self, announcement: &str) -> &str {
        println!("        注意：{announcement}");
        self.part
    }
}

/// 借用了输入的极简分词器。
struct Tokenizer<'a> {
    input: &'a str,
    pos: usize,
}

impl<'a> Tokenizer<'a> {
    fn new(input: &'a str) -> Self {
        Self { input, pos: 0 }
    }

    /// 返回的切片借自 `input`（`'a`），和 `&mut self` 的借用无关。
    fn next_token(&mut self) -> Option<&'a str> {
        let bytes = self.input.as_bytes();
        while self.pos < bytes.len() && bytes[self.pos].is_ascii_whitespace() {
            self.pos += 1;
        }
        if self.pos >= bytes.len() {
            return None;
        }
        let start = self.pos;
        while self.pos < bytes.len() && !bytes[self.pos].is_ascii_whitespace() {
            self.pos += 1;
        }
        Some(&self.input[start..self.pos])
    }
}

/// 字符串字面量本来就是 `'static`：编译进二进制，程序全程有效。
fn greeting() -> &'static str {
    "程序全程有效"
}

/// 把堆上的 `String` 变成 `'static` 引用：代价是这块内存永远不会被回收。
fn leak_string(text: String) -> &'static str {
    Box::leak(text.into_boxed_str())
}

/// 泛型参数上也能加生命周期约束：`T` 必须活得不比 `'a` 短。
fn print_ref<'a, T>(value: &'a T) -> &'a T
where
    T: fmt::Display + 'a,
{
    println!("        打印：{value}");
    value
}

/// 演示生命周期标注、省略规则、结构体里的引用与 `'static`。
pub fn lifetimes_demo() {
    println!("\n========== rust10_lifetimes: 生命周期 ==========");

    // 1. 引用的有效范围
    println!("\n--- 1. 生命周期就是引用的有效范围 ---");
    let outer = String::from("外层变量");
    {
        let inner = String::from("内层变量");
        let borrowed = &inner;
        println!("    内层块里可以借用：{borrowed}");
    }
    println!("    出了块 inner 就被 drop 了，指向它的引用也不可能再存在");
    let outer_ref = &outer;
    println!("    外层引用可以一直用到作用域结束：{outer_ref}");

    // 2. 函数签名里的生命周期
    println!("\n--- 2. 函数签名里的生命周期 ---");
    let a = String::from("short");
    let b = "much longer string";
    println!("    longest(&a, b) = {}", longest(&a, b));
    {
        let c = String::from("tiny");
        println!("    longest(b, &c) = {}", longest(b, &c));
    }
    println!("    （结果只在 a、b 都有效的范围内可用）");

    // 3. 省略规则
    println!("\n--- 3. 什么时候不用写 ---");
    println!("    first_word(\"hello world\") = {}", first_word("hello world"));
    println!("    first_word(\"oneword\") = {}", first_word("oneword"));
    println!("    （只有一个输入引用时，编译器知道返回值借的就是它）");

    // 4. 结构体里的引用
    println!("\n--- 4. 结构体里的引用 ---");
    let text = String::from("hello rusty world");
    let excerpt = Excerpt { part: &text[..5] };
    println!("    excerpt = {excerpt:?}");
    println!("    excerpt.part() = {}", excerpt.part());
    println!("    （text 必须比 excerpt 活得久，编译器会盯着）");

    // 5. 方法返回值的生命周期
    println!("\n--- 5. 方法返回值的生命周期 ---");
    let picked = excerpt.announce("这一行是提示");
    println!("    announce 返回 = {picked}");
    println!("    （返回值借的是 self.part，不是 announcement）");

    // 6. 实战：分词器
    println!("\n--- 6. 实战：借用输入的极简分词器 ---");
    let input = String::from("  rust lifetimes are  fun ");
    let mut tokenizer = Tokenizer::new(&input);
    let mut tokens: Vec<&str> = Vec::new();
    while let Some(token) = tokenizer.next_token() {
        tokens.push(token);
    }
    println!("    tokens = {tokens:?}");
    println!("    共 {} 个词，输入本身没有被复制", tokens.len());

    // 7. 'static
    println!("\n--- 7. 'static 的真实含义 ---");
    println!("    字面量：{}", greeting());
    let leaked = leak_string(String::from("泄漏出来的字符串"));
    println!("    泄漏出来：{leaked}");
    println!("    （'static 不要求值真的活到程序结束，只要求它「可以」活那么久；");
    println!("      Box::leak 是漏掉内存换 'static，别在循环里用）");

    // 8. 泛型 + 生命周期
    println!("\n--- 8. 泛型与生命周期可以同时出现 ---");
    let number = 42;
    let same = print_ref(&number);
    println!("    返回的还是同一个引用：{same}");
    let owned = String::from("trait 与生命周期");
    let _ = print_ref(&owned);

    // 9. 本质
    println!("\n--- 9. 生命周期的本质 ---");
    println!("    生命周期参数描述的是「几个引用之间的约束关系」，不是「某个值活多久」；");
    println!("    签名里写 'a 的意思是：返回的引用不会比传进来的引用活得更久。");

    println!("\n========== 生命周期演示结束 ==========");
}
