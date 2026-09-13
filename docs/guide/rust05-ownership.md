# 第 5 章 · 所有权与借用

前面四章的代码里一直有几个「怪现象」：`String` 传进函数之后外面就不能用了，`for x in v` 循环一圈之后 `v` 也没了，加个 `&` 又都好了。这些不是零散的语法规定，而是同一套机制的三种表现，这套机制叫**所有权**。

它是 Rust 最出名的部分，也是最容易劝退的部分。但换个角度看：所有权规则换来的是**不用 GC 也能保证内存安全**。编译器在编译期就把「谁拥有这块内存、什么时候释放、谁能同时读写」算清楚，运行时不需要垃圾回收器，也不需要引用计数，性能开销是零。

这一章的建议读法：把示例代码敲一遍，然后**故意改错几处**，对照 5.11 的报错原文看编译器怎么拦你。所有权规则光看记不住，被报错拦几次就记住了。

本章配套代码在 `src/rust05_ownership/mod.rs`，执行 `cargo run` 可以看到全部输出。

## 5.1 值存在哪：栈与堆

要理解所有权，先要分清值存在哪里：

```rust
let n = 42;                    // 大小固定，放在栈上
let s = String::from("rust");  // 栈上放「指针 + 长度 + 容量」，字符数据放堆上
```

栈上的数据大小在编译期已知，进函数、出函数时整块挪动，速度快；堆上的数据大小运行时才能确定，需要有人申请、也需要有人释放。

| 值 | 存在哪 | 谁负责释放 |
| --- | --- | --- |
| `i32`、`bool`、`char`、元素固定的数组 | 栈 | 离开作用域自动释放 |
| `String` 的字符、`Vec<T>` 的元素 | 堆 | 栈上的 `String` / `Vec` 负责 |
| `&str`、`&[T]`、`&String` | 栈上的一个引用，指向别处 | 不负责，它只是借用 |

「谁负责释放」是 C 和 Rust 分野的地方。C 里堆内存要手动 `free`，忘了就泄漏，释放两次就崩溃；有 GC 的语言靠运行时追踪。Rust 选了第三条路：让**每个值都有一个明确的所有者**，所有者一走，值就释放。这件事完全在编译期决定，所以既不漏也不重。

## 5.2 三条规则

所有权的全部内容可以压成三句话：

1. Rust 里每一个值都有一个**所有者**（owner）。
2. 同一个值在**同一时刻只有一个所有者**。
3. 所有者**离开作用域**时，值被释放。

```rust
{
    let s = String::from("rust"); // s 是这个 String 的所有者
    println!("{s}");
}                                  // 作用域结束，s 离开，堆上的字符被释放
```

这三条规则本身很好记，难的是它们在各种场景下的推论：赋值会发生什么、传参会发生什么、借用算不算所有者——接下来几节逐个展开。

## 5.3 作用域与 `Drop`

释放发生在「所有者离开作用域」的那一刻。同一作用域里有多个变量时，释放顺序是**声明顺序的逆序**（后声明的先释放）：

```rust
struct Noisy(&'static str);

impl Drop for Noisy {
    fn drop(&mut self) {
        println!("释放 {}", self.0);
    }
}

fn show_drop_order() {
    let _first = Noisy("first");
    let _second = Noisy("second");   // 后声明
}
```

实测输出：

```text
    （函数即将返回：second 先释放，first 后释放）
    释放 second
    释放 first
```

内层代码块同理，块一结束就释放：

```text
    内层块结束前
    释放 内层变量
    内层块已经结束
```

`Drop` trait 可以理解成「这个类型被释放时要做什么」。标准库已经替 `String`、`Vec`、`File` 实现好了：`String` 释放堆上的字符，`File` 关闭文件描述符，`MutexGuard` 解锁。你自己的类型只要持有这类资源，编译器就会沿着字段自动调用它们的 `Drop`。

也可以手动提前释放：`drop(owned)` 之后，值立刻消失，不用等作用域结束（实测会在 `drop(owned) 之后立刻释放` 这句话之前打印「释放 手动释放」）。

