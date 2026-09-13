# 第 1 章 · 打印输出

学任何语言的第一件事都是把东西显示出来，因为它是后面所有章节的调试手段。Rust 的打印看起来简单，但格式说明符、`Debug` 与 `Display`、缓冲行为这几块经常让人困惑，这一章把它们一次讲清楚。

本章配套代码在 `src/rust01_print/mod.rs`，在仓库根目录执行 `cargo run` 就能看到全部输出。

## 1.1 四个基本宏

```rust
fn main() {
    print!("不带换行");
    print!("可以接着写, ");
    println!("println 会自动换行");

    eprintln!("这行走标准错误，通常显示成红色");
}
```

| 宏 | 目标 | 是否换行 | 缓冲行为 |
| --- | --- | --- | --- |
| `print!` | stdout | 否 | 行缓冲，遇到换行才刷 |
| `println!` | stdout | 是 | 行缓冲 |
| `eprint!` | stderr | 否 | 无缓冲，立即出现 |
| `eprintln!` | stderr | 是 | 无缓冲 |

它们都是宏而不是函数。`println!("...")` 大致等价于 `writeln!(io::stdout(), "...")`，这一点在 1.9 节讲性能时很关键。

一个细节：`println!` 在写完整行期间持有 stdout 的锁，所以多线程同时打印时，不同行的内容不会互相穿插。

## 1.2 占位符：一对大括号就是一个坑位

```rust
let n = 42;
println!("顺序取值: {} + {} = {}", 1, 2, 1 + 2);
println!("按编号取，可以重复用: {0} + {1} + {0}", "a", "b");
println!("命名参数: 1.0 发布于 {year} 年", year = 2015);
println!("直接捕获变量: n = {n}, 两倍是 {}", n * 2);
println!("括号里也能放表达式: {} 和 {}", 10 * 10, "abc".len());
```

输出：

```text
顺序取值: 1 + 2 = 3
按编号取，可以重复用: a + b + a
命名参数: 1.0 发布于 2015 年
直接捕获变量: n = 42, 两倍是 84
括号里也能放表达式: 100 和 3
```

几点说明：

- `{0}` 这种编号参数可以重复使用。混用编号和 `{}` 时，`{}` 的计数会接着最后一次编号 +1 继续走，容易看错，建议一个格式串里只用一种风格。
- `{n}` 这种「变量捕获」是 Rust 2021 起支持的，变量名直接写进大括号即可，不必再列参数。
- 参数类型必须和占位符匹配：`{}` 要求该类型实现了 `Display`，具体见 1.6 节。

## 1.3 转义：大括号要写两遍

格式串里的大括号是语法，要打印字面量必须双写：

```rust
println!("一层大括号: {{}}，对应一个占位符: {}", 1);
println!("制表符\t换行\n反斜杠\\引号\"");
```

输出：

```text
一层大括号: {}，对应一个占位符: 1
制表符	换行
反斜杠\引号"
```

常用转义还有 `\r`（回车，做进度条用）、`\0`（空字符）、`\u{1b}`（ESC，写 ANSI 颜色用，见 1.10 节）。

## 1.4 格式说明符：冒号后面的完整语法

大括号里冒号后面的部分是格式说明符，完整结构是：

```text
{ 参数: [[填充字符]对齐][符号][#][0][宽度][.精度][类型] }
```

逐项看例子，都能直接跑：

```rust
println!("|{:>10}|", "右对齐");
println!("|{:<10}|", "左对齐");
println!("|{:^10}|", "居中");
println!("|{:*^10}|", "填充星号");
println!("|{:08}| 用 0 补到 8 位", 42);
println!("|{:+}| 正数也显示加号", 7);
println!("精度: {:.3}", 1.0 / 3.0);
```

各部分的含义：

| 部分 | 写法 | 作用 |
| --- | --- | --- |
| 对齐 | `<` `^` `>` | 左对齐 / 居中 / 右对齐。数字默认右对齐，其他类型默认左对齐 |
| 填充 | `{:*^10}` | 填充字符写在最前面，必须紧跟对齐符号 |
| 符号 | `{:+}` | 正数也打印出 `+` |
| `#` | `{:#x}` | 加上进制前缀，如 `0x`、`0b` |
| `0` | `{:08}` | 用 0 补齐到指定宽度 |
| 宽度 | `{:10}` | 最小宽度，内容更长时不会截断 |
| 精度 | `{:.2}` | 浮点表示小数位数，字符串表示最多保留的字符数 |
| 类型 | `{:x}` | 指定进制或格式类型，见 1.5 节 |

