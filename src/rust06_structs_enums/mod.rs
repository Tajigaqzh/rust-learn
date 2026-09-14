//! 第 6 章配套代码：结构体与枚举。
//!
//! 运行方式：`cargo run`，输出接在第 5 章后面。

use std::fmt;

/// 矩形：最典型的「把几个字段放在一起」。
#[derive(Debug, Clone, PartialEq)]
struct Rectangle {
    width: f64,
    height: f64,
}

impl Rectangle {
    /// 关联函数：习惯上用来当构造函数，第一个参数不是 `self`。
    fn new(width: f64, height: f64) -> Self {
        Self { width, height } // 字段初始化简写
    }

    /// 只读方法：借用 `self`，不夺走所有权。
    fn area(&self) -> f64 {
        self.width * self.height
    }

    /// 需要改自己的字段时用 `&mut self`。
    fn scale(&mut self, factor: f64) {
        self.width *= factor;
        self.height *= factor;
    }

    /// 返回新值而不是改自己，方便链式调用。
    fn doubled(&self) -> Self {
        Self::new(self.width * 2.0, self.height * 2.0)
    }

    fn is_square(&self) -> bool {
        (self.width - self.height).abs() < f64::EPSILON
    }
}

/// 给自定义类型实现 `Display`，就能直接用 `{rect}` 打印。
impl fmt::Display for Rectangle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}x{}（面积 {:.2}）",
            self.width,
            self.height,
            self.area()
        )
    }
}

/// 元组结构体：字段没有名字，靠位置区分。
#[derive(Debug)]
struct Point(i32, i32);

/// 单元结构体：没有任何字段，常用来表达「这是一个类型」。
#[derive(Debug)]
struct Marker;

/// 枚举：一个类型，多种形态，每个变体可以带不同的数据。
#[derive(Debug)]
enum Shape {
    Circle { radius: f64 },
    Rect { width: f64, height: f64 },
    Triangle(f64, f64, f64),
}

impl Shape {
    fn area(&self) -> f64 {
        match self {
            Shape::Circle { radius } => std::f64::consts::PI * radius * radius,
            Shape::Rect { width, height } => width * height,
            Shape::Triangle(a, b, c) => {
                let s = (a + b + c) / 2.0;
                (s * (s - a) * (s - b) * (s - c)).sqrt()
            }
        }
    }

    fn name(&self) -> &'static str {
        match self {
            Shape::Circle { .. } => "圆",
            Shape::Rect { .. } => "矩形",
            Shape::Triangle(..) => "三角形",
        }
    }
}

/// 用 `Option` 表达「可能没有」：返回第一个偶数。
fn first_even(values: &[i32]) -> Option<i32> {
    values.iter().find(|value| **value % 2 == 0).copied()
}

/// 一半，奇数返回 `None`，用来演示 `let ... else`。
fn halve(n: i32) -> Option<i32> {
    if n % 2 == 0 { Some(n / 2) } else { None }
}

fn describe_half(n: i32) -> String {
    let Some(half) = halve(n) else {
        return format!("{n} 不是偶数，算不了一半");
    };
    format!("{n} 的一半是 {half}")
}

/// 返回借用：面积最大的形状。
fn widest(shapes: &[Shape]) -> Option<&Shape> {
    let mut best: Option<&Shape> = None;
    for shape in shapes {
        if best.is_none() || shape.area() > best.unwrap().area() {
            best = Some(shape);
        }
    }
    best
}

/// 状态机：`enum` + `impl` 组合起来用。
#[derive(Debug)]
enum TrafficLight {
    Red,
    Green,
    Yellow,
}

impl TrafficLight {
    fn next(&self) -> Self {
        match self {
            TrafficLight::Red => TrafficLight::Green,
            TrafficLight::Green => TrafficLight::Yellow,
            TrafficLight::Yellow => TrafficLight::Red,
        }
    }

    fn action(&self) -> &'static str {
        match self {
            TrafficLight::Red => "停",
            TrafficLight::Green => "行",
            TrafficLight::Yellow => "等",
        }
    }
}