## 5.4 移动（move）：所有权换了主人

把 `String` 赋给另一个变量，发生的是**移动**：

```rust
let original = String::from("rust");
let moved = original;         // 所有权从 original 移到 moved
println!("{moved}");          // 可以
// println!("{original}");    // 取消注释会报 E0382：borrow of moved value
```

关键点：**移动不是深拷贝**。`String` 在栈上是「指针 + 长度 + 容量」三个字段，移动只把这三个字段复制过去，堆上的字符数据一块都没动。移动完成后，`original` 在编译期就被标记为「已失效」——名字还在，但不允许再读写。

如果允许两个变量都指向同一块堆内存，它们各自离开作用域时都会去释放它，那就是**双重释放**（double free），C 里最经典的崩溃和安全漏洞来源。Rust 的办法很直接：移动之后，原来的名字就不能再用了。

传参也是移动，返回值则可以把所有权交回来：

```rust
fn take_and_report(text: String) -> usize { text.len() }   // 所有权进来
fn give_back(text: String) -> String { text }               // 所有权出去

let owned = String::from("ownership");
let len = take_and_report(owned);   // owned 被移动进函数
// println!("{owned}");             // 这里已经不能用了
let again = give_back(String::from("back"));  // 函数把所有权还回来
```

实测输出里这两行能看清整个过程：

```text
    调用前 owned 长度 = 9
    take_and_report 拿到了「ownership」，长度 9
    函数返回后只剩长度 9，owned 这个名字已经不能用了
```

这种「交出去、再还回来」的写法能用，但很啰嗦。Rust 的解决办法是借用（5.6），也就是说：**大部分时候根本不需要交出所有权**。

## 5.5 `Copy` 与 `clone`

并不是所有类型赋值都会移动。像 `i32` 这种「完全存在栈上、复制一份没有额外代价」的类型，赋值就是简单复制，原变量照常可用：

```rust
let n = 42;
let m = n;
println!("{n} {m}");        // 两个都能用
```

这类类型实现了 `Copy` trait。判断标准很简单：**只由 `Copy` 类型组成、且不需要释放任何资源**。常见的有：

| 是 `Copy` | 不是 `Copy` |
| --- | --- |
| `i32` / `u64` / `f64` / `bool` / `char` | `String` |
| `&T`（引用本身是 `Copy`） | `Vec<T>` |
| `[i32; 3]`、`(i32, bool)` 这样元素全 `Copy` 的复合类型 | `Box<T>`、`File` |

对照一下实测输出：元组 `(1, 2.5, true, 'x')` 复制之后原变量还能打印；而 `Vec` 移动之后只能叫新名字。

`Copy` 是「轻量复制」，`clone` 则是**显式的、可能很贵的复制**：

```rust
let a = String::from("data");
let mut b = a.clone();       // 堆上重新分配一份
b.push_str("-modified");
println!("a = {a} 没被动过，b = {b}");
```

实测 `a = data 没被动过，b = data-modified`。`clone` 会把堆上的数据真真切切复制一遍，所以在循环里、在热路径上乱 `clone` 是要花钱的。编译器报 `E0382` 时经常建议 `clone()`，那是**能让代码编过**的最快办法，但不一定是最好的办法——先问一句：这里真的需要两个所有者吗？还是只需要借来看一眼？

## 5.6 借用（`&`）：只看不拿

借用就是引用：**用 `&` 拿到访问权，但不取得所有权**。函数参数用引用，调用方的变量就不会被移动：

```rust
fn count_chars(text: &str) -> usize {
    text.chars().count()
}

let text = String::from("所有权");
let counted = count_chars(&text);
println!("{counted}，借用之后 text 还能用: {text}");
```

实测 `count_chars(&text) = 3，借用之后 text 还能用: 所有权`。函数里拿到的 `&str` 指向 `text` 拥有的那块内存，函数返回时借用结束，谁也没有释放任何东西。

传参时 `&text`（`&String`）能自动转成 `&str`，这叫 **deref coercion**，所以同一个函数既能收 `String` 也能收字面量：

