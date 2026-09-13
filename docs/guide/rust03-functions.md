# 第 3 章 · 函数与表达式

前两章的代码都写在 `main` 里，这一章开始把它们拆成函数。同时要讲清楚一个 Rust 和其他语言差别比较大的概念：**表达式与语句**。搞懂它，后面 `if`、`match`、块、闭包的写法都会顺；反过来，很多「看着明明没错却编译不过」的错误都出在这里。

本章配套代码在 `src/rust03_functions/mod.rs`。

## 3.1 函数签名拆解

```rust
fn add(a: i32, b: i32) -> i32 {
    a + b
}
```

从左到右看：

| 部分 | 说明 |
| --- | --- |
| `fn` | 定义函数的关键字 |
| `add` | 函数名，习惯用小写下划线 `snake_case` |
| `(a: i32, b: i32)` | 参数列表，**每个参数都必须写类型** |
| `-> i32` | 返回类型，不写就表示返回 `()`（单元类型） |
| `{ a + b }` | 函数体，是一个块，块的值就是返回值 |

参数必须标注类型，这一点和第 2 章的 `let` 不同——`let x = 5` 可以让编译器推断，但函数签名是给调用者看的契约，必须明确写出来。

调用函数就是写名字加括号：`add(2, 3)`。函数名本身也是一个值，可以存进变量、当参数传递，见 3.8 节。

## 3.2 返回值：最后一行不要写分号

函数体的**最后一个表达式的值**就是返回值，这个表达式叫尾表达式：

```rust
fn square(x: i32) -> i32 {
    x * x   // 没有分号，它就是返回值
}
```

如果加上分号，函数就变成返回 `()`，编译直接失败。实测报错是这样的：

```rust
fn f() -> i32 {
    5;
}
```

```text
error[E0308]: mismatched types
 --> src/main.rs:1:11
  |
1 | fn f() -> i32 {
  |    -      ^^^ expected `i32`, found `()`
  |    |
  |    implicitly returns `()` as its body has no tail or `return` expression
2 |     5;
  |      - help: remove this semicolon to return this value
```

注意最后那行 `help`：编译器不只是报错，还直接告诉你「把分号删掉就能返回这个值」。**Rust 的报错信息一定要读到底**，`help` 和 `note` 往往已经给出答案了。

另一种返回方式是 `return`，用于提前结束函数：

```rust
fn absolute(x: i32) -> i32 {
    if x < 0 {
        return -x;   // 提前返回
    }
    x                // 正常路径走这里
}
```

两种风格怎么选？社区的偏好是：**能用尾表达式就用尾表达式**，`return` 留给「循环里跳出」或「多层嵌套里提前退出」这类真正需要提前结束的场景。上面这个 `absolute` 其实可以更紧凑地写成：

```rust
fn absolute(x: i32) -> i32 {
    if x < 0 { -x } else { x }
}
```

## 3.3 表达式与语句

这是本章最重要的一节。

- **表达式**：会求出一个值。`1 + 2`、`x * x`、`if c { 1 } else { 2 }`、`{ ... }` 都是表达式。
- **语句**：执行一个动作，值是 `()`（单元类型）。`let x = 5;` 是语句。

块是表达式，它的值是最后一行（同样不能带分号）：

```rust
let block_value = {
    let base = 1;
    base + 1        // 块的值是 2
};
println!("{block_value}"); // 2
```

而在最后一行的表达式后面加分号，会把「表达式」变成「语句」，值就没了：

| 写法 | 类型 | 值 |
| --- | --- | --- |
| `{ 5 }` | `i32` | `5` |
| `{ 5; }` | `()` | 无 |
| `let x = 5;` | `()` | 无 |

这也是 3.2 节那个报错的根本原因：`5;` 是一个语句，所以函数体没有尾表达式，编译器认为它返回 `()`。

### `let` 不是表达式

在不少语言里「赋值」是有值的表达式，可以写 `a = b = 5`。Rust 里 `let` 是语句，不能嵌套在表达式位置：

```rust
let x = let y = 5;
```

实测报错：

```text
error: expected expression, found `let` statement
 --> src/main.rs:2:13
  |
2 |     let x = let y = 5;
  |             ^^^
```

这个设计避免了赋值返回值被误用（比如 `if x = 5` 这种经典笔误），也让变量的作用域更好推理。

## 3.4 `if` 是表达式，但两条分支类型必须一致

