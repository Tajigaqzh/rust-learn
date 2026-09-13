# 第 2 章 · 变量与基本类型

第 1 章解决了「怎么把值显示出来」，这一章解决「值本身是什么」。变量绑定、整数与浮点的边界、字符与字符串的区别、数组与切片，这些是后面所有内容的地基——尤其第 5 章的所有权，几乎全部建立在「值存在哪、有多大、谁能改」这几个问题上。

本章配套代码在 `src/rust02_variables/mod.rs`，执行 `cargo run` 可以看到全部输出。

## 2.1 `let`：默认不可变

```rust
let answer = 42;      // 不可变，之后不能改
let mut counter = 0;  // 加 mut 才能改
counter += 1;
```

想给 `answer` 重新赋值，编译器会直接拦下来，报错原文是 `cannot assign twice to immutable variable`。

这不是为了限制你，而是把「这个值后面会不会变」变成编译期信息：读代码时看到没有 `mut`，就能确定它不会在中途变化。所以社区的习惯是**只在确实需要改的时候才写 `mut`**，能不写就不写。

命名规范也跟着这个思路走：

| 种类 | 规范 | 例子 |
| --- | --- | --- |
| 变量、函数、模块 | `snake_case` 小写下划线 | `max_retry`、`read_line` |
| 常量、静态变量 | `SCREAMING_SNAKE_CASE` 全大写 | `MAX_RETRY` |
| 类型、trait、枚举 | `CamelCase` 大驼峰 | `Point`、`Book` |

规范不是强制语法，但违反会有 `warning`，而且 `cargo fmt` 和 `clippy` 都按这套习惯检查。

## 2.2 遮蔽（shadowing）：和 `mut` 不是一回事

用 `let` 再绑一个同名变量，叫遮蔽。它会**新建一个变量**，把旧的盖住：

```rust
let x = 5;
let x = x + 1;        // 新变量 = 旧值 + 1，x 变成 6

let spaces = "   ";
let spaces = spaces.len(); // 类型从 &str 变成了 usize
```

第 5 章会专门讲为什么这很重要，现在先记住差别：

| | `mut` | 遮蔽 |
| --- | --- | --- |
| 变量个数 | 一个 | 每次 `let` 都新建一个 |
| 能否改类型 | 不能 | 能 |
| 旧值 | 被覆盖 | 依然存在，只是名字被挡住 |
| 典型用途 | 循环计数、累加 | 用几步加工出一个值，每步类型都可能不同 |

遮蔽也有作用域限制：在内层块里遮蔽，出了块就恢复成外层的值。

```rust
let x = 5;
{
    let x = x * 2;
    println!("{x}"); // 10
}
println!("{x}"); // 5，外层的 x 没被动过
```

## 2.3 类型推断与标注

Rust 有类型推断，大多数时候不用写明类型：

```rust
let inferred = 7;        // 没有其他线索时，整数默认 i32
let suffixed = 7u8;      // 用后缀直接指定
let annotated: u64 = 7;  // 用标注指定
let parsed: i32 = "42".parse().unwrap();
```

没有线索时，整数默认 `i32`，浮点默认 `f64`。这两个默认值不是随便选的：`i32` 在 64 位机器上通常是最快的整数宽度，`f64` 的取值范围和精度都能满足绝大多数场景。

一个实用技巧：如果编译错误里出现 `{integer}` 或 `{float}`，意思是「推断还没定下来，需要更多信息」。比如：

```rust
let x = 5;
let y = x + 1.0;   // error[E0277]: cannot add a float to an integer
```

编译器指着这一行说 `no implementation for {integer} + {float}`——`{integer}` 就是「还没定下来的整数类型」。这类错误加一个显式标注（`let x: f64 = 5.0;`）就解决了。

## 2.4 整数类型：一共 12 种

| 类型 | 字节数 | 取值范围 |
| --- | --- | --- |
| `i8` / `u8` | 1 | -128 ~ 127 / 0 ~ 255 |
| `i16` / `u16` | 2 | ±3.2 万 / 0 ~ 65535 |
| `i32` / `u32` | 4 | ±21 亿 / 0 ~ 42 亿 |
| `i64` / `u64` | 8 | ±9.2×10¹⁸ / 0 ~ 1.8×10¹⁹ |
| `i128` / `u128` | 16 | 极大 |
| `isize` / `usize` | 平台相关 | 64 位机器上是 8 字节 |

