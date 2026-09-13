# 第 11 章 · 闭包与迭代器

第 3 章说过「函数是一等公民」，第 9 章讲了 trait。这一章把两者合起来，讲 Rust 里最常用的两组工具：

- **闭包**（closure）：能捕获周围变量的匿名函数。
- **迭代器**（iterator）：对一串元素做流水线处理，惰性求值、可组合。

它们构成了 Rust 的「函数式」一面，也是日常写业务代码时用得最多的东西——你会在任何一份真实的 Rust 代码里看到 `map`、`filter`、`collect` 和 `unwrap_or_else(|| ...)`。

本章配套代码在 `src/rust11_closures_iterators/mod.rs`，执行 `cargo run` 可以看到全部输出。

## 11.1 闭包：能捕获环境的函数

闭包的语法是 `|参数| 表达式`：

```rust
let double = |n: i32| n * 2;
double(21);                       // 42
```

和普通函数相比，闭包多了一个能力：**它可以捕获定义它的作用域里的变量**。

```rust
let threshold = 10;
let above = |n: i32| n > threshold;   // 捕获了 threshold
above(20);                             // true
```

实测 `double(21) = 42`、`above(20)（阈值 10）= true`。

几个细节：

- 参数和返回值的类型**通常可以省略**，编译器从调用处推断；需要写时和函数一样用 `:` 和 `->`。
- 闭包体只有一句表达式时大括号可以省；多句时写 `{ ... }`。
- 闭包是**值**，可以存进变量、当参数传、当返回值返回（第 3 章「函数指针」是它的近亲，但函数指针不能捕获环境）。
- 每个闭包都有自己**唯一的匿名类型**，所以写类型时不能写 `Closure`，只能用 `impl Fn(...)` 或 `Box<dyn Fn(...)>`——这一点到 11.4 会更清楚。

## 11.2 三种捕获方式与 `Fn` 家族

闭包捕获环境的方式有三种，编译器会**按需选择权限最小的一种**：

| 捕获方式 | 什么时候用 | 对应 trait |
| --- | --- | --- |
| 不可变借用 | 只读捕获的变量 | `Fn` |
| 可变借用 | 修改捕获的变量 | `FnMut` |
| 移动（取得所有权） | 消费捕获的变量 | `FnOnce` |

```rust
let data = vec![1, 2, 3];
let sum_ref = || data.iter().sum::<i32>();   // 不可变借用
sum_ref();
println!("{data:?}");                        // data 还能用

let mut counter = 0;
let mut bump = || counter += 1;              // 可变借用，闭包本身要 mut
bump();
bump();
println!("{counter}");                       // 2

let owned = String::from("move 进去");
let take = move || owned.len();              // 把 owned 移动进闭包
take();
```

实测输出：不可变借用求和 `6`、借用后 `data` 仍是 `[1, 2, 3]`；调用两次后 `counter = 2`；`move` 闭包算出长度 `11`（一个汉字 3 字节 + 8 字节）。

三个 trait 的调用限制：

| trait | 能调用几次 | 典型场景 |
| --- | --- | --- |
| `Fn` | 任意次 | 只读的映射、判断 |
| `FnMut` | 任意次，但要 `mut` 绑定 | 累加、收集、带状态的处理 |
| `FnOnce` | **只能一次** | 消费掉捕获的值（比如把 `String` 返回出去） |

`FnOnce` 只能调用一次的原因很直白：它把捕获的值消费掉了，第二次调用时那个值已经不存在。强行调用两次会报 `E0382`（见 11.12 案例 3）。

这三种 trait 是**包含关系**：能当 `Fn` 用的闭包也能当 `FnMut` 和 `FnOnce` 用，反之不行。写函数签名时选刚好够用的那个：接收 `Fn` 的约束最宽松（调用方给什么都行），接收 `FnMut` 次之。

## 11.3 `move`：把捕获的值搬进闭包

