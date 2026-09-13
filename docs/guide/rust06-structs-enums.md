# 第 6 章 · 结构体与枚举

前五章一直在用别人定义好的类型：`i32`、`String`、`Vec`、`Option`。这一章开始定义自己的类型。

结构体解决「**把相关的几个值打包成一个整体**」：`Rectangle` 把宽和高放在一起，`User` 把名字和年龄放在一起。

枚举解决「**一个东西可能有几种形态，每种形态带的数据还不一样**」：`Shape` 可能是圆、矩形或三角形；`Option<T>` 可能是 `Some(值)` 或 `None`；一个网络消息可能是「登录」「发文本」或者「断开」。

两者配合 `impl` 和 `match`，构成了 Rust 里最常用的建模方式：**枚举负责表达「是什么」，`match` 负责「分别怎么处理」，结构体负责「含着哪些数据」**。这套组合在第 7 章的集合、第 8 章的错误处理里会反复出现。

本章配套代码在 `src/rust06_structs_enums/mod.rs`，执行 `cargo run` 可以看到全部输出。

## 6.1 定义、创建与访问

```rust
#[derive(Debug)]
struct Rectangle {
    width: f64,
    height: f64,
}

let rect = Rectangle { width: 3.0, height: 4.0 };
println!("{} x {}", rect.width, rect.height);   // 用点号访问字段
```

几个约定：

- 类型名用大驼峰 `Rectangle`，字段名用小写下划线 `width`（第 2 章 2.1 的命名规范）。
- **每个字段都必须初始化**，少一个就报 `E0063: missing field`（见 6.14）。
- 字段默认是私有的，只有同一个模块里能直接访问；跨模块访问要配 `pub`（第 14 章讲模块时展开）。
- 结构体本身不实现任何 trait，`{:?}` 都打不出来，除非 `derive` 一下（见 6.6）。

创建出来的结构体是一个值，自然遵守第 5 章的所有权规则：字段里有 `String`，整个结构体就没有 `Copy`，赋值和传参会移动它。

## 6.2 字段简写与更新语法

两个常用的简写：

```rust
let width = 5.0;
let height = 2.5;

// 变量名和字段名一样时，可以只写一次
let rect = Rectangle { width, height };

// 其余字段从另一个值拿，只覆盖想改的
let bigger = Rectangle { width: 10.0, ..rect };
```

更新语法要留意所有权：`..rect` 会把 `rect` 里**没被显式覆盖的字段**移动过来。上面这段能编过，是因为 `f64` 是 `Copy`，字段被复制而不是移动，所以 `rect` 之后还能用（实测输出里 `原来的 shorthand 还能用` 就是这一行）。如果字段是 `String`，`..rect` 之后 `rect` 就不能再用了——和普通移动一样。

## 6.3 `impl`：方法与关联函数

给结构体挂函数用 `impl` 块：

```rust
impl Rectangle {
    // 关联函数：第一个参数不是 self，习惯用来当构造函数
    fn new(width: f64, height: f64) -> Self {
        Self { width, height }
    }

    // 方法：第一个参数是 self 的某种形式
    fn area(&self) -> f64 {
        self.width * self.height
    }

    fn scale(&mut self, factor: f64) {
        self.width *= factor;
        self.height *= factor;
    }
}
```

术语上有两个区别：

| | 关联函数 | 方法 |
| --- | --- | --- |
| 第一个参数 | 没有 `self` | `self` / `&self` / `&mut self` |
| 调用方式 | `Rectangle::new(3.0, 4.0)` | `rect.area()` |
| 典型用途 | 构造函数、工具函数 | 读字段、改字段 |

`Self` 是「当前类型」的简写，在 `impl Rectangle` 里 `Self` 就是 `Rectangle`。`new` 不是关键字，只是社区约定俗成的构造函数名；Rust 没有构造函数语法，`Rectangle { ... }` 本身就是创建值的方式，`new` 只是把它包装得更好看一点。