`i` 是有符号（可负），`u` 是无符号。`isize` / `usize` 的宽度跟着平台走，主要用来做下标、长度和内存相关计算——`.len()` 返回的就是 `usize`。

选哪个的原则很简单：先按数据范围选够用的，拿不准就用 `i32`；涉及数组下标和长度时用 `usize`；`u8` 适合原始字节。

## 2.5 字面量写法

```rust
let decimal = 1_000_000; // 下划线只是给人看的，不影响值
let hex = 0xff;          // 255
let octal = 0o77;        // 63
let binary = 0b1010_1010; // 170
let byte = b'A';         // 字节字面量，类型是 u8，值是 65
```

下划线可以随便加，`1_0000_00` 也合法，但没人这么写。二进制字面量在做位运算和权限标志时特别直观：`0b1010` 一眼能看出第 1、3 位是 1。

字节字面量 `b'A'` 的类型是 `u8`，不是 `char`——这是新手常见的误解，它主要用于处理原始字节流。

## 2.6 整数溢出：调试版和发布版行为不同

先看实测结果。把 `i32::MAX` 加一：

| 编译方式 | 结果 |
| --- | --- |
| 调试版（`cargo run`） | `panicked at 'attempt to add with overflow'` 直接崩溃 |
| 发布版（`cargo run --release`） | 环绕成 `-2147483648` |

调试版帮你抓错，发布版为了性能不做检查（环绕是补码运算的自然结果）。这种差异很危险：**调试时正常，上线后数值悄悄变成负数**。

所以只要可能溢出，就应该用显式方法表达你的意图：

| 方法 | 行为 | 返回类型 |
| --- | --- | --- |
| `wrapping_add(1)` | 环绕 | `i32` |
| `checked_add(1)` | 溢出返回 `None` | `Option<i32>` |
| `saturating_add(1)` | 卡在最大值 | `i32` |
| `overflowing_add(1)` | 同时给结果和是否溢出 | `(i32, bool)` |

实测 `i32::MAX` 上的表现：

```text
wrapping_add(1)   = -2147483648
checked_add(1)    = None
saturating_add(1) = 2147483647
```

`checked_*` 适合「溢出属于错误」的场景（比如解析用户输入），`saturating_*` 适合「溢出就截断到边界」的场景（比如音频、图像处理）。

## 2.7 浮点：`0.1 + 0.2 != 0.3`

浮点类型只有两个：`f32` 和 `f64`，分别遵循 IEEE 754 单精度和双精度，默认是 `f64`。

```rust
let sum = 0.1f64 + 0.2f64;
println!("{sum:.20}");             // 0.30000000000000004441
println!("{}", sum == 0.3f64);     // false
println!("{}", (sum - 0.3f64).abs() < f64::EPSILON); // true
```

这不是 Rust 的问题，而是二进制浮点的固有特性：0.1 和 0.2 都无法用二进制精确表示，相加后的误差落在第 17 位小数上。所以：

- **不要用 `==` 比较浮点数**，要么比差值是否小于阈值，要么用专门处理精度的方法（`approx` crate 或比较定点整数）。
- **金额、计数等需要精确的场合不要用浮点**，用整数分或定点小数库。
- `f64` 大约能保证 15~16 位有效十进制数字，`f32` 只有约 7 位，科学计算默认用 `f64`。

浮点还有三个特殊值：

```rust
println!("{} {}", f64::INFINITY, f64::NEG_INFINITY); // inf -inf
let nan_a = f64::NAN;
let nan_b = nan_a;
println!("{}", nan_a == nan_b);      // false，NaN 不等于任何值，包括自己
println!("{}", nan_a.is_nan());      // true，判断 NaN 要用 is_nan()
```

## 2.8 `bool` 和 `char`

`bool` 只有 `true` / `false` 两个值，占 1 字节。它不能和整数互转——`if 1` 这种写法在 Rust 里不合法，条件必须是 `bool`。

```rust
let flag: bool = 3 > 2;
println!("{flag}, {}", !flag); // true, false
```

`char` 是**单个 Unicode 标量值**，用单引号，占 4 字节；字符串用双引号，是另一回事：

```rust
let ch: char = '中';
let text = "中";
println!("{}", size_of::<char>()); // 4
println!("{}", text.len());        // 3，UTF-8 编码后是 3 字节
```

这个对比很容易搞混：`char` 类型本身固定 4 字节（足够放下任何 Unicode 码点），而一个汉字存成 UTF-8 字符串占 3 字节。记住 `'中'` 是 char，`"中"` 是字符串，两者不能直接比较。