宽度和精度也可以是变量，用 `$` 引用：

```rust
let w = 8;
let p = 3;
println!("|{:>w$}| |{:<w$}|", "ab", "cd"); // |      ab| |cd      |
println!("{:.p$}", 1.0 / 3.0);              // 0.333
```

### 两个实测出来的反直觉结论

第一，Rust 没有 Python 那种「空格符号」。符号位只支持 `+`，写 `{: }` 不会报错，但也不会补空格：

```rust
assert_eq!(format!("{: }", 5).len(), 1); // 结果就是 "5"
```

第二，宽度按**字符个数**算，不按显示宽度算。两个汉字算 2 个字符，但在终端里占 4 列，所以中英混排的表格会歪：

```rust
println!("|{:>10}|", "中文"); // |        中文|  视觉上比预期宽两列
```

要真正对齐得按显示宽度自己计算，常见做法是引入 `unicode-width` 之类的 crate。

## 1.5 数值格式化

```rust
let n = 42;
println!("二进制 {n:b} / 带前缀 {n:#b}");
println!("八进制 {n:o} / 十六进制 {n:x} 或 {n:X} / 带前缀 {n:#x}");
println!("科学计数法 {:e} / 大写 {:E}", 1234.5, 1234.5);
```

输出：

```text
二进制 101010 / 带前缀 0b101010
八进制 52 / 十六进制 2a 或 2A / 带前缀 0x2a
科学计数法 1.2345e3 / 大写 1.2345E3
```

常用类型字符：`b` 二进制、`o` 八进制、`x` / `X` 十六进制、`e` / `E` 科学计数法、`p` 指针地址。

### 精度是「向偶数舍入」

很多人以为 `{:.0}` 就是四舍五入，其实是 round half to even，正好落在中间的 .5 会向偶数靠：

| 表达式 | 结果 |
| --- | --- |
| `format!("{:.0}", 0.5)` | `0` |
| `format!("{:.0}", 1.5)` | `2` |
| `format!("{:.0}", 2.5)` | `2` |
| `format!("{:.0}", 3.5)` | `4` |

更要留意浮点本身的表示误差：`0.125` 是二进制精确值，`{:.2}` 得到 `0.12`；而 `0.135` 实际存的是略小于 0.135 的数，`{:.2}` 得到 `0.14`，看似「不规律」。涉及金额计算不要用浮点，改用整数分或者定点小数库。

顺带一提，Rust 没有千分位分隔符，`println!("{:,}", 1234567)` 会直接编译失败，报错原文是 `python's numeric grouping ',' is not supported in rust format strings`。要千分位得自己写函数或用 crate。

## 1.6 `Debug` 和 `Display`：两套独立的格式化接口

`{}` 依赖 `std::fmt::Display`，`{:?}` 依赖 `std::fmt::Debug`。区别在于：

- `Debug` 面向开发者，输出结构化信息，用于调试和日志，可以 `#[derive(Debug)]` 自动生成。
- `Display` 面向使用者，输出给人看的文本，必须手写实现。

```rust
#[derive(Debug)]
struct Point {
    x: i32,
    y: i32,
}

let p = Point { x: 3, y: -1 };
println!("单行 {:?}", p);   // Point { x: 3, y: -1 }
println!("美化 {:#?}", p);  // 每个字段单独一行并缩进
```

`{:#?}` 是带缩进的美化版本，字段一多就看它。`dbg!` 宏内部也用 `Debug`，它会把值和文件名行号一起打到 stderr，并返回原值：

```rust
let doubled = dbg!(21 * 2); // [src/main.rs:2:19] 21 * 2 = 42
println!("{doubled}");      // 42
```

较新版本的 Rust 还支持十六进制 Debug，`{:x?}` 和 `{:X?}` 打印字节数组特别顺手：

```rust
println!("{:x?}", [255u8, 16]); // [ff, 10]
```

### 自己实现 `Debug`

想改字段名或顺序时可以手写 `impl`，用 `debug_struct` 能自动获得 `{:#?}` 的缩进效果：

```rust
use std::fmt;

impl fmt::Debug for Point {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Point")
            .field("横坐标", &self.x)
            .field("纵坐标", &self.y)
            .finish()
    }
}
```

注意 `derive` 和手写 `impl` 不能同时存在，两个都写会报 E0119 conflicting implementations。