一个 `impl` 块里可以放很多函数，也可以写多个 `impl` 块（比如拆成「构造」「计算」「打印」几组），效果一样。

## 6.4 `self` / `&self` / `&mut self` 怎么选

这是初学者最常见的困惑，规则其实和普通函数参数完全一致（第 5 章的借用规则照搬过来）：

| 写法 | 含义 | 什么时候用 |
| --- | --- | --- |
| `&self` | 只读借用 | 大部分方法，默认选它 |
| `&mut self` | 可变借用 | 要改字段，比如 `scale` |
| `self` | 拿走所有权 | 把 `self` 拆掉、消费掉，比如 `into_*` 方法 |

```rust
impl Rectangle {
    fn doubled(&self) -> Self {          // 不改自己，返回新值
        Self::new(self.width * 2.0, self.height * 2.0)
    }

    fn is_square(&self) -> bool {
        (self.width - self.height).abs() < f64::EPSILON
    }
}
```

实测：

```text
    base = 2x3（面积 6.00）
    doubled = 4x6（面积 24.00）
    doubled 是正方形吗：false
    4x4 是正方形吗：true
```

`self`（不带 `&`）意味着调用这个方法会把值消费掉，之后原变量就不能再用了：

```rust
let s = User { name: String::from("x") };
let a = s.into_name();     // s 被移动进方法
let b = s.into_name();     // 报 E0382：use of moved value
```

标准库里的 `to_*` / `as_*` 通常只借用，`into_*` 通常拿走所有权，命名本身就是提示。

## 6.5 元组结构体与单元结构体

字段名不重要时，可以省掉名字，用位置区分：

```rust
struct Point(i32, i32);

let p = Point(3, -1);
println!("{} {}", p.0, p.1);      // 用 .0 / .1 访问
let Point(x, y) = p;              // 也能解构
```

连字段都没有的结构体叫**单元结构体**，作用是「占一个类型」：

```rust
struct Marker;
let marker = Marker;
```

看起来没用，但它实现 trait 之后就有了意义：比如 `struct Meters(f64)` 能防止你把米和秒相加，单元结构体常被当作标记类型或者 trait 的载体。第 9 章讲 trait 时还会遇到它们。

元组结构体的一个实战价值是**给同一个底层类型加区分**：

```rust
struct Celsius(f64);
struct Fahrenheit(f64);
```

两个都是 `f64`，但类型不同，混用会编译失败——这比用裸 `f64` 安全得多。

## 6.6 `derive` 与 `Display`

结构体默认什么 trait 都没有，连 `{:?}` 都打不出来。用 `#[derive(...)]` 自动实现最常用的几个：

```rust
#[derive(Debug, Clone, PartialEq)]
struct Rectangle {
    width: f64,
    height: f64,
}
```

| derive | 得到的 ability | 常见用途 |
| --- | --- | --- |
| `Debug` | `{:?}`、`{:#?}` 打印 | 调试、测试里的断言信息 |
| `Clone` | `.clone()` | 需要两份独立数据 |
| `Copy` | 赋值即复制（元素都 `Copy` 时才行） | 小的值类型 |
| `PartialEq` / `Eq` | `==`、`!=` | 比较、去重、`assert_eq!` |
| `Default` | `Type::default()` | 字段很多、多数取默认值 |
| `Hash` | 能放进 `HashMap` 的键 | 第 7 章 |

两个最常用的打印格式：

```rust
println!("{rect:?}");    // 单行：Rectangle { width: 2.0, height: 3.0 }
println!("{rect:#?}");   // 多行美化
```

`{:#?}` 实测输出：

```text
Rectangle {
    width: 2.0,
    height: 3.0,
}
```

`Debug` 是给开发者看的；想给用户看的输出，就自己实现 `Display`（第 1 章 1.6 讲过两者的区别）：

```rust
use std::fmt;

impl fmt::Display for Rectangle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}x{}（面积 {:.2}）", self.width, self.height, self.area())
    }
}

println!("{rect}");     // 2x3（面积 6.00）
```

