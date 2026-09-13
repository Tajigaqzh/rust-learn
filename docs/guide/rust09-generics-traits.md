# 第 9 章 · 泛型与 trait

前面几章一直在和两个东西打交道：`Vec<T>` 里的 `T`（泛型），和 `Display`、`Error`、`Hash`、`Ord`、`From` 这些名字（trait）。这一章把它们讲清楚。

一句话概括：

- **泛型**解决「同一套逻辑，适用于多种类型」——写一遍代码，编译器为每种具体类型各生成一份。
- **trait** 解决「同一套逻辑，依赖某些能力」——`T: Display` 的意思是「我不关心 `T` 是什么，但它必须能格式化打印」。

两者合起来，就是 Rust 里「抽象」的全部基础。第 11 章的迭代器、第 12 章的智能指针、第 13 章的并发，都是这套机制的延伸。

本章配套代码在 `src/rust09_generics_traits/mod.rs`，执行 `cargo run` 可以看到全部输出。

## 9.1 从重复代码到泛型

想写一个「找最大值」的函数，不给泛型就得每种类型写一份：

```rust
fn largest_i32(list: &[i32]) -> i32 { /* ... */ }
fn largest_char(list: &[char]) -> char { /* ... */ }
fn largest_str(list: &[&str]) -> &str { /* ... */ }
```

三份代码除了类型标注，逻辑一模一样。泛型把它压成一份：

```rust
fn largest<T: PartialOrd + Copy>(list: &[T]) -> T {
    let mut largest = list[0];
    for &item in list {
        if item > largest {
            largest = item;
        }
    }
    largest
}
```

实测它对三种类型都管用：

```text
    largest(&[3, 9, 4]) = 9
    largest(&['a', 'z', 'm']) = z
    largest(&["apple", "pear", "fig"]) = pear
```

`<T: PartialOrd + Copy>` 是**约束**（bound），意思是「`T` 可以是任何类型，前提是它能比较大小（`PartialOrd`）并且能按值复制（`Copy`）」。函数体里用到的 `>` 和 `&item` 分别需要这两个能力，所以必须写上。

如果不写会怎样？编译器会指着出问题的运算符给你报错，并且**直接告诉你该加哪个约束**（见 9.15 案例 1、2）。

## 9.2 泛型结构体与泛型方法

结构体的字段类型也可以泛化（第 6 章的结构体加上类型参数）：

```rust
#[derive(Debug, Clone, Copy, PartialEq)]
struct Point<T> {
    x: T,
    y: T,
}

let int_point = Point { x: 3, y: 4 };       // Point<i32>
let float_point = Point { x: 1.5, y: 2.5 }; // Point<f64>
```

注意 `x` 和 `y` 用的是**同一个** `T`，所以 `Point { x: 1, y: 2.5 }` 编不过——想混用就得写 `Point<T, U>`，声明两个类型参数。

给泛型结构体加方法时，`impl` 上要带上类型参数：

```rust
impl<T: fmt::Display> Point<T> {
    fn show(&self) -> String {
        format!("({}, {})", self.x, self.y)
    }
}
```

`impl<T: fmt::Display>` 读作「对**任意**满足 `Display` 的 `T`，给 `Point<T>` 实现这些方法」。约束写在 `impl` 上，块里所有方法都共享它。

实测 `int_point = (3, 4)`、`float_point = (1.5, 2.5)`——一份代码，两种类型。

`derive` 也是同理：`#[derive(Debug, Clone, Copy, PartialEq)]` 会生成「`T` 满足条件时才成立」的实现，`Point<i32>` 有 `Copy`，而 `Point<String>` 没有（因为 `String` 不是 `Copy`）。

## 9.3 只为特定类型实现方法

`impl` 上的类型参数可以写具体类型，这样方法就只有那种类型才有：

```rust
impl Point<f64> {
    fn mixup(&self, other: Point<f64>) -> Point<f64> {
        Point { x: self.x, y: other.y }
    }
}
```

实测 `mixup 之后 = (1.5, 9)`——`x` 取自自己，`y` 取自参数。而 `int_point` 上**没有** `mixup` 方法，因为 `Point<i32>` 不匹配 `Point<f64>`。

这个能力在标准库和第三方库里很常用：`Vec<u8>` 有 `extend_from_slice`，别的 `Vec<T>` 没有；`String` 有 `push_str`，`Vec<u8>` 没有。**「只在对的类型上开放方法」是编译期就能保证的类型安全**。