`move` 强制闭包**按值**捕获所有用到的变量：

```rust
let owned = String::from("move 进去");
let take = move || owned.len();
println!("{}", take());
// println!("{owned}");      // 报 E0382：owned 已经被移进闭包
```

什么时候必须 `move`？

- 闭包要**活得比当前作用域长**：返回出去、交给新线程（第 13 章）、存进结构体。
- 闭包体需要**拿到所有权**：比如把捕获的 `String` 返回出去。

`move` 的代价是外面的变量不能再用了（`E0382`，见 11.12 案例 2）。注意 `move` 只是「怎么捕获」，不改变闭包是 `Fn` 还是 `FnMut`——一个 `move` 闭包如果只读捕获的值，仍然是 `Fn`。

## 11.4 闭包作为参数和返回值

接收闭包时，用 `Fn` 家族做约束：

```rust
// 接收 Fn：可以多次调用，不改环境
fn apply_twice<F: Fn(i32) -> i32>(f: F, value: i32) -> i32 {
    f(f(value))
}

// 接收 FnMut：闭包本身要声明成 mut
fn apply_and_count<F: FnMut()>(mut f: F, times: usize) {
    for _ in 0..times {
        f();
    }
}

// 接收 FnOnce：只调用一次
fn consume_once<F: FnOnce() -> String>(f: F) -> String {
    f()
}
```

实测：`apply_twice(|n| n + 3, 1) = 7`、`apply_and_count` 三次后 `ticks = 3`、`consume_once` 拿走了 `move` 进去的字符串。

**返回**闭包时只能写 `impl Fn`（或装箱成 `Box<dyn Fn>`），因为每个闭包都是匿名类型：

```rust
fn make_adder(n: i32) -> impl Fn(i32) -> i32 {
    move |x| x + n          // 必须 move：n 是局部变量，闭包要活得比函数调用长
}

let add_ten = make_adder(10);
println!("{}", add_ten(5));  // 15
```

如果这里不写 `move`，编译器会报 `E0373`（闭包可能比 `n` 活得长）——这是 `move` 最常见的用法。

```text
error[E0373]: closure may outlive the current function, but it borrows `n`, which is owned by the current function
 --> src/main.rs:2:5
  |
2 |     |x| x + n
  |     ^^^     - `n` is borrowed here
  |     |
  |     may outlive borrowed value `n`
  |
note: closure is returned here
help: to force the closure to take ownership of `n` (and any other referenced variables), use the `move` keyword
  |
2 |     move |x| x + n
  |     ++++
```

实测加上 `move` 后 `make_adder(10)` 返回的闭包可以反复调用，`add_ten(5) = 15`、`add_ten(1) = 11`。

## 11.5 迭代器的三种起点

同一种「把容器变成迭代器」的需求，有三种写法，区别在迭代出什么类型：

| 写法 | 迭代出的元素 | 之后原来的容器 |
| --- | --- | --- |
| `v.iter()` | `&T` | 还能用 |
| `v.iter_mut()` | `&mut T` | 还能用（元素被改过） |
| `v.into_iter()` | `T`（所有权转移） | 不能用了 |

```rust
let mut numbers = vec![1, 2, 3];
for n in numbers.iter() { }        // 只读
for n in numbers.iter_mut() { *n *= 10; }   // 能改
let moved: Vec<i32> = numbers.into_iter().collect();   // 拿走
```

实测：`iter()` 打印 `1 2 3`；`iter_mut()` 之后是 `[10, 20, 30]`；`into_iter()` 收集到 `[10, 20, 30]`。

`for x in v` 用的是哪一种，由 `v` 的类型决定：`v` 是 `Vec<T>` 就用 `into_iter`（移动）；写 `for x in &v` 就是 `iter`；`for x in &mut v` 就是 `iter_mut`。这也是第 4 章那句「`for x in v` 之后 `v` 不能用了」的原因。

## 11.6 惰性求值：不消费就不干活