`impl fmt::Display for Rectangle` 读作「给 `Rectangle` 实现 `Display` trait」。形式固定，照抄即可，细节到第 9 章讲 trait 时会清楚。

## 6.7 枚举：一个类型多种形态

枚举定义的是「所有可能的取值」：

```rust
enum TrafficLight {
    Red,
    Green,
    Yellow,
}

let light = TrafficLight::Red;
```

和别的语言里的枚举最大的不同：**Rust 的枚举变体可以带数据**，而且每个变体带的数据可以不一样。

```rust
enum Shape {
    Circle { radius: f64 },              // 结构体风格的变体
    Rect { width: f64, height: f64 },
    Triangle(f64, f64, f64),             // 元组风格的变体
}
```

这句话的意思是：一个 `Shape` 类型的值，要么是一个圆（带半径），要么是一个矩形（带宽高），要么是一个三角形（带三条边）——不可能三者都是，也不可能什么都不是。**「多选一」这件事由类型系统保证**，不需要靠文档约定，也不需要运行时检查。

这正是枚举比「用一个 `kind` 字段加一堆可空字段」强的地方：那些写法允许构造出「`kind` 是圆但半径是 `None`」这种非法状态，而枚举让非法状态根本写不出来。

## 6.8 带数据的变体与 `impl`

枚举也能挂 `impl`，方法里用 `match` 分情况处理：

```rust
impl Shape {
    fn area(&self) -> f64 {
        match self {
            Shape::Circle { radius } => std::f64::consts::PI * radius * radius,
            Shape::Rect { width, height } => width * height,
            Shape::Triangle(a, b, c) => {
                let s = (a + b + c) / 2.0;
                (s * (s - a) * (s - b) * (s - c)).sqrt()   // 海伦公式
            }
        }
    }
}
```

注意 `match self` 里拿到的 `radius`、`width` 都是 `&f64`（因为 `self` 是 `&Shape`），可以直接参与运算——Rust 会在解构时把字段借出来。

实测：

```text
    圆      Circle { radius: 1.0 } -> 面积 3.1416
    矩形     Rect { width: 3.0, height: 4.0 } -> 面积 12.0000
    三角形    Triangle(3.0, 4.0, 5.0) -> 面积 6.0000
```

变体也可以只用来分类，不带数据（6.7 的红绿灯就是），或者用 `..` 忽略数据：

```rust
fn name(&self) -> &'static str {
    match self {
        Shape::Circle { .. } => "圆",
        Shape::Rect { .. } => "矩形",
        Shape::Triangle(..) => "三角形",
    }
}
```

## 6.9 `match` 与穷尽检查

`match` 是处理枚举的标准工具，它和 `switch` 最大的区别是**必须覆盖所有情况**，少一个都编不过：

```rust
enum Shape {
    Circle(f64),
    Rect(f64, f64),
}

fn area(s: Shape) -> f64 {
    match s {
        Shape::Circle(r) => 3.14 * r * r,
        // 少写了 Rect，编译失败
    }
}
```

```text
error[E0004]: non-exhaustive patterns: `Shape::Rect(_, _)` not covered
 --> src/main.rs:3:11
  |
3 |     match s {
  |           ^ pattern `Shape::Rect(_, _)` not covered
  |
note: `Shape` defined here
 --> src/main.rs:1:6
  |
1 | enum Shape { Circle(f64), Rect(f64, f64) }
  |      ^^^^^                ---- not covered
  = note: the matched value is of type `Shape`
help: ensure that all possible cases are being handled by adding a match arm with a wildcard pattern or an explicit pattern as shown
  |
4 ~         Shape::Circle(r) => 3.14 * r * r,
5 ~         Shape::Rect(_, _) => todo!(),
  |
```

这条规则带来两个好处：以后给枚举**新增一个变体**，所有没处理它的 `match` 会立刻编译失败，编译器帮你把所有该改的地方列出来；反过来，读到一段能编过的 `match`，就能确信所有情况都处理了。