```text
    String 传 &owned: 5
    字面量直接传: 7
```

借用有一条铁律：**引用必须始终有效**。所以不能把函数内部的局部变量借出去：

```rust
fn make() -> &str {
    let local = String::from("local");
    &local[..]          // 编译失败：local 在函数结束时就被释放了
}
```

报错是 `E0515: cannot return value referencing local variable 'local'`（原文见 5.11）。`local` 的生命在函数结束时终止，返回的引用会立刻变成悬垂指针；Rust 不允许这种事发生，所以只能在编译期拒绝。

## 5.7 可变借用（`&mut`）与借用规则

只想改内容时，用可变借用：

```rust
fn shout(text: &mut String) {
    text.push('!');
    text.make_ascii_uppercase();
}

let mut doc = String::from("rust");
shout(&mut doc);
println!("{doc}");     // RUST!
```

这里有两个「必须」：变量本身要声明成 `let mut`（否则报 `E0596`），传参要写 `&mut doc`。把 `mut` 写进类型里，是为了让「这个函数会改我的值」在签名上就看得见。

借用检查器管的就三件事：

| 同时存在的借用 | 允许吗 |
| --- | --- |
| 任意多个 `&T` | ✅ 只读，互不干扰 |
| 恰好一个 `&mut T` | ✅ 独占，别人不能看也不能改 |
| `&mut T` 和任何 `&T` / `&mut T` 同时存在 | ❌ |

实测里两个不可变借用可以同时存在：`多个不可变借用可以共存: [1, 2, 3] [1, 2, 3]`。

为什么这么严？因为「读」和「写」同时发生是一切内存错误的源头：读取时数据被释放（use after free）、遍历时容器扩容导致指针失效、多线程下的数据竞争，本质都是同一件事。Rust 把这条规则放在编译期检查：**要么大家一起只读，要么一个人独占修改**。这样既不需要 GC，也不需要读写锁，代价为零。

顺带说一句，这条规则也是后面并发那一章的基础：`&T` 可以安全地跨线程共享，`&mut T` 天然独占，所以 Rust 能「无畏并发」（第 13 章）。

## 5.8 借用到「最后一次使用」为止

借用规则听起来很严，但实际写代码时并没有那么难受，因为**借用的作用域不是代码块的范围，而是到它最后一次被使用为止**。这个规则叫 NLL（non-lexical lifetimes）：

```rust
let mut data = vec![10, 20, 30];
let head = &data[0];
println!("先看一眼 head = {head}");
data.push(40);        // 借用在上一行已经用完了，这里可以改
println!("借用已经结束，可以改: {data:?}");
```

实测输出：

```text
    先看一眼 head = 10
    借用已经结束，可以改: [10, 20, 30, 40]
```

如果反过来，把 `println!("{head}")` 挪到 `data.push(40)` 后面，编译器立刻报 `E0502`：不可变借用「later used here」，而中间发生了可变借用。

所以遇到借用冲突时，第一反应不该是「加 clone」，而是**调整使用顺序**或者**缩小借用的范围**：先在借用还活着的时候把该读的读完，再动它。Rust 1.31 引入 NLL 之前，借用确实要活到代码块结束，那时候这种代码是编不过的——现在能过，是因为编译器真的去算了「最后一次使用」。

## 5.9 切片：`&str` 与 `&[T]`

切片是「指向连续一段数据的引用 + 长度」，它**不拥有数据**，只是借用了其中一段：

```rust
let sentence = String::from("hello rusty world");
let hello = &sentence[..5];     // 前 5 个字节
let rusty = &sentence[6..11];   // 第 6 到第 10 个字节
```

范围写法的几种形式：

| 写法 | 含义 |
| --- | --- |
| `&s[..]` | 全部 |
| `&s[a..b]` | 从 `a` 到 `b`（不含 `b`） |
| `&s[..b]` / `&s[a..]` | 省略的一侧取到头 |

数组和 `Vec` 一样能切：`&scores[1..3]`。实测 `数组切片: [90, 75]，长度 2`。