## 9.4 trait：定义共享行为

trait 描述的是一组行为：**任何实现它的类型，都能被当作「拥有这种能力的东西」来使用**。

```rust
trait Summary {
    fn summarize(&self) -> String;
}

struct NewsArticle {
    title: String,
    author: String,
}

impl Summary for NewsArticle {
    fn summarize(&self) -> String {
        format!("《{}》，作者 {}", self.title, self.author)
    }
}
```

`impl Summary for NewsArticle` 是对编译器的承诺：这个类型提供了 `summarize` 这个方法。看起来像第 6 章的 `impl`，区别在于**这次实现的是「契约」**：trait 规定了方法名、参数和返回类型，实现者只负责填内容。

实测两种类型各自的输出：

```text
    article.summarize() = 《Rust 所有权》，作者 Ada
    tweet.summarize()   = @rustlang: 所有权规则很好用，借用检查器帮我抓了个 bug
```

一个类型可以实现多个 trait；一个 trait 也可以被很多类型实现。第 1 章手写的 `impl Display for Point`、第 8 章手写的 `impl Error for ConfigError`，用的都是这个语法。

## 9.5 默认方法与覆盖

trait 里的方法可以有默认实现，实现者不写就用默认的：

```rust
trait Summary {
    fn summarize(&self) -> String;

    fn preview(&self) -> String {
        format!("[摘要] {}", self.summarize())
    }
}
```

`preview` 依赖了同 trait 里的 `summarize`——**默认方法可以调用其他方法**，哪怕那些方法还没被实现。这样实现者只需要写出核心逻辑（`summarize`），附带能力（`preview`）就自动有了。

想改掉默认行为，就在 `impl` 里重写同名方法：

```rust
impl Summary for Tweet {
    fn summarize(&self) -> String {
        format!("@{}: {}", self.user, self.content)
    }

    fn preview(&self) -> String {          // 覆盖默认实现
        let text: String = self.content.chars().take(12).collect();
        format!("[推文] {text}...")
    }
}
```

实测对比：

```text
    article.preview()   = [摘要] 《Rust 所有权》，作者 Ada
    tweet.preview()     = [推文] 所有权规则很好用，借用检...
```

`NewsArticle` 用了默认实现，`Tweet` 覆盖了它。标准库的 `Iterator`、`Error`、`Default` 都是这种「一个必写方法 + 一堆默认方法」的设计。

## 9.6 三种约束写法

「要求某类型实现某 trait」有三种写法，作用相同，看场景选：

```rust
// 1. impl Trait：最短，适合只有一个泛型参数
fn notify(item: &impl Summary) -> String {
    format!("通知：{}", item.preview())
}

// 2. 类型参数 + 冒号：要写多个约束、或者返回值里要用到 T 时用这个
fn notify_all<T: Summary>(items: &[T]) -> Vec<String> { /* ... */ }

// 3. where 子句：约束多、类型参数多时最清楚
fn describe_pair<A, B>(a: &A, b: &B) -> String
where
    A: Summary,
    B: fmt::Display,
{
    format!("{} / {}", a.preview(), b)
}
```

`impl Trait` 其实是语法糖：`item: &impl Summary` 等价于 `item: &T` 加 `T: Summary`。但它在**返回值位置**的含义不同（下一节）。

多个约束可以用 `+` 连起来：

```rust
fn show_it<T: Summary + fmt::Display>(value: &T) { }
fn show_it_too<T>(value: &T) where T: Summary + fmt::Display { }
```

实测三种写法都能正常工作：

```text
    notify(&article) = 通知：[摘要] 《Rust 所有权》，作者 Ada
    notify_all -> [摘要] 《所有权》，作者 Ada
    notify_all -> [摘要] 《借用检查器》，作者 Grace
    describe_pair(&article, &42) = [摘要] 《Rust 所有权》，作者 Ada / 42
```

注意 `notify_all` 收的是 `&[T]`——**同一个切片里只能是一种类型**。想装不同类型，得用下一节的 trait 对象。

## 9.7 返回 `impl Trait` 的限制

把 `impl Trait` 写在返回类型上，意思是「我返回某个实现了 `Summary` 的类型，但具体是哪个先不告诉你」：

```rust
fn make_article() -> impl Summary {
    NewsArticle {
        title: String::from("Rust 所有权"),
        author: String::from("Ada"),
    }
}
```

