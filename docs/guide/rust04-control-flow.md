# 第 4 章 · 控制流

前三章写的都是「一条接一条往下执行」的代码。这一章让程序学会挑路走、学会重复：`if` 负责选择，`loop` / `while` / `for` 负责重复，`break` / `continue` 负责提前退出。

Rust 的控制流有几处和别的语言不一样，先记住这几条，后面每一节都围绕它们展开：

- **没有真值判断**。`if` 后面必须是 `bool`，写 `if 1 { }` 直接编译失败。
- **没有三元运算符 `?:`**，因为 `if` 本身就是表达式，可以直接 `let x = if c { a } else { b };`。
- **没有 `do-while`**。需要「至少执行一次」时，用 `loop` + `break` 模拟。
- **只有 `loop` 能 `break 值`**，「找到就带着结果退出」不用先声明 `mut` 变量。
- **循环可以起名字**（标签），`break 'outer` 一次跳出多层嵌套。

本章配套代码在 `src/rust04_control_flow/mod.rs`，执行 `cargo run` 可以看到全部输出。

## 4.1 `if`：条件必须是 `bool`

```rust
let score = 82;
if score >= 60 {
    println!("及格");
}
```

条件可以写得很长，但类型必须是 `bool`。Rust 不做「非零即真」这种隐式转换，所以下面这句编不过：

```rust
if 1 {
    println!("hi");
}
```

```text
error[E0308]: mismatched types
 --> src/main.rs:2:8
  |
2 |     if 1 { println!("hi"); }
  |        ^ expected `bool`, found integer
```

报错只有一行，但信息很足：位置指向条件，`expected bool, found integer` 直接说明原因。修法就是显式比较，比如 `if count != 0 { }`。

另一个新手常见的错误是把比较写成赋值：

```rust
let x = 0;
if x = 5 {
    println!("five");
}
```

```text
error[E0308]: mismatched types
 --> src/main.rs:3:8
  |
3 |     if x = 5 { println!("five"); }
  |        ^^^^^ expected `bool`, found `()`
  |
help: you might have meant to compare for equality
  |
3 |     if x == 5 { println!("five"); }
  |           +
```

注意 `help` 那行直接给出了修法：`==`。这也顺带说明了为什么 Rust 里赋值不能当条件——赋值是语句，类型是 `()`，永远不会变成 `bool`（第 3 章 3.3 讲过语句和表达式的区别），所以这类笔误在编译期就被拦住了。

最后一点语法细节：**`if` 后面的大括号不能省略**。`if x > 0 println!("正数");` 是语法错误，不像某些语言可以省掉括号。

## 4.2 `else if` 链：从上到下短路

把多个条件串起来就是 `else if` 链。判断顺序就是书写顺序，**第一条命中的分支执行完就结束**，后面的条件连求值都不会发生：

```rust
fn grade(score: u32) -> &'static str {
    if score >= 90 {
        "优秀"
    } else if score >= 75 {
        "良好"
    } else if score >= 60 {
        "及格"
    } else {
        "不及格"
    }
}
```

实测 `[95, 82, 60, 41]` 四个分数，输出依次是 `优秀` / `良好` / `及格` / `不及格`，每个分支各命中一次。

「短路」有两个实际后果：

1. **顺序不能随便写**。如果把 `score >= 60` 放到最前面，那么 95 也会走进「及格」分支，后面的 `>= 90` 永远没机会执行。
2. **条件可以依赖前面的判断**。后面的分支不需要重复排除前面的情况，因为能走到那里就说明前面的条件都是假。

如果所有分支都不命中、又没有 `else`，那就什么都不做，整个 `if` 的值是 `()`。这在只是「顺手做点事」的场景完全正常：

```rust
if score < 60 {
    println!("需要加油");
}
```

## 4.3 `if` 当表达式用

第 3 章讲过 `if` 是表达式，这里看它实际怎么用。两种写法做同一件事：

```rust
let a = 7;

// 语句风格：先给默认值，再改
let mut level = "普通";
if a > 5 {
    level = "优先";
}

// 表达式风格：一次赋值完成
let level = if a > 5 { "优先" } else { "普通" };
```