因为没有三元运算符，Rust 直接用 `if` 表达式代替：

```rust
let label = if block_value > 2 { "大于 2" } else { "不大于 2" };
```

既然是表达式，它就得有确定的类型，所以：

- **两条分支的类型必须相同**，否则报 `if and else have incompatible types`。
- **当结果需要被使用时 `else` 不能省**。实测 `let value = if condition { 1 };` 的报错是：

```text
error[E0317]: `if` may be missing an `else` clause
  = note: `if` expressions without `else` evaluate to `()`
  = help: consider adding an `else` block that evaluates to the expected type
```

注意 `note` 那句话解释了原因：没有 `else` 的 `if` 值是 `()`，所以它和 `i32` 不匹配。

如果你只是要执行动作而不取值，`if` 当语句用是完全可以的，这时它的值就是 `()`：

```rust
if x < 0 {
    println!("负数");
}
```

对比一下「语句风格」和「表达式风格」，这是初学者最常写错的两种写法：

```rust
// 语句风格：先给默认值，再改
let mut result = 0;
if x > 0 {
    result = x;
}

// 表达式风格：一次赋值完成，不需要 mut
let result = if x > 0 { x } else { 0 };
```

第二种更符合 Rust 的习惯：不用 `mut`，就不会有忘记初始化分支的问题，编译器也能帮你检查所有分支。

## 3.5 参数：默认按值传递

```rust
fn count_chars(text: &str) -> usize {
    text.chars().count()
}
```

参数的传递方式和所有权规则有关，这是第 5 章的主题，这里先建立两个直觉：

1. **默认按值传递**。对 `i32`、`bool`、`char` 这类小类型来说等于复制一份，原变量照常能用；但对 `String` 这类拥有堆内存的类型，传进去会把所有权交出去，原变量就不能再用了。
2. **只想「看看」就传引用**。上面这个函数收 `&str`，调用时写 `count_chars(&owned)`，函数用完 `owned` 还在。

好消息是，函数参数优先收 `&str` / `&[T]` 这个习惯能避免大部分所有权问题：

```rust
let owned = String::from("rust-learn");
println!("{}", count_chars(&owned));   // 借用，owned 还能用
println!("{}", count_chars("字面量"));  // 字面量本身也是 &str
```

还有一个关键区别：**普通函数不能访问调用方的局部变量**，需要什么就通过参数传进来（常量、静态变量、其他函数当然还是能直接用）。能「捕获」环境里变量的那种函数叫闭包，第 11 章讲。

## 3.6 返回多个值：用元组

Rust 函数只能返回一个值，要返回多个就打包成元组：

```rust
fn divmod(a: i32, b: i32) -> (i32, i32) {
    (a / b, a % b)
}

let (quotient, remainder) = divmod(17, 5);
println!("17 除以 5 = {quotient} 余 {remainder}"); // 3 余 2
```

元素多了以后元组可读性会变差，那时候应该定义一个结构体（第 6 章），给每个字段起名字。

## 3.7 提前返回：`return` 和循环里的退出

`return` 的价值在「不用把所有分支都写成表达式」的场景，典型就是在循环里找到目标就收工：

```rust
fn sum_until_negative(values: &[i32]) -> i32 {
    let mut total = 0;
    for &value in values {
        if value < 0 {
            return total;   // 直接结束整个函数，后面的元素不再看
        }
        total += value;
    }
    total
}
```

实测 `sum_until_negative(&[3, 5, -2, 7])` 得到 `8`——只看 3 和 5，遇到 `-2` 就返回了；传入全是正数时得到 `17`。

注意 `for &value in values` 里的 `&`：`values` 是 `&[i32]`，迭代出来的是 `&i32`，加上 `&` 直接解构成 `i32` 来用。这种「在模式里解引用」的写法在第 6 章讲模式匹配时会正式展开。

顺带一提，如果函数只是「找第一个满足条件的元素」，Rust 更习惯用迭代器：

```rust
let found = values.iter().find(|&&v| v < 0);
```

clippy 看到手写循环加 `return` 时也会提示这一点。迭代器是第 11 章的内容，现在用循环完全没问题。

## 3.8 函数是一等公民

函数名本身就是一个值，可以存进变量、当参数传给别的函数：