**字符串切片按字节算，而中文一个字占 3 个字节**，所以按「字」去切很容易切在字符中间：

```rust
let chinese = String::from("中文切片");
let bad = &chinese[..1];   // 运行时 panic
```

```text
thread 'main' panicked at src/main.rs:3:21:
end byte index 1 is not a char boundary; it is inside '中' (bytes 0..3 of string)
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
```

（新版本 rustc 的 panic 头里还会带上线程编号，`thread 'main' (12345) panicked at ...`，那串数字每次不一样。）

这不是类型错误，编译器拦不住，只能在运行时 panic。安全的做法是按字符取：

```rust
let safe = chinese.chars().take(2).collect::<String>();   // 中文
```

如果下标的来源不确定，用 `get` 拿到 `Option` 比直接下标更稳：`chinese.get(..1)` 返回 `None` 而不是 panic（`Option` 在第 6 章讲）。

切片最实用的地方是「函数返回字符串里的一段，而不是新建一个 `String`」：

```rust
fn first_word(text: &str) -> &str {
    match text.find(' ') {
        Some(index) => &text[..index],
        None => text,
    }
}
```

实测 `first_word 返回切片而不是新 String: hello`。注意这个函数没有分配任何内存，返回的 `&str` 直接指向传进来的那串字符。

## 5.10 `String` 与 `&str` 的取舍

新手最常问的一个问题：到底该写 `String` 还是 `&str`？

| 场景 | 用什么 | 原因 |
| --- | --- | --- |
| 要拥有并修改文本（读文件、拼接） | `String` | 需要自己那块堆内存 |
| 函数只读地看一段文本 | `&str` | 谁都能传，不夺走所有权 |
| 结构体字段保存文本 | `String` | 借用字段要写生命周期（第 10 章） |
| 字符串字面量 | `&'static str` | 编译进二进制，程序全程有效 |
| 要把借用变成长期拥有 | `.to_string()` / `String::from(...)` | 显式复制一份 |

最有用的一条经验是：**函数参数优先收 `&str`**。

```rust
fn count_chars(text: &str) -> usize { text.chars().count() }

let owned = String::from("owned");
count_chars(&owned);        // String 借用进来
count_chars("literal");     // 字面量直接传
```

反过来，如果参数写成 `String`，调用方就得把所有权交出去，或者被迫 `clone()` 一次。收 `&str` 则两种调用都成立，函数体里能做的一切（读长度、切分、比较）都不受影响。

同理，返回值能借就借：`first_word` 返回 `&str` 而不是 `String`，省掉一次堆分配；只有当结果确实无法指向输入（比如拼接后的新字符串）时，才返回 `String`。

一个具体的取舍例子：

```rust
// 借用版：不分配，但返回值活不过 sentence
fn first_word(text: &str) -> &str { /* ... */ }

// 拥有版：分配一次，但可以独立存活
fn first_word_owned(text: &str) -> String {
    first_word(text).to_string()
}
```

需要把结果存进结构体、跨线程传递或者比输入活得更久时，就选拥有版；其余情况借出去更省。

## 5.11 九个真实报错怎么读

这一章的报错值得逐个认识，因为以后写 Rust 会经常撞上它们。下面全部来自实际编译，行号是各自最小例子的行号。

**案例 1：移动之后再使用（`E0382`）**

```rust
let s = String::from("hello");
let t = s;
println!("{s}");
```

```text
error[E0382]: borrow of moved value: `s`
 --> src/main.rs:4:16
  |
2 |     let s = String::from("hello");
  |         - move occurs because `s` has type `String`, which does not implement the `Copy` trait
3 |     let t = s;
  |             - value moved here
4 |     println!("{s}");
  |                ^ value borrowed here after move
  |
help: consider cloning the value if the performance cost is acceptable
  |
3 |     let t = s.clone();
  |              ++++++++
```

注意报错里那句 `s has type String, which does not implement the Copy trait`——编译器直接告诉了你「移动」的原因。

