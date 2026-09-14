# 第 15 章 · 宏

从第 1 章开始我们就一直在用宏：`println!`、`format!`、`vec!`、`assert_eq!`、`matches!`。这一章看它们是怎么定义的，以及**什么时候该自己写一个**。

Rust 的宏分两类：

| 类型 | 长什么样 | 用途 | 放在哪 |
| --- | --- | --- | --- |
| 声明宏 | `macro_rules! name { ... }` | 按模式匹配生成代码，日常宏基本都是这种 | 任何 crate |
| 过程宏 | `#[derive(X)]`、`#[attr]`、`name!(...)` | 读 Rust 代码、生成 Rust 代码 | 独立的 `proc-macro` crate |

宏的本质是**编译期展开**：`println!("{x}")` 在编译后就是一段普通代码，运行期没有任何额外开销。代价是**写起来更绕、读起来更难**——所以社区的共识是：**能用函数、泛型、trait 解决的问题，就别用宏**。

本章配套代码在 `src/rust15_macros/mod.rs`，执行 `cargo run` 可以看到全部输出。

## 15.1 宏和函数的区别

| | 函数 | 宏 |
| --- | --- | --- |
| 参数个数 | 固定 | 可以任意个（`println!`） |
| 参数类型 | 运行时值 | **语法片段**（表达式、类型、标识符……） |
| 展开时机 | 调用 | 编译期 |
| 能不能生成代码 | 不能 | 能（生成 `struct`、`impl`、函数） |
| 调用写法 | `f(x)` | `m!(x)` |
| 可读性 | 好 | 差，报错信息也更绕 |

一句判断标准：**当「参数的数量或种类」本身在变化时**，宏才必要。`println!` 之所以是宏，就是因为它要接受任意多个参数，还要在编译期检查格式串。

## 15.2 第一个 `macro_rules!`

```rust
macro_rules! say_hello {
    () => {
        println!("你好，宏！");
    };
}

say_hello!();
```

结构可以拆成三部分：

```text
macro_rules! 宏名 {
    (匹配模式) => { 展开结果 };
    (另一套模式) => { 另一套展开 };
}
```

- 一个宏可以有多条规则，**从上到下依次尝试**，第一条匹配上的生效。
- 规则之间用 `;` 分隔。
- 参数以 `$` 开头；匹配失败时报 `no rules expected the token ...`（见 15.11）。

实测 `say_hello!()` 输出 `你好，宏！`。

**顺序很重要**：`macro_rules!` 是**按文本顺序生效**的，必须先定义后使用。写反了会报：

```text
error: cannot find macro `say_hello` in this scope
 --> src/main.rs:2:5
  |
2 |     say_hello!();
  |     ^^^^^^^^^ consider moving the definition of `say_hello` before this call
  |
note: a macro with the same name exists, but it appears later
```

这条报错的 `help` 已经很贴心了：把定义挪到调用之前即可。

## 15.3 片段说明符：`$x:什么`

`$` 后面的冒号部分叫**片段说明符**（fragment specifier），它规定这里能匹配哪种语法：

| 说明符 | 匹配 | 例子 |
| --- | --- | --- |
| `expr` | 表达式 | `1 + 2`、`foo()` |
| `ident` | 标识符 | `name`、`Point` |
| `ty` | 类型 | `Vec<i32>`、`&str` |
| `pat` | 模式 | `Some(x)`、`_` |
| `path` | 路径 | `std::mem::swap` |
| `stmt` | 语句 | `let x = 1;` |
| `block` | 块 | `{ ... }` |
| `literal` | 字面量 | `42`、`"hi"` |
| `tt` | 单个 token 树 | 任意一个 token，或一对括号 |
| `item` | 项 | `fn`、`struct`、`impl`…… |
| `vis` | 可见性 | `pub`、`pub(crate)`、空 |
| `lifetime` | 生命周期 | `'a` |

```rust
macro_rules! show {
    ($label:expr, $value:expr) => {
        println!("    {} = {}", $label, $value);
    };
}

let width = 3;
show!("width * 2", width * 2);       // width * 2 = 6
show!("字符串长度", "hello".len());   // 字符串长度 = 5
```