这是理解迭代器的关键：**适配器只是「记下要做什么」，真正的计算发生在消费的那一刻**。

```rust
let values = vec![1, 2, 3];
let iter = values.iter().map(|n| {
    println!("正在计算 {n}");
    n * 2
});
println!("迭代器已经建好，但还没有任何计算发生");
let doubled: Vec<i32> = iter.collect();
```

实测输出：

```text
    迭代器已经建好，但还没有任何计算发生
        正在计算 1
        正在计算 2
        正在计算 3
    collect 之后 = [2, 4, 6]
```

「正在计算」三行出现在 `collect` 之后，说明 `map` 里的闭包直到消费时才被调用——这就是**惰性**。

惰性有实打实的好处：`v.iter().map(f).next()` 只会对第一个元素调用一次 `f`，不管你后面写了多少适配器；如果只是想要第一个满足条件的元素，剩下的不会被处理。惰性的代价是容易忘记消费——那样什么都不会发生，编译器会警告（见 11.12 案例 4）。

**适配器**（返回新迭代器，惰性）和**消费器**（触发计算，返回结果）是两类东西，分清楚就不会写错。

## 11.7 常用适配器

```rust
let scores = vec![55, 92, 68, 77, 45, 88];

scores.iter().copied()                    // &i32 -> i32
      .filter(|s| *s >= 60)               // 只保留满足条件的
      .map(|s| s + 5)                     // 逐个变换
      .collect::<Vec<i32>>();             // 消费
```

实测这一条链得到 `[97, 73, 82, 93]`（及格分数 `[92, 68, 77, 88]` 各加 5）。

常用的适配器一览：

| 适配器 | 作用 |
| --- | --- |
| `map(f)` | 每个元素变成 `f(元素)` |
| `filter(f)` | 只保留 `f` 返回 `true` 的元素 |
| `filter_map(f)` | 返回 `Option`，自动丢掉 `None` |
| `take(n)` / `skip(n)` | 取前 n 个 / 跳过前 n 个 |
| `take_while(f)` / `skip_while(f)` | 按条件取 / 跳过前缀 |
| `enumerate()` | 带上下标，产出 `(usize, T)` |
| `zip(other)` | 两个迭代器配对，产出元组 |
| `chain(other)` | 首尾相接 |
| `rev()` | 反向（需要迭代器支持双端） |
| `flatten()` | 把嵌套的层压平 |
| `flat_map(f)` | 先 `map` 再压平 |
| `peekable()` | 允许 `peek()` 看一眼下一个 |

实测几条：

```text
    前两名 = [97, 73]
    跳过第一名 = [73, 82, 93]
    chain = [1, 2, 3, 4]
    zip = ["ada:36", "linus:54"]
    enumerate: 0=55 1=92 2=68 3=77 4=45 5=88
    flatten = [1, 2, 3]
```

适配器可以随便串，链子写多长都行——因为每一步都只是「包一层」，没有真正的中间容器。

## 11.8 常用消费器

消费器会驱动迭代，返回一个结果：

| 消费器 | 返回 |
| --- | --- |
| `collect()` | 收集成某个容器 |
| `sum()` / `product()` | 求和 / 求积 |
| `count()` | 元素个数 |
| `fold(init, f)` | 带累加器的遍历（最通用的一个） |
| `any(f)` / `all(f)` | 是否存在 / 是否全部满足 |
| `find(f)` | 第一个满足条件的元素，返回 `Option` |
| `position(f)` | 第一个满足条件的下标，返回 `Option` |
| `max()` / `min()` | 最大值 / 最小值，返回 `Option` |
| `for_each(f)` | 逐个执行副作用 |

实测：

```text
    sum = 31
    count = 8
    max / min = Some(9) / Some(1)
    fold 求积（前四个）= 12
    有大于 8 的数吗 = true
    全都大于 0 吗 = true
    第一个偶数 = Some(4)
    5 的下标 = Some(4)
```