**案例 2：传进函数之后再使用（`E0382`，附赠最有用的一条 note）**

```rust
fn consume(s: String) -> usize { s.len() }

fn main() {
    let s = String::from("hello");
    let n = consume(s);
    println!("{n} {s}");
}
```

```text
error[E0382]: borrow of moved value: `s`
 --> src/main.rs:6:20
  |
4 |     let s = String::from("hello");
  |         - move occurs because `s` has type `String`, which does not implement the `Copy` trait
5 |     let n = consume(s);
  |                     - value moved here
6 |     println!("{n} {s}");
  |                    ^ value borrowed here after move
  |
note: consider changing this parameter type in function `consume` to borrow instead if owning the value isn't necessary
 --> src/main.rs:1:15
  |
1 | fn consume(s: String) -> usize { s.len() }
  |    -------    ^^^^^^ this parameter takes ownership of the value
  |    |
  |    in this function
help: consider cloning the value if the performance cost is acceptable
  |
5 |     let n = consume(s.clone());
  |                      ++++++++
```

`note` 那行建议的就是本章 5.10 的结论：参数如果可以，改成 `&str` / `&T`。

**案例 3：不可变借用还没用完，就发生了可变借用（`E0502`）**

```text
error[E0502]: cannot borrow `v` as mutable because it is also borrowed as immutable
 --> src/main.rs:4:5
  |
3 |     let first = &v[0];
  |                  - immutable borrow occurs here
4 |     v.push(4);
  |     ^^^^^^^^^ mutable borrow occurs here
5 |     println!("{first}");
  |                ----- immutable borrow later used here
```

最后一行 `immutable borrow later used here` 是修复线索：把 `println!` 挪到 `push` 之前就好了（5.8 的 NLL）。

**案例 4：同时要两个可变借用（`E0499`）**

```text
error[E0499]: cannot borrow `s` as mutable more than once at a time
 --> src/main.rs:4:13
  |
3 |     let a = &mut s;
  |             ------ first mutable borrow occurs here
4 |     let b = &mut s;
  |             ^^^^^^ second mutable borrow occurs here
5 |     println!("{a} {b}");
  |                - first borrow later used here
```

**案例 5：借用期间把值移动走（`E0505`）**

```text
error[E0505]: cannot move out of `s` because it is borrowed
 --> src/main.rs:4:13
  |
2 |     let s = String::from("hi");
  |         - binding `s` declared here
3 |     let r = &s;
  |             -- borrow of `s` occurs here
4 |     let t = s;
  |             ^ move out of `s` occurs here
5 |     println!("{r} {t}");
  |                - borrow later used here
```

**案例 6：变量没声明 `mut` 就想可变借用（`E0596`）**

```text
error[E0596]: cannot borrow `v` as mutable, as it is not declared as mutable
 --> src/main.rs:3:5
  |
3 |     v.push(1);
  |     ^ cannot borrow as mutable
  |
help: consider changing this to be mutable
  |
2 |     let mut v = Vec::new();
  |         +++
```

`help` 里那三个 `+++` 就是让编译器帮你加 `mut`。

**案例 7：临时值被提前释放，引用悬空（`E0716`）**

```rust
let s = String::from("hi").as_str();
println!("{s}");
```

```text
error[E0716]: temporary value dropped while borrowed
 --> src/main.rs:2:13
  |
2 |     let s = String::from("hi").as_str();
  |             ^^^^^^^^^^^^^^^^^^         - temporary value is freed at the end of this statement
  |             |
  |             creates a temporary value which is freed while still in use
3 |     println!("{s}");
  |                - borrow later used here
  |
help: consider using a `let` binding to create a longer lived value
  |
2 ~     let binding = String::from("hi");
3 ~     let s = binding.as_str();
  |
```

临时 `String` 在语句结束时就被释放了，`&str` 指向了已经不存在的内存。`help` 给的修法正是「先 `let` 绑到一个变量上，再借」。

**案例 8：返回局部变量的引用（`E0515`）**