其中 `tt` 最灵活——`$($arg:tt)*` 是「把剩下的 token 原样传给别人」的标准写法，`println!` 就是靠它把参数转发给 `format_args!` 的。本章的 `log_tagged!` 用了同样的手法：

```rust
macro_rules! log_tagged {
    ($tag:ident, $($arg:tt)*) => {
        println!("    [{}] {}", stringify!($tag), format!($($arg)*));
    };
}

log_tagged!(info, "value = {value}, 类型 = {}", type_name_of_val(&value));
```

实测输出 `[info] value = 42, 类型 = u32`——`stringify!` 把标识符变成了字符串 `"info"`。

## 15.4 重复匹配：`$(...)` 加 `+` / `*` / `?`

重复是宏最强大的部分：

| 写法 | 含义 |
| --- | --- |
| `$( ... ),+` | 一个或多个，用逗号分隔 |
| `$( ... ),*` | 零个或多个 |
| `$( ... )?` | 零个或一个（可选） |
| `$( ),?`（单独写） | 允许尾随逗号 |

自己实现一个 `vec!`：

```rust
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
```

两处重复的用法不同：

- `$($element:expr),+` 出现在**匹配模式**里：表示「一个或多个表达式，用逗号分隔」。
- `$( v.push($element); )+` 出现在**展开结果**里：表示「对每个 `element` 重复这一段」，`$element` 会依次被替换。

> 演示代码里，`my_vec!` 的调用处和 `make_container!` 所在的块都写了
> `#[allow(clippy::vec_init_then_push)]`。clippy 对普通代码会提示「别先建空
> `Vec` 再 `push`」（直接 `vec![...]` 更短），但这里展开成语句序列**正是要展示
> 的点**，所以显式放行。
>
> 顺带记住这个细节：**属性要写在调用处**（或包住调用的块上）。写在
> `macro_rules!` 定义上、或者直接挂在宏调用语句上，都不会生效——lint 是在
> 展开之后的代码上判定的。

实测：

```text
    my_vec![] = []
    my_vec![1, 2, 3] = [1, 2, 3]
    带尾随逗号 my_vec![1, 2, 3,] = [1, 2, 3]
```

注意第二条规则里的 `$(,)?`——**没有它，`my_vec![1, 2, 3,]` 会匹配失败**。给宏加上「容忍尾随逗号」是社区的通用做法（`vec!`、`println!` 都支持）。

## 15.5 递归展开

宏可以在展开结果里再调用自己，这就是**递归宏**：

```rust
macro_rules! count {
    () => { 0 };
    ($head:tt $($tail:tt)*) => { 1 + count!($($tail)*) };
}

count!(a b c)      // 展开成 1 + (1 + (1 + 0)) = 3
```

第一行是**递归出口**（空输入返回 0），第二行每次吃掉一个 `tt`、把剩下的传给自己。实测 `count!(a b c) = 3`。

递归深度有上限（默认 128），写过头会报：

```text
error: recursion limit reached while expanding `recurse!`
 --> src/main.rs:3:9
  |
3 |         recurse!()
  |         ^^^^^^^^^^
  |
  = help: consider increasing the recursion limit by adding a `#![recursion_limit = "256"]` attribute
```

（那个 `recurse!` 是无限递归的反面例子：`() => { recurse!() }`——它永远展开不完。）

## 15.6 卫生性：宏里的变量不会「串味」

宏在展开时会尽量避免和你代码里的名字打架，这个性质叫**卫生性**（hygiene）：

```rust
macro_rules! double_with_internal {
    ($value:expr) => {{
        let doubled = $value * 2;
        doubled
    }};
}