注意 `max` / `min` / `find` / `position` 返回的都是 `Option`——「空迭代器没有最大值」是真实存在的情况，标准库不 panic，交给你处理（第 6 章的思路）。

`fold` 值得一提：`sum`、`count`、`max` 其实都能用 `fold` 写出来，它是最通用也最灵活的消费器：

```rust
nums.iter().fold(0, |acc, n| acc + n)    // 等价于 nums.iter().sum()
```

## 11.9 `collect` 的目标类型

`collect` 能收集成任何实现了 `FromIterator` 的类型，常见的三种：

```rust
let chars: Vec<char> = "hello".chars().collect();
let reversed: String = chars.iter().rev().collect();

let mut counts: HashMap<&str, usize> = HashMap::new();
for word in sentence.split_whitespace() {
    *counts.entry(word).or_insert(0) += 1;
}
```

实测 `Vec<char> = ['h', 'e', 'l', 'l', 'o']`、倒序拼成 `String = olleh`、词频（排序后）`[("lazy", 1), ("quick", 1), ("the", 3)]`。

`collect` 的类型**必须能从上下文推断出来**，否则报 `E0283`（见 11.12 案例 6）。三种写清楚的姿势：

```rust
let v: Vec<i32> = iter.collect();                    // 变量标注
let v = iter.collect::<Vec<i32>>();                  // turbofish
let v: HashMap<_, _> = pairs.collect();              // 交给函数返回类型推断
```

`collect` 还能收集成 `Result`：`iter.collect::<Result<Vec<_>, _>>()` 会在遇到第一个 `Err` 时整体失败——第 8 章的错误处理在这里和迭代器接上了。

## 11.10 自定义迭代器

自己实现 `Iterator` 只需要写一个方法：`next`。

```rust
struct Fibonacci {
    a: u64,
    b: u64,
}

impl Iterator for Fibonacci {
    type Item = u64;                 // 关联类型（第 9 章）

    fn next(&mut self) -> Option<u64> {
        let next = self.a + self.b;
        self.a = self.b;
        self.b = next;
        Some(self.a)                 // 永远不返回 None：无限迭代器
    }
}
```

实测 `Fibonacci::new().take(8)` 得到 `[1, 1, 2, 3, 5, 8, 13, 21]`，`nth(9)` 得到第 10 项 `Some(55)`。

三个要点：

- `next` 返回 `Option<Self::Item>`：`Some` 表示还有元素，`None` 表示迭代结束。**返回 `None` 之后再调用 `next`，行为是未定义的契约**，标准库的适配器都假设你不会这么干。
- 无限迭代器完全合法（比如上面的斐波那契、`std::iter::repeat`），配合 `take` / `take_while` 使用。
- 一旦实现了 `Iterator`，**所有适配器自动获得**——`map`、`filter`、`sum`、`collect` 全部可用。这就是第 9 章「trait 带默认方法」的威力：你只写了 `next`，却得到几十个方法。

实测缺少 `next` 时的报错见 11.12 案例 5——`E0046` 会把该写的方法签名都列出来。

## 11.11 零成本抽象：迭代器 vs 手写循环

同一个任务，两种写法：

```rust
// 手写循环
let mut manual = Vec::new();
for n in &data {
    if n % 2 == 0 {
        manual.push(n * n);
    }
}

// 迭代器链
let chained: Vec<i32> = data.iter().filter(|n| *n % 2 == 0).map(|n| n * n).collect();
```

实测两者结果都是 `[4, 16, 36]`。性能上，**迭代器版本通常和手写循环一样快，甚至更快**：适配器是零大小类型，会被内联展开，边界检查也常常能被优化掉。这就是社区说的「零成本抽象」——用高层写法不付运行时代价。

那什么时候仍然写 `for` 循环？一般是这几种情况：循环体里有复杂的控制流（多层 `break`、标签跳转）、需要在循环里做多种操作、或者提前返回很多次。**可读性优先**：链子太长（超过三四步）时，拆成 `for` 循环反而更清楚。