`match` 本身是表达式（第 3 章），所以能直接用来赋值：

```rust
let label = match light {
    TrafficLight::Red => "停",
    TrafficLight::Green => "行",
    TrafficLight::Yellow => "等",
};
```

## 6.10 `match` 的常用写法

几种每天都会用到的模式：

```rust
let n = 42;
match n {
    0 => "零".to_string(),                 // 字面量
    1..=9 => format!("个位数 {n}"),         // 范围（含两端）
    v if v < 0 => format!("负数 {v}"),      // guard：额外的判断条件
    _ => format!("大数 {n}"),               // 兜底
}
```

实测 `0 -> 零`、`3 -> 个位数 3`、`42 -> 大数 42`、`-5 -> 负数 -5`。

想在匹配的同时拿到值，用 `@` 绑定：

```rust
match n {
    v @ 1..=10 => format!("{v} 落在 1..=10"),
    v @ 11..=20 => format!("{v} 落在 11..=20"),
    other => format!("{other} 更大"),
}
```

`|` 用来把多个模式合成一个分支，`_` 是兜底。解构结构体、配合 guard 一起用也很自然：

```rust
for rect in [Rectangle::new(4.0, 4.0), Rectangle::new(2.0, 5.0)] {
    match rect {
        Rectangle { width, height } if width == height => println!("{width} x {height} 是正方形"),
        Rectangle { width, height } => println!("{width} x {height} 是长方形"),
    }
}
```

实测两行输出分别是「解构 + guard：4 x 4 是正方形」和「解构：2 x 5 是长方形」——注意 guard 的匹配顺序：**先判断的放前面**，和 `if / else if` 一样。

## 6.11 `Option<T>`：没有 null 的世界

标准库里的 `Option` 就是一个普通的枚举：

```rust
enum Option<T> {
    Some(T),
    None,
}
```

它把「可能没有值」这件事写进了类型里。函数要表达「可能找不到」，就返回 `Option`：

```rust
fn first_even(values: &[i32]) -> Option<i32> {
    for &value in values {
        if value % 2 == 0 {
            return Some(value);
        }
    }
    None
}
```

实测 `first_even(&[1, 3, 4, 7]) = Some(4)`、`first_even(&[1, 3, 5]) = None`。

常见的处理方式：

| 写法 | 含义 |
| --- | --- |
| `match` | 两个分支都处理，最完整 |
| `if let Some(x) = ...` | 只关心有值的情况 |
| `.unwrap_or(default)` | 没有值时给一个默认值 |
| `.unwrap()` / `.expect("msg")` | 没有值就 panic，适合「这里不可能没有」的场合 |
| `.is_some()` / `.is_none()` | 只判断有没有，不取值 |
| `.map(f)` | 有值时对里面的值做变换，仍然是 `Option` |
| `.copied()` / `.cloned()` | 把 `Option<&T>` 变成 `Option<T>` |

标准库很多方法都返回 `Option`，索引类操作尤其明显：

```rust
let numbers = vec![10, 20, 30];
numbers.get(1);    // Some(&20)
numbers.get(9);    // None，不 panic
```

对比一下就明白它的价值：`numbers[9]` 会 panic，`numbers.get(9)` 让你自己决定怎么办。**能用 `get` 就不用下标**，尤其是在下标来自外部输入的时候。

`Option` 也是「Rust 没有 null」的答案：别的语言里 null 值可以出现在任何引用上，调用方得靠文档和运气；Rust 把「可能为空」变成类型 `Option<T>`，不处理 `None` 就用不了里面的值，编译器盯着你。第 8 章会讲另一个枚举 `Result<T, E>`，专门表达「可能失败」。

## 6.12 `if let`、`let ... else` 与 `while let`

只关心一个分支时，`if let` 比 `match` 短：

```rust
if let Some(n) = first_even(&[2, 5]) {
    println!("拿到 {n}");
}
```

需要在「不匹配」时提前退出，用 `let ... else`（Rust 1.65 起稳定）：