### 自己实现 `Display`

```rust
use std::fmt;

struct User {
    name: String,
    age: u32,
}

impl fmt::Display for User {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}（{} 岁）", self.name, self.age)
    }
}

let u = User { name: "小明".into(), age: 18 };
println!("{u}");   // 小明（18 岁）
println!("{u:?}"); // 报错：User 没有实现 Debug
```

这段代码恰好说明两件事：`impl Display` 只解决 `{}`，不解决 `{:?}`；反过来 `#[derive(Debug)]` 也不会让你能用 `{}`。

### `Option` 和 `Result` 只能用 `{:?}`

`Option<T>` 和 `Result<T, E>` 都没有实现 `Display`，想打印就用 `{:?}`：

```rust
let o = Some(3);
let r: Result<i32, String> = Ok(1);
println!("{:?} {:?}", o, r); // Some(3) Ok(1)
```

写 `println!("{}", o)` 会得到 `error[E0277]: Option<{integer}> doesn't implement Display`。

### 字符串的 `Debug` 会转义

```rust
let s = String::from("a\nb");
println!("{}", s);   // 真的换行
println!("{:?}", s); // "a\nb"
```

写日志时 `{:?}` 更安全，能看清到底有没有换行、引号和不可见字符。

## 1.7 stdout 还是 stderr

约定是：程序的正常结果走 stdout，日志、警告、进度提示走 stderr。这样用户可以把结果重定向到文件，提示仍然显示在屏幕上：

```bash
cargo run > result.txt
```

`eprintln!` 还有两个值得记住的特性：它立即输出（无缓冲），也不会被 stdout 的缓冲拖慢，所以做进度和排查问题时优先用它。

## 1.8 缓冲与 flush：为什么 `print!` 有时没反应

stdout 是行缓冲的：只有写入了换行，或者缓冲区满了，内容才会真正刷到屏幕上。所以下面这行的提示可能迟迟不出现：

```rust
print!("请输入名字: "); // 光标停住了，但提示文字还没显示
```

要立刻显示就手动刷一次：

```rust
use std::io::{self, Write};

print!("请输入名字: ");
io::stdout().flush().unwrap();
```

同样的手法配合 `\r`（回车，把光标移回行首）可以做出原地刷新的进度条：

```rust
use std::io::{self, Write};

fn main() {
    for i in 0..=10 {
        print!("\r进度 {:>3}% [{}]", i * 10, "#".repeat(i));
        io::stdout().flush().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(60));
    }
    println!();
}
```

## 1.9 性能：热循环里别用 `println!`

`println!` 每次调用都要重新拿一次 stdout 的锁，还会触发系统调用。循环里打十万行会明显变慢。两种常用优化：

先拿锁，再批量写：

```rust
use std::io::{self, Write};

fn main() -> io::Result<()> {
    let stdout = io::stdout();
    let mut out = stdout.lock();
    for i in 1..=100_000 {
        writeln!(out, "line {i}")?;
    }
    Ok(())
}
```

或者先拼进 `String`，最后一次性输出。`writeln!` 能写 `String`，因为它实现了 `fmt::Write`：

```rust
use std::fmt::Write as _;

let mut buf = String::new();
writeln!(buf, "{:>6} | {:.2}", "总计", 99.5).unwrap();
writeln!(buf, "{:>6} | {:.2}", "折扣", 9.95).unwrap();
print!("{buf}");
```

另外，`println!` 写失败会 panic。典型场景是管道提前关闭：

```bash
myapp | head -n 1
```

`head` 退出后管道断开，后面的 `println!` 会因 EPIPE 直接崩溃。需要容忍这种情况时改用 `writeln!` 并检查错误：

```rust
use std::io::{self, ErrorKind, Write};

if let Err(e) = writeln!(io::stdout(), "hello") {
    if e.kind() != ErrorKind::BrokenPipe {
        eprintln!("写 stdout 失败: {e}");
    }
}
```

## 1.10 其他实用技巧

终端颜色用 ANSI 转义序列即可，`\u{1b}[31m` 是红色，`\u{1b}[0m` 复位：

```rust
println!("\u{1b}[31m错误\u{1b}[0m: 文件不存在");
```

Windows Terminal 和主流的 Linux / macOS 终端都支持。重定向到文件时这些控制字符会留在文件内容里，所以更完整的做法是先判断 `std::io::stdout().is_terminal()`，再决定要不要着色。

