# 第 10 章 · 生命周期

生命周期这个词听起来很吓人，但它要回答的问题非常朴素：**这个引用还能用多久？**

前面的章节其实一直在和它打交道，只是没正面讲：

- 第 5 章：`E0515`（不能返回局部变量的引用）、`E0716`（临时值提前释放）。
- 第 7 章：`sorted_counts` 返回 `HashMap` 里的键，撞上 `E0106`。
- 第 9 章：`Box<dyn Error>` 为什么默认依赖 `'static`。

这一章把它们收口。先说结论：**生命周期参数是写给编译器看的约束关系，不是运行时概念，不会生成任何额外代码**。你写 `<'a>` 的时候，并没有让任何东西「活得更久」，只是向编译器承诺「这几个引用之间的关系是这样的」。

本章配套代码在 `src/rust10_lifetimes/mod.rs`，执行 `cargo run` 可以看到全部输出。

## 10.1 生命周期就是引用的有效范围

每个引用都借自某个所有者，所有者的作用域决定了引用能用的范围：

```rust
let outer = String::from("外层变量");
{
    let inner = String::from("内层变量");
    let borrowed = &inner;
    println!("{borrowed}");      // 可以：borrowed 在 inner 的作用域内
}                                // inner 在这里被 drop
// println!("{borrowed}");      // 不行：引用比所有者活得久
println!("{outer}");             // 可以：outer 还在
```

实测输出：

```text
    内层块里可以借用：内层变量
    出了块 inner 就被 drop 了，指向它的引用也不可能再存在
    外层引用可以一直用到作用域结束：外层变量
```

**生命周期不描述「值活多久」，而是「引用有效多久」**。`inner` 这个 `String` 从声明活到 `}`；`borrowed` 这个引用只能活在同一范围内。

大部分时候这些都不用你操心：第 5 章讲的 NLL（借用到「最后一次使用」为止）让编译器自己算得很准，只有「信息不够」时才需要手写标注。什么时候信息不够？主要是**函数签名和结构体定义**——这两处编译器看不到调用方，只能靠你写清楚。

## 10.2 借用检查器怎么推理

借用检查器的逻辑可以拆成三步：

1. 找出每个变量的作用域；
2. 找出每个引用的所有使用点；
3. 检查每个使用点是否都落在「被借用的值还活着」的范围内。

用一个例子看它怎么失败：

```rust
fn main() {
    let result;
    {
        let short = String::from("短命");
        result = &short;
    }
    println!("{result}");
}
```

```text
error[E0597]: `short` does not live long enough
 --> src/main.rs:5:18
  |
4 |         let short = String::from("短命");
  |             ----- binding `short` declared here
5 |         result = &short;
  |                  ^^^^^^ borrowed value does not live long enough
6 |     }
  |     - `short` dropped here while still borrowed
7 |     println!("{result}");
  |                ------ borrow later used here
```

这条报错是**教科书级别**的，读它的顺序就是借用检查器的推理顺序：

| 报错里的提示行 | 含义 |
| --- | --- |
| `binding 'short' declared here` | 被借用的值在哪声明 |
| `borrowed value does not live long enough` | 引用在这里被创建 |
| `'short' dropped here while still borrowed` | 值的生命在这里结束 |
| `borrow later used here` | 引用在这里还想用 —— 冲突就出在这儿 |

以后看到「does not live long enough」的报错，按这个顺序找那几行，问题基本一眼就能定位：**要么让值的生命更长，要么让引用的使用更早结束**。

## 10.3 悬垂引用：`E0515` 与 `E0716`

第 5 章见过这两种「引用指向已经消失的值」：

```rust
fn make(s: &str) -> &str {
    let local = String::from("local");
    &local[..]                  // 返回局部变量的引用
}
```

```text
error[E0515]: cannot return value referencing local variable `local`
 --> src/main.rs:3:5
  |
3 |     &local[..]
  |     ^-----^^^^
  |     ||
  |     |`local` is borrowed here
  |     returns a value referencing data owned by the current function
```