```rust
fn add(a: i32, b: i32) -> i32 {
    a + b
}

fn apply(f: fn(i32, i32) -> i32, a: i32, b: i32) -> i32 {
    f(a, b)   // 像调用普通函数一样调用参数
}

let op: fn(i32, i32) -> i32 = add;  // 函数指针
println!("{}", op(4, 5));            // 9
println!("{}", apply(add, 6, 7));    // 13
```

`fn(i32, i32) -> i32` 是函数指针类型，注意它是**类型**，和定义函数用的 `fn` 关键字同名但含义不同。

实际项目里更常见的是闭包（能捕获环境、写法更短），函数指针主要用于「把行为当配置传进去」的场合，比如排序的比较函数。第 11 章会把两者放在一起比较。

另外，`s.len()`、`v.push(1)` 这种带点的调用形式叫方法调用，它是 `impl` 块里定义的函数，第 6 章讲。

## 3.9 `const fn`：编译期就能算出结果

给函数加上 `const` 关键字，它就变成可以在编译期求值的函数：

```rust
const fn const_square(x: i32) -> i32 {
    x * x
}

const SQUARE_OF_5: i32 = const_square(5); // 编译期就算好了

let buffer = [0u8; const_square(4) as usize]; // 可以当数组长度
```

它和普通函数不冲突：同一份逻辑既能编译期用，也能运行时用，`const_square(6)` 一样能正常调用。

`const fn` 的能力有明确边界，只能用编译器能在编译期执行的语法。实测把 `for` 循环写进去会报：

```text
error[E0015]: cannot use `for` loop on `std::ops::Range<i32>` in constant functions
  = note: calls in constant functions are limited to constant functions, tuple structs and tuple variants
```

改成 `while` 就行了——因为 `for` 循环需要调用 `Iterator` 的 trait 方法，而那套调用在常量求值里还没稳定：

```rust
const fn sum_all(values: &[i32]) -> i32 {
    let mut total = 0;
    let mut i = 0;
    while i < values.len() {
        total += values[i];
        i += 1;
    }
    total
}

const TOTAL: i32 = sum_all(&[1, 2, 3]); // 6
```

平时写业务代码几乎不需要 `const fn`，它的主要用途是让常量计算、数组长度这类东西更可读。

## 3.10 文档注释与 `cargo doc`

`///` 是文档注释，写在函数、结构体这些定义的上方，会被 `cargo doc` 收集成 HTML 文档：

```rust
/// 两数相加。
///
/// 支持负数，溢出行为和 `+` 一致。
fn add(a: i32, b: i32) -> i32 {
    a + b
}
```

模块开头用 `//!`（注意是 `!`）写模块级说明——本项目的 `src/rust0X_xxx/mod.rs` 开头用的就是它。跑 `cargo doc --open` 就能看到自己写的文档站。

文档注释里的代码块默认会被 `cargo test` 当成测试执行，所以例子写错会被测试抓出来，这是 Rust 很有特色的一点。

## 3.11 三个真实报错怎么读

这一节把新手最常撞上的三个错误摆在一起，学会看它们的 `help` / `note`，能省下大量搜索时间。

**案例 1：函数明明返回了值，却报 `expected i32, found ()`**

原因几乎总是尾表达式多写了分号。报错里那行 `help: remove this semicolon to return this value` 就是答案。

**案例 2：`if` 忘了写 `else`**

```text
error[E0317]: `if` may be missing an `else` clause
  = note: `if` expressions without `else` evaluate to `()`
```

`note` 说明了机制：没有 `else` 的 `if` 值是 `()`。要么补上 `else`，要么别把 `if` 的结果赋给变量。

**案例 3：`cannot apply unary operator '-' to type '...'`**

这是「负号用错类型」的报错。实测三种真实情况：

| 你写的代码 | 报错原文 | 原因与修法 |
| --- | --- | --- |
| `let x: u32 = 5; let y = -x;` | `error[E0600]: cannot apply unary operator '-' to type 'u32'`，note 为 `unsigned values cannot be negated` | 无符号类型不能取负，先转成有符号：`-(x as i32)` |
| `let y = -i32;` | `error[E0423]: expected value, found builtin type 'i32'` | 把类型名当值用了，应该写具体数值或变量 |
| 自己定义了同名类型（如 `struct i32;`） | `error[E0600]: ... to type 'i32'` | 自定义类型把内置类型遮住了，实现 `Neg` 或改名 |