常用转义：`'\n'`、`'\t'`、`'\''`、`'\\'`、`'\u{1F600}'`（表情符号）。

## 2.9 元组：把不同类型打包

```rust
let record: (i32, &str, f64) = (1, "文本", 3.5);
println!("{} {} {}", record.0, record.1, record.2);

let (id, name, weight) = record; // 解构
println!("{id} {name} {weight}");
```

元组用 `t.0`、`t.1` 这样的下标访问，也可以一次性解构成多个变量。如果只关心其中几个，用 `_` 占位：`let (_, name, _) = record;`。

那个 `(1, "文本", 3.5)` 实测占 32 字节，而不是 4 + 16 + 8 = 28 字节——多出来的是**内存对齐的填充**。现在不用细究，第 12 章讲内存布局时再展开。

两个限制值得记住：

- 标准库只为最多 **12 个元素**的元组实现了 `Debug`、`PartialEq` 这些 trait。真写了 13 个元素的元组，`{:?}` 会报 `doesn't implement Debug`。超过这个规模就该定义结构体了。
- 空元组 `()` 是个特殊类型，叫单元类型，占 0 字节，表示「没有值」。没有返回值的函数返回的就是它。

## 2.10 数组与切片

数组长度固定，而且**长度是类型的一部分**：

```rust
let numbers: [i32; 3] = [1, 2, 3];
let zeros = [0u8; 4];              // 重复语法：4 个 0u8
println!("{numbers:?} 长度 {}", numbers.len());

let slice: &[i32] = &numbers[1..];  // 切片：借用数组的一段
println!("{slice:?} 长度 {}", slice.len());
```

`[i32; 3]` 和 `[i32; 4]` 是**不同**的类型，不能互相赋值。长度固定带来两个好处：编译器能在栈上分配，而且下标越界可以在编译期发现一部分。

切片 `&[T]` 是「某个数组的一部分」的借用，长度要到运行时才知道。数组和切片都能用 `.len()`、下标和 `{:?}`，`for` 循环遍历的方式也一样，所以在实际代码里两者经常混着用。

越界访问的处理分两种情况。**下标是编译期常量时直接编译失败**，这是实测的报错：

```rust
let numbers = [1, 2, 3];
println!("{}", numbers[5]);
```

```text
error: this operation will panic at runtime
  | index out of bounds: the length is 3 but the index is 5
```

**下标要运行时才知道时**，会真的 panic：

```rust
let index: usize = 从用户输入解析出来的值;
println!("{}", numbers[index]);
```

```text
panicked at 'index out of bounds: the len is 3 but the index is 5'
```

想让越界变成可处理的错误，用 `.get()`，它返回 `Option`：

```rust
let numbers = [1, 2, 3];
match numbers.get(5) {
    Some(value) => println!("取到了 {value}"),
    None => println!("下标越界了"),
}
```

需要动态长度的序列时用 `Vec<T>`，第 7 章讲。

## 2.11 `String` 和 `&str`：拥有与借用

这是 Rust 里第一个「看起来像同一件事，其实是两种东西」的地方。

```rust
let literal: &str = "字面量";                    // 借用，数据在程序的二进制里
let mut owned: String = String::from(literal);   // 拥有，数据在堆上
owned.push_str(" + push_str");                   // 可以增长

println!("{literal} / {owned}");
```

| | `&str` | `String` |
| --- | --- | --- |
| 数据在哪 | 别处（二进制、其他 String） | 自己持有堆内存 |
| 能否增长 | 不能 | 能，`push_str` / `push` |
| 大小 | 固定 16 字节（指针 + 长度） | 固定 24 字节（指针 + 长度 + 容量） |
| 常见来源 | 字符串字面量、`&s`、函数参数 | `String::from`、`to_string`、拼接结果 |

常用转换：

```rust
let owned: String = "abc".to_string();  // &str -> String
let owned2 = String::from("abc");       // 同上
let borrowed: &str = &owned;            // String -> &str（自动解引用）
let borrowed2 = owned.as_str();         // 更明确的写法
```

基本原则：**函数参数优先收 `&str`**，这样 `String` 和字面量都能传进来；需要自己持有数据时才用 `String`。第 5 章会讲清楚背后的所有权规则，这里先按这个原则用。

### 字节 vs 字符：字符串下标是个坑

