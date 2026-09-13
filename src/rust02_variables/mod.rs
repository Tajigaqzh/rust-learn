//! 第 2 章配套代码：变量与基本类型。
//!
//! 运行方式：`cargo run`，本模块的输出紧接着第 1 章打印演示。

use std::mem::{size_of, size_of_val};

/// 常量：编译期就确定，习惯用全大写加下划线命名。
const MAX_RETRY: u32 = 3;

/// 静态变量：程序运行期间一直存在，有固定地址。
static APP_NAME: &str = "rust-learn";

/// 演示变量、类型、字面量、复合类型等基础内容。
pub fn variables_demo() {
    println!("\n========== rust02_variables: 变量与基本类型 ==========");

    // 1. let 绑定默认不可变，mut 才是可变的
    println!("\n--- 1. 不可变绑定与 mut ---");
    let answer = 42;
    let mut counter = 0;
    counter += 1;
    println!("不可变的 answer = {answer}");
    println!("可变的 counter 可以自增: {counter}");

    // 2. 遮蔽（shadowing）：重新用 let 绑定同名变量，可以改类型
    println!("\n--- 2. 遮蔽 shadowing ---");
    let x = 5;
    let x = x + 1;
    let spaces = "   ";
    let spaces = spaces.len();
    println!("shadowing 之后 x = {x}");
    println!("spaces 从 &str 变成了 usize: {spaces}");

    // 3. 类型推断与显式标注；没有线索时整数默认 i32，浮点默认 f64
    println!("\n--- 3. 类型推断 ---");
    let inferred = 7; // 默认 i32
    let suffixed = 7u8;
    let annotated: u64 = 7;
    let parsed: i32 = "42".parse().unwrap();
    println!("默认 {inferred} / 后缀 {suffixed} / 标注 {annotated} / 解析 {parsed}");
    println!("推断结果占 {} 字节", size_of_val(&inferred));

    // 4. 整数类型与取值范围
    println!("\n--- 4. 整数类型 ---");
    println!(
        "i8   占 {} 字节，范围 {} ~ {}",
        size_of::<i8>(),
        i8::MIN,
        i8::MAX
    );
    println!(
        "i32  占 {} 字节，范围 {} ~ {}",
        size_of::<i32>(),
        i32::MIN,
        i32::MAX
    );
    println!("u8   占 {} 字节，范围 0 ~ {}", size_of::<u8>(), u8::MAX);
    println!("u64  最大值 {}", u64::MAX);
    println!("usize 占 {} 字节，常用来做下标和长度", size_of::<usize>());

    // 5. 字面量写法：下划线分隔、进制前缀、字节字面量
    println!("\n--- 5. 字面量写法 ---");
    let decimal = 1_000_000;
    let hex = 0xff;
    let octal = 0o77;
    let binary = 0b1010_1010;
    let byte = b'A';
    println!("十进制 {decimal} / 十六进制 {hex} / 八进制 {octal} / 二进制 {binary}");
    println!("字节字面量 {byte}，当字符看是 {p}", p = byte as char);

    // 6. 溢出：调试版会 panic，发布版会环绕，稳妥写法用显式方法
    println!("\n--- 6. 溢出 ---");
    println!("i32::MAX = {}", i32::MAX);
    println!("wrapping_add(1)   = {}", i32::MAX.wrapping_add(1));
    println!("checked_add(1)    = {:?}", i32::MAX.checked_add(1));
    println!("saturating_add(1) = {}", i32::MAX.saturating_add(1));

    // 7. 浮点：0.1 + 0.2 不等于 0.3
    println!("\n--- 7. 浮点 ---");
    let sum = 0.1f64 + 0.2f64;
    println!("0.1 + 0.2 = {sum:.20}");
    println!("和 0.3 相等吗: {}", sum == 0.3f64);
    println!(
        "在 EPSILON 误差内吗: {}",
        (sum - 0.3f64).abs() < f64::EPSILON
    );
    println!("无穷大 {}，负无穷 {}", f64::INFINITY, f64::NEG_INFINITY);
    let nan_a = f64::NAN;
    let nan_b = nan_a;
    println!("NAN 等于自己吗: {}", nan_a == nan_b);

    // 8. 布尔值
    println!("\n--- 8. 布尔 ---");
    let flag: bool = 3 > 2;
    println!(
        "bool 占 {} 字节，值是 {flag}，取反是 {}",
        size_of::<bool>(),
        !flag
    );

    // 9. char 与字符串的字节、字符之分
    println!("\n--- 9. char 与 &str ---");
    let ch: char = '中';
    let text = "中文";
    print!("char 占 {} 字节，值是 {ch}；", size_of::<char>());
    println!(
        "字符串 \"{text}\" 占 {} 字节，但只有 {} 个字符",
        text.len(),
        text.chars().count()
    );

    // 10. 元组：可以装不同类型，用索引或解构取值
    println!("\n--- 10. 元组 ---");
    let record: (i32, &str, f64) = (1, "文本", 3.5);
    println!("按索引取: {} {} {}", record.0, record.1, record.2);
    let (id, name, weight) = record;
    println!("解构之后: {id} {name} {weight}");
    println!("元组占 {} 字节", size_of_val(&record));

    // 11. 数组：长度固定且是类型的一部分
    println!("\n--- 11. 数组与切片 ---");
    let numbers: [i32; 3] = [1, 2, 3];
    let zeros = [0u8; 4];
    println!(
        "{numbers:?} 长度 {}；{zeros:?} 长度 {}",
        numbers.len(),
        zeros.len()
    );
    let slice: &[i32] = &numbers[1..];
    println!("切片 {slice:?} 长度 {}", slice.len());

    // 12. String 与 &str：一个拥有数据，一个是借用
    println!("\n--- 12. String 与 &str ---");
    let literal: &str = "字面量";
    let mut owned: String = String::from(literal);
    owned.push_str(" + push_str");
    println!("字面量 {literal}；拥有的字符串 {owned}");
    println!("按字节切掉第一个字: {}", &owned[3..]);
    println!("String 占 {} 字节（字符串本体在堆上）", size_of::<String>());

    // 13. 类型转换：as 会截断或饱和，From / TryFrom 更安全
    println!("\n--- 13. 类型转换 ---");
    let big: i64 = 300;
    println!("300i64 as u8 = {}（只保留低 8 位）", big as u8);
    println!("-1i32 as u32 = {}", -1i32 as u32);
    println!(
        "1.9f64 as i32 = {}，1e30f64 as i32 = {}（饱和到上限）",
        1.9f64 as i32, 1e30f64 as i32
    );
    println!("u16::from(200u8) = {}（无损）", u16::from(200u8));
    println!("u8::try_from(300i64) = {:?}", u8::try_from(300i64));

    // 14. const 与 static
    println!("\n--- 14. const 与 static ---");
    println!("const MAX_RETRY = {MAX_RETRY}，static APP_NAME = {APP_NAME}");
    let buffer = [0u8; MAX_RETRY as usize];
    println!("const 可以直接当数组长度: {buffer:?}");

    // 15. 单元类型 ()
    println!("\n--- 15. 单元类型 ---");
    log_only();
    let unit = ();
    println!("单元类型占 {} 字节，值是 {unit:?}", size_of::<()>());

    println!("\n========== 变量与基本类型演示结束 ==========");
}

/// 没有返回值的函数，返回类型就是 `()`。
fn log_only() {
    println!("这个函数只是打印，返回值是 ()");
}