这里有个值得记住的判据：**内置的 `i32` 一定是实现了取负的**，所以如果报错信息里赫然写着 `i32`，那这个 `i32` 就不是你以为的那个 `i32`——要么操作数其实是无符号类型（报错会写 `u32`/`usize`/`u64`），要么你写的是类型名而不是值，要么它被你自己的类型遮住了。在 IDE 里把鼠标悬停在变量上看一眼真实类型，或者用 `let () = x;` 这类「故意写错」的技巧让编译器打印出类型，都能很快定位。

## 3.12 常见坑速查

| 现象 | 原因 | 解法 |
| --- | --- | --- |
| `expected i32, found ()` | 尾表达式多写了分号 | 删掉最后那行的分号 |
| `if` 赋值报缺 `else` | 没 `else` 的 `if` 值是 `()` | 补 `else`，或改用语句形式 |
| 两个分支类型不一致 | `if` 是表达式，必须有统一类型 | 让两边同类型，或各自转成 `String` |
| `let x = let y = 5;` 报错 | `let` 是语句不是表达式 | 拆成两行 |
| 参数没写类型 | 函数签名必须明确类型 | 补上 `: i32` 之类 |
| 传了 `String` 之后原变量不能用了 | 参数默认按值传递，所有权移走了 | 传 `&s` 或 `.clone()` |
| 函数里用不了外面的变量 | 函数不捕获环境 | 通过参数传入，或改用闭包 |
| `const fn` 里写 `for` 报 E0015 | 常量求值不支持迭代器调用 | 改成 `while` |
| 函数指针类型写不出 | 语法是 `fn(i32) -> i32` | 注意括号里有参数类型、箭头后是返回类型 |

## 3.13 练习

1. 写 `fn max_of_three(a: i32, b: i32, c: i32) -> i32`，要求不使用 `return`，只用表达式。
2. 写 `fn describe(n: i32) -> &'static str`，返回 `"正数"` / `"负数"` / `"零"`。想一想为什么返回类型要写 `&'static str` 而不是 `&str`（第 10 章会正式讲生命周期）。
3. 写 `fn average(values: &[f64]) -> Option<f64>`，空数组返回 `None`。用 `match` 或 `if` 处理两种返回值。
4. 写 `fn first_index_divisible_by(values: &[i32], divisor: i32) -> Option<usize>`，在循环里用 `return` 提前返回下标。
5. 故意把某个函数的尾表达式加上分号，把编译器给的 `help` 抄下来，再改回去。

### 参考答案

::: details 第 1 题

```rust
fn max_of_three(a: i32, b: i32, c: i32) -> i32 {
    if a >= b && a >= c {
        a
    } else if b >= c {
        b
    } else {
        c
    }
}
```

不用 `return`，整个 `if` 链就是一个表达式，它的值就是函数返回值。

:::

::: details 第 3 题

```rust
fn average(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    let sum: f64 = values.iter().sum();
    Some(sum / values.len() as f64)
}

fn main() {
    println!("{:?}", average(&[1.0, 2.0, 4.0])); // Some(2.3333333333333335)
    println!("{:?}", average(&[]));              // None
}
```

:::

::: details 第 4 题

```rust
fn first_index_divisible_by(values: &[i32], divisor: i32) -> Option<usize> {
    for (index, &value) in values.iter().enumerate() {
        if value % divisor == 0 {
            return Some(index);
        }
    }
    None
}
```

`enumerate()` 同时给出下标和元素值，这是循环里带下标的常用写法。

:::

## 3.14 小结

- 函数签名里参数必须写类型，没写返回类型就是返回 `()`。
- 函数体最后一行是尾表达式，**不要加分号**；加分号会让返回值变成 `()`。
- 表达式有值，语句没有值。`let` 是语句，不能嵌套在表达式位置。
- 块、`if`、`match` 都是表达式；`if` 两条分支类型必须一致，缺 `else` 时值是 `()`。
- 参数默认按值传递，不想交出所有权就传引用；函数不捕获外部变量，那是闭包的事。
- 要返回多个值用元组，字段多了就定义结构体。
- `return` 留给提前退出，日常优先用表达式风格。
- 函数是一等公民，可以存进变量、当参数传递。
- `const fn` 能在编译期求值，但不能用 `for` 循环。
- 读报错时一定要看 `help` 和 `note`，它们经常直接给出修法。

下一章讲控制流，`if` / `loop` / `while` / `for` 的完整用法，以及带返回值的 `loop` 和循环标签——你会发现它们全都是表达式，本章的规则会反复用到。