```rust
let s = String::from("hi").as_str();   // 临时 String 在语句结束就被释放
println!("{s}");
```

```text
error[E0716]: temporary value dropped while borrowed
 -->
2 |     let s = String::from("hi").as_str();
  |             ^^^^^^^^^^^^^^^^^^         - temporary value is freed at the end of this statement
  |             |
  |             creates a temporary value which is freed while still in use
3 |     println!("{s}");
  |                - borrow later used here
  |
help: consider using a `let` binding to create a longer lived value
```

两种修法：**要么把结果变成拥有型数据**（返回 `String` 而不是 `&str`），**要么让被借用的值活得更久**（先用 `let` 绑定再借用）。生命周期标注救不了这两种情况——它只能描述关系，不能延长任何东西的生命。

## 10.4 函数签名里的生命周期标注

函数体内，编译器能看见所有变量；但**调用方的变量它看不见**。所以「返回的引用借自哪个参数」这件事，必须在签名里说清楚。

```rust
fn longest(a: &str, b: &str) -> &str {
    if a.len() >= b.len() { a } else { b }
}
```

```text
error[E0106]: missing lifetime specifier
 --> src/main.rs:1:33
  |
1 | fn longest(a: &str, b: &str) -> &str {
  |               ----     ----     ^ expected named lifetime parameter
  |
  = help: this function's return type contains a borrowed value, but the signature does not say whether it is borrowed from `a` or `b`
help: consider introducing a named lifetime parameter
  |
1 | fn longest<'a>(a: &'a str, b: &'a str) -> &'a str {
  |           ++++     ++          ++          ++
```

`note` 那句话就是全部原因：**返回值借自 `a` 还是 `b`，签名没写，编译器不会替你猜**（猜错了会影响调用方的检查）。

加上标注之后：

```rust
fn longest<'a>(a: &'a str, b: &'a str) -> &'a str {
    if a.len() >= b.len() { a } else { b }
}
```

读法：**存在某个生命周期 `'a`，`a` 和 `b` 都至少活到 `'a`，返回值也活到 `'a`**。实际含义是「返回的引用不会比两个输入里活得短的那个更久」：

```rust
let a = String::from("short");
let b = "much longer string";
println!("{}", longest(&a, b));     // 可以

{
    let c = String::from("tiny");
    println!("{}", longest(b, &c)); // 可以：结果只用在这个块里
}                                    // 出了块，longest(b, &c) 的结果就不能再用了
```

注意一个常见误解：`'a` 不是「取两者中较短的那个生命周期」，而是**一个由调用方选定的、双方都满足的参数**。编译器在调用处选一个「足够小」的 `'a`，然后检查你有没有越界使用。

## 10.5 省略规则：什么时候不用写

日常代码里很少真的手写 `<'a>`，因为编译器有一套**生命周期省略规则**（lifetime elision）。三条规则按顺序应用：

1. 每个输入引用各自获得一个生命周期参数：`fn f(x: &str, y: &str)` 相当于 `fn f<'a, 'b>(x: &'a str, y: &'b str)`。
2. **如果只有一个输入引用**，这个生命周期就赋给所有输出：`fn first_word(text: &str) -> &str` 相当于 `fn first_word<'a>(text: &'a str) -> &'a str`。
3. **如果有 `&self` 或 `&mut self`**，`self` 的生命周期赋给所有输出。

规则 2 解释了为什么这些函数不用标注：

```rust
fn first_word(text: &str) -> &str { /* ... */ }        // 一个输入引用
fn first_line(text: &str) -> Option<&str> { }          // 一个输入引用
```

实测 `first_word("hello world") = hello`、`first_word("oneword") = oneword`。

规则 3 解释了方法：

```rust
impl<'a> Excerpt<'a> {
    fn part(&self) -> &str { self.part }                        // 返回值跟着 &self
    fn announce(&self, announcement: &str) -> &str { self.part } // 也是跟着 &self
}
```

`announce` 有两个输入引用（`&self` 和 `announcement`），按规则 2 是没法自动推断的——但规则 3 生效了：**返回值的生命周期跟着 `self`**，和 `announcement` 无关。所以下面这段是合法的：`announcement` 传一个临时字符串进去，返回值仍然指向 `self.part`。