let doubled = 100;
let result = double_with_internal!(5);
```

实测 `外部的 doubled = 100`、`宏返回 = 10`——宏内部那个 `doubled` 和外部的同名变量**互不影响**，也不会报「重复定义」。

要区分两种情况：

- **宏体内自己写的标识符**（上面那个 `doubled`）是卫生的，只在展开的代码块里有效。
- **通过 `$xxx` 传进来的标识符**（比如 `$value`、`$name`）是**调用方的**，展开后就在调用方的作用域里生效——所以 `define_point!(Point, 3, 4)` 生成的 `Point` 类型，调用方可以直接用。

卫生性减少了「展开后名字撞车」的意外，但也带来一个常见副作用：**展开结果里可能出现多余的 `mut` 或未使用的变量警告**。比如本章练习的 `min_of!(3)` 只有一个参数时，`let mut min` 里的 `mut` 就没用了，编译器会警告。宏内部加 `#[allow(unused_mut)]` 就能压掉：

```rust
macro_rules! min_of {
    ($first:expr $(, $rest:expr)*) => {{
        #[allow(unused_mut)]
        let mut min = $first;
        $( if $rest < min { min = $rest; } )*
        min
    }};
}
```

## 15.7 宏的作用域与导出

`macro_rules!` 的作用域和普通项**不一样**：

- **按文本顺序生效**：只能在定义之后、同一个模块（及其子模块）里使用。
- **想要跨模块/跨 crate 使用，必须加 `#[macro_export]`**：它会把宏导出到 **crate 根**，别的 crate 就能 `use 包名::宏名;`（2018 版之后）。

```rust
#[macro_export]
macro_rules! my_vec {
    // ...
}
```

导出宏里引用自己 crate 的东西时，要用 **`$crate`**：

```rust
#[macro_export]
macro_rules! make_thing {
    () => {
        $crate::Thing::new()      // 无论被哪个 crate 调用，都指向本 crate 的 Thing
    };
}
```

如果写成 `crate::Thing`，调用方那边就会解析成**调用方的 crate**，直接报找不到——这是导出宏最常见的坑。`$crate` 保证路径永远指向「定义宏的那个 crate」。

模块内也可以用 `pub use` 重导出宏，效果类似：

```rust
mod helpers {
    macro_rules! internal_macro { () => {}; }
    pub(crate) use internal_macro;      // 让其他模块也能用
}
```

## 15.8 标准库的宏是怎么展开的

理解宏之后，`println!` 这类「魔法」就不再神秘了——它们都是普通宏，展开成普通函数调用：

| 宏 | 大致展开成 |
| --- | --- |
| `println!("{x}")` | `std::io::_print(format_args!("{x}"))` |
| `vec![1, 2, 3]` | `<[_]>::into_vec(Box::new([1, 2, 3]))` |
| `assert_eq!(a, b)` | 比较 `a`、`b`，不等就 `panic!` 并把两边的 `{:?}` 打出来 |
| `matches!(x, Some(_))` | `match x { Some(_) => true, _ => false }` |
| `write!(buf, "...")` | 调用 `std::fmt::Write::write_fmt` |

这也是为什么 `println!` 的格式串检查发生在**编译期**：`format_args!` 展开时就会校验占位符和参数是否对得上。

想亲眼看展开结果，有两个办法：

```bash
cargo install cargo-expand     # 一次性安装
cargo expand                   # 打印整个 crate 展开后的代码
cargo expand rust15_macros     # 只看某个模块
```

或者用 nightly 的 `cargo rustc -- -Zunpretty=expanded`。展开结果通常很长（一个 `println!` 能展开出几十行），**不建议在生产代码里依赖它**，但排查「宏到底做了什么」时非常有用。

## 15.9 过程宏：读代码、写代码

过程宏是「输入 Rust 代码、输出 Rust 代码」的函数，分三种：

| 种类 | 写法 | 例子 |
| --- | --- | --- |
| derive 宏 | `#[derive(MyTrait)]` | `Debug`、`Clone`、`serde::Serialize`、`thiserror::Error` |
| 属性宏 | `#[my_attr]` | `#[tokio::main]`、`#[test]`、`#[wasm_bindgen]` |
| 函数式宏 | `my_macro!(...)` | `sqlx::query!`、`include_str!` 的增强版 |