实测 `make_article().preview() = [摘要] 《Rust 所有权》，作者 Ada`。调用方只能使用 `Summary` 里的方法，看不到 `NewsArticle` 的字段——这是一种**有意隐藏实现细节**的手段。

限制是：**一个函数只能返回一种具体类型**。下面这段编不过：

```rust
fn make(flag: bool) -> impl Greet {
    if flag { A } else { B }      // A 和 B 是两个不同的类型
}
```

报错是 `E0308: if and else have incompatible types`，`help` 会建议改成 `Box<dyn Greet>`（见 9.15 案例 7）。原因是 `impl Trait` 在编译期就绑定到一个具体类型，编译器不能让它随运行时的分支变化。

## 9.8 trait 对象与 `dyn`

要在一个容器里装**多种类型**，就得用 trait 对象：

```rust
let items: Vec<Box<dyn Summary>> = vec![
    Box::new(NewsArticle { /* ... */ }),
    Box::new(Tweet { /* ... */ }),
];

fn print_all(items: &[Box<dyn Summary>]) {
    for item in items {
        println!("{}", item.summarize());
    }
}
```

实测：

```text
    《泛型》，作者 Linus
    @ada: trait 对象可以混装
```

`dyn Summary` 读作「某个实现了 `Summary` 的类型，具体是哪个运行时才知道」。`Box<dyn Summary>` 是指向它的智能指针（第 12 章讲 `Box`）；也可以借用：`&dyn Summary`。

三个关键点：

1. **`dyn` trait 需要「能建虚表」**。像 `fn create() -> Self` 这种「返回 `Self`」或者带泛型参数的方法，没法确定具体类型，会让 trait 变得不能当对象用，报 `E0038`（见 9.15 案例 3）。修法是给方法加 `where Self: Sized`，把它排除在虚表之外。
2. **调用方法是动态分发**：编译期不知道具体类型，运行时通过虚表（vtable）找到真正的方法。多一次间接跳转，但换来灵活性。
3. **标准库的 `Box<dyn Error>` 就是 trait 对象**。第 8 章用它统一了各种错误类型，代价正是「错误类型被擦除、没法 `match`」。

## 9.9 静态分发 vs 动态分发

两种写法都能实现多态，代价不同：

| | 泛型（静态分发） | `dyn`（动态分发） |
| --- | --- | --- |
| 什么时候确定类型 | 编译期 | 运行时 |
| 调用开销 | 无（可以内联） | 一次虚表跳转 |
| 二进制体积 | 每种类型生成一份代码，可能更大 | 只有一份 |
| 能否混装不同类型 | 不能 | 能 |
| 典型场景 | 算法、容器（`Vec<T>`、`sort`） | 插件式结构、`Box<dyn Error>`、异构集合 |

给泛型函数的每种具体类型各生成一份代码，这个过程叫**单态化**（monomorphization）：`largest::<i32>` 和 `largest::<char>` 在编译后是两个独立的函数，所以调用它和手写一个 `i32` 版本一样快。代价是编译时间变长、二进制变大。

选择的经验法则：**编译期能确定类型就优先泛型**（零开销）；只有当「运行时才知道要放什么类型」「一个容器要装多种类型」时才用 `dyn`。

## 9.10 关联类型

trait 里可以先留一个「类型占位符」，等实现时才确定：

```rust
trait Container {
    type Item;                                  // 关联类型
    fn first(&self) -> Option<&Self::Item>;
    fn len(&self) -> usize;
}

impl<T> Container for Vec<T> {
    type Item = T;                              // 这里才定下来
    fn first(&self) -> Option<&T> { self.as_slice().first() }
    fn len(&self) -> usize { Vec::len(self) }
}
```

实测 `Container::first(&numbers) = Some(10)`、`Container::len(&numbers) = 3`、空 `Vec` 的 `first = None`。

（注意要用 `Container::first(&numbers)` 这种写法：`numbers.first()` 会优先命中 `Vec` 自带的方法。）

**关联类型和泛型参数的区别**，看标准库最清楚：

```rust
trait Iterator {
    type Item;                       // 关联类型
    fn next(&mut self) -> Option<Self::Item>;
}
```

`Iterator` 用关联类型，是因为「一个迭代器只能产出一种元素」——`vec![1,2,3].iter()` 的 `Item` 就是 `&i32`，唯一确定。如果用泛型参数写成 `trait Iterator<Item>`，同一个类型就能有 `Iterator<i32>`、`Iterator<String>` 等多份实现，`for` 循环就不知道该用哪一份了。