规则用不上时才需要手写：**多个输入引用、且返回值可能来自任意一个**（就是 `longest`）。

## 10.6 结构体里存引用

结构体字段里放引用，必须写生命周期参数：

```rust
struct Excerpt {
    part: &str,
}
```

```text
error[E0106]: missing lifetime specifier
 --> src/main.rs:2:11
  |
2 |     part: &str,
  |           ^ expected named lifetime parameter
  |
help: consider introducing a named lifetime parameter
  |
1 ~ struct Excerpt<'a> {
2 ~     part: &'a str,
  |
```

编译器给不出别的答案：它不知道这个引用指向谁、该活多久，必须由你声明。加上之后：

```rust
#[derive(Debug)]
struct Excerpt<'a> {
    part: &'a str,
}

let text = String::from("hello rusty world");
let excerpt = Excerpt { part: &text[..5] };
```

读法：**`Excerpt<'a>` 的实例不能比它借用的那段文本活得更久**。实测 `excerpt = Excerpt { part: "hello" }`、`excerpt.part() = hello`。

这条约束由编译器检查，而且很实用：你在函数里返回 `Excerpt` 时，编译器会顺便帮你确认「被借用的文本活得够久」。

实践建议：**结构体里存引用是在「借别人的数据」，只有在「这是个视图/解析结果、确实不该拥有数据」时才这么做**；否则老老实实存 `String`，少一场和生命周期的搏斗（第 5 章 5.10 的结论在这里依然适用）。

## 10.7 `impl` 上的生命周期

给带生命周期的结构体写方法，`impl` 上也要带上参数：

```rust
impl<'a> Excerpt<'a> {
    fn part(&self) -> &str {
        self.part                 // 返回值跟着 &self（省略规则 3）
    }
}
```

`impl<'a> Excerpt<'a>` 读作「对**任意**生命周期 `'a`，给 `Excerpt<'a>` 实现这些方法」。和泛型结构体 `impl<T> Point<T>` 是同一个套路。

有时候想说得更明确：**返回的引用借的是底层文本，而不是 `self`**。那就把 `'a` 写出来：

```rust
impl<'a> Excerpt<'a> {
    fn part_borrowing_source(&self) -> &'a str {
        self.part
    }
}
```

两者差别很微妙，但真实存在：

```rust
let text = String::from("hello");
let part = {
    let excerpt = Excerpt { part: &text[..] };
    excerpt.part_borrowing_source()   // 借的是 text，不是 excerpt
};                                     // excerpt 在这里被 drop
println!("{part}");                    // 仍然可用，因为 text 还活着
```

如果返回类型写的是 `-> &str`（跟着 `&self`），`excerpt` 一 drop，返回值就不能用了。**返回值的生命周期该跟谁，由你想表达的关系决定**。

两种写法的差别实测过：`-> &'a str` 那版能编过并打印 `hello`；换成 `-> &str` 之后同一个程序报错：

```text
error[E0597]: `excerpt` does not live long enough
  --> src/main.rs:16:9
   |
14 |     let part = {
   |         ---- borrow later stored here
15 |         let excerpt = Excerpt { part: &text[..5] };
   |             ------- binding `excerpt` declared here
16 |         excerpt.part_tied_to_self()
   |         ^^^^^^^ borrowed value does not live long enough
17 |     };
   |     - `excerpt` dropped here while still borrowed
```

## 10.11 生命周期相关的报错速查

这一章遇到的报错都是同一个主题的不同侧面：