```rust
fn describe_half(n: i32) -> String {
    let Some(half) = halve(n) else {
        return format!("{n} 不是偶数，算不了一半");
    };
    format!("{n} 的一半是 {half}")
}
```

实测 `8 的一半是 4` / `7 不是偶数，算不了一半`。这种写法的好处是：**解构之后，后面所有代码都在「值一定存在」的前提下写**，不用再层层嵌套。它替代了以前要写一坨 `match` 的「卫语句」模式。

`while let` 则是「一直取到没有为止」，第 4 章见过：

```rust
let mut stack = vec![1, 2, 3];
while let Some(top) = stack.pop() {
    print!("{top} ");
}
```

三者的分工：`let ... else` 处理「不满足就没法继续」，`if let` 处理「满足时做点事」，`while let` 处理「反复取直到取不到」。

## 6.13 综合：用枚举建模

把 `enum` + `impl` + `match` 合起来，就是一个小状态机：

```rust
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
```

实测：

```text
    第 1 步：Red -> 停
    第 2 步：Green -> 行
    第 3 步：Yellow -> 等
    第 4 步：Red -> 停
```

这个模式可以扩展到很多地方：订单状态（待支付 → 已支付 → 已发货）、连接状态（未连接 → 连接中 → 已连接 → 断开）、解析器的当前节点。它的好处是**状态和动作都写在同一处**，新增一个状态时编译器会强制你补上所有 `match` 分支。

再往前一步，如果状态里还要带数据（比如「已连接」要记住地址、「失败」要记住原因），就直接写成带数据的变体：

```rust
enum Connection {
    Idle,
    Connecting { host: String },
    Connected { host: String, latency_ms: u32 },
    Failed { reason: String },
}
```

用结构体加 `bool` 也能表达这些状态，但那样可以构造出「既 connected 又 failed」的非法值；枚举从类型上就杜绝了。

## 6.14 十个真实报错怎么读

全部来自实际编译，行号是各自最小例子的行号。

**案例 1：结构体初始化少了字段（`E0063`）**

```text
error[E0063]: missing field `height` in initializer of `Rectangle`
 --> src/main.rs:2:21
  |
2 | fn main() { let r = Rectangle { width: 1.0 }; println!("{}", r.width); }
  |                     ^^^^^^^^^ missing `height`
```

**案例 2：访问不存在的字段（`E0609`）**

```text
error[E0609]: no field `depth` on type `Rectangle`
 --> src/main.rs:2:77
  |
2 | fn main() { let r = Rectangle { width: 1.0, height: 2.0 }; println!("{}", r.depth); }
  |                                                                             ^^^^^ unknown field
  |
  = note: available fields are: `width`, `height`
```

`note` 会列出所有可用字段，拼错字段名时很省事。

**案例 3：没 `derive(Debug)` 就想 `{:?}`（`E0277`）**

```text
error[E0277]: `Point` doesn't implement `Debug`
 --> src/main.rs:4:15
  |
4 |     println!("{p:?}");
  |               ^^^^^ `Point` cannot be formatted using `{:?}` because it doesn't implement `Debug`
  |
  = help: the trait `Debug` is not implemented for `Point`
help: consider annotating `Point` with `#[derive(Debug)]`
  |
1 + #[derive(Debug)]
2 | struct Point { x: i32 }
  |
```

**案例 4：没有 `PartialEq` 就写 `==`（`E0369`）**

```text
error[E0369]: binary operation `==` cannot be applied to type `P`
 --> src/main.rs:5:22
  |
5 |     println!("{}", a == b);
  |                    - ^^ - P
  |                    |
  |                    P
  |
note: an implementation of `PartialEq` might be missing for `P`
 --> src/main.rs:1:1
  |
1 | struct P { x: i32 }
  | ^^^^^^^^ must implement `PartialEq`
help: consider annotating `P` with `#[derive(PartialEq)]`
  |
1 + #[derive(PartialEq)]
2 | struct P { x: i32 }
  |