反过来，`HashMap<K, V>` 用泛型参数，正是因为「同一个 `HashMap` 类型可以是任意键值组合」。**一句话判断：实现是不是唯一的？唯一就用关联类型，可以有多种就用泛型参数。**

关联类型也是 `dyn Iterator` 必须补上 `Item` 的原因——虚表里得知道最终类型是什么（见 9.15 案例 6）。

## 9.11 运算符重载

Rust 的运算符都是 trait 的语法糖：`a + b` 就是 `Add::add(a, b)`。所以给自定义类型实现 `Add`，它就能用 `+`：

```rust
use std::ops::Add;

impl<T: Add<Output = T>> Add for Point<T> {
    type Output = Point<T>;                 // 加法产出的类型

    fn add(self, other: Point<T>) -> Point<T> {
        Point { x: self.x + other.x, y: self.y + other.y }
    }
}
```

实测 `int_point + Point { 10, 20 } = (13, 24)`。

其他运算符同理：`-` 是 `Sub`、`*` 是 `Mul`、`==` 是 `PartialEq`、`[]` 是 `Index`、`+=` 是 `AddAssign`。一般优先用 `derive`（`PartialEq`、`PartialOrd`），需要自定义语义时才手写。

注意 `+` 默认会**按值取走**两个操作数（第 7 章 `String + &str` 的谜题就在这儿）。想让 `+` 不移动，就给引用类型实现，或者改用 `AddAssign`（`+=`）。

## 9.12 孤儿规则与一致性

实现 trait 有一条硬性限制，叫**孤儿规则**：`impl Trait for Type` 里，**trait 和类型至少有一个是你这个 crate 里定义的**。

```rust
// 都不行：trait（Display）和类型（Vec）都是外部的
impl std::fmt::Display for Vec<i32> { }
// 行：trait 是本地的
impl MyTrait for Vec<i32> { }
// 行：类型是本地的
impl std::fmt::Display for MyType { }
```

违反时报 `E0117`（见 9.15 案例 4）。这条规则的目的是**保证一致性**：如果谁都能给 `Vec<i32>` 加一个 `Display` 实现，那么两个 crate 各自加一份，下游代码就无从判断该用哪个了。

实在需要给外部类型加外部 trait 的方法，标准做法是**新类型包装**（newtype）：

```rust
struct MyVec(Vec<i32>);       // 自己的类型

impl std::fmt::Display for MyVec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.0)
    }
}
```

另外，**同一个类型不能对同一个 trait 实现两次**，否则报 `E0119`（见 9.15 案例 5）。这也是「一致性」的另一面：实现必须唯一。

## 9.13 supertrait：trait 之间的依赖

trait 可以要求「实现我的类型，必须也实现别的 trait」：

```rust
trait MyError: std::fmt::Debug + std::fmt::Display {
    fn code(&self) -> u32;
}
```

冒号后面的部分叫 **supertrait**。第 8 章实现 `Error` 时看到的报错里写着 `pub trait Error: Debug + Display`，意思就是「想实现 `Error`，先得实现 `Debug` 和 `Display`」。标准库这么设计，是因为错误类型既要能 `{}` 打印给用户看，也要能 `{:?}` 打印给开发者看。

trait 也可以给别的 trait 提供默认方法，让实现者自动获得一批能力：

```rust
trait Named {
    fn name(&self) -> String;

    fn greet(&self) -> String {
        format!("你好，我是 {}", self.name())
    }
}
```

## 9.14 常用标准 trait 一览

这一章出现的东西，其实就是反复出现的这批 trait：

| trait | 能力 | 怎么获得 |
| --- | --- | --- |
| `Debug` | `{:?}` 打印 | `#[derive(Debug)]` |
| `Display` | `{}` 打印 | 手写 `impl` |
| `Clone` / `Copy` | 复制 | `derive`（`Copy` 需要所有字段都 `Copy`） |
| `PartialEq` / `Eq` | `==`、去重 | `derive` |
| `PartialOrd` / `Ord` | `<`、`sort()` | `derive`（浮点只有 `PartialOrd`） |
| `Hash` | 当作 `HashMap` 的键 | `derive` |
| `Default` | `Type::default()` | `derive` |
| `From` / `Into` | `?` 转换、`.into()` | 手写 `From`（`Into` 自动获得） |
| `Iterator` | `for` 循环、迭代器适配器 | 手写 `next`（第 11 章） |
| `Error` | 错误类型 | `Debug` + `Display` + `impl Error` |
| `Drop` | 离开作用域时的清理 | 手写（第 5 章） |
| `Deref` | 自动解引用（`&String` → `&str`） | 手写（第 12 章） |