和声明宏最大的区别：

1. **它们必须放在独立的 crate 里**，`Cargo.toml` 要声明：

   ```toml
   [lib]
   proc-macro = true
   ```

   原因是编译顺序：编译器需要**先编译好**过程宏，再用它处理当前 crate 的代码，同一个 crate 里做不到这一点。

2. **它们操作的是语法树**，通常依赖三个库：

   ```toml
   [dependencies]
   syn = "2"            # 把 Rust 代码解析成语法树
   quote = "1"          # 用模板生成 Rust 代码
   proc-macro2 = "1"    # 兼容性层
   ```

3. **它们能读到的信息更多**：derive 宏能拿到结构体的字段名、类型、属性，所以才能「照着结构体自动生成 `impl`」。

```rust
// proc-macro crate 里的最小 derive（示意）
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(Hello)]
pub fn hello_derive(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let name = &ast.ident;
    quote! {
        impl Hello for #name {
            fn hello(&self) -> String {
                format!("我是 {}", stringify!(#name))
            }
        }
    }
    .into()
}
```

上面这段就是 `#[derive(Debug)]` 的原理：**读结构体名和字段，生成一段 `impl`**。我们平时用的 `serde`、`thiserror`、`clap` 的 derive，本质上都是这个套路，只是复杂得多。

本章不实现过程宏——它需要新建 crate 和引入 `syn` / `quote` 依赖，超出了「配套代码只依赖标准库」的范围。等你以后写库或工具时，第 24 章的 Cargo 知识加上这一节的思路就够上手了。

## 15.10 什么时候该写宏

宏是「最后的手段」，社区的判断标准大致是：

| 情况 | 该用什么 |
| --- | --- |
| 逻辑依赖「值的种类」，类型在编译期已知 | 泛型 + trait（第 9 章） |
| 逻辑依赖「值的数量」但类型相同 | 迭代器 / 切片（第 11 章） |
| 需要可变参数、或参数种类不固定 | 宏 |
| 需要生成重复的样板代码（一堆 `impl`） | derive 过程宏 |
| 需要引入新语法（自定义 DSL） | 声明宏或函数式过程宏 |
| 只是想让代码短一点 | 别用宏，拆函数更清楚 |

两个现实约束也值得记住：

1. **宏的报错信息对使用者不友好**。错误位置常常指向宏内部而不是调用处，调试成本高。
2. **宏对 IDE 不友好**。跳转、补全、格式化都可能失效——写库时这点尤其重要。

所以：**能用函数和泛型解决的，别用宏；确实需要「可变参数」或「生成代码」时，宏才是正确答案**。

## 15.11 五个真实报错与警告怎么读

**案例 1：宏定义在使用之后**

```text
error: cannot find macro `say_hello` in this scope
 --> src/main.rs:2:5
  |
2 |     say_hello!();
  |     ^^^^^^^^^ consider moving the definition of `say_hello` before this call
  |
note: a macro with the same name exists, but it appears later
 --> src/main.rs:5:14
  |
5 | macro_rules! say_hello {
  |              ^^^^^^^^^
warning: unused macro definition: `say_hello`
```

`help` 直接给出了修法。注意同时出现的「unused macro definition」警告——因为编译器认为那个宏从来没被成功调用过。

**案例 2：没有规则能匹配（`no rules expected`）**

```rust
macro_rules! only_expr {
    ($e:expr) => { $e };
}
only_expr!(let x = 1;);
```

```text
error: no rules expected keyword `let`
 --> src/main.rs:8:24
  |
1 | macro_rules! only_expr {
  | ---------------------- when calling this macro
8 |     let _ = only_expr!(let x = 1;);
  |                        ^^^ no rules expected this token in macro call
  |
note: while trying to match meta-variable `$e:expr`
 --> src/main.rs:2:6
  |
2 |     ($e:expr) => {
  |      ^^^^^^^
```

`note` 明确指出卡在哪个片段上：`expr` 不接受语句。想接受语句就把说明符改成 `$e:stmt`。