表达式风格少写了一个 `mut`，而且**编译器能保证每条路径都会赋上值**，不存在「某个分支忘了赋值」的可能。所以在「根据条件产生一个值」的场景，优先用表达式风格。

返回值也一样：

```rust
fn max_of(a: i32, b: i32) -> i32 {
    if a >= b { a } else { b }
}
```

整个 `if` 就是函数体的最后一行（尾表达式），它的值就是返回值，不需要 `return`，也不需要中间变量。

要注意的是：用了表达式风格，两条分支的类型就必须一致，而且不能少 `else`（这是第 3 章那个 `E0317` 报错的场景）。如果只是要执行动作、不产生值，语句风格反而更自然，没必要为了统一而硬写一个空的 `else`。

## 4.4 `loop`：唯一能 `break` 出值的循环

`loop { }` 就是死循环，唯一的出口是 `break`。它的特别之处在于：**`break` 后面可以跟一个值，这个值就是整个 `loop` 表达式的值**。

```rust
let mut power = 1;
let reached = loop {
    power *= 2;
    if power > 100 {
        break power;   // 带着结果退出，reached 就是 128
    }
};
```

实测输出 `2 的幂里第一个超过 100 的是 128`。

这个写法解决了一个很常见的需求：**循环里算出结果，循环外要用**。换成 `while` 就得先在循环外声明一个可变变量接住，而 `loop` 直接把结果递出来，代码更短，也不用担心某条路径忘了赋值。

两个细节：

- 同一个 `loop` 里所有 `break` 的值必须是**同一个类型**。写 `if c { break 1; } else { break "x"; }` 会报 `E0308`：`expected integer, found &str`（见 4.11）。
- 如果循环的出口只有不带值的 `break;`，那 `loop` 的类型就是 `()`，和普通循环没区别。

顺便一提，`while true { }` 也能写死循环，但惯用写法是 `loop { }`，写了 `while true` 编译器会警告（见 4.11）。`loop` 还有一个技术优势：编译器知道它「只有 `break` 才能出来」，所以 `let x = loop { ... };` 不需要给 `x` 一个初始值，类型完全由 `break` 的值决定。

适合用 `loop` 的场景：重试直到成功、状态机主循环、需要带着结果退出的查找。

## 4.5 `while`：条件循环，以及 `continue` 的经典坑

`while` 先判断条件，成立才执行循环体，所以**可能一次都不执行**：

```rust
let mut fuel = 3;
while fuel > 0 {
    println!("倒计时 {fuel}");
    fuel -= 1;
}
```

实测输出 `倒计时 3` / `2` / `1`，最后 `fuel = 0`。

`while` 和 `loop` 的分工是：条件能写在循环头、一眼看明白就用 `while`；需要「先执行一次再判断」或者「带着值退出」才用 `loop`。

### 坑：`continue` 跳过了计数器推进

这是 `while` 里最容易写出死循环的地方：

```rust
let mut i = 0;
while i < 3 {
    if i % 2 == 0 {
        continue;   // i 是偶数就跳过……包括下面的 i += 1
    }
    i += 1;
}
```

`i` 一开始是 0，满足 `i % 2 == 0`，`continue` 跳回条件判断；而推进语句 `i += 1` 在 `continue` 后面，永远执行不到，于是 `i` 永远是 0，循环卡死。

配套代码给这个错误示范加了一根「保险丝」，实测转了 6 圈后主动退出，输出 `错误示范：转了 6 圈，i 一直是 0（主动退出）`——既能看到现象，又不会真的把 `cargo run` 卡住。

修法有两种。第一种是把推进语句提到 `continue` 之前：

```rust
let mut j = 0;
let mut evens = 0;
while j < 6 {
    let current = j;
    j += 1;                  // 先推进，再做任何 continue 判断
    if current % 2 != 0 {
        continue;
    }
    evens += 1;
}
```

实测 `0..6 里的偶数有 3 个`。这个「先把状态推进，再决定要不要 `continue`」的顺序，值得当成写 `while` 的肌肉记忆。

第二种修法是干脆别用 `while`——遍历固定范围本来就更适合 `for`，`for` 的推进由迭代器负责，`continue` 不可能跳过它。见下一节。