## 11.12 七个报错与一个警告怎么读

**案例 1：两个闭包同时可变借用一个变量（`E0499`）**

```rust
let mut v = vec![1, 2, 3];
let mut add = || v.push(4);
let mut clear = || v.clear();
```

```text
error[E0499]: cannot borrow `v` as mutable more than once at a time
 --> src/main.rs:4:21
  |
3 |     let mut add = || v.push(4);
  |                   -- - first borrow occurs due to use of `v` in closure
  |                   |
  |                   first mutable borrow occurs here
4 |     let mut clear = || v.clear();
  |                     ^^ - second borrow occurs due to use of `v` in closure
  |                     |
  |                     second mutable borrow occurs here
5 |     add();
  |     --- first borrow later used here
```

和第 5 章的可变借用规则完全一致，只是这次「借用者」是闭包。解法：让两个闭包的生命周期不重叠，或者把它们合并成一个闭包。

**案例 2：`move` 之后外部变量不能再用（`E0382`）**

```text
error[E0382]: borrow of moved value: `s`
 --> src/main.rs:5:16
  |
2 |     let s = String::from("hello");
  |         - move occurs because `s` has type `String`, which does not implement the `Copy` trait
3 |     let c = move || s.len();
  |             ------- - variable moved due to use in closure
  |             |
  |             value moved into closure here
5 |     println!("{s}");
  |                ^ value borrowed here after move
  |
help: consider cloning the value before moving it into the closure
```

**案例 3：把只能调用一次的闭包调用了两次（`E0382`）**

```rust
let s = String::from("hello");
let c = move || s;      // 消费 s，所以是 FnOnce
let a = c();
let b = c();
```

```text
error[E0382]: use of moved value: `c`
 --> src/main.rs:5:13
  |
4 |     let a = c();
  |             --- `c` moved due to this call
5 |     let b = c();
  |             ^ value used here after move
  |
note: closure cannot be invoked more than once because it moves the variable `s` out of its environment
 --> src/main.rs:3:21
  |
3 |     let c = move || s;
  |                     ^
note: this value implements `FnOnce`, which causes it to be moved when called
```

`note` 里直接点名了 `FnOnce`——看到这个提示就知道：这个闭包只能调用一次。

**案例 4：适配器没有消费（警告）**

```rust
v.iter().map(|n| n * 2);     // 忘了 collect / sum / for_each
```

```text
warning: unused `Map` that must be used
 --> src/main.rs:3:5
  |
3 |     v.iter().map(|n| n * 2);
  |     ^^^^^^^^^^^^^^^^^^^^^^^
  |
  = note: iterators are lazy and do nothing unless consumed
  = note: `#[warn(unused_must_use)]` (part of `#[warn(unused)]`) on by default
help: use `let _ = ...` to ignore the resulting value
```

`note` 那句话就是 11.6 的结论：**迭代器是惰性的，不消费就不干活**。这条警告专门用来抓这种「写了半天什么都没发生」的 bug。

**案例 5：实现 `Iterator` 但没写 `next`（`E0046`）**

```text
error[E0046]: not all trait items implemented, missing: `next`
 --> src/main.rs:5:1
  |