`len()` 返回的是**字节数**，不是字符数：

```rust
let text = "中文";
println!("{}", text.len());          // 6，每个汉字 3 字节
println!("{}", text.chars().count()); // 2，真正的字符数
```

更危险的是按字节切片，切在字符中间会 panic：

```rust
let s = "中文";
let sliced = &s[0..1];
```

实测的报错是：

```text
end byte index 1 is not a char boundary; it is inside '中' (bytes 0..3 of string)
```

所以处理中文、emoji 时不要用 `s[0]`（这连编译都过不了，索引必须是 `usize` 且边界合法），要用 `chars()` 迭代，或者按字符边界切片 `&s[0..3]`。要按字符随机访问，先把字符收集成 `Vec<char>`。

遍历的两种方式：

```rust
let text = "中文";
for b in text.bytes() { print!("{b} "); }   // 228 184 173 230 150 135
for c in text.chars() { print!("{c} "); }   // 中 文
```

## 2.12 类型转换：`as`、`From`、`TryFrom`

Rust 不会隐式转换数值类型，`i32` 和 `i64` 相加必须显式处理。三种手段各有定位：

### `as`：可能丢数据，但一定成功

```rust
let big: i64 = 300;
println!("{}", big as u8);      // 44，只保留低 8 位
println!("{}", -1i32 as u32);   // 4294967295，按位重解释
println!("{}", 1.9f64 as i32);  // 1，向零截断
println!("{}", 1e30f64 as i32); // 2147483647，饱和到上限
```

整数之间是截断（保留低位）；浮点转整数是**向零截断**，超出范围的浮点会饱和到边界，`NaN` 变成 0。这些规则不难记，但正因为 `as` 从不报错，它也是最容易悄悄出问题的方式——只在确定安全时用。

### `From` / `Into`：保证无损

```rust
let small: u8 = 200;
let wide: u16 = u16::from(small);   // 只会变宽，不可能失败
println!("{wide}");                 // 200
```

`u8 -> u16` 这种「必然无损」的转换都实现了 `From`。反过来 `u16 -> u8` 没有实现 `From`，因为可能放不下。`From` 保证了转换的正确性，所以优先用它。

### `TryFrom`：可能失败的用这个

```rust
println!("{:?}", u8::try_from(200i64)); // Ok(200)
println!("{:?}", u8::try_from(300i64)); // Err(TryFromIntError(PosOverflow))
```

`try_from` 返回 `Result`，配合第 8 章的 `?` 运算符就能优雅地传播错误。处理外部输入（用户输入、文件内容、网络数据）时应该用它，而不是 `as`。

| 手段 | 可能失败 | 推荐场景 |
| --- | --- | --- |
| `as` | 不报错，但可能丢数据 | 位运算、明确知道范围、性能敏感的底层代码 |
| `From` / `.into()` | 不会失败 | 无损转换，日常首选 |
| `TryFrom` / `.try_into()` | 返回 `Result` | 处理外部数据，需要知道失败原因 |

## 2.13 `const` 和 `static`

```rust
const MAX_RETRY: u32 = 3;
static APP_NAME: &str = "rust-learn";

let buffer = [0u8; MAX_RETRY as usize]; // const 可以直接当数组长度
```

| | `const` | `static` |
| --- | --- | --- |
| 求值时机 | 编译期 | 程序启动时 |
| 内存地址 | 没有固定地址，用到就内联展开 | 有固定的内存地址 |
| 能否被借用 | 不能取稳定地址 | 可以取 `&APP_NAME` |
| 可变 | 永远不可变 | 要改需要 `unsafe` 的 `static mut` |
| 典型用途 | 配置常量、数组长度、数学常数 | 全局状态、需要固定地址的场合 |

日常写代码基本只用 `const`。`static mut` 从 Rust 2024 起已经不再推荐，需要共享可变状态时用第 12、13 章的 `Mutex`、`OnceLock` 这些工具。

另一个区别是重复定义：`const` 允许在不同作用域重复定义同名常量（各自独立），`static` 全局唯一。

## 2.14 常见坑速查