## 4.6 `for`：遍历的默认选择

只要任务是「把一堆东西过一遍」，就用 `for`。范围、数组、切片、`Vec` 都能直接遍历：

```rust
for i in 1..=5 { }              // 1 2 3 4 5，..= 含尾
for i in 0..5 { }               // 0 1 2 3 4，.. 不含尾
for i in (0..5).rev() { }       // 4 3 2 1 0，倒着走
for i in (0..10).step_by(3) { } // 0 3 6 9，隔几个走一步
```

容器有几种写法，区别是「每次迭代出什么类型」和「原来的容器还能不能用」：

| 写法 | 每次拿到的元素 | 循环后原来的容器 |
| --- | --- | --- |
| `for x in arr`（`arr: [i32; 3]`） | `i32` | 还能用（数组被复制） |
| `for x in v`（`v: Vec<T>`） | `T`，所有权被移进循环 | **不能用了**，报 `E0382` |
| `for x in &v` | `&T` | 还能用 |
| `for x in &mut v` | `&mut T` | 还能用，且能改元素 |
| `for (i, x) in v.iter().enumerate()` | `(usize, &T)` | 还能用 |

这张表的第二行才是重点：**`for x in v` 对 `Vec` 来说是「把 `v` 交出去」**，因为 `Vec` 没有实现 `Copy`。只想看看内容就写 `for x in &v`，这是最常用的形式。

数组稍微特殊：`[i32; 3]` 这种元素是 `Copy` 的数组，按值迭代相当于复制一份，原数组还能继续用；但如果元素本身不 `Copy`（比如 `[String; 3]`），数组同样会被移走。

实测输出：

```text
数组按值迭代: 88 92 79
Vec 借用迭代: Ada Linus Graydon （循环后 names 还能用，长度 3）
enumerate -> 0: Ada
enumerate -> 1: Linus
enumerate -> 2: Graydon
切片迭代: 92 79
```

`enumerate()` 解决了「循环里同时要下标和值」的需求，比自己维护一个 `let mut i = 0;` 更安全。第 11 章会解释 `for` 背后其实是迭代器的 `next()` 调用，以及 `.rev()` / `.step_by()` 这些适配器怎么串起来。

## 4.7 `break`、`continue` 与循环标签

- `break`：立刻结束**当前这一层**循环，跳到循环后面继续执行。
- `continue`：结束本轮，进入**当前这一层**的下一轮。

在嵌套循环里，光用 `break` 只会跳出最内层，外层照常继续：

```rust
let mut runs = 0;
for _ in 0..3 {
    for j in 0..10 {
        runs += 1;
        if j == 2 {
            break;   // 只跳出内层，外层还会再转两圈
        }
    }
}
```

实测内层一共进入了 9 次（外层 3 圈，每圈内层跑 3 次）。

想要一次跳出多层，就给循环起个名字，也就是**循环标签**，名字用 `'` 开头：

```rust
'outer: for i in 1..=4 {
    for j in 1..=4 {
        if i * j > 6 {
            break 'outer;   // 一次跳出两层
        }
    }
}
```

实测内层一共进入 8 次：`i = 2, j = 4` 时发现 `2 * 4 > 6`，直接结束整个双层循环。把标签去掉，内层会一直跑满，结果就是 16 次。

`continue` 也能带标签，表示「跳过本轮的剩余部分，直接进入**外层**的下一轮」：

```rust
'rows: for y in 0..3 {
    for x in 0..3 {
        if x > y {
            continue 'rows;   // 内层剩下的都不做了
        }
        print!("({x},{y}) ");
    }
}
```

实测输出 `(0,0) (0,1) (1,1) (0,2) (1,2) (2,2)`，正好是一个下三角——这是标签最实用的写法之一。

**一条硬规则**：`break` 和 `continue` 只能待在循环（或带标签的块）里，写在别处报 `E0268`（见 4.11）。至于「提前结束整个函数」，那是 `return` 的活（第 3 章 3.7）。

## 4.8 `while let` 与 `if let`：边循环边取走