5 | impl Iterator for Counter {
  | ^^^^^^^^^^^^^^^^^^^^^^^^^ missing `next` in implementation
  |
  = help: implement the missing item: `fn next(&mut self) -> Option<<Self as Iterator>::Item> { todo!() }`
```

`help` 里把那行签名都写好了，补上即可。

**案例 6：`filter` 的闭包没返回 `bool`（`E0308`）**

```text
error[E0308]: mismatched types
 --> src/main.rs:2:60
  |
2 |     let v: Vec<i32> = vec![1, 2, 3].into_iter().filter(|n| n + 1).collect();
  |                                                            ^^^^^ expected `bool`, found integer
```

`filter` 要的是「判断」，不是「变换」——想变换就用 `map`。

**案例 7：`collect` 的目标类型推断不出来（`E0283`）**

```text
error[E0283]: type annotations needed
    --> src/main.rs:2:9
     |
2 |     let v = vec![1, 2, 3].iter().map(|n| n + 1).collect();
     |         ^                                       ------- type must be known at this point
     |
     = note: the type must implement `FromIterator<i32>`
help: consider giving `v` an explicit type
     |
2 |     let v: Vec<_> = vec![1, 2, 3].iter().map(|n| n + 1).collect();
     |          ++++++++
```

`collect` 能收成很多种容器（`Vec`、`HashMap`、`String`、`Result`……），所以编译器需要你告诉它要哪一种。

## 11.13 常见坑速查

| 现象 | 原因 | 解法 |
| --- | --- | --- |
| 写了 `map` 却什么都没发生 | 迭代器惰性，没有消费 | 加 `collect` / `sum` / `for_each` |
| 两个闭包都想改同一个变量 | 可变借用不能同时存在 | 合并闭包，或缩短其中一个的生命 |
| `move` 之后外部变量不能用了 | 所有权已转移进闭包 | 提前 `clone`，或重新设计数据归属 |
| 闭包只能调用一次 | 它消费了捕获的值，是 `FnOnce` | 改成不消费（借用），或接受只能调一次 |
| 函数返回闭包报 `E0373` | 闭包借用了局部变量 | 加 `move` |
| 想给闭包写类型写不出来 | 每个闭包是匿名类型 | 用 `impl Fn(...)` 或 `Box<dyn Fn(...)>` |
| `filter` 里想做变换 | 用错了适配器 | 变换用 `map`，`filter` 只做判断 |
| `collect` 报 `type annotations needed` | 目标类型不明确 | 变量标注、turbofish 或让返回类型来定 |
| 迭代器链太长看不懂 | 一步做了太多事 | 拆成 `for` 循环，或中间 `let` 绑定 |
| `iter()` 里拿到的是 `&T` 不好处理 | 迭代器产出引用 | `.copied()` / `.cloned()`，或用 `into_iter()` |
| 自定义迭代器 `next` 里算出值再改状态 | 借用/移动顺序容易写错 | 先取出要返回的值，再更新 `self` 的字段 |

## 11.14 练习

1. 写 `fn apply_n<F: FnMut(i32) -> i32>(mut f: F, start: i32, n: usize) -> i32`，把 `start` 迭代应用 `f` 共 `n` 次。
2. 用迭代器写 `fn count_words(text: &str) -> HashMap<&str, usize>`，统计每个单词出现次数（提示：`split_whitespace` + `entry`）。
3. 写 `fn squares_of_evens(values: &[i32]) -> Vec<i32>`，用 `filter` + `map` + `collect` 返回所有偶数的平方。
4. 定义 `struct Countdown { current: u32 }` 并实现 `Iterator`，让它从 `current` 倒数到 1，然后 `0` 时返回 `None`。
5. 写 `fn moving_average(values: &[f64], window: usize) -> Vec<f64>`，用 `windows` + `map` + `collect` 算滑动平均；`window` 为 0 或大于长度时返回空 `Vec`。

（第 2、3 题是 11.9 和 11.7 示例的直接变体，这里不再单独给答案。）

### 参考答案

::: details 第 1 题

```rust
fn apply_n<F: FnMut(i32) -> i32>(mut f: F, start: i32, n: usize) -> i32 {
    let mut value = start;
    for _ in 0..n {
        value = f(value);
    }
    value
}

fn main() {
    println!("{}", apply_n(|n| n * 2, 1, 5));   // 32

    let mut calls = 0;
    let result = apply_n(|n| { calls += 1; n + 1 }, 0, 3);
    println!("{result} / calls = {calls}");     // 3 / calls = 3
}
```

用 `FnMut` 而不是 `Fn`：第二个闭包修改了捕获的 `calls`，属于可变借用——这也是参数必须写成 `mut f` 的原因。如果约束写成 `Fn`，那个闭包就传不进来（`FnMut` 闭包不保证满足 `Fn`）。

:::

::: details 第 4 题

```rust
struct Countdown {
    current: u32,
}

impl Iterator for Countdown {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        if self.current == 0 {
            None
        } else {
            let value = self.current;   // 先取出要返回的值
            self.current -= 1;          // 再更新状态（借用规则要求分开写）
            Some(value)
        }
    }
}

