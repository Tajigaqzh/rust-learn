# 第 29 章 · 类型系统进阶

> 前置：第 9 章（泛型与 trait）、第 10 章（生命周期）。本章假设你已经
> 熟悉 trait bound、关联类型和生命周期省略规则。

第 9 章和第 10 章把泛型、trait、生命周期各自讲完了。本章把它们拼起来，
补上四个「拼起来才成立」的进阶主题：**const 泛型**（让常量参与类型）、
**GAT**（让关联类型带泛型参数）、**trait 中的 `impl Trait`**（把返回类型的
决定权从实现者挪到签名）、**高阶生命周期 `for<'a>`**（对所有生命周期成立
的函数类型）。

这四个主题的共同点：它们都出现在你「照着第 9、10 章的知识写代码，却
编译不过」的时刻。学完本章，标准库和主流 crate 签名里的奇怪符号应该
都能读顺了。

本章代码在 `src/rust29_typesystem/`，`cargo run` 看输出，
`cargo test rust29_typesystem` 跑本章测试。

## 29.1 为什么需要这四种东西

先把四个主题解决的问题各用一句话说清：

| 主题 | 解决的问题 | 一句话版本 |
| --- | --- | --- |
| const 泛型 | `[T; 3]` 和 `[T; 4]` 是不同类型，泛型怎么覆盖所有长度？ | 让**编译期常量**当类型参数 |
| GAT | 关联类型 `Item` 没有生命周期参数，迭代器想「借出」元素怎么办？ | 让关联类型**带泛型参数** |
| trait 中的 `impl Trait` | trait 想表达「返回某个满足约束的类型」，但不想定义关联类型 | 签名写 `-> impl Trait`，**实现者自由选具体类型** |
| `for<'a>` | `fn(&str) -> &str` 怎么表示「接受**任意**生命周期的引用」？ | 对所有 `'a` 都成立的**高阶**类型 |

## 29.2 const 泛型：让数组长度参与类型

`[T; 3]` 和 `[T; 4]` 是两个不同的类型——这是第 2 章就知道的事实。
但第 9 章的泛型 `fn f<T>(x: T)` 覆盖不了它们：类型参数只能绑定**类型**，
绑不了**长度**。Rust 1.51 起的 const 泛型（最小可用版，min const generics）
补上了这块：

```rust
fn sum<const N: usize>(arr: [i32; N]) -> i32 {
    arr.iter().sum()
}

sum([1, 2, 3]);      // N 推导为 3
sum([1, 2, 3, 4]);   // N 推导为 4，同一个函数
```

`<const N: usize>` 读作「一个名为 N 的 const 参数，类型 usize」。
调用时通常不用写——编译器从数组长度推导。

标准库自己就是这套机制的最大用户。在 const 泛型之前，标准库得为
`[T; 0]`、`[T; 1]`……`[T; 32]` 用宏各生成一份实现；现在一行搞定：

```rust
impl<T, const N: usize> IntoIterator for [T; N] { ... }
```

### 维度进类型：编译期形状检查

const 泛型最划算的用法是把「业务规则」编码进类型。本章配套代码里的
`Matrix<const R: usize, const C: usize>` 是标准示例：

```rust
struct Matrix<const R: usize, const C: usize> {
    data: [[f64; C]; R],
}

fn mat_mul<const R: usize, const K: usize, const C: usize>(
    a: &Matrix<R, K>,
    b: &Matrix<K, C>,
) -> Matrix<R, C> { ... }
```

注意 `K` 同时出现在两个操作数里——类型系统因此天然要求「左列数 =
右行数」：

```rust
let a = Matrix::<2, 3>::from_coords(|r, c| (r * 3 + c) as f64);
let b = Matrix::<3, 2>::from_coords(|r, c| (r * 2 + c + 1) as f64);
let c = mat_mul(&a, &b);   // OK：Matrix<2,2>

// let bad = mat_mul(&a, &a);
// error[E0308]: expected `&Matrix<3, _>`, found `&Matrix<2, 3>`
```

`Matrix<2, 3> * Matrix<2, 3>` 不是运行时 panic，是**编译错误**。
维度错误从「半夜上线后崩」提前到「保存文件那一刻」。