**案例 3：片段没写完整（`expected expression`）**

```text
error: expected expression, found end of macro arguments
 --> src/main.rs:8:27
  |
2 |     ($e:expr) => {
  |      ------- while parsing argument for this `expr` macro fragment
8 |     let _ = take_expr!(1 +);
  |                           ^ expected expression
```

宏参数是按语法片段解析的，`1 +` 不是完整表达式。

**案例 4：递归停不下来**

```text
error: recursion limit reached while expanding `recurse!`
 --> src/main.rs:3:9
  |
3 |         recurse!()
  |         ^^^^^^^^^^
  |
  = help: consider increasing the recursion limit by adding a `#![recursion_limit = "256"]` attribute
```

两种情况：**递归宏忘了写出口**（像上面这个），或者展开层数超出了默认上限。前者要加终止规则，后者可以调 `#![recursion_limit = "..."]`。

**案例 5：定义了却没用（警告）**

```text
warning: unused macro definition: `never_used`
 --> src/main.rs:1:14
  |
1 | macro_rules! never_used {
  |              ^^^^^^^^^^
  = note: `#[warn(unused_macros)]` (part of `#[warn(unused)]`) on by default
```

没加 `#[macro_export]` 的宏如果在本 crate 里没被用到，就会报这个警告——通常是拼错了宏名，或者忘了调用。

## 15.12 常见坑速查

| 现象 | 原因 | 解法 |
| --- | --- | --- |
| 报「cannot find macro」 | 宏定义在调用之后 | 把 `macro_rules!` 挪到前面（或提到模块顶层） |
| `no rules expected ...` | 参数形式没被任何规则匹配 | 对照说明符，必要时加一条兜底规则 |
| `my_vec![1, 2, 3,]` 匹配失败 | 没写 `$(,)?` | 在规则末尾允许尾随逗号 |
| 展开后出现多余的 `unused_mut` / `unused_variables` 警告 | 某些分支下展开结果用不到 | 宏里加 `#[allow(...)]` |
| 导出宏里写 `crate::Foo` 报找不到 | 解析到了调用方的 crate | 改用 `$crate::Foo` |
| 跨 crate 用不了宏 | 忘了 `#[macro_export]` | 加上导出，或 `pub use` 重导出 |
| 报「recursion limit reached」 | 递归宏没有出口，或层数过多 | 补终止规则，或调 `recursion_limit` |
| 报错位置总是指向宏内部 | 宏展开后行号信息有限 | 把逻辑拆到普通函数里，宏只做转发 |
| IDE 补全/跳转失效 | 宏生成的名字 IDE 看不见 | 减少宏的使用范围 |
| 想调试宏到底生成了什么 | 展开结果不可见 | `cargo expand`（或 nightly 的 `-Zunpretty=expanded`） |

## 15.13 练习

1. 写 `min_of!` 宏：返回参数里的最小值，至少要传一个参数；支持 `min_of!(3) == 3` 和 `min_of!(3, 1, 4) == 1`。
2. 写 `hashmap_of!` 宏：用 `key => value` 的写法直接构造 `HashMap`，支持尾随逗号。
3. 写 `my_assert_eq!` 宏：两个值不相等时 panic，并把两个值都打印出来（模仿标准库的 `assert_eq!`）。
4. 说明 `$crate` 的作用：为什么导出宏里引用本 crate 的路径必须写 `$crate::` 而不是 `crate::`？
5. 用自己的话解释两件事：(a) 什么是宏的「卫生性」？(b) 为什么 `macro_rules!` 必须定义在使用之前？

（第 4、5 题是 15.6 和 15.7 讨论过的内容，这里不再单独给答案。）

### 参考答案

::: details 第 1 题

```rust
macro_rules! min_of {
    ($first:expr $(, $rest:expr)*) => {{
        #[allow(unused_mut)]
        let mut min = $first;
        $( if $rest < min { min = $rest; } )*
        min
    }};
}

fn main() {
    println!("{}", min_of!(3));              // 3
    println!("{}", min_of!(3, 1, 4, 1, 5));  // 1
    println!("{}", min_of!(2.5, -1.0, 0.5)); // -1
}
```