看别人的代码时，**约束列表就是一份能力清单**：`fn process<T: Read + Seek>(...)` 告诉你「这个函数需要能读、能跳转的东西」，不用看实现就知道该传什么。

## 9.15 七个真实报错怎么读

**案例 1：泛型里用了没约束的能力（`E0277`）**

```rust
fn print_it<T>(value: T) {
    println!("{value}");
}
```

```text
error[E0277]: `T` doesn't implement `std::fmt::Display`
 --> src/main.rs:2:15
  |
2 |     println!("{value}");
  |               ^^^^^^^ `T` cannot be formatted with the default formatter
  |
  = note: in format strings you may be able to use `{:?}` (or {:#?} for pretty-print) instead
help: consider restricting type parameter `T` with trait `Display`
  |
1 | fn print_it<T: std::fmt::Display>(value: T) {
  |              ++++++++++++++++++++
```

**案例 2：泛型里用了 `>`（`E0369`）**

```text
error[E0369]: binary operation `>` cannot be applied to type `&T`
 --> src/main.rs:4:17
  |
4 |         if item > largest {
  |            ---- ^ ------- &T
  |            |
  |            &T
  |
help: consider restricting type parameter `T` with trait `PartialOrd`
  |
1 | fn largest<T: std::cmp::PartialOrd>(list: &[T]) -> &T {
  |             ++++++++++++++++++++++
```

这两个案例值得放在一起看：**泛型函数里能做什么，完全由约束决定**。编译器不但告诉你缺什么，还把该加的约束写好了。

**案例 3：trait 不能当对象用（`E0038`）**

```rust
trait NotObjectSafe {
    fn create() -> Self;
}

fn make() -> Box<dyn NotObjectSafe> { todo!() }
```

```text
error[E0038]: the trait `NotObjectSafe` is not dyn compatible
 --> src/main.rs:5:18
  |
5 | fn make() -> Box<dyn NotObjectSafe> {
  |                  ^^^^^^^^^^^^^^^^^ `NotObjectSafe` is not dyn compatible
  |
note: for a trait to be dyn compatible it needs to allow building a vtable
 --> src/main.rs:2:8
  |
1 | trait NotObjectSafe {
  |       ------------- this trait is not dyn compatible...
2 |     fn create() -> Self;
  |        ^^^^^^ ...because associated function `create` has no `self` parameter
help: consider turning `create` into a method by giving it a `&self` argument
  |
2 |     fn create(&self) -> Self;
  |               +++++
help: alternatively, consider constraining `create` so it does not apply to trait objects
  |
2 |     fn create() -> Self where Self: Sized;
  |                         +++++++++++++++++
```

原因写得很清楚：虚表里放的是「对某个具体类型的指针」，而 `create() -> Self` 连 `self` 都没有，也没有具体类型可返回。两种修法都给了：改成 `&self` 方法，或者加 `where Self: Sized` 把它排除在对象之外。

**案例 4：孤儿规则（`E0117`）**

```text
error[E0117]: only traits defined in the current crate can be implemented for types defined outside of the crate
 --> src/main.rs:3:1
  |
3 | impl fmt::Display for Vec<i32> {
  | ^^^^^^^^^^^^^^^^^^^^^^--------
  |                       |
  |                       `Vec` is not defined in the current crate
  |
  = note: define and implement a trait or new type instead
```

`note` 里说的 "new type" 就是 9.12 的新类型包装。

**案例 5：重复实现同一个 trait（`E0119`）**

```text
error[E0119]: conflicting implementations of trait `Greet` for type `i32`
 --> src/main.rs:4:1
  |
3 | impl Greet for i32 { fn hi(&self) {} }
  | ------------------ first implementation here
4 | impl Greet for i32 { fn hi(&self) {} }
  | ^^^^^^^^^^^^^^^^^^ conflicting implementation for `i32`
```

**案例 6：`dyn` 少了关联类型（`E0191`）**