fn main() {
    let values: Vec<u32> = Countdown { current: 5 }.collect();
    println!("{values:?}");                     // [5, 4, 3, 2, 1]
    let sum: u32 = Countdown { current: 4 }.sum();
    println!("{sum}");                          // 10
}
```

实测输出 `[5, 4, 3, 2, 1]` 和 `10`。实现完 `next` 之后，`collect`、`sum`、`take` 这些方法全都自动可用——因为它们是 `Iterator` 的默认方法（第 9 章）。

:::

::: details 第 5 题

```rust
fn moving_average(values: &[f64], window: usize) -> Vec<f64> {
    if window == 0 || window > values.len() {
        return Vec::new();
    }
    values
        .windows(window)
        .map(|w| w.iter().sum::<f64>() / window as f64)
        .collect()
}

fn main() {
    println!("{:?}", moving_average(&[1.0, 2.0, 3.0, 4.0], 2));  // [1.5, 2.5, 3.5]
    println!("{:?}", moving_average(&[1.0, 2.0], 5));            // []
    println!("{:?}", moving_average(&[1.0, 2.0], 0));            // []
}
```

`windows(n)` 产出长度为 `n` 的滑动窗口切片，正好适合这种需求。注意 `window as f64` 要把 `usize` 转成浮点才能做除法（第 2 章的类型转换）；`window == 0` 必须单独判断，否则 `windows(0)` 会 panic。

:::

## 11.15 小结

- 闭包是能捕获环境的匿名函数；捕获方式有三种：不可变借用（`Fn`）、可变借用（`FnMut`）、移动（`FnOnce`），编译器自动选权限最小的一种。

- `Fn` 可以反复调用，`FnMut` 需要 `mut` 绑定，`FnOnce` 只能调用一次；三者的约束关系是 `Fn ⊂ FnMut ⊂ FnOnce`。

- 闭包要活过当前作用域（返回、跨线程、存结构体）就得加 `move`；代价是外部变量不能再用。

- 接收闭包用 `F: Fn(...)` 之类的约束，返回闭包只能写 `impl Fn(...)` 或 `Box<dyn Fn(...)>`。

- 迭代器有三个起点：`iter()`（`&T`）、`iter_mut()`（`&mut T`）、`into_iter()`（`T`）；`for x in v` 用哪个由 `v` 的类型决定。

- **适配器是惰性的**：`map` / `filter` / `take` 只是包一层，直到 `collect` / `sum` / `for_each` 这样的消费器驱动它才真正计算。

- `filter` 只做判断（返回 `bool`），变换用 `map`；`fold` 是最通用的消费器，`sum` / `count` / `max` 都能用它写出来。

- `collect` 可以收成 `Vec`、`String`、`HashMap` 等，目标类型必须能从上下文推断；收成 `Result` 能在遇到第一个错误时整体失败。

- 实现 `Iterator` 只需要写 `next`，其余几十个方法都是默认方法；无限迭代器合法，配合 `take` 使用。

- 迭代器和手写循环性能相当（零成本抽象），选择标准是**可读性**：链子短就用迭代器，控制流复杂就用 `for`。

下一章讲**智能指针**：`Box`、`Rc` / `Arc`、`RefCell`，以及它们如何解决「一个值有多个所有者」和「内部可变性」这两个本章和前面留下的话题。