| 错误 | 触发场景 | 修法 |
| --- | --- | --- |
| `E0106: missing lifetime specifier` | 多个输入引用、返回值是引用；结构体字段是引用 | 手写 `<'a>` 标注，或结构体加 `<'a>` 参数 |
| `E0515: cannot return value referencing local variable` | 返回函数内部局部变量的引用 | 返回 `String` 等拥有型数据 |
| `E0597: x does not live long enough` | 引用的使用点超出了被借值的作用域 | 让值的生命更长，或让引用早点用完 |
| `E0716: temporary value dropped while borrowed` | 临时值的引用活过了语句 | 先用 `let` 绑定，再借用 |
| `lifetime may not live long enough` | 返回的生命周期和实际返回的数据不匹配 | 统一生命周期，或加 `'b: 'a` 约束 |
| `borrowed value does not live long enough` | 泛型结构体/方法约束没写全 | 检查 `impl<'a>` 上有没有漏参数 |

一个实用技巧：**报错里的 `help` 经常直接给出可复制的签名**（`help: consider introducing a named lifetime parameter` 后面就是完整的 `fn longest<'a>(...) -> &'a str`）。照抄之后如果还有其他错误，说明问题不在标注本身，而是设计上有更根本的冲突。

## 10.12 什么时候该换个思路

和生命周期搏斗十分钟还没进展时，通常是设计层面该调整了。几个常见出路：

| 症状 | 换个思路 |
| --- | --- |
| 函数想返回内部构造出来的东西的引用 | 返回 `String` / `Vec<T>` 等拥有型数据 |
| 结构体字段想存引用，但到处要用 `<'a>` | 存 `String`，必要时用 `.clone()`（第 5 章的取舍） |
| 多个对象互相引用（图、树的双向链接） | 用索引（`Vec` + `usize`）或用 `Rc` / `Arc` + `RefCell`（第 12 章） |
| 到处要 `'static` | 说明数据该被拥有，而不是借用 |
| `impl<'a, 'b, 'c>` 越写越长 | 抽象层次太深，考虑拆函数或拆类型 |
| 借用了某个东西，之后又想改它 | 先结束借用再改（第 5 章 NLL），或用 `clone` 换独立副本 |

社区共识是：**生命周期标注大多出现在库的类型定义和方法签名里**。如果你在写业务逻辑时到处手写 `'a`、`'b`，多半是设计上让借用跨越了太长的距离——让它变成拥有关系会更简单。

## 10.13 常见坑速查

| 现象 | 原因 | 解法 |
| --- | --- | --- |
| 「明明只用一个参数也要写生命周期」 | 函数有多个输入引用 | 检查是否真的只有一个引用输入 |
| 结构体加了 `<'a>` 之后到处都要写 | 借用贯穿了整个类型 | 改存拥有型数据 |
| `&'static str` 只能来自字面量 | 其实不是 | 也可以来自 `Box::leak`，但那会泄漏内存 |
| 以为 `'static` 就是「活到程序结束」 | 理解偏了 | 它表示「可以活那么久、不借自局部变量」 |
| `T: 'static` 以为等于不可变 | 两回事 | 它表示类型里没有短命引用 |
| 加了生命周期标注反而报更多错 | 标注写错了关系 | 先想清楚返回值到底借自谁 |
| 编译器建议加 `'static` 就照做 | 可能是在掩盖设计问题 | 先问「这个数据该不该被拥有」 |
| 方法返回 `&str`，drop 了对象还要用 | 返回值跟着 `&self` | 需要跟源头就返回 `&'a str`（10.7） |

## 10.14 练习

1. 写 `fn longest_word(sentence: &str) -> &str`，返回最长的单词，空串返回 `""`；想一想为什么这里不需要手写生命周期。
2. 解释 `fn pick(a: &str, b: &str) -> &str` 为什么编不过，并写出两种能编过的修法（一种改签名，一种改返回类型）。
3. 定义 `struct Book<'a> { title: &'a str, year: u32 }`，加 `#[derive(Debug)]`，实现 `fn title(&self) -> &str` 和 `fn older_than(&self, other: &Book<'_>) -> bool`。
4. 写 `fn first_line<'a>(text: &'a str) -> Option<&'a str>`，返回第一行（提示：`lines().next()`）。
5. 用自己的话解释这两句话对不对：(a)「`&'static str` 只可能来自字符串字面量」；(b)「`T: 'static` 表示 `T` 是不可变的」。

