//! `print!` / `println!` / `eprintln!` / `format!` 等打印相关用法的演示模块。
//!
//! 运行方式：`cargo run`，结果在终端（stdout + stderr）一起输出。

use std::fmt::Write as _;

/// 用来演示 `{:?}` / `{:#?}` 的自定义类型。
#[derive(Debug)]
struct Point {
    x: i32,
    y: i32,
}

/// 演示与打印相关的常见用法，供 `main` 直接调用。
pub fn print_demo() {
    println!("========== rust01_print: 打印用法演示 ==========");

    // 1. print! 不换行，println! 自动换行
    println!("\n--- 1. print! 与 println! ---");
    print!("print! 不换行, ");
    print!("可以继续往右写, ");
    println!("println! 会在末尾换行。");

    // 2. 占位符按顺序、按位置、按名字取值
    println!("\n--- 2. 占位符与参数 ---");
    println!("顺序取值: {} + {} = {}", 1, 2, 1 + 2);
    // 用编号可以把同一个参数取多次，编号之间不会互相影响
    println!("位置参数可重复: {0} + {1} + {0} = {0}{1}{0}", "a", "b");
    let lang = "Rust";
    let author = "Graydon Hoare";
    // 命名参数写在格式串后面，参数多的时候比数位置更清楚
    println!(
        "命名参数: {lang} 由 {author} 创建，1.0 发布于 {year} 年",
        year = 2015
    );

    // 3. Rust 2021+ 可以直接把变量名写进大括号里捕获
    let version = 2024;
    println!(
        "变量捕获: {lang} edition {version}, 明年是 {0}",
        version + 1
    );
    println!("大括号里还能放表达式: {} 和 {}", 10 * 10, "abc".len());

    // 4. 转义大括号本身要用 {{ 和 }}
    println!("\n--- 3. 转义与特殊字符 ---");
    println!("字面量大括号: {{}}，对应占位符: {}", 1);
    println!("制表符\t换行\n反斜杠\\引号\"");

    // 5. 宽度、对齐、填充、精度、补零
    println!("\n--- 4. 宽度 / 对齐 / 精度 ---");
    println!("|{:>10}|", "右对齐");
    println!("|{:<10}|", "左对齐");
    println!("|{:^10}|", "居中");
    println!("|{:*^10}|", "填充星号");
    println!("|{:08}| 补零", 42);
    println!("精度: {:.3}（默认四舍五入）", 1.0 / 3.0);
    println!("保留两位加单位: {:.2} 元", 12.3456);

    // 6. 数值进制与科学计数法
    println!("\n--- 5. 数值格式化 ---");
    let n = 42;
    println!("二进制 {n:b} / 带前缀 {n:#b}");
    println!("八进制 {n:o} / 十六进制 {n:x} 或 {n:X} / 带前缀 {n:#x}");
    println!("科学计数法 {:e} / 大写 {:E}", 1234.5, 1234.5);

    // 7. Debug 打印：{:?} 适合调试，{:#?} 是带缩进的美化版
    println!("\n--- 6. Debug 打印 ---");
    let nums = vec![1, 2, 3];
    let point = Point { x: 3, y: -1 };
    println!("单行 {:?}", nums);
    println!("单行 {:?}", point);
    println!("美化 {:#?}", point);
    println!("字段也能正常读取: x + y = {}", point.x + point.y);

    // 8. format! 只是生成字符串，不会打印
    println!("\n--- 7. format! 生成字符串 ---");
    let msg = format!("{}-{}-{}", 2026, 9, 13);
    println!("format! 的结果: {msg}，长度 {}", msg.len());

    // 9. writeln! 可以把格式化结果写进任意实现了 fmt::Write 的目标（如 String）
    println!("\n--- 8. writeln! 写入 String ---");
    let mut buf = String::new();
    writeln!(buf, "{:>6} | {:.2}", "总计", 99.5).unwrap();
    writeln!(buf, "{:>6} | {:.2}", "折扣", 9.95).unwrap();
    print!("{buf}");

    // 10. eprintln! 输出到 stderr，常用于日志和报错
    println!("\n--- 9. eprintln! 输出到 stderr ---");
    eprintln!("这行来自 eprintln!，走的是标准错误流");

    // 11. dbg! 会打印“文件:行号 = 值”并返回该值，适合临时排查
    println!("\n--- 10. dbg! 快速调试 ---");
    let doubled = dbg!(21 * 2);
    println!("dbg! 会返回原值，所以还能继续用: {doubled}");

    println!("\n========== 演示结束 ==========");
}