```rust
fn make(s: &str) -> &str {
    let local = String::from("local");
    &local[..]
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

**案例 9：两个输入引用，编译器猜不出返回值借的是哪个（`E0106`）**

```rust
fn longer(a: &str, b: &str) -> &str {
    if a.len() >= b.len() { a } else { b }
}
```

```text
error[E0106]: missing lifetime specifier
 --> src/main.rs:1:32
  |
1 | fn longer(a: &str, b: &str) -> &str {
  |              ----     ----     ^ expected named lifetime parameter
  |
  = help: this function's return type contains a borrowed value, but the signature does not say whether it is borrowed from `a` or `b`
help: consider introducing a named lifetime parameter
  |
1 | fn longer<'a>(a: &'a str, b: &'a str) -> &'a str {
  |          ++++     ++          ++          ++
```

只有**一个**输入引用时（比如 `first_word(text: &str) -> &str`），编译器能推断出返回值借的就是它，所以不用写生命周期；有两个输入引用时就必须显式标注。这是第 10 章的主题。

## 5.12 常见坑速查

| 现象 | 原因 | 解法 |
| --- | --- | --- |
| 用了之后报 `borrow of moved value` | 值已经被移动走了 | 改成借用 `&`，或确需两份时 `clone()` |
| 函数参数写 `String`，调用方之后不能用 | 参数拿走了所有权 | 参数改成 `&str` / `&T` |
| `for x in v` 之后 `v` 不能用了 | `for` 按值迭代移动了 `Vec` | 写 `for x in &v` |
| 一边遍历一边改集合，编译器报错 | 遍历是借用，修改要可变借用 | 先收集要改的，循环外再改（第 7 章有惯用法） |
| 两个 `&mut` 报 `E0499` | 可变借用必须独占 | 缩小作用域，先后使用而不是同时持有 |
| 借用期间移动/释放，报 `E0505` | 借用还没结束 | 先让借用用完，再移动 |
| 想改却没加 `mut`，报 `E0596` | 绑定是不可变的 | `let mut v = ...`，或给参数加 `&mut` |
| 从 `Vec`/下标里取值报 `E0507` | 不能把元素从容器里移出来 | `&v[0]` 借用，或 `v[0].clone()` |
| 返回局部变量的引用，报 `E0515` | 局部变量出了函数就没了 | 返回 `String` 等拥有型数据 |
| `String::from("x").as_str()` 报 `E0716` | 临时值被提前释放 | 先 `let` 绑定，再借用 |
| 中文按 `&s[..1]` 切片 panic | 字符串切片按字节，中文占 3 字节 | 用 `chars().take(n)` 或 `s.get(..n)` |
| 结构体里想存借用 | 借用需要生命周期参数 | 先存 `String`，或等第 10 章 |

## 5.13 练习

1. 写 `fn last_word(text: &str) -> &str`：返回最后一个空格之后的部分，没有空格就返回整串。要求返回切片而不是新建 `String`。
2. 写 `fn double_all(values: &mut Vec<i32>)`：把每个元素乘以 2，就地修改，不返回新容器。
3. 写 `fn longest_word(words: &[String]) -> &str`：返回最长的那个单词，空切片返回 `""`；想一想为什么这里不需要手写生命周期参数。
4. 下面这段代码编不过，请指出原因，并分别用「借用」和 `clone` 两种方式修好，说明你更倾向哪种：

   ```rust
   fn main() {
       let name = String::from("rust");
       let upper = name;
       println!("{name} -> {upper}");
   }
   ```

5. 写 `fn longer(a: &str, b: &str) -> &str`，让它在 `a` 不短于 `b` 时返回 `a`，否则返回 `b`。它会编译失败，请把 `E0106` 的报错原文抄下来，想一想编译器缺的是哪条信息。

（第 2 题是 5.7 节 `shout` 的同类写法，这里不再单独给答案。）

### 参考答案

::: details 第 1 题

```rust
fn last_word(text: &str) -> &str {
    match text.rfind(' ') {
        Some(index) => &text[index + 1..],
        None => text,
    }
}

fn main() {
    println!("[{}]", last_word("hello rusty world")); // [world]
    println!("[{}]", last_word("single"));            // [single]
    println!("[{}]", last_word(""));                  // []
}
```

`rfind` 从右边找空格，找不到就返回 `None`（整串就是一个词）。和 `first_word` 一样，返回的切片直接指向传进来的字符串，没有任何分配。

:::

::: details 第 3 题

```rust
fn longest_word(words: &[String]) -> &str {
    let mut longest: &str = "";
    for word in words {
        if word.len() > longest.len() {
            longest = word;      // &String 自动转成 &str
        }
    }
    longest
}

fn main() {
    let words = vec![String::from("rust"), String::from("ownership"), String::from("borrow")];
    println!("{}", longest_word(&words)); // ownership
    println!("{}", longest_word(&[]));    // 空串
}
```

参数只有一个引用输入 `&[String]`，返回值只能借自它，所以编译器能自动推断出生命周期，不用手写。如果参数变成两个引用，就必须标了（见第 5 题）。

:::

::: details 第 4 题

`let upper = name;` 把 `name` 移动走了，`upper` 成为新的所有者，所以后面的 `println!("{name}")` 报 `E0382`。

```rust
// 修法一：借用，name 仍然是所有者
let name = String::from("rust");
let upper = &name;
println!("{name} -> {upper}");
```

```rust
// 修法二：clone，各持一份
let name = String::from("rust");
let upper = name.clone();
println!("{name} -> {upper}");
```

优先用借用：它不分配内存，也不改变所有权关系。只有当 `upper` 需要比 `name` 活得更久（比如作为返回值、存进结构体、送去别的线程）时，才需要 `clone`。

:::

::: details 第 5 题

```text
error[E0106]: missing lifetime specifier
 --> src/main.rs:1:32
  |
1 | fn longer(a: &str, b: &str) -> &str {
  |              ----     ----     ^ expected named lifetime parameter
  |
  = help: this function's return type contains a borrowed value, but the signature does not say whether it is borrowed from `a` or `b`
help: consider introducing a named lifetime parameter
  |
1 | fn longer<'a>(a: &'a str, b: &'a str) -> &'a str {
  |          ++++     ++          ++          ++
```

缺的信息是：**返回的引用借自谁**。两个输入引用各有一条独立的生命周期，编译器不能替你选。加上 `<'a>` 标注之后，函数就承诺「返回值和 `a`、`b` 活一样久」，调用方也能据此检查。第 10 章会专门讲生命周期。

:::

## 5.14 小结

- 每个值有且只有一个所有者，所有者离开作用域时值被释放；释放顺序是声明顺序的逆序。

- 赋值和传参默认是**移动**：栈上的元数据复制过去，堆上的数据不动，原变量在编译期失效，从根上避免双重释放。

- `Copy` 类型赋值是复制（`i32`、`bool`、`char`、引用、元素全 `Copy` 的复合类型）；`clone` 是显式的、可能很贵的复制，能用借用解决就别 `clone`。

- 借用（`&`）不取得所有权；同一时间要么多个 `&T`，要么一个 `&mut T`，这条规则换来的是零开销的内存安全。

- 借用的生命到**最后一次使用**为止（NLL），所以遇到借用冲突先调整顺序、缩小范围，而不是急着 `clone`。

- 切片不拥有数据，`&str` / `&[T]` 都是「引用 + 长度」；字符串切片按**字节**算，中文要用 `chars()` 或 `get()`。

- 参数优先收 `&str` / `&[T]` / `&T`，返回值能借就借；要跨作用域存活或要修改时才用 `String` 这类拥有型数据。

- 报错速记：`E0382` 移动后使用、`E0502` 借用冲突、`E0499` 多个可变借用、`E0505` 借用期间移动、`E0596` 忘了 `mut`、`E0716` 临时值提前释放、`E0515` 返回局部引用、`E0106` 缺生命周期标注。

下一章讲**结构体与枚举**：数据的归属关系清楚了，接下来该学怎么把相关的值组合成自己的类型，以及用 `match` 把它们拆开。