```text
error[E0191]: the value of the associated type `Item` in `Iterator` must be specified
 --> src/main.rs:1:21
  |
1 | fn count(iter: &dyn Iterator) -> usize {
  |                     ^^^^^^^^
  |
help: specify the associated type
  |
1 | fn count(iter: &dyn Iterator<Item = /* Type */>) -> usize {
  |                             +++++++++++++++++++
```

**案例 7：`impl Trait` 返回了两种类型（`E0308`）**

```text
error[E0308]: `if` and `else` have incompatible types
 --> src/main.rs:9:26
  |
9 |     if flag { A } else { B }
  |               -          ^ expected `A`, found `B`
  |
help: you could change the return type to be a boxed trait object
  |
8 - fn make(flag: bool) -> impl Greet {
8 + fn make(flag: bool) -> Box<dyn Greet> {
  |
help: if you change the return type to expect trait objects, box the returned expressions
  |
9 |     if flag { Box::new(A) } else { Box::new(B) }
  |               +++++++++ +          +++++++++ +
```

## 9.16 常见坑速查

| 现象 | 原因 | 解法 |
| --- | --- | --- |
| `T doesn't implement Display` | 泛型函数里用了没写进约束的能力 | 加 `T: Display`（编译器会告诉你加哪个） |
| `binary operation > cannot be applied to &T` | 缺 `PartialOrd` 约束 | `T: PartialOrd`（要按值取用再加 `Copy`） |
| `the trait ... is not dyn compatible` | 方法没有 `self`、返回 `Self` 或带泛型参数 | 加 `&self`，或 `where Self: Sized` |
| 想给 `Vec<i32>` 实现 `Display` 报 `E0117` | 孤儿规则 | 用新类型包装 |
| 同一个 trait 实现两次报 `E0119` | 实现必须唯一 | 删掉重复的 `impl`，或合并逻辑 |
| `dyn Iterator` 报缺少 `Item` | 关联类型必须写出来 | `dyn Iterator<Item = i32>` |
| `impl Trait` 里 `if/else` 返回不同类型 | 一个函数只能返回一种具体类型 | 改成 `Box<dyn Trait>` |
| `Vec<Box<dyn Trait>>` 里要取具体类型的字段 | trait 对象只有 trait 里的方法 | 把需要的方法加到 trait 上，或做向下转型 |
| `numbers.first()` 想调 trait 的方法却调到了 `Vec` 的 | 固有方法优先于 trait 方法 | 写成 `Container::first(&numbers)` |
| 泛型编译变慢、二进制变大 | 单态化为每种类型生成代码 | 类型种类多到失控时改用 `dyn` |
| 关联类型和泛型参数分不清 | 两者语义不同 | 问「实现唯一吗」：唯一用关联类型，否则用泛型 |

## 9.17 练习

1. 写泛型函数 `fn count_greater<T: PartialOrd>(values: &[T], threshold: &T) -> usize`，统计严格大于阈值的元素个数；用 `i32` 和 `&str` 各验证一次。
2. 定义 `trait Area { fn area(&self) -> f64; }`，给它加一个默认方法 `fn describe(&self) -> String`（输出「面积 x.xx」），然后用 `Circle { radius }` 和 `Rect { width, height }` 两个结构体实现它。
3. 写 `fn total_area(shapes: &[Box<dyn Area>]) -> f64`，把一组面积加起来；再用一个 `Vec<Box<dyn Area>>` 同时装圆和矩形验证。
4. 给 9.11 节的 `Point<T>` 实现 `std::ops::Sub`，让两个 `Point` 可以相减；再想一想：为什么 `Add` 的实现里要写 `type Output = Point<T>`。
5. 定义带关联类型的 trait `First { type Item; fn first(&self) -> Option<&Self::Item>; }`，为 `Vec<T>` 实现它，并说明为什么这里用关联类型而不是泛型参数。

（第 4 题是 9.11 节运算符重载的直接变体，这里不再单独给答案。）

### 参考答案

::: details 第 1 题

```rust
fn count_greater<T: PartialOrd>(values: &[T], threshold: &T) -> usize {
    let mut count = 0;
    for value in values {
        if value > threshold {
            count += 1;
        }
    }
    count
}

fn main() {
    println!("{}", count_greater(&[1, 5, 9, 3], &4));      // 2
    println!("{}", count_greater(&["apple", "pear"], &"fig")); // 1
}
```