/// 演示结构体、方法、枚举、`match` 与 `Option`。
pub fn structs_enums_demo() {
    println!("\n========== rust06_structs_enums: 结构体与枚举 ==========");

    // 1. 创建与访问
    println!("\n--- 1. 结构体：创建与访问 ---");
    let mut rect = Rectangle::new(3.0, 4.0);
    println!("    rect = {rect:?}");
    println!("    width = {}, height = {}", rect.width, rect.height);
    rect.scale(2.0);
    println!("    scale(2.0) 之后 = {rect:?}");

    let width = 5.0;
    let height = 2.5;
    let shorthand = Rectangle { width, height }; // 变量名和字段名相同时可以简写
    println!("    字段简写 = {shorthand:?}");

    let bigger = Rectangle {
        width: 10.0,
        ..shorthand
    };
    println!("    更新语法 = {bigger:?}");
    println!("    原来的 shorthand 还能用 = {shorthand:?}");

    // 2. 方法与关联函数
    println!("\n--- 2. 方法与关联函数 ---");
    let base = Rectangle::new(2.0, 3.0);
    let doubled = base.doubled();
    println!("    base = {base}");
    println!("    doubled = {doubled}");
    println!("    doubled 是正方形吗：{}", doubled.is_square());
    println!(
        "    4x4 是正方形吗：{}",
        Rectangle::new(4.0, 4.0).is_square()
    );

    // 3. derive 与 Display
    println!("\n--- 3. derive 与 Display ---");
    let a = Rectangle::new(2.0, 3.0);
    let b = a.clone();
    println!("    a == b：{}", a == b);
    println!("    美化 Debug：{a:#?}");

    // 4. 元组结构体与单元结构体
    println!("\n--- 4. 元组结构体与单元结构体 ---");
    let p = Point(3, -1);
    println!("    p = {p:?}，p.0 = {}, p.1 = {}", p.0, p.1);
    let Point(x, y) = p;
    println!("    解构之后 x = {x}, y = {y}");
    let marker = Marker;
    println!("    单元结构体的值 = {marker:?}");

    // 5. 枚举：一个类型多种形态
    println!("\n--- 5. 枚举：一个类型多种形态 ---");
    let shapes = [
        Shape::Circle { radius: 1.0 },
        Shape::Rect {
            width: 3.0,
            height: 4.0,
        },
        Shape::Triangle(3.0, 4.0, 5.0),
    ];
    for shape in &shapes {
        println!(
            "    {:<6} {shape:?} -> 面积 {:.4}",
            shape.name(),
            shape.area()
        );
    }

    // 6. match 的常用写法
    println!("\n--- 6. match 的常用写法 ---");
    for n in [0, 3, 42, -5] {
        let label = match n {
            0 => "零".to_string(),
            1..=9 => format!("个位数 {n}"),
            v if v < 0 => format!("负数 {v}"),
            _ => format!("大数 {n}"),
        };
        println!("    {n} -> {label}");
    }

    for n in [5, 15, 25] {
        let range = match n {
            v @ 1..=10 => format!("{v} 落在 1..=10"),
            v @ 11..=20 => format!("{v} 落在 11..=20"),
            other => format!("{other} 更大"),
        };
        println!("    {range}");
    }

    for rect in [Rectangle::new(4.0, 4.0), Rectangle::new(2.0, 5.0)] {
        match rect {
            Rectangle { width, height } if width == height => {
                println!("    解构 + guard：{width} x {height} 是正方形")
            }
            Rectangle { width, height } => {
                println!("    解构：{width} x {height} 是长方形")
            }
        }
    }

    // 7. Option<T>
    println!("\n--- 7. Option<T>：没有 null 的世界 ---");
    println!(
        "    first_even(&[1, 3, 4, 7]) = {:?}",
        first_even(&[1, 3, 4, 7])
    );
    println!(
        "    first_even(&[1, 3, 5])    = {:?}",
        first_even(&[1, 3, 5])
    );

    match first_even(&[1, 3, 4]) {
        Some(n) => println!("    match 拿到 {n}"),
        None => println!("    match 啥也没找到"),
    }
    println!(
        "    unwrap_or 兜底 = {}",
        first_even(&[1, 3, 5]).unwrap_or(0)
    );
    if let Some(n) = first_even(&[2, 5]) {
        println!("    if let 拿到 {n}");
    }

    let numbers = [10, 20, 30];
    println!("    numbers.get(1) = {:?}", numbers.get(1));
    println!("    numbers.get(9) = {:?}", numbers.get(9));

    // 8. let ... else
    println!("\n--- 8. let ... else：不匹配就提前退出 ---");
    println!("    {}", describe_half(8));
    println!("    {}", describe_half(7));

    // 9. 枚举当状态机
    println!("\n--- 9. 综合：枚举当状态机 ---");
    let mut light = TrafficLight::Red;
    for step in 1..=4 {
        println!("    第 {step} 步：{:?} -> {}", light, light.action());
        light = light.next();
    }

    // 10. 返回 Option<&Shape>
    println!("\n--- 10. 返回值：Option<&Shape> ---");
    match widest(&shapes) {
        Some(shape) => println!("    面积最大的是 {}（{:.4}）", shape.name(), shape.area()),
        None => println!("    一个形状都没有"),
    }
    println!("    空切片时：{:?}", widest(&[]).map(|shape| shape.name()));

    println!("\n========== 结构体与枚举演示结束 ==========");
}