（第 2 题是 10.4 和 10.8 讨论过的场景，这里不再单独给答案。）

### 参考答案

::: details 第 1 题

```rust
fn longest_word(sentence: &str) -> &str {
    let mut longest: &str = "";
    for word in sentence.split_whitespace() {
        if word.len() > longest.len() {
            longest = word;
        }
    }
    longest
}

fn main() {
    println!("[{}]", longest_word("rust is a systems language")); // [language]
    println!("[{}]", longest_word(""));                            // []
}
```

不用写生命周期，是因为**只有一个输入引用** `sentence`——按省略规则 2，编译器把它的生命周期赋给返回值。`""` 最初是 `&'static str`，但变量的生命周期由编译器推断成 `sentence` 的，所以后面才能赋给它更短的 `word`。

:::

::: details 第 3 题

```rust
#[derive(Debug)]
struct Book<'a> {
    title: &'a str,
    year: u32,
}

impl<'a> Book<'a> {
    fn title(&self) -> &str {
        self.title
    }

    fn older_than(&self, other: &Book<'_>) -> bool {
        self.year < other.year
    }
}

fn main() {
    let a = Book { title: "Rust 程序设计", year: 2019 };
    let b = Book { title: "The Rust Book", year: 2023 };
    println!("{} / {}", a.title(), b.title());
    println!("a 比 b 老：{}", a.older_than(&b));
}
```

实测输出 `Rust 程序设计 / The Rust Book` 和 `a 比 b 老：true`。

两个细节：`title(&self) -> &str` 靠省略规则 3 自动跟着 `&self`；`older_than` 的参数写了 `&Book<'_>`，匿名生命周期表示「可以传任何生命周期的 `Book` 进来」，这里只需要借用比较年份，不需要统一两者。

:::

::: details 第 4 题

```rust
fn first_line<'a>(text: &'a str) -> Option<&'a str> {
    text.lines().next()
}

fn main() {
    println!("{:?}", first_line("第一行\n第二行")); // Some("第一行")
    println!("{:?}", first_line(""));               // None
}
```

这里手写 `<'a>` 是为了让意图明确：**返回的切片借自 `text`，`text` 活着它就活着**。其实按省略规则 2 它也可以不写——`fn first_line(text: &str) -> Option<&str>` 效果完全一样。练习里写出来只是提醒：`Option` / `Result` 里包着引用时，规则照样适用。

:::

## 10.15 小结

- 生命周期描述的是**引用有效多久**，不是值活多久；它是编译期概念，不产生任何运行时代码。

- 借用检查器按「声明 → 借用 → 值销毁 → 引用使用」四个位置推理，报错里通常把这四行都标出来。

- 悬垂引用有两类：返回局部变量的引用（`E0515`）和临时值提前释放（`E0716`）；生命周期标注救不了它们，只能改成拥有型数据 or 延长绑定。

- 函数签名要手写生命周期，是因为编译器看不到调用方；`fn longest<'a>(a: &'a str, b: &'a str) -> &'a str` 表示「返回值不会比两个输入里短命的那个更久」。

- 省略规则让大部分函数不用写标注：一个输入引用时输出跟着它；有 `&self` 时输出跟着 `self`。

- 结构体存引用必须带 `<'a>`；方法写在 `impl<'a> Excerpt<'a>` 里，返回值想跟着 `self` 就写 `-> &str`，想跟着底层数据就写 `-> &'a str`。

- 多个生命周期参数配合 `'b: 'a` 约束可以表达「谁的引用能活得更久」。

- `'static` 表示「这个引用可以活到程序结束、不借自任何局部变量」，不等于「值一定活到程序结束」；`T: 'static` 表示类型里没有短命引用，这是 `Box<dyn Error>` 的默认要求。

- 和生命周期搏斗太久，往往说明该把借用改成拥有（`String`、`Vec`、`Rc` / `Arc`），而不是继续加标注。

下一章讲**闭包与迭代器**：`map` / `filter` / `collect` 这些写法，以及闭包如何捕获环境变量——那里会看到生命周期和借用规则的又一次组合。