`format!` 只生成字符串不输出，返回的 `String` 可以继续拼接和传参，实际项目里通常用它组装内容、最后一次性打印。

`std::format_args!` 也可以存进变量，但它借用参数，不能当返回值带出函数：

```rust
fn make() -> std::fmt::Arguments<'static> {
    let s = String::from("x");
    std::format_args!("{}", s) // error[E0515]: cannot return value referencing local variable `s`
}
```

## 1.11 常见坑速查

| 现象 | 原因 | 解法 |
| --- | --- | --- |
| 打印结构体报 E0277 | 类型没有实现 `Debug` | 加 `#[derive(Debug)]`，并用 `{:?}` |
| `{:?}` 报错但 `{}` 正常（或反过来） | 两个 trait 相互独立 | 分别实现，或统一用 `Debug` |
| 程序输出顺序看起来乱 | stdout 有缓冲，stderr 没有 | 需要顺序时先 `flush()` |
| 中文表格对不齐 | 宽度按字符数计算，不按显示列数 | 中文列单独处理，或用 `unicode-width` |
| `{:,}` 编译失败 | Rust 不支持千分位分组 | 自己写格式化函数 |
| `{: }` 没有补空格 | 符号位只支持 `+` | 手动拼一个空格 |
| 打印大括号报格式串错误 | 大括号是语法，需要转义 | 每个大括号写两遍 |
| `Option` / `Result` 用 `{}` 报错 | 它们没有实现 `Display` | 用 `{:?}` |
| 管道下游退出后程序崩溃 | `println!` 遇到 EPIPE 会 panic | 改用 `writeln!` 并忽略 `BrokenPipe` |
| 提示文字/进度条不刷新 | stdout 行缓冲，没有换行就不刷 | 调用 `flush()` |

## 1.12 练习

1. 打印一行 `姓名: 小明, 得分: 90.5`，要求得分固定两位小数。
2. 打印一个 3×3 的乘法表，让每列宽度统一；再把表头换成中文，观察对齐发生了什么变化。
3. 用 `{:b}` 和 `{:#x}` 打印 `255`，比较带不带 `#` 的差别。
4. 定义 `struct Book { title: String, price: f64 }`，实现 `Display` 输出「书名 ¥价格」，再 `derive(Debug)` 用 `{:#?}` 打印对比。
5. 用 `\r` 和 `flush()` 做一个从 0% 到 100% 的进度条，跑完把那一行清掉。

### 参考答案

::: details 第 2 题

```rust
for i in 1..=3 {
    for j in 1..=3 {
        print!("{:>4}", i * j);
    }
    println!();
}

// 换成中文表头后，你会发现「行」占两个显示列但只算一个字符宽度，
// 单纯用 {:<6} 排出来的竖线对不齐。
println!("{:<6}|{:<6}", "行", "列");
```

:::

::: details 第 4 题

```rust
use std::fmt;

#[derive(Debug)]
struct Book {
    title: String,
    price: f64,
}

impl fmt::Display for Book {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ¥{:.2}", self.title, self.price)
    }
}

fn main() {
    let b = Book {
        title: "Rust 程序设计".into(),
        price: 99.0,
    };
    println!("{b}");    // Rust 程序设计 ¥99.00
    println!("{b:#?}"); // 结构化的多行输出
}
```

:::

::: details 第 5 题

```rust
use std::io::{self, Write};

fn main() {
    let total = 30;
    for i in 0..=total {
        let percent = i * 100 / total;
        print!("\r[{:<30}] {:>3}%", "#".repeat(i), percent);
        io::stdout().flush().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    println!("\r{:<40}", ""); // 用空格把进度条覆盖掉
}
```

:::

## 1.13 小结

- `print!` / `println!` 走 stdout，`eprint!` / `eprintln!` 走 stderr，后者无缓冲。
- 占位符支持顺序、编号、命名和变量捕获四种取值方式。
- 格式说明符结构是 `[[填充]对齐][符号][#][0][宽度][.精度][类型]`，宽度和精度都能用变量加 `$`。
- `{}` 需要 `Display`（手写实现），`{:?}` 需要 `Debug`（可 derive）。
- stdout 行缓冲，想提前显示要 `flush()`；热循环里先拿锁批量写，或先拼字符串。
- 精度是向偶数舍入，涉及金额不要用浮点。

下一章从变量和基本类型开始，把「值存在哪、什么时候被移动」这条主线铺开，为第 5 章的所有权打基础。