| 现象 | 原因 | 解法 |
| --- | --- | --- |
| 报错里有 `{integer}` / `{float}` | 类型推断还没定下来 | 加显式类型标注 |
| 调试正常，发布版数值变负 | release 不做溢出检查 | 用 `checked_*` / `saturating_*` |
| `0.1 + 0.2 == 0.3` 是 false | 二进制浮点无法精确表示 | 比差值或用定点数 |
| `NaN == NaN` 是 false | IEEE 754 规定 | 用 `is_nan()` |
| 改了变量报 `cannot assign twice` | 绑定默认不可变 | 加 `mut`，或改用遮蔽 |
| `i32` 和 `i64` 相加报错 | Rust 不做隐式转换 | `as` 或 `From` / `TryFrom` |
| 数组下标越界 | 常量下标编译期就报错，运行时下标会 panic | 用 `.get(i)` 返回 `Option` |
| 字符串下标编译不过 | 索引类型必须是 `usize`，且不能保证字符边界 | 用 `chars()` 或按字节边界切片 |
| 中文切一半 panic | 切在了多字节字符中间 | 按 `char_indices()` 找边界 |
| `len()` 得到的数字比字符数大 | `len()` 是字节数 | 用 `chars().count()` |
| 13 个元素的元组不能 `{:?}` | 标准库只实现到 12 元组 | 改用结构体 |
| `'a'` 和 `"a"` 类型不匹配 | char 与字符串是两种类型 | 用 `&"a"` 或 `'a'.to_string()` |

## 2.15 练习

1. 定义 `let temperature: f64 = 36.6;`，用 `f64` 运算把它转成华氏度（公式：`c * 9.0 / 5.0 + 32.0`）并保留一位小数打印。
2. 用遮蔽把一个字符串字面量先变成它的字节长度，再变成它的字符个数，中途打印每一步的类型和值。
3. 打印 `u8` 的最大值，然后分别用 `wrapping_add`、`checked_add`、`saturating_add` 加一，观察三种结果。
4. 用数组保存三次测量结果 `[1.5, 2.25, 3.75]`，计算平均值并打印。想一想为什么 `sum / 3` 编不过，要写成什么。
5. 把 `let input: i64 = 300;` 按三种方式转成 `u8`：`as`、`try_from`、以及先判断范围再转换。哪一种最适合处理用户输入？

### 参考答案

::: details 第 2 题

```rust
let text = "中文 abc";
let text = text.len();          // usize，字节数
println!("字节长度 {text}");
let text = text.to_string();    // String
println!("变成字符串后 {text}");
let text = text.chars().count(); // 又变回 usize
println!("字符个数 {text}");
```

:::

::: details 第 4 题

```rust
let samples = [1.5, 2.25, 3.75];
let sum: f64 = samples.iter().sum();
let average = sum / samples.len() as f64; // 长度是 usize，必须转成 f64
println!("平均值 {average:.2}");
```

`sum / 3` 编不过，是因为数组长度是 `usize`，而 Rust 不做隐式数值转换——`f64` 除以 `usize` 没有实现。

:::

::: details 第 5 题

```rust
let input: i64 = 300;

println!("as: {}", input as u8);                       // 44，悄悄错了
println!("try_from: {:?}", u8::try_from(input));       // Err(PosOverflow)

let checked = if (0..=u8::MAX as i64).contains(&input) {
    Some(input as u8)
} else {
    None
};
println!("范围判断: {checked:?}");                      // None
```

处理用户输入用 `try_from`（或先判范围）：`as` 会静默截断，把 300 变成 44，这种错误最难排查。

:::

## 2.16 小结

- `let` 默认不可变，需要改才加 `mut`；遮蔽是新建变量，可以改类型，和 `mut` 不是一回事。
- 没有线索时整数推断为 `i32`，浮点是 `f64`；错误信息里的 `{integer}` 表示类型还没定。
- 整数有 12 种，按范围选择；下标和长度用 `usize`。
- 溢出在调试版 panic、发布版环绕，涉及边界值要用 `checked_*` / `saturating_*`。
- 浮点不能直接用 `==` 比较，金额别用浮点，NaN 要用 `is_nan()` 判断。
- `char` 是 4 字节的 Unicode 字符，`&str` 的 `len()` 是字节数。
- 元组可以装不同类型但最多 12 个元素能打印；数组长度是类型的一部分，切片长度在运行时确定。
- `&str` 是借用、`String` 拥有数据，函数参数优先收 `&str`。
- 转换优先用 `From`，可能失败用 `TryFrom`，`as` 只在确定安全时用。
- 常量用 `const`，全局状态才考虑 `static`。

下一章讲函数与表达式，会把这些类型组合成可以复用的单元，并解释「表达式」和「语句」的区别——这个区别直接决定了函数怎么写返回值。