配套代码里还有一个细节值得看：`Matrix::zeros` 里的 `[[0.0; C]; R]`
之所以能写，正是因为 `C` 和 `R` 是编译期常量——数组长度必须是常量，
而 const 参数就是常量。

### `std::array::from_fn`：按坐标构造数组

`[T; N]` 没法用 `map` 构造（move 出数组的问题），标准库提供了
`from_fn`，配合 const 泛型推导长度：

```rust
let squares: [u32; 5] = std::array::from_fn(|i| i * i);
// [0, 1, 4, 9, 16]
```

## 29.3 const 泛型的限制

「最小可用版」意味着限制不少，写代码前先知道边界：

1. **只支持整型、`bool`、`char`**（以及由它们组成的常量表达式）。
   `struct Bad<const S: String>` 编译不过——`String` 不是合法的
   const 参数类型。
2. **不能对 const 参数做任意运算**。`[u8; N * 2]` 可以（常量表达式），
   但「`N > 0` 时才有某个方法」做不到——没有 `where N > 0` 这种条件
   impl（那需要特化或 const generics 的完整版，都还没稳定）。
3. **不能把 const 参数当运行时值用**。`println!("{N}")` 在函数体里
   不行——N 在类型层，不是运行层；想用得先绑定 `const LOCAL: usize = N;`。

第三条常被搞混。const 参数活在类型的世界里：它能出现在类型位置
（`[u8; N]`）和常量表达式里，但函数体里的普通语句看不到它。

一个能做的例子（配套代码 `const_param_limits`）：

```rust
const fn const_param_limits() {
    // 常量上下文里可以对常量做算术：
    const DOUBLE: usize = 3 * 2;
    let _arr: [u8; DOUBLE] = [0; DOUBLE];
    // 但做不到：impl<const N: usize> Foo for Bar where N > 0
}
```

## 29.4 GAT：让关联类型带泛型参数

第 9.10 章讲过关联类型：`trait Iterator { type Item; }`，实现者决定
`Item` 是什么。但 `Item` 有个隐藏限制——**它不能带生命周期参数**。
这不是语法洁癖问题，是一道真实的能力缺口：

标准 `Iterator` 的 `next` 签名是：

```rust
fn next(&mut self) -> Option<Self::Item>
```

`Item` 是**拥有**的类型。如果迭代器内部想复用一个缓冲区、每次只
**借出**一块（比如零拷贝的分词器），`Item` 就得是 `&str`——可这个
`&str` 借自迭代器自身，生命周期和每次 `next` 调用绑定。`type Item;`
表达不了「每次调用时的那个生命周期」。

GAT（generic associated types，Rust 1.65 稳定）就是答案——关联类型
自己带泛型参数：

```rust
trait LendingIterator {
    type Item<'a>
    where
        Self: 'a;

    fn next(&mut self) -> Option<Self::Item<'_>>;
}
```

读法：`type Item<'a>` 声明「实现者要给一个**生命周期参数化的**关联
类型」；`next` 返回 `Self::Item<'_>` 时，那个 `'_` 就是本次调用的
生命周期。配套代码里的 `WordSplitter` 是完整实现：

```rust
struct WordSplitter<'s> {
    rest: &'s str,          // 剩余待切分文本
}

impl LendingIterator for WordSplitter<'_> {
    type Item<'a> = &'a str where Self: 'a;

    fn next(&mut self) -> Option<Self::Item<'_>> {
        let rest = self.rest.trim_start();
        if rest.is_empty() {
            return None;
        }
        let end = rest.find(char::is_whitespace).unwrap_or(rest.len());
        self.rest = &rest[end..];
        Some(&rest[..end])          // 借自自身状态
    }
}
```

`WordSplitter` 产出的 `&str` 借自它自己的 `rest` 字段。如果用标准
`Iterator` 写，`Item = &'a str` 里的 `'a` 只能来自迭代器类型本身，
borrow checker 会拒绝「产出比自身活得久的引用」。GAT 让关联类型
**跟着每次调用走**，问题消失。