实测输出 `3`、`1`、`-1`。`#[allow(unused_mut)]` 是必需的：只传一个参数时，`$( ... )*` 展开成空，`min` 就没被改过，编译器会警告「变量不需要 mut」——**宏展开出的代码也要符合 lint 规则**，所以内部加 `allow` 是常见做法。

:::

::: details 第 2 题

```rust
macro_rules! hashmap_of {
    ($($key:expr => $value:expr),+ $(,)?) => {{
        let mut map = std::collections::HashMap::new();
        $( map.insert($key, $value); )+
        map
    }};
}

fn main() {
    let scores = hashmap_of! {
        "ada" => 90,
        "linus" => 75,
    };
    let mut items: Vec<_> = scores.into_iter().collect();
    items.sort();
    println!("{items:?}");     // [("ada", 90), ("linus", 75)]
}
```

实测 `[("ada", 90), ("linus", 75)]`。这个宏在社区里有真实实现（`maplit` crate 的 `hashmap!`），思路完全一样：**用重复匹配把 `key => value` 列表展开成一串 `insert`**。注意外层用 `{{ }}` 包成一个块表达式，这样宏调用本身是个值，可以直接赋值。

:::

::: details 第 3 题

```rust
macro_rules! my_assert_eq {
    ($left:expr, $right:expr) => {{
        let left = $left;
        let right = $right;
        if left != right {
            panic!("断言失败：{left:?} != {right:?}");
        }
    }};
}

fn main() {
    my_assert_eq!(1 + 1, 2);
    println!("第一个断言通过");

    let result = std::panic::catch_unwind(|| my_assert_eq!("a", "b"));
    println!("第二个断言是否 panic：{}", result.is_err());
}
```

实测输出：

```text
第一个断言通过
thread 'main' panicked at ...: 断言失败："a" != "b"
第二个断言是否 panic：true
```

两个技巧：**先把 `$left` / `$right` 绑定到局部变量再比较**，这样每个参数只求值一次（避免 `my_assert_eq!(f(), g())` 里 `f()` 被调用两次）；`panic!` 里用了 `{:?}`，所以要求类型实现 `Debug`——标准库的 `assert_eq!` 也是这么做的。

:::

## 15.14 小结

- 宏是**编译期展开的代码模板**，运行期没有额外开销；参数是「语法片段」而不是值，数量和种类都可以变化。

- `macro_rules!` 由一组「模式 `=>` 展开」规则组成，从上到下匹配；**必须先定义后使用**，否则报 `cannot find macro`。

- 片段说明符决定参数能匹配什么：`expr` / `ident` / `ty` / `pat` / `tt` / `item` / `vis` 等；最灵活的 `tt` 适合原样转发。

- 重复匹配用 `$( ... )+` / `*` / `?`；在展开侧写 `$( ... )+` 就会对每个匹配到的元素重复一次；`$(,)?` 用来容忍尾随逗号。

- 宏可以递归展开（`count!(a b c)`），记得写终止规则；展开深度有上限。

- 卫生性保证宏内部定义的标识符不会和调用方冲突；但 `$xxx` 传进来的标识符属于调用方。

- 跨模块、跨 crate 用宏要 `#[macro_export]`；导出宏里引用本 crate 的路径必须写 `$crate::`。

- `println!` / `vec!` / `assert_eq!` / `matches!` 都是普通宏，展开成普通函数调用与 `match`。

- 过程宏（derive / 属性 / 函数式）能读语法树、生成代码，必须放在独立的 `proc-macro` crate，通常配合 `syn` + `quote`；`Debug`、`serde`、`tokio::main` 都是它们的产物。

- 宏是最后手段：**能用函数、泛型、trait 解决的别用宏**，因为宏更难读、更难调试，对 IDE 也不友好。

下一章讲**async/await**：`Future` 模型、`.await`、异步运行时，以及「异步里不能阻塞」这条最容易踩的规则。