有些容器是「取一次少一个」，比如 `Vec::pop()` 返回 `Option`：还有值就是 `Some(x)`，空了就是 `None`。这种「条件本身就是一次匹配尝试」的循环，用 `while let` 写最自然：

```rust
let mut stack = vec![1, 2, 3];
while let Some(top) = stack.pop() {
    print!("{top} ");
}
```

实测输出 `3 2 1`，结束后 `stack` 长度为 0。每一轮都尝试匹配 `Some(top)`：匹配成功就执行循环体，返回 `None` 就结束循环。

`if let` 是它的一次性版本，只在需要一个分支时用：

```rust
if let Some(top) = stack.pop() {
    println!("栈顶是 {top}");
}
```

这里的 `Some(top)` 属于枚举模式匹配，第 6 章会正式展开，现在只要理解「条件是一个匹配尝试」就够了。

`while let` 有一个和 `while` 同源的坑：**如果循环体不改变被匹配的值，它会一直匹配下去**。程序不会报错，也不会警告。

## 4.9 带标签的块也能「带值」

第 3 章说过块是表达式。给块加个标签，`break '标签 值` 就能从块里带着值出来：

```rust
let found = 'search: {
    for candidate in 1..=10 {
        if candidate % 7 == 0 {
            break 'search candidate;
        }
    }
    0   // 循环走完都没找到时，块的值是这个
};
```

实测 `'search 块的值是 7`。

它和 4.4 的 `loop` + `break 值` 是同一个思路，好处是能带着值从**嵌套结构**里提前退出，不用额外声明 `mut` 变量。块的值依然遵循第 3 章的规则：最后一行（不带分号）就是它的值。

## 4.10 三种循环怎么选

| 场景 | 首选 | 原因 |
| --- | --- | --- |
| 遍历范围、数组、`Vec`、切片 | `for` | 边界和推进都交给迭代器，不用管下标 |
| 循环次数不确定，条件决定 | `while` | 条件写在循环头，一眼能看懂 |
| 至少要执行一次 | `loop` + `break` | Rust 没有 `do-while` |
| 要带着值退出 | `loop` | 唯一支持 `break 值` 的循环 |
| 要一次跳出多层 | 任意循环 + 标签 | `break 'outer` |

判断标准可以简化成一句话：**能用 `for` 就用 `for`**；不能，再问「要先执行一次吗」，是就用 `loop`，否就用 `while`。

没有 `do-while` 时，「先执行一次再判断」用 `loop` 模拟：

```rust
let mut n = 0;
loop {
    n += 1;              // 先做一次
    if n >= 3 {
        break;           // 再判断要不要停
    }
}
```

实测 `n` 最终是 3。把里面的 `if` 换成「条件不成立就 `break`」，就是标准的 do-while 语义。

## 4.11 五个真实报错怎么读

和上一章一样，错误信息都来自实际编译，行号按各自的最小例子标注。

**案例 1：`if` 的条件不是 `bool`**

```text
error[E0308]: mismatched types
 --> src/main.rs:2:8
  |
2 |     if 1 { println!("hi"); }
  |        ^ expected `bool`, found integer
```

**案例 2：把 `==` 写成了 `=`**

```text
error[E0308]: mismatched types
 --> src/main.rs:3:8
  |
3 |     if x = 5 { println!("five"); }
  |        ^^^^^ expected `bool`, found `()`
  |
help: you might have meant to compare for equality
  |
3 |     if x == 5 { println!("five"); }
  |           +
```

**案例 3：在 `while` 里 `break` 一个值**

```text
error[E0571]: `break` with value from a `while` loop
 --> src/main.rs:2:26
  |
2 |     let x = while true { break 5; };
  |             ----------   ^^^^^^^ can only break with a value inside `loop` or breakable block
  |             |
  |             you can't `break` with a value in a `while` loop
  |
help: use `break` on its own without a value inside this `while` loop
```

**案例 4：`break` 写在循环外面**

```text
error[E0268]: `break` outside of a loop or labeled block
 --> src/main.rs:1:13
  |
1 | fn main() { break; }
  |             ^^^^^ cannot `break` outside of a loop or labeled block
```