### 代价：借出的元素攒不到一起

GAT 的能力有对价。配套代码里实测过：

```rust
// error[E0499]: cannot borrow `splitter` as mutable more than once
let mut words = Vec::new();
while let Some(word) = splitter.next() {
    words.push(word);        // 每个 word 都借自 splitter
}
```

收集失败的原因就是 GAT 签名本身：`Item<'a>` 的 `'a` 绑定在每次
`next` 的 `&mut self` 借用上，`Vec` 同时持有多个元素 = 同时持有
多个活跃的 `&mut splitter`。**借出的东西只能当场用**——这正是
lending iterator 的设计意图（复用内部缓冲区），也是它和标准
`Iterator` 最本质的差别。需要收集时，要么把元素 `to_string()` 拥有
化，要么老实用标准 `Iterator`。

GAT 最著名的应用在异步世界：`Future` 本身可以看作 GAT 特化前的
 workaround——第 30 章的 `Pin<&mut Self>` 问题与它同根同源。日常
 你最可能撞见 GAT 的地方是嵌入式/IO 库的「借出缓冲区」接口。

## 29.5 trait 中的 `impl Trait`（RPITIT）

第 9.6/9.7 讲过函数签名里的 `impl Trait`：参数位置是 `T: Trait` 的
糖，返回位置是「返回某个满足 Trait 的具体类型，但我不告诉你具体是啥」。

Rust 1.75 之前，**trait 方法不能**用返回位置 `impl Trait`。想表达
「这个方法返回某个实现 Display 的东西」，只能定义关联类型：

```rust
// 老写法：每个实现者都要指定关联类型
trait Style0 {
    type Rendered: fmt::Display;
    fn render(&self) -> Self::Rendered;
}
```

这很重：实现者被迫把返回类型「广播」出去，哪怕调用方根本不关心。
Rust 1.75 的 RPITIT（return-position impl trait in traits）补齐：

```rust
trait Style {
    fn render(&self) -> impl fmt::Display;
}

struct Plain(String);
struct Shouty(String);

impl Style for Plain {
    fn render(&self) -> impl fmt::Display {
        &self.0                       // 返回引用
    }
}

impl Style for Shouty {
    fn render(&self) -> impl fmt::Display {
        format!("{}!!!", self.0)      // 返回新 String
    }
}
```

两个实现返回**不同的具体类型**，签名只承诺 `Display`。决定权从
实现者（关联类型）挪到了**签名**（RPITIT）。

### 和 trait 对象的边界

RPITIT 不是万能钥匙。它最大的限制和 9.7 讲的一样：**同一个签名、
不同实现返回不同类型，所以没法装箱成 `dyn Style`**——虚表需要每个
方法一个固定的返回布局，而「impl Trait」因实现而异。需要动态分发
时，还是得回到关联类型或 `Box<dyn Trait>`。

### `use<'a, T>`：精确控制捕获

默认情况下，trait 方法返回的 `impl Trait` **捕获所有输入生命周期**
（这样 `&self` 借的东西才能合法返回）。有时你想明确「不捕获某个
生命周期」来获得更多灵活性（比如返回值要 `'static`），edition 2024
提供了 `use<...>` 精确捕获语法：

```rust
trait Loader {
    // 只捕获 'a 和 T，不捕获 &self 的生命周期：
    fn parse<'a, T: Parse<'a>>(&self, input: &'a str) -> impl Parse<'a> + use<'a, T>;
}
```

日常写业务代码很少需要它，但读标准库和 futures 生态的签名时会遇到。
认得它、知道它是「精确声明返回类型里用到哪些泛型参数」就够了。

## 29.6 高阶生命周期 `for<'a>`

最后一块拼图。考虑这个函数：

```rust
fn apply(f: fn(&str) -> &str, input: &str) -> &str {
    f(input)
}
```

`f` 的参数和返回值都是引用。问题：`f` 的类型里的生命周期是什么？
调用方手里有自己作用域的 `input`——难道 `f` 只能接受「生命周期恰好
等于这次调用」的函数？

当然不是。`fn(&str) -> &str` 的完整类型是：

