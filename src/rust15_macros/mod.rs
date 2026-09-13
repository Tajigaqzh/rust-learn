//! 第 15 章配套代码：宏。
//!
//! 运行方式：`cargo run`，输出接在第 14 章后面。
//!
//! 这里定义的都是**声明宏**（`macro_rules!`）：它们写在本模块里，
//! 所以在下面 `macros_demo` 之前定义才能被调用——宏是按文本顺序生效的。

/// 最简单的声明宏：没有参数。
macro_rules! say_hello {
    () => {
        println!("    你好，宏！");
    };
}

/// 带参数的宏：`$label:expr` 表示「这里放一个表达式」。
macro_rules! show {
    ($label:expr, $value:expr) => {
        println!("    {} = {}", $label, $value);
    };
}

/// 重复匹配：`$(...),+` 表示「一个或多个，用逗号分隔」。
macro_rules! my_vec {
    () => {
        Vec::new()
    };
    ($($element:expr),+ $(,)?) => {{
        let mut v = Vec::new();
        $( v.push($element); )+
        v
    }};
}

/// 重复 + 递归：把参数展开成加法表达式。
macro_rules! sum_of {
    () => {
        0
    };
    ($first:expr $(, $rest:expr)*) => {
        $first $(+ $rest)*
    };
}

/// 演示卫生性：宏内部的 `doubled` 不会和外部的同名变量打架。
macro_rules! double_with_internal {
    ($value:expr) => {{
        let doubled = $value * 2;
        doubled
    }};
}

/// 实用一点的宏：带标签的日志，`$($arg:tt)*` 原样转发给 `format!`。
macro_rules! log_tagged {
    ($tag:ident, $($arg:tt)*) => {
        println!("    [{}] {}", stringify!($tag), format!($($arg)*));
    };
}

/// 生成代码：宏可以生成整个类型和它的 `impl`。
macro_rules! define_point {
    ($name:ident, $x:expr, $y:expr) => {
        struct $name {
            x: i32,
            y: i32,
        }

        impl $name {
            fn new() -> Self {
                Self { x: $x, y: $y }
            }

            fn sum(&self) -> i32 {
                self.x + self.y
            }
        }
    };
}

/// 生成语句：`$ty:ty` 匹配一个类型。
macro_rules! make_container {
    ($name:ident, $ty:ty) => {
        let mut $name: Vec<$ty> = Vec::new();
    };
}

/// 演示声明宏的几种常见写法。
pub fn macros_demo() {
    println!("\n========== rust15_macros: 宏 ==========");

    println!("\n--- 1. 最简单的声明宏 ---");
    say_hello!();

    println!("\n--- 2. 带参数的宏 ---");
    let width = 3;
    show!("width * 2", width * 2);
    show!("字符串长度", "hello".len());

    println!("\n--- 3. 重复匹配 ---");
    let empty: Vec<i32> = my_vec![];
    let numbers = my_vec![1, 2, 3];
    println!("    my_vec![] = {empty:?}");
    println!("    my_vec![1, 2, 3] = {numbers:?}");
    println!("    带尾随逗号 my_vec![1, 2, 3,] = {:?}", my_vec![1, 2, 3,]);
    println!("    sum_of!(1, 2, 3, 4) = {}", sum_of!(1, 2, 3, 4));
    println!("    sum_of!() = {}", sum_of!());

    println!("\n--- 4. 卫生性 ---");
    let doubled = 100;
    let result = double_with_internal!(5);
    println!("    外部的 doubled = {doubled}");
    println!("    宏内部也叫 doubled，但互不影响；宏返回 = {result}");

    println!("\n--- 5. 片段说明符 ---");
    let value = 42u32;
    log_tagged!(
        info,
        "value = {value}, 类型 = {}",
        std::any::type_name_of_val(&value)
    );
    define_point!(Point, 3, 4);
    let point = Point::new();
    println!(
        "    define_point! 生成了类型：{}+{}={}",
        point.x,
        point.y,
        point.sum()
    );
    make_container!(names, String);
    names.push(String::from("ada"));
    println!("    make_container! 生成了变量：{names:?}");

    println!("\n--- 6. 标准库的宏也是这么展开的 ---");
    println!("    println!(\"...\")  →  std::io::_print(format_args!(\"...\"))");
    println!("    vec![1, 2, 3]     →  <[_]>::into_vec(Box::new([1, 2, 3]))");
    println!("    assert_eq!(a, b)  →  比较失败时 panic，并把两边的值打印出来");

    println!("\n--- 7. 过程宏能做什么 ---");
    println!("    #[derive(Debug)] 是 derive 过程宏：读结构体定义，生成 impl Debug");
    println!("    #[tokio::main] 是属性过程宏：把 async fn main 改写成普通 fn main");
    println!("    serde 的 #[derive(Serialize)] 也是；它们必须放在独立的 proc-macro crate");

    println!("\n========== 宏演示结束 ==========");
}