**案例 5：`for` 按值遍历移动了 `Vec`，之后又用它**

```text
error[E0382]: borrow of moved value: `v`
 --> src/main.rs:4:20
  |
2 |     let v = vec![1, 2, 3];
  |         - move occurs because `v` has type `Vec<i32>`, which does not implement the `Copy` trait
3 |     for x in v { println!("{x}"); }
  |              - `v` moved due to this implicit call to `.into_iter()`
4 |     println!("{}", v.len());
  |                    ^ value borrowed here after move
  |
help: consider iterating over a slice of the `Vec<i32>`'s content to avoid moving into the `for` loop
  |
3 |     for x in &v { println!("{x}"); }
  |              +
```

这个报错的 `help` 写得很清楚：加一个 `&`。第 5 章会把「移动」这件事彻底讲明白，现在先记住 `for x in &v` 这个写法。

**最后是一个必须认识的警告**：`break` 后面的代码永远不会执行，编译器会提醒你。

```text
warning: unreachable statement
 --> src/main.rs:4:9
  |
3 |         break;
  |         ----- any code following this expression is unreachable
4 |         println!("never");
  |         ^^^^^^^^^^^^^^^^^ unreachable statement
  |
  = note: `#[warn(unreachable_code)]` (part of `#[warn(unused)]`) on by default
```

同样要留意的是：`let x = loop {};` 这种「永远出不来」的循环，编译器也只是警告它后面的代码不可达。**编译器不会替你从死循环里救人**——没有 `break` 的循环它拦不住，真正的卡死只能靠自己检查退出条件，或者在调试时用日志确认循环变量有没有在推进。

另外，写 `while true` 时编译器会提示换成 `loop`：

```text
warning: denote infinite loops with `loop { ... }`
 --> src/main.rs:2:13
  |
2 |     let x = while true { break 5; };
  |             ^^^^^^^^^^ help: use `loop`
  |
  = note: `#[warn(while_true)]` on by default
```

## 4.12 常见坑速查

| 现象 | 原因 | 解法 |
| --- | --- | --- |
| `if` 条件写 `=` 报 `expected bool, found ()` | 赋值不是比较，类型是 `()` | 改成 `==` |
| `if 1` / `if x` 报 `expected bool` | Rust 没有真值判断 | 显式比较，如 `if x != 0` |
| `while` 里 `continue` 后死循环 | 推进语句被 `continue` 跳过 | 把推进放到 `continue` 之前，或改用 `for` |
| `while` / `for` 里 `break 值` 报 `E0571` | 只有 `loop` 支持带值退出 | 换 `loop`，或把值写进外层变量 |
| 多个 `break` 的值类型不一致 | 同一个 `loop` 只能产出一个类型 | 统一类型，或用枚举包一层（第 6 章） |
| `for x in v` 之后 `v` 不能用了 | 按值迭代移动了所有权 | 写 `for x in &v` |
| `break` 只跳出了内层 | 无标签的 `break` 只作用于当前层 | 给外层加标签，`break 'outer` |
| `break` 写在循环外报 `E0268` | `break` 只能待在循环里 | 检查循环范围，或改用 `return` |
| `while let` 一直匹配、停不下来 | 循环体没消费掉被匹配的值 | 让循环体改变匹配对象（如 `pop()`） |
| `let x = loop { };` 后面的代码不可达 | 没有 `break`，循环永远不结束 | 补上 `break 值` 或退出条件 |
| `while true` 有编译警告 | 死循环的惯用写法是 `loop` | 改成 `loop { }` |

## 4.13 练习