```rust
for<'a> fn(&'a str) -> &'a str
```

读作：「对**所有**生命周期 `'a`，`fn(&'a str) -> &'a str`」。
这叫**高阶 trait bound**（higher-ranked trait bound, HRTB）。它和
普通泛型的区别在于「谁来选生命周期」：

| 写法 | 含义 | 实例化 |
| --- | --- | --- |
| `fn f<'a>(g: G<'a>)` | `'a` 是**调用者选定**的参数 | 每个生命周期一个不同的 G |
| `for<'a> fn(&'a str)` | 对**所有** `'a` 成立 | 一个类型，覆盖所有调用 |

配套代码里的 `apply_to_any_lifetime` 演示了这一点：不管传入的
`input` 活多久，`first_word` 都能处理——因为它的类型天然满足
「对所有生命周期成立」。

### 真正需要手写 `for<'a>` 的场景

日常大部分时候编译器会自动推导 HRTB（`fn(&str) -> &str` 参数位置
自动就是高阶的）。需要**手写**的主要是闭包和 trait 对象：

```rust
// 一个闭包，对任意生命周期的输入都成立：
let strip: Box<dyn for<'a> Fn(&'a str) -> &'a str> =
    Box::new(|s: &str| s.strip_prefix(">>>").unwrap_or(s));

assert_eq!(strip(">>> tokio"), "tokio");
```

配套代码的 `boxed_hrtb_closure` 就是这个例子。注意闭包体内**不能
捕获带生命周期的东西**——捕获了 `&'x str` 的闭包只对 `'a = 'x`
成立，就不再是 `for<'a>` 了。这就是为什么「对任意生命周期成立」
的闭包只能捕获拥有值或 `'static` 数据。

### 在报错信息里认出它

学 HRTB 最实际的收益是读得懂这类报错：

```text
error[E0277]: the trait bound `for<'a> Fn(&'a str) -> &'a str` is not
satisfied
```

翻译：你需要一个「对任意生命周期都成立」的函数/闭包，但给到的那个
只对某个具体生命周期成立。九成是闭包捕获了非 'static 的引用。

## 29.7 真实报错怎么读

**例 1：const 参数类型不合法**

```text
error: `String` is forbidden as the type of a const generic parameter
```

const 参数只接受整型/`bool`/`char`。想要「字符串常量参数」，当前
只能用 `&'static str`……也不行——用枚举或 `u8` 当标签代替。

**例 2：GAT 关联类型缺 where 子句**

```text
error[E0309]: unknown lifetime
note: the associated type may not live long enough
help: consider adding an explicit lifetime bound
      `type Item<'a> = ... where Self: 'a`
```

GAT 的关联类型带 `'a` 时，编译器通常要求 `where Self: 'a`
（实现者的每个生命周期都要覆盖 `'a`）。报错里的 help 就是答案，
照抄即可。

**例 3：RPITIT 想返回不同类型**

```text
error[E0308]: mismatched types
```

同一个 trait 方法的**同一个实现**里，两个分支返回 `&self.0` 和
`format!(...)` 会报 E0308——一个 impl 的 RPIT 只能对应一个隐藏类型。
想返回「多种可能」，返回 `Box<dyn Trait>`（或重新设计让实现者声明
关联类型）。

## 29.8 常见坑速查

| 坑 | 症状 | 解法 |
| --- | --- | --- |
| 把 const 参数当运行时值 | 函数体里 `println!("{N}")` 编译不过 | `const LOCAL: usize = N;` 或改成普通参数 |
| 想要 `where N > 0` | E0277 或语法错误 | 做不到；改用 `assert!` 运行时检查或类型驱动设计 |
| GAT 忘写 `where Self: 'a` | E0309 | 按编译器 help 补上 |
| RPITIT 的实现里返回两种类型 | E0308 | `Box<dyn Trait>` 或关联类型 |
| RPITIT 想要 `dyn Trait` 对象 | 不是对象安全 | 关联类型或 `Box<dyn>` 返回 |
| 闭包捕获引用后想当 `for<'a>` 用 | E0277 `for<'a> Fn(...)` not satisfied | 闭包只能捕获拥有值/'static 数据 |
| `[T; N]` 和 `Vec<T>` 混用报错 | E0308 expected `Vec<_>`, found `[_; 3]` | `.into()`/`.to_vec()` 转换，或泛型写 `impl IntoIterator` |