```

**案例 5：枚举变体名写错（`E0599`）**

```text
error[E0599]: no variant, associated function, or constant named `Triangle` found for enum `Shape` in the current scope
 --> src/main.rs:2:28
  |
1 | enum Shape { Circle, Square }
  | ---------- variant, associated function, or constant `Triangle` not found for this enum
2 | fn main() { let s = Shape::Triangle; println!("{s:?}"); }
  |                            ^^^^^^^^ variant, associated function, or constant not found in `Shape`
```

**案例 6：`match` 漏了分支（`E0004`）**

```text
error[E0004]: non-exhaustive patterns: `Shape::Rect(_, _)` not covered
 --> src/main.rs:3:11
  |
3 |     match s {
  |           ^ pattern `Shape::Rect(_, _)` not covered
  |
note: `Shape` defined here
 --> src/main.rs:1:6
  |
1 | enum Shape { Circle(f64), Rect(f64, f64) }
  |      ^^^^^                ---- not covered
  = note: the matched value is of type `Shape`
help: ensure that all possible cases are being handled by adding a match arm with a wildcard pattern or an explicit pattern as shown
  |
4 ~         Shape::Circle(r) => 3.14 * r * r,
5 ~         Shape::Rect(_, _) => todo!(),
  |
```

`help` 直接把缺的那一行写好了，照着补就行。

**案例 7：把 `Option` 当成里面的值用（`E0308`）**

```rust
let maybe = Some(5);
let n: i32 = maybe;
```

```text
error[E0308]: mismatched types
 --> src/main.rs:3:18
  |
3 |     let n: i32 = maybe;
  |            ---   ^^^^^ expected `i32`, found `Option<{integer}>`
  |            |
  |            expected due to this
  |
  = note: expected type `i32`
             found enum `Option<{integer}>`
help: consider using `Option::expect` to unwrap the `Option<{integer}>` value, panicking if the value is an `Option::None`
  |
3 |     let n: i32 = maybe.expect("REASON");
  |                       +++++++++++++++++
```

`Option<i32>` 和 `i32` 是两个不同的类型，必须先「打开」。`help` 建议的 `expect` 会 panic；更稳的写法是 `match` 或 `unwrap_or`。

**案例 8：对 `Option` 调用里面那个类型的方法（`E0599`）**

```text
error[E0599]: no method named `push` found for enum `Option<T>` in the current scope
 --> src/main.rs:3:11
  |
3 |     maybe.push(1);
  |           ^^^^ method not found in `Option<{integer}>`
```

顺带一个更迷惑的版本：写 `maybe.len()` 会报 `E0624: method 'len' is private`——`Option` 内部确实有个没公开的 `len`，所以报的不是「找不到方法」而是「方法私有」。两个报错的解法一样：先取出里面的值，再调用它的方法。

**案例 9：`let` 里用了可能不匹配的模式（`E0005`）**

```text
error[E0005]: refutable pattern in local binding
 --> src/main.rs:2:9
  |
2 |     let Some(x) = Some(5);
  |         ^^^^^^^ pattern `None` not covered
  |
  = note: `let` bindings require an "irrefutable pattern", like a `struct` or an `enum` with only one variant
  = note: for more information, visit https://doc.rust-lang.org/book/ch19-02-refutability.html
  = note: the matched value is of type `Option<i32>`
help: you might want to use `let...else` to handle the variant that isn't matched
  |
2 |     let Some(x) = Some(5) else { todo!() };
  |                           ++++++++++++++++
```

`let` 要求模式**一定能匹配**，`Some(x)` 有可能碰上 `None`，所以不行；`help` 给出的 `let ... else` 正是 6.12 讲的写法。

**案例 10：`self` 方法把值消费掉之后再用（`E0382`）**

```text
error[E0382]: use of moved value: `s`
 --> src/main.rs:6:13
  |
4 |     let s = S { name: String::from("x") };
  |         - move occurs because `s` has type `S`, which does not implement the `Copy` trait
5 |     let a = s.into_name();
  |               ----------- `s` moved due to this method call
6 |     let b = s.into_name();
  |             ^ value used here after move
  |
note: `S::into_name` takes ownership of the receiver `self`, which moves `s`
 --> src/main.rs:2:23
  |
2 | impl S { fn into_name(self) -> String { self.name } }
  |                       ^^^^
```

关键提示在 `note` 那行：`takes ownership of the receiver 'self'`。看到它就知道这个方法签名是 `self` 而不是 `&self`。

## 6.15 常见坑速查

| 现象 | 原因 | 解法 |
| --- | --- | --- |
| `missing field ... in initializer` | 创建结构体时少了字段 | 补全字段，或用 `..other` 复用 |
| `no field x on type Y` | 字段名拼错，或访问了私有的 | 看 `note` 里的可用字段列表 |
| `{:?}` 报 `doesn't implement Debug` | 结构体/枚举没 `derive(Debug)` | 加上 `#[derive(Debug)]` |
| `==` 报 `cannot be applied to type` | 没实现 `PartialEq` | 加上 `#[derive(PartialEq)]` |
| `match` 报 `non-exhaustive patterns` | 漏了变体 | 补分支，或显式用 `_` 兜底 |
| `method not found in Option` | `Option` 本身没有内部类型的方法 | 先 `match` / `unwrap_or` / `if let` 取出值 |
| `expected i32, found Option<i32>` | 忘了处理「可能没有」 | `unwrap_or`、`match` 或 `let ... else` |
| `let` 解构报 `refutable pattern` | 模式可能匹配不上 | 用 `if let`、`let ... else` 或 `match` |
| 方法调用后原变量不能用了 | 方法签名是 `self`，消费了值 | 改成 `&self`，或调整调用顺序 |
| 改了字段但结构体没变 | 方法签名是 `&self`，不是 `&mut self` | 改成 `&mut self`，变量声明成 `let mut` |
| 新增枚举变体后一堆编译错误 | `match` 穷尽检查生效了 | 按提示补上每个 `match` 的分支 |
| 结构体想放进 `HashMap` 当键 | 缺 `Hash` + `Eq` | `#[derive(Hash, PartialEq, Eq)]`（第 7 章） |

## 6.16 练习

1. 定义 `struct User { name: String, age: u32, email: Option<String> }`，给它加 `#[derive(Debug, Clone, PartialEq)]`，写关联函数 `new(name: &str, age: u32) -> Self`，以及方法 `greeting(&self) -> String`（有邮箱时带上邮箱，没有就用一句普通问候）。
2. 定义 `enum Direction { North, East, South, West }`，实现 `fn turn_right(&self) -> Self` 和 `fn name(&self) -> &'static str`，用循环打印「连续右转四次回到原点」。
3. 定义 `enum Payment { Cash(f64), Card { amount: f64, last4: String } }`，实现 `fn describe(&self) -> String`，用 `match` 分别输出两种支付方式的描述。
4. 给 `struct User` 写 `fn find_user<'a>(users: &'a [User], name: &str) -> Option<&'a User>`，分别用 `match` 和 `if let` 处理返回值。（提示：这里必须手写生命周期 `<'a>`，第 10 章会解释为什么；也可以先只写 `name` 单一输入的版本 `fn find_by_name(user: &User, name: &str) -> bool` 对比一下。）
5. 给 `Rectangle` 加方法 `fn can_hold(&self, other: &Rectangle) -> bool`，判断自己能否装下另一个矩形（宽和高都要更大）。

（第 1、2 题是 6.3 和 6.7 代码的直接变体，这里不再单独给答案。）

### 参考答案

::: details 第 3 题

```rust
#[derive(Debug)]
enum Payment {
    Cash(f64),
    Card { amount: f64, last4: String },
}

impl Payment {
    fn describe(&self) -> String {
        match self {
            Payment::Cash(amount) => format!("现金 {amount:.2} 元"),
            Payment::Card { amount, last4 } => {
                format!("尾号 {last4} 的卡支付 {amount:.2} 元")
            }
        }
    }
}

fn main() {
    let payments = [
        Payment::Cash(12.5),
        Payment::Card { amount: 88.0, last4: String::from("1234") },
    ];
    for payment in &payments {
        println!("{}", payment.describe());
    }
}
```

实测输出 `现金 12.50 元` 和 `尾号 1234 的卡支付 88.00 元`。注意 `match self` 时 `amount`、`last4` 都是引用，`format!` 会自己处理。

:::

::: details 第 4 题

```rust
#[derive(Debug)]
struct User {
    name: String,
    age: u32,
}

fn find_user<'a>(users: &'a [User], name: &str) -> Option<&'a User> {
    for user in users {
        if user.name == name {
            return Some(user);
        }
    }
    None
}

fn main() {
    let users = vec![
        User { name: String::from("ada"), age: 36 },
        User { name: String::from("linus"), age: 54 },
    ];

    match find_user(&users, "ada") {
        Some(user) => println!("找到 {}（{} 岁）", user.name, user.age),
        None => println!("没找到"),
    }

    if let Some(user) = find_user(&users, "grace") {
        println!("{}", user.name);
    } else {
        println!("grace 不在名单里");
    }
}
```

实测第一段输出 `找到 ada（36 岁）`，第二段输出 `grace 不在名单里`。

这里的 `<'a>` 是必须的：有两个输入引用，编译器无法判断返回值借的是哪个（就是第 5 章 5.11 案例 9 的 `E0106`）。标注 `&'a [User]` 和 `Option<&'a User>` 之后，意思就明确了——**返回的引用活不过 `users`**。

:::

::: details 第 5 题

```rust
#[derive(Debug)]
struct Rectangle {
    width: f64,
    height: f64,
}

impl Rectangle {
    fn can_hold(&self, other: &Rectangle) -> bool {
        self.width > other.width && self.height > other.height
    }
}

fn main() {
    let big = Rectangle { width: 10.0, height: 8.0 };
    let small = Rectangle { width: 3.0, height: 2.0 };
    println!("big 装得下 small：{}", big.can_hold(&small));
    println!("small 装得下 big：{}", small.can_hold(&big));
}
```

实测输出 `true` / `false`。两个参数都是 `&Rectangle`，谁的所有权都没被拿走——这也是方法最常见的形态：`&self` 加 `&其他`。

:::

## 6.17 小结

- 结构体把相关字段打包，创建时必须初始化所有字段；字段名和变量名相同时可以简写，`..other` 用来复用其余字段。

- `impl` 块里放关联函数（没有 `self`，如 `new`）和方法（有 `self`）；`Self` 指当前类型。

- `&self` 只读、`&mut self` 可改、`self` 消费掉值；选择方法和第 5 章的借用规则完全一致，`into_*` 通常就是消费型的。

- 元组结构体省掉字段名，单元结构体没有字段；它们能给同一个底层类型加上类型区分。

- `#[derive(Debug, Clone, PartialEq, ...)]` 是最常用的自动实现；`Debug` 给开发者看，`Display` 给用户看，后者要手写 `impl`。

- 枚举的变体可以带不同数据，天然表达「多选一」，非法状态写不出来。

- `match` 必须穷尽所有情况，这是特性不是负担：新增变体时编译器会点名所有需要修改的地方。

- `match` 支持字面量、范围、`|`、`_`、guard（`if` 条件）和 `@` 绑定；它本身是表达式，可以直接赋值。

- `Option<T>` 把「可能没有」写进类型；用 `match`、`if let`、`let ... else`、`unwrap_or` 处理，`get()` 比下标安全。

- 结构体 + 枚举 + `impl` + `match` 是 Rust 建模的标准姿势，状态机是最典型的例子。

下一章讲**常用集合**：`Vec`、`String`、`HashMap` 的日常用法、性能特征和坑，所有权规则在集合上会体现得特别明显。