这里只需要 `PartialOrd`，不需要 `Copy`：`for value in values` 迭代出的是 `&T`，比较两个引用不需要把值复制出来——和 9.1 的 `largest` 对比一下，后者要把「当前最大值」存在变量里，所以多了 `Copy` 约束。**约束应该刚好够用，多加会限制调用方**。

:::

::: details 第 3 题

（第 2 题的 trait 定义和两个实现也在这里。）

```rust
trait Area {
    fn area(&self) -> f64;

    fn describe(&self) -> String {
        format!("面积 {:.2}", self.area())
    }
}

struct Circle {
    radius: f64,
}

struct Rect {
    width: f64,
    height: f64,
}

impl Area for Circle {
    fn area(&self) -> f64 {
        std::f64::consts::PI * self.radius * self.radius
    }
}

impl Area for Rect {
    fn area(&self) -> f64 {
        self.width * self.height
    }
}

fn total_area(shapes: &[Box<dyn Area>]) -> f64 {
    let mut total = 0.0;
    for shape in shapes {
        total += shape.area();
    }
    total
}

fn main() {
    let shapes: Vec<Box<dyn Area>> = vec![
        Box::new(Circle { radius: 1.0 }),
        Box::new(Rect { width: 3.0, height: 4.0 }),
    ];
    for shape in &shapes {
        println!("{}", shape.describe());   // 面积 3.14 / 面积 12.00
    }
    println!("total = {:.4}", total_area(&shapes)); // 15.1416
}
```

两个点：`Circle` 和 `Rect` 是不同类型，只有 `Box<dyn Area>` 能把它们放进同一个 `Vec`；`total_area` 里调用的 `shape.area()` 就是动态分发——运行时才知道是哪种形状。

:::

::: details 第 5 题

```rust
trait First {
    type Item;
    fn first(&self) -> Option<&Self::Item>;
}

impl<T> First for Vec<T> {
    type Item = T;

    fn first(&self) -> Option<&T> {
        self.as_slice().first()
    }
}

fn main() {
    let numbers = vec![3, 1, 4];
    println!("{:?}", First::first(&numbers));   // Some(3)
    let words = vec![String::from("a"), String::from("b")];
    println!("{:?}", First::first(&words));     // Some("a")
}
```

用关联类型而不是泛型参数，是因为**一个 `Vec<T>` 的「第一个元素类型」是唯一确定的**：`Vec<i32>` 的 `Item` 只能是 `i32`。如果写成 `trait First<Item>`，理论上可以给同一个 `Vec<i32>` 实现 `First<i32>` 和 `First<String>` 两份，调用时反而要额外说明用哪一份。

另外注意 `First::first(&numbers)` 的写法：`numbers.first()` 会命中 `Vec` 自带的固有方法，想调 trait 里的必须显式写出来。

:::

## 9.18 小结

- 泛型让「同一套逻辑适用于多种类型」；编译器为每种具体类型各生成一份代码，这个过程叫单态化，运行时零开销。

- 泛型函数里能做什么，完全由 **trait bound** 决定；缺约束时编译器会直接告诉你该加哪个（`E0277` / `E0369`）。

- 泛型结构体、泛型方法、为特定类型（如 `Point<f64>`）单独实现方法，都是同一套机制的用法。

- trait 定义行为契约；一个必写方法配一堆默认方法（`Iterator`、`Error`、`Default` 都是这个套路），实现者可以覆盖默认实现。

- 三种约束写法等价：`&impl Trait`、`<T: Trait>`、`where T: Trait`；约束多时用 `where` 更好读。

- 返回 `impl Trait` 能隐藏具体类型，但一个函数只能返回一种类型；要随分支返回不同类型就用 `Box<dyn Trait>`。

- trait 对象（`dyn`）是动态分发，能在一个容器里混装不同类型，代价是一次虚表跳转；`Box<dyn Error>` 就是典型用例。

- 关联类型用于「实现唯一」的场景（`Iterator::Item`），泛型参数用于「同一类型可以有多种组合」（`HashMap<K, V>`）。

- 运算符就是 trait 的语法糖：`+` 是 `Add`、`==` 是 `PartialEq`、`[]` 是 `Index`。

- 孤儿规则要求 trait 和类型至少有一个是本地的；想给外部类型加外部 trait 的方法，用新类型包装。

- `trait Error: Debug + Display` 这种写法是 supertrait，表示「实现我的人必须先实现它们」。

下一章讲**生命周期**：把前几章零零散散出现的 `<'a>`、`'static`、`E0106` 一次性讲清楚。