1. 写 `fn fizzbuzz(n: u32)`：用 `for` 遍历 `1..=n`，3 的倍数打印 `Fizz`，5 的倍数打印 `Buzz`，同时是两者倍数打印 `FizzBuzz`，其余打印数字本身。
2. 写 `fn collatz_steps(mut n: u64) -> u32`：用 `while` 实现考拉兹序列（偶数除以 2，奇数乘 3 加 1），返回到达 1 需要的步数；`n` 本来就是 1 时返回 0。注意别写出 4.5 节那种被 `continue` 跳过的推进。
3. 写 `fn first_above(values: &[i32], threshold: i32) -> Option<i32>`：返回第一个大于阈值的元素，要求用 `for` 加提前 `return`，对照第 3 章 3.7 的写法。实测 `first_above(&[3, 9, 4, 12], 5)` 应得到 `Some(9)`。
4. 用两层 `for` 打印 9×9 乘法表，要求只打印下三角（`j <= i` 的部分），用 `continue 'rows` 跳过每行剩下的列。
5. 写 `fn find_pair(values: &[i32], target: i32) -> Option<(usize, usize)>`：找两个下标（可以相同）使两数之和等于 `target`，用带标签的 `loop` 在找到时一次跳出两层并返回这对下标。

### 参考答案

::: details 第 2 题

```rust
fn collatz_steps(mut n: u64) -> u32 {
    let mut steps = 0;
    while n != 1 {
        n = if n % 2 == 0 { n / 2 } else { n * 3 + 1 };
        steps += 1;
    }
    steps
}

fn main() {
    println!("{}", collatz_steps(1)); // 0
    println!("{}", collatz_steps(6)); // 8
}
```

这里不需要 `continue`，因为「推进」本身就是循环体的主体。真要用 `continue`，先把 `n` 更新完再判断，就不会跳过了。

:::

::: details 第 4 题

```rust
'rows: for i in 1..=9 {
    for j in 1..=9 {
        if j > i {
            println!();
            continue 'rows;   // 这一行剩下的列不打了，直接进入下一行
        }
        print!("{j}x{i}={:<3}", i * j);
    }
    println!();
}
```

`{:<3}` 让每格左对齐占 3 个字符，列宽才整齐（第 1 章讲过的宽度语法）。

:::

::: details 第 5 题

```rust
fn find_pair(values: &[i32], target: i32) -> Option<(usize, usize)> {
    let len = values.len();
    let mut i = 0;
    'search: loop {
        if i >= len {
            break None;
        }
        let mut j = i;
        while j < len {
            if values[i] + values[j] == target {
                break 'search Some((i, j));
            }
            j += 1;
        }
        i += 1;
    }
}

fn main() {
    println!("{:?}", find_pair(&[2, 7, 11, 15], 9));   // Some((0, 1))
    println!("{:?}", find_pair(&[2, 7, 11, 15], 100)); // None
}
```

`break None;` 也带着值，所以整个 `loop` 的类型是 `Option<(usize, usize)>`，两条出口类型一致——这正是 4.4 节说的「同一个 `loop` 里所有 `break` 的值必须同类型」。

:::

## 4.14 小结

- `if` 的条件必须是 `bool`，没有真值判断；把 `==` 写成 `=` 会在编译期被拦下。

- `else if` 链从上到下短路，顺序决定了哪些分支能被执行到。

- `if` 是表达式，`let x = if c { a } else { b };` 不需要 `mut`，编译器还会保证每条路径都赋值。

- `loop` 是唯一能 `break 值` 的循环，各条 `break` 的类型必须一致；`while` / `for` 里带值 `break` 报 `E0571`。

- `while` 适合条件驱动的循环，但要小心 `continue` 跳过推进语句造成死循环。

- `for` 是遍历的默认选择：`..=` 含尾、`..` 不含尾，`rev()` / `step_by()` 改变走法，`enumerate()` 同时给下标。

- `for x in v` 会移动 `Vec`，只想看内容用 `&v`；数组元素是 `Copy` 时，按值迭代相当于复制。

- `break` / `continue` 只作用于当前一层，加标签（`'outer`）才能一次跳出多层；带标签的块也能用 `break 'blk 值` 产出值。

- `while let` 把「一直取到没有为止」写得最短，但要保证循环体真的在消费被匹配的值。

- 循环相关的报错先看错误码：`E0308` 多半是条件类型不对，`E0571` 是 `break` 带值用错了循环，`E0268` 是 `break` 不在循环里，`E0382` 是 `for` 移动了容器。

下一章进入 Rust 最有分量的部分：**所有权与借用**。这一章留下的那个问题——「`for x in v` 之后 `v` 为什么不能用了」——下一章会给出完整的规则，而不只是「加个 `&`」的修法。