## 29.9 练习

1. 写一个 `fn dot<const N: usize>(a: [f64; N], b: [f64; N]) -> f64`，
   并验证 `[1.0, 2.0] · [3.0, 4.0] = 11.0`。
2. 给 `LendingIterator` 写第二个实现 `Chars<'a>`：逐字符借出
   `&'a str` 里的一段（提示：每次 `next` 切下第一个字素簇）。
3. 定义 `trait Provider { fn get(&self) -> impl fmt::Debug; }`，
   写两个实现分别返回 `i32` 和 `Vec<String>`，然后用
   `dbg!` 打印两个结果。
4. 把 `boxed_hrtb_closure` 里的闭包改成捕获一个 `&'a str` 前缀
   （`|s: &str| s.strip_prefix(prefix)`），观察编译器报什么错，
   并解释为什么。

### 参考答案

<details>
<summary>点开前先自己写</summary>

```rust
// 1. dot：const 参数保证两个数组等长——长度不等直接编译不过。
fn dot<const N: usize>(a: [f64; N], b: [f64; N]) -> f64 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}
assert_eq!(dot([1.0, 2.0], [3.0, 4.0]), 11.0);
// dot([1.0], [1.0, 2.0])   // E0308：[_; 1] 和 [_; 2] 不同类型
```

```rust
// 2. 逐「字符」借出（这里按 char 切，严谨版用第 18 章的字素簇）：
struct Chars<'a> { rest: &'a str }
impl LendingIterator for Chars<'_> {
    type Item<'a> = &'a str where Self: 'a;
    fn next(&mut self) -> Option<Self::Item<'_>> {
        let mut it = self.rest.char_indices();
        let (_, first) = it.next()?;
        let end = it.next().map_or(self.rest.len(), |(i, _)| i);
        let out = &self.rest[..end];
        self.rest = &self.rest[end..];
        Some(out)
    }
}
```

```rust
// 3. RPITIT 的两个实现返回不同 Debug 类型：
struct P1;
struct P2;
impl Provider for P1 {
    fn get(&self) -> impl fmt::Debug { 42 }
}
impl Provider for P2 {
    fn get(&self) -> impl fmt::Debug { vec![String::from("x")] }
}
dbg!(P1.get());   // 42
dbg!(P2.get());   // ["x"]
```

```rust
// 4. 捕获引用后闭包的类型是 Fn(&'b str) -> Option<&'b str> 的某个
//    具体实例（'b 被捕获值钉死），不再满足 for<'a>。
//    报错形如：
//    error[E0277]: expected a `std::ops::Fn<(&str,)>` closure, found
//    a closure with an inferred lifetime
//    要对任意生命周期成立，闭包就不能捕获带生命周期的数据——
//    把 prefix 改成 String（拥有值）即可恢复 for<'a>。
```

</details>

## 29.10 小结

- const 泛型让编译期常量（整型/`bool`/`char`）参与类型，`[T; N]`
  一套实现覆盖所有长度，还能把业务规则（矩阵形状）编码成编译错误。
- 它是「最小可用版」：不能有任意类型参数，不能条件 impl，const 参数
  不出现在运行时表达式里。
- GAT 让关联类型带泛型参数，`type Item<'a>` 使 trait 能表达「借出」
  语义——标准 `Iterator` 做不到的事。
- RPITIT（1.75+）让 trait 方法直接返回 `impl Trait`，返回类型的
  决定权从实现者挪到签名；代价是不能用于 `dyn` 对象。
- `for<'a>` 表达「对所有生命周期成立」，函数指针自动推导，闭包和
  trait 对象需要手写；闭包一旦捕获引用就失去 `for<'a>` 资格。

下一章把这些类型系统知识带到 async 世界——`Pin<&mut Self>` 那个
从第 16 章遗留至今的悬问，答案就藏在「自引用类型」里。
