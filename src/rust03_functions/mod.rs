//! 第 3 章配套代码：函数与表达式。
//!
//! 运行方式：`cargo run`，输出接在第 2 章后面。

/// 编译期常量，值由下面的 `const fn` 在编译期算出来。
const SQUARE_OF_5: i32 = const_square(5);

/// 累加，遇到第一个负数就用 `return` 提前结束。
fn sum_until_negative(values: &[i32]) -> i32 {
    let mut total = 0;
    for &value in values {
        if value < 0 {
            return total;
        }
        total += value;
    }
    total
}

/// 演示函数定义、返回值、表达式与语句的区别。
pub fn functions_demo() {
    println!("\n========== rust03_functions: 函数与表达式 ==========");

    // 1. 定义与调用
    println!("\n--- 1. 定义与调用 ---");
    greet();
    println!("add(2, 3) = {}", add(2, 3));
    println!("square(5) = {}", square(5));

    // 2. 返回值：尾表达式、return、元组
    println!("\n--- 2. 返回值 ---");
    println!("absolute(-7) = {}", absolute(-7));
    let (quotient, remainder) = divmod(17, 5);
    println!("17 除以 5 = {quotient} 余 {remainder}");

    // 3. 表达式有值，语句没有
    println!("\n--- 3. 表达式与语句 ---");
    let block_value = {
        let base = 1;
        base + 1 // 这一行没有分号，它就是块的值
    };
    let label = if block_value > 2 {
        "大于 2"
    } else {
        "不大于 2"
    };
    println!("块的值是 {block_value}，判定为 {label}");

    // 4. 提前返回
    println!("\n--- 4. 提前返回 ---");
    println!("遇到负数就停: {}", sum_until_negative(&[3, 5, -2, 7]));
    println!("全是正数时: {}", sum_until_negative(&[3, 5, 2, 7]));

    // 5. 函数是一等公民，可以存进变量、当参数传
    println!("\n--- 5. 函数指针 ---");
    let op: fn(i32, i32) -> i32 = add;
    println!("通过函数指针调用 op(4, 5) = {}", op(4, 5));
    println!("函数当参数传入 apply(add, 6, 7) = {}", apply(add, 6, 7));

    // 6. const fn：编译期就能算出结果
    println!("\n--- 6. const fn ---");
    println!("编译期常量 SQUARE_OF_5 = {SQUARE_OF_5}");
    println!("运行时调用 const_square(6) = {}", const_square(6));

    // 7. 参数按值传递，借用要显式写 &
    println!("\n--- 7. 参数传递 ---");
    let owned = String::from("rust-learn");
    let counted = count_chars(&owned);
    println!("借用传参后 String 仍然可用: {owned}，字符数 {counted}");

    println!("\n========== 函数与表达式演示结束 ==========");
}

/// 没有返回值，返回类型就是 `()`。
fn greet() {
    println!("你好，函数！");
}

/// 两个整数相加。
fn add(a: i32, b: i32) -> i32 {
    a + b
}

/// 求平方，函数体最后一行不带分号，它就是返回值。
fn square(x: i32) -> i32 {
    x * x
}

/// 求绝对值，用 `return` 提前返回。
fn absolute(x: i32) -> i32 {
    if x < 0 {
        return -x;
    }
    x
}

/// 用一个元组返回多个值。
fn divmod(a: i32, b: i32) -> (i32, i32) {
    (a / b, a % b)
}

/// 把函数当参数接收，调用它并返回结果。
fn apply(f: fn(i32, i32) -> i32, a: i32, b: i32) -> i32 {
    f(a, b)
}

/// 可以在编译期求值的函数。
const fn const_square(x: i32) -> i32 {
    x * x
}

/// 参数优先收 `&str`，这样 String 和字面量都能传进来。
fn count_chars(text: &str) -> usize {
    text.chars().count()
}