## 10.8 多个生命周期参数与约束

想明确「返回值只借自第一个参数」，就得声明两个生命周期：

```rust
fn pick<'a, 'b>(a: &'a str, b: &'b str) -> &'a str {
    b
}
```

```text
error: lifetime may not live long enough
 --> src/main.rs:3:5
  |
1 | fn pick<'a, 'b>(a: &'a str, b: &'b str) -> &'a str {
  |         --  -- lifetime `'b` defined here
  |         |
  |         lifetime `'a` defined here
3 |     b
  |     ^ function was supposed to return data with lifetime `'a` but it is returning data with lifetime `'b`
  |
  = help: consider adding the following bound: `'b: 'a`
```

报错说得非常直白：函数承诺返回 `'a` 的东西，实际返回的却是 `'b` 的。三种修法：

```rust
// 1. 让两个参数用同一个生命周期：调用方必须保证两者都活够
fn pick<'a>(a: &'a str, b: &'a str) -> &'a str { b }

// 2. 加约束：'b 至少和 'a 一样长（写出来就是 b: &'a str 也行）
fn pick<'a, 'b: 'a>(a: &'a str, b: &'b str) -> &'a str { b }

// 3. 干脆返回拥有型数据，把问题消掉
fn pick(a: &str, b: &str) -> String { b.to_string() }
```

`'b: 'a` 这种写法叫**生命周期约束**（outlives bound），读作「`'b` 比 `'a` 活得长（至少一样长）」。也可以写成 `where 'b: 'a` 的形式放在函数签名后面。

## 10.9 `'static` 的真实含义

`'static` 是最常被误解的一个：

```rust
fn greeting() -> &'static str {
    "程序全程有效"
}
```

字符串字面量存在于编译出的二进制里，程序运行期间一直在，所以它天然是 `'static` 的。但 `'static` 的含义**不是「这个值一定会活到程序结束」，而是「这个引用可以活到程序结束」——它不借自任何局部变量**。

所以下面这种也是合法的：

```rust
fn leak_string(text: String) -> &'static str {
    Box::leak(text.into_boxed_str())
}
```

实测输出 `泄漏出来：泄漏出来的字符串`。`Box::leak` 把堆上的内存「漏掉」——不再有人负责释放它，于是引用可以合法地活到程序结束。这不是什么好习惯（内存永远收不回来），只在少数场景才用，**千万别写在循环里**。

还要区分两个写法：

| 写法 | 含义 |
| --- | --- |
| `&'static T` | 一个借用到程序结束的引用 |
| `T: 'static` | 类型 `T` 里**不含**任何短命的引用（`i32`、`String`、`Vec<u8>` 都满足；`&'a str` 不满足） |

`T: 'static` 是第 8 章 `Box<dyn Error>` 默认要求 `'static` 的原因：`Box<dyn Error>` 可以跨线程、跨作用域传递，编译器必须确认它**没有藏着某个局部变量的引用**。

最后一句提醒：**`'static` 经常是「我讲不清生命周期」时的逃避方案**。如果一个设计到处需要 `'static`，通常说明数据该被拥有（`String`、`Arc`）而不是借用，或者该用 `Rc` / `Arc`（第 12 章）。

## 10.10 泛型参数上的生命周期约束

生命周期和泛型可以同时出现，`T: 'a` 表示「`T` 活得比 `'a` 长」：

```rust
fn print_ref<'a, T>(value: &'a T) -> &'a T
where
    T: fmt::Display + 'a,
{
    println!("打印：{value}");
    value
}
```

实测 `print_ref(&42)` 打印 `42` 并把同一个引用返回；`print_ref(&owned)` 对 `String` 也成立。

`T: 'a` 的意思是：**`T` 内部不能含有比 `'a` 更短命的引用**。对 `i32`、`String` 这类拥有型数据，这条约束天然满足；如果 `T` 本身是 `&'b Something`，那就要求 `'b: 'a`。

日常写代码时这条约束大多由编译器自动推导，只有在写复杂泛型结构体（比如自己实现容器、迭代器）时才需要显式写出来。
