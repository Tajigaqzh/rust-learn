# 第 7 章 · 常用集合

集合是「一组同类型值的容器」。日常写代码，绝大多数需求都落在这三个类型上：

| 集合 | 装什么 | 底层结构 | 典型用途 |
| --- | --- | --- | --- |
| `Vec<T>` | 一串同类型元素，有序、可重复 | 连续内存（动态数组） | 列表、排序、批量处理 |
| `String` | UTF-8 文本 | 本质是 `Vec<u8>` 加一层保证 | 拼接、解析、格式化 |
| `HashMap<K, V>` | 键值对，按键查找 | 哈希表 | 计数、索引、缓存、配置 |

这一章的重点不只是「有哪些方法」，还有三件事：**所有权规则在集合上怎么体现**（第 5 章的内容在这里最容易撞上）、**复杂度**（知道哪个操作会变慢）、以及**几个高频坑**（`sort` 排不了浮点、字符串不能按下标取、`HashMap` 顺序不固定）。

本章配套代码在 `src/rust07_collections/mod.rs`，执行 `cargo run` 可以看到全部输出。

## 7.1 `Vec<T>`：创建与增删改查

```rust
let mut scores = vec![88, 92, 75];   // 宏写法，最常用
let empty: Vec<i32> = Vec::new();    // 空的，需要标注类型

scores.push(100);                    // 末尾追加
scores.insert(0, 60);                // 指定位置插入
let removed = scores.remove(0);      // 按下标删除并返回
let popped = scores.pop();           // 从末尾弹出，返回 Option<T>
```

实测：

```text
    push(100) 之后 = [88, 92, 75, 100]
    insert(0, 60) 之后 = [60, 88, 92, 75, 100]
    remove(0) 移除了 60，剩下 = [88, 92, 75, 100]
    pop() 拿走 Some(100)，剩下 = [88, 92, 75]
    len = 3，第一个 = 88，最后一个 = 75
```

注意 `pop()` 返回的是 `Option<T>`（第 6 章的内容）：空 `Vec` 上 `pop()` 得到 `None` 而不是错误。而 `remove(i)` 在下标越界时会 panic，用它之前要保证下标有效。

复杂度值得记一下：

| 操作 | 复杂度 | 说明 |
| --- | --- | --- |
| `push` / `pop`（末尾） | 均摊 O(1) | 偶尔触发扩容，摊销下来是常数 |
| `insert(0, x)` / `remove(0)`（头部） | O(n) | 后面的元素都得整体挪一格 |
| `v[i]` 按下标访问 | O(1) | 连续内存，直接算地址 |
| 按值查找（`contains`） | O(n) | 得从头扫 |

所以「频繁在头部插入删除」不适合 `Vec`，那是 `VecDeque` 的场景；「频繁按值查找」则应该考虑 `HashMap`。

## 7.2 下标 vs `get`

两种访问方式，区别只有一个：越界时怎么办。

```rust
let scores = vec![88, 92, 75];
scores[1];        // 92，越界会 panic
scores.get(1);    // Some(&92)，越界返回 None
```

实测：

```text
    scores.get(1) = Some(92)
    scores.get(99) = None
    first / last 安全取值 = 88 / 75
```

写 `scores[99]` 的运行时输出是：

```text
thread 'main' panicked at src/main.rs:3:21:
index out of bounds: the len is 3 but the index is 99
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
```

panic 本身不算「未定义行为」，但这行程序就没了。**下标来自外部输入时（用户输入、解析出来的数字、计算结果）一律用 `get`**，把「可能没有」交给 `Option`，让调用方决定怎么办；只有下标是自己代码里刚检查过、确实不可能越界时，才用 `[]`。

`first()` / `last()` 是另外两个常用方法，返回 `Option<&T>`，很适合「可能空」的场景：

```rust
let first = scores.first().copied().unwrap_or_default();   // 88
```

## 7.3 遍历与借用规则

第 5 章的借用规则在 `Vec` 上体现得最直接：

```rust
let mut scores = vec![88, 92, 75];

for score in &scores { }        // 只读借用，不能改
for score in &mut scores {      // 可变借用，能改元素
    if *score < 90 {
        *score += 5;            // 改自己这一项，合法
    }
}
for score in scores { }         // 按值迭代，Vec 被移动（元素 Copy 时相当于复制）
```

实测第二种写法之后：`小于 90 的加 5 分 = [93, 92, 80]`。

但下面这种就编不过：

```rust
for x in &v {
    if *x == 2 {
        v.push(4);      // 报错
    }
}
```

遍历是**不可变借用**，`push` 需要**可变借用**，两者不能同时存在；而且 `push` 可能触发扩容，把元素搬到新地址，正在遍历的引用会立刻失效。这正是借用规则要拦住的场景（报错原文见 7.13 案例 1）。

需要「边看边改」时的三种做法：

```rust
// 1. 先收集要改的内容，循环外再动原 Vec
let mut to_add = Vec::new();
for x in &v {
    if *x == 2 {
        to_add.push(4);
    }
}
v.extend(to_add);

// 2. 只保留或删除元素：用 retain，最省事
v.retain(|n| *n != 2);

// 3. 按下标循环：能编过，但要小心越界和长度变化
for i in 0..v.len() {
    if v[i] == 2 {
        v.remove(i);
        break;
    }
}
```

「先收集、后修改」是最通用的思路，第 11 章学了迭代器之后会写得更好看。

## 7.4 `len` 与 `capacity`

这两个数字经常被混淆：

- `len()`：当前**装了几个**元素。
- `capacity()`：当前**已经分配了多大**的空间，不用重新分配就能再装多少。

```rust
let mut numbers = Vec::with_capacity(10);
// len = 0, capacity = 10
numbers.push(1);
numbers.push(2);
numbers.push(3);
// len = 3, capacity = 10
numbers.shrink_to_fit();
// len = 3, capacity = 3
```

实测输出正是这三行。理解它的意义在于：`push` 超出 `capacity` 时，`Vec` 会**申请一块更大的内存、把元素搬过去、释放旧的**——这件事本身是 O(n)。因为容量是成倍增长的，均摊到每次 `push` 仍然是 O(1)；但如果能提前知道大概要装多少，`with_capacity(n)` 可以省掉这几次搬家：

```rust
let mut v = Vec::with_capacity(1000);   // 明确要装 1000 个
v.reserve(500);                          // 已经建好了，再预留 500 个位置
```

`len` 决定能访问哪些下标，`capacity` 只是性能细节，不会出现在 `{:?}` 里。

## 7.5 `Vec` 常用方法

一批值得记住的方法：

```rust
let mut data = vec![3, 1, 4, 1, 5, 9, 2, 6];
data.sort();                         // 原地升序（元素需要 Ord）
data.dedup();                        // 去掉相邻重复（一般先 sort）
data.retain(|n| n % 2 == 0);         // 只留下满足条件的
data.contains(&4);                   // 是否包含某个值，O(n)
```

实测：`sort = [1, 1, 2, 3, 4, 5, 6, 9]`、`dedup = [1, 2, 3, 4, 5, 6, 9]`、`retain(只留偶数) = [2, 4, 6]`。

`windows` 和 `chunks` 用来「按窗口」或「按块」看数据：

```rust
for window in data.windows(2) { }    // [2,4] [4,6]：相邻两个一组，窗口平移
for chunk in data.chunks(2) { }      // [2,4] [6]：每两个一组，不重叠
```

`split_off` 把一个 `Vec` 从中间切开，返回后半段：

```rust
let mut left = vec![1, 2, 3, 4, 5];
let right = left.split_off(3);       // left = [1,2,3]，right = [4,5]
```

方法很多，不必背。记住三个思路就够了：**要改就用 `&mut` 上的方法**（`sort` / `retain` / `push`），**要问就用 `&self` 上的方法**（`contains` / `len` / `windows`），**要拿走元素就用返回 `Option` 的方法**（`pop` / `get`）。

## 7.6 `String` 的本质：UTF-8 字节序列

第 5 章提过一句「字符串切片按字节算」，这里把账算清楚：

```rust
let zh = String::from("中文abc");
zh.len();               // 9：字节数
zh.chars().count();     // 5：字符数
```

实测：

```text
    「中文abc」len()（字节）= 9，chars().count()（字符）= 5
    bytes(): 228 184 173 230 150 135 97 98 99
    chars(): 中 文 a b c
```

一个汉字占 3 个字节，所以 3 个字节的中文 + 3 个 ASCII 字符一共 9 个字节。`String` 存的就是这 9 个字节——它**不知道**什么是「第 1 个字符」，只知道第几个字节。

这解释了三件事：

1. **`len()` 是字节数**，不是字数。做「字数统计」要用 `chars().count()`；英文单词数则用 `split_whitespace().count()`。
2. **不能用 `s[0]` 取值**（报错见 7.13 案例 3），因为「第 0 个字节」不是一个完整字符，Rust 干脆不允许。
3. **按字节切片可能 panic**：`&zh[..1]` 切到汉字中间，运行时直接 panic（第 5 章 5.9 有完整报错）。安全写法是 `zh.chars().take(2).collect::<String>()` 或者 `zh.get(..n)`。

按字节遍历用 `bytes()`，按字符遍历用 `chars()`；真要按「用户感知的一个字」处理（比如 emoji 组合、带音标的字母），还需要 grapheme cluster，那是第 18 章 `unicode-segmentation` 的内容。

## 7.7 `String` 常用操作

```rust
let mut text = String::from("  rust  ");
text.trim();                    // "rust"，返回切片，不修改原串
text.push_str("learn");         // 追加字符串
text.push('!');                 // 追加单个字符
```

实测：`trim = [rust]`，`push_str` 之后是 `[  rust  learn]`（原来的空格还在，`trim` 只是返回了一个视图），`push('!')` 之后是 `[  rust  learn!]`。

拼接的三种方式，注意所有权：

```rust
let hello = String::from("hello ");
let world = String::from("world");
let joined = hello + &world;    // hello 被移动，world 只是借用

let parts = ["rust", "learn", "note"];
parts.join("-");                 // "rust-learn-note"，不改任何原值

format!("{hello} {world}");      // 谁都不动，返回新 String（推荐）
```

实测 `+ 拼接 = [hello world]，world 还能用 = [world]`——`+` 的左侧必须交出所有权（因为要在它的缓冲区后面追加），右侧只要 `&str` 就行。**能选的话优先 `format!`**：它不消费任何操作数，可读性也更好。

其他高频方法：

```rust
joined.replace("world", "rust");   // 替换，返回新 String
joined.starts_with("hello");       // true
joined.contains("world");          // true
"42".trim().parse::<i32>();        // Ok(42)，失败返回 Err
"a,b,c".split(',');                // 迭代出 &str 片段
"中文".to_uppercase();             // 能正确处理 Unicode
```

实测：`replace = hello rust`、`starts_with("hello") = true，contains("world") = true`、`"42".trim().parse::<i32>() = Ok(42)`。

`parse` 返回 `Result`（第 8 章细讲），失败时可以用 `unwrap_or` 兜底：

```rust
let n: i32 = "abc".parse::<i32>().unwrap_or(-1);   // -1，不 panic
```

## 7.8 `String` 与 `&str` 之间的转换

四个方向的转换记住这几行就够：

```rust
let owned = String::from("owned");

let borrowed: &str = &owned;        // &String -> &str，最常见的「借用」
let same: &str = owned.as_str();    // 等价写法，意图更明确

let from_literal = "literal".to_string();      // &str -> String
let from_literal2 = String::from("literal");   // 等价写法
```

实测两行借用输出都是 `[owned]`，两个字面量转换输出都是 `[literal]`。

为什么可以直接把 `&owned` 传给要 `&str` 的函数（而不是先转成 `&str`）？因为 `String` 实现了 `Deref<Target = str>`，编译器在需要 `&str` 的地方会自动把 `&String` 转过去，这个机制叫 **deref coercion**（第 12 章讲 `Deref` 时会解释原理）。

实测：

```text
    shout(&owned) = OWNED!
    shout("literal") = LITERAL!
```

同一个函数既收得了 `String` 的借用，也收得了字面量——这就是第 5 章 5.10 那条「参数优先收 `&str`」的实际收益。

## 7.9 `HashMap`：插入与查询

```rust
use std::collections::HashMap;

let mut ages: HashMap<String, u32> = HashMap::new();
ages.insert(String::from("ada"), 36);
ages.insert(String::from("ada"), 37);      // 同一个键：覆盖旧值
ages.get("ada");                            // Some(&37)
ages.get("grace");                          // None
ages.contains_key("ada");                    // true
```

实测 `len = 2`（`ada` 只算一次）、`get("ada") = Some(37)`、`get("grace") = None`、`contains_key("linus") = true`。

两个细节：

1. **`insert` 返回旧值**。`insert(k, v)` 的返回值是 `Option<V>`——如果这个键原来有值，你会拿到旧的那个。要做「只插入不覆盖」，用 `.entry(k).or_insert(v)`（下一节）。
2. **`get` 接受 `&K`，但也能用别的类型查**。键是 `String`，`ages.get("ada")` 传的是 `&str` 却照样能查——这是 `Borrow` trait 的作用（第 9 章会讲 trait）。传错类型会怎样？报错见 7.13 案例 5。

和 `Vec` 一样，`get` 返回 `Option`，`map[k]` 则在键不存在时 panic：

```text
thread 'main' panicked at src/main.rs:6:23:
no entry found for key
```

所以「不确定存不存在」时一律 `get`。

## 7.10 `entry`：缺失就插入，存在就更新

「统计词频」是最经典的需求，新手写法是先 `get` 再 `insert`：

```rust
// 笨办法：查两次，还得处理「第一次见」的分支
if let Some(count) = counts.get(word) {
    counts.insert(word, count + 1);
} else {
    counts.insert(word, 1);
}
```

`entry` API 把这三行变成一行：

```rust
for word in text.split_whitespace() {
    *counts.entry(word).or_insert(0) += 1;
}
```

读法：`entry(word)` 拿到这个键的「入口」，`or_insert(0)` 表示「没有就插入 0」，返回的是**值的可变引用**，所以可以直接 `+= 1`。

实测统计 `"the quick brown the lazy the"`：

```text
    brown -> 1
    lazy -> 1
    quick -> 1
    the -> 3
```

`entry` 家族还有几个变体，按需选：

| 方法 | 含义 |
| --- | --- |
| `or_insert(v)` | 不存在就插入 `v`，返回值的可变引用 |
| `or_insert_with(f)` | 同上，但插入的值靠闭包算出来（比较贵时才用） |
| `or_default()` | 不存在就插入类型的默认值，比如 `0`、空 `Vec` |
| `and_modify(f).or_insert(v)` | 存在就先改，不存在再插入 |

## 7.11 遍历 `HashMap`

```rust
for (name, year) in &langs {        // 迭代出 (&K, &V)
    println!("{name} {year}");
}
for name in langs.keys() { }        // 只要键
for year in langs.values() { }      // 只要值
for (name, year) in langs { }       // 按值迭代，map 被移动（第 5 章的规则）
```

实测（先按键排序再打印）：

```text
    python 发布于 1991
    go 发布于 2009
    rust 发布于 2015
```

**`HashMap` 的遍历顺序是不确定的**：它取决于哈希值和内部桶的布局，同一份数据在不同运行、不同版本里顺序都可能不同。所以：

- 想按固定顺序打印，先收集到 `Vec` 再 `sort`（配套代码里的 `sorted_counts` 就是这么写的）；
- 不要在测试里断言「第一个元素是某某」；
- 需要保持插入顺序就用 `BTreeMap`（按 key 排序）或其他有序结构。

## 7.12 自定义类型当键

键必须实现 `Hash` 和 `Eq`，自定义类型加一行 `derive` 就行：

```rust
#[derive(Debug, Hash, PartialEq, Eq)]
struct Coord {
    x: i32,
    y: i32,
}

let mut grid: HashMap<Coord, &str> = HashMap::new();
grid.insert(Coord { x: 0, y: 0 }, "起点");
grid.get(&Coord { x: 1, y: 2 });     // 按值查，Some("目标")
```

实测 `表里有 2 项`、`get(1, 2) = Some("目标")`。

漏了 derive 会报一条不太直观的错误（`insert` 方法「存在但 trait bound 不满足」，见 7.13 案例 4）——`help` 里会直接告诉你补 `#[derive(Eq, Hash, PartialEq)]`。

顺带一提，配套代码在 `sorted_counts` 上踩了一次生命周期：返回值里的 `&str` 借的是 map 里的键，编译器分不清该跟「map 的借用」还是「键自己的生命周期」走，所以必须手写 `<'a>`。报错原文也在 7.13 案例 7。

## 7.13 七个真实报错怎么读

**案例 1：边遍历边 `push`（`E0502`，顺带一个 `E0596`）**

```text
error[E0596]: cannot borrow `v` as mutable, as it is not declared as mutable
 --> src/main.rs:5:13
  |
5 |             v.push(4);
  |             ^ cannot borrow as mutable
  |
help: consider changing this to be mutable
  |
2 |     let mut v = vec![1, 2, 3];
  |         +++
error[E0502]: cannot borrow `v` as mutable because it is also borrowed as immutable
 --> src/main.rs:5:13
  |
3 |     for x in &v {
  |              --
  |              |
  |              immutable borrow occurs here
  |              immutable borrow later used here
5 |             v.push(4);
  |             ^^^^^^^^^ mutable borrow occurs here
```

两个错误一起看：先要 `mut`（不然连可变借用都不允许），加了 `mut` 之后仍然会撞上 `E0502`——遍历期间的不可变借用还没结束。解法见 7.3。

**案例 2：给浮点数组排序（`E0277`）**

```rust
let mut v = vec![3.0, 1.0, 2.0];
v.sort();
```

```text
error[E0277]: the trait bound `{float}: Ord` is not satisfied
 --> src/main.rs:3:7
  |
3 |     v.sort();
  |       ^^^^ the trait `Ord` is not implemented for `{float}`
  |
  = help: the following other types implement trait `Ord`:
            i128
            i16
            i32
            ...
```

原因：浮点数有 `NaN`，`NaN != NaN`，所以没法定义全序，只有偏序 `PartialOrd`。`sort()` 要求 `Ord`，因此编不过。改用 `sort_by` 自己给规则：

```rust
v.sort_by(|a, b| a.partial_cmp(b).unwrap());                  // 确认没有 NaN
v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));  // NaN 当相等处理
```

**案例 3：用下标取字符串里的字符（`E0277`）**

```text
error[E0277]: the type `str` cannot be indexed by `{integer}`
   --> src/main.rs:3:22
    |
3 |     println!("{}", s[0]);
    |                      ^ string indices are ranges of `usize`
    |
    = help: the trait `SliceIndex<str>` is not implemented for `{integer}`
    = note: you can use `.chars().nth()` or `.bytes().nth()`
```

`help` 直接给了替代方案：`s.chars().nth(0)`。不过要注意 `nth` 是 O(n) 的，循环里连续取字符时应该先用 `chars()` 收集，或者直接迭代。

**案例 4：自定义类型当键但没实现 `Hash`/`Eq`（`E0599`）**

```text
error[E0599]: the method `insert` exists for struct `HashMap<Point, i32>`, but its trait bounds were not satisfied
 --> src/main.rs:7:9
  |
3 | struct Point { x: i32, y: i32 }
  | ------------ doesn't satisfy `Point: Eq` or `Point: Hash`
...
7 |     map.insert(Point { x: 1, y: 2 }, 10);
  |         ^^^^^^
  |
  = note: the following trait bounds were not satisfied:
          `Point: Eq`
          `Point: Hash`
help: consider annotating `Point` with `#[derive(Eq, Hash, PartialEq)]`
  |
3 + #[derive(Eq, Hash, PartialEq)]
4 | struct Point { x: i32, y: i32 }
  |
```

报错说「方法存在但约束不满足」，`help` 里连要 derive 哪几个 trait 都列好了。

**案例 5：`get` 传错了类型（`E0308`）**

```rust
let mut map: HashMap<String, i32> = HashMap::new();
map.insert(String::from("a"), 1);
map.get(0);
```

```text
error[E0308]: mismatched types
    --> src/main.rs:6:30
     |
   6 |     println!("{:?}", map.get(0));
     |                          --- ^ expected `&_`, found integer
     |                          |
     |                          arguments to this method are incorrect
     |
     = note: expected reference `&_`
                     found type `{integer}`
help: consider borrowing here
     |
   6 |     println!("{:?}", map.get(&0));
     |                              +
```

两个信息：`get` 要的是**引用**（`&K`），而且键的类型必须对得上（这里是 `String`，不是整数）。

**案例 6：用 `+` 拼接字符串后原变量不能用了（`E0382`）**

```text
error[E0382]: borrow of moved value: `a`
 --> src/main.rs:6:16
  |
2 |     let a = String::from("hello ");
  |         - move occurs because `a` has type `String`, which does not implement the `Copy` trait
4 |     let c = a + &b;
  |             - value moved here
6 |     println!("{a}");
  |                ^ value borrowed here after move
  |
help: consider cloning the value if the performance cost is acceptable
  |
4 |     let c = a.clone() + &b;
  |              ++++++++
```

`+` 会消费左边的 `String`。想保留原来的值，要么 `a.clone() + &b`，要么改用 `format!("{a}{b}")`（推荐，不消费任何一方）。

**案例 7：返回 `HashMap` 里的键，需要生命周期（`E0106`）**

```rust
fn sorted_counts(counts: &HashMap<&str, usize>) -> Vec<(&str, usize)> {
```

```text
error[E0106]: missing lifetime specifier
  --> src/rust07_collections/mod.rs:17:57
   |
17 | fn sorted_counts(counts: &HashMap<&str, usize>) -> Vec<(&str, usize)> {
   |                          ---------------------          ^ expected named lifetime parameter
   |
   = help: this function's return type contains a borrowed value, but the signature does not say which one of `counts`'s 2 lifetimes it is borrowed from
help: consider introducing a named lifetime parameter
   |
17 | fn sorted_counts<'a>(counts: &'a HashMap<&'a str, usize>) -> Vec<(&'a str, usize)> {
   |                 ++++          ++          ++                       ++
```

这里有两个生命周期：`counts` 这个引用本身，以及 map 里键自己的 `&str`。返回值里的 `&str` 借的是后者，所以标注成：

```rust
fn sorted_counts<'a>(counts: &HashMap<&'a str, usize>) -> Vec<(&'a str, usize)> {
```

这段代码是写配套示例时真实撞上的，第 10 章会把生命周期的规则讲透。

## 7.14 常见坑速查

| 现象 | 原因 | 解法 |
| --- | --- | --- |
| 边遍历边 `push`/`remove` 报 `E0502` | 遍历期间是借用 | 先收集再改，或用 `retain` |
| `v.sort()` 对 `Vec<f64>` 报 `Ord` 未实现 | 浮点只有偏序 | `sort_by(\|a, b\| a.partial_cmp(b).unwrap())` |
| `s[0]` 取字符报 `str cannot be indexed` | 字符串按字节，不能按字符下标 | `s.chars().nth(0)`，或直接 `for c in s.chars()` |
| 中文字符串 `len()` 比字数大 | `len()` 是字节数 | 用 `chars().count()` |
| `&s[..n]` 切中文 panic | 切在字符中间 | `chars().take(n).collect::<String>()` 或 `get(..n)` |
| `map["missing"]` panic | 下标访问要求键存在 | 用 `get`，配 `unwrap_or` |
| `HashMap` 遍历顺序每次都不同 | 哈希布局不保证有序 | 先收集到 `Vec` 排序，或换 `BTreeMap` |
| `insert` 覆盖了旧值 | `insert` 的语义就是覆盖 | 用 `entry().or_insert()` 只插不改 |
| 自定义类型当键报 trait bound | 缺少 `Hash`/`Eq` | `#[derive(Hash, PartialEq, Eq)]` |
| `a + &b` 之后 `a` 不能用了 | `+` 消费左操作数 | `format!` 或 `a.clone() + &b` |
| 返回 map 里的键报 `E0106` | 需要指明借用哪一个生命周期 | 手写 `<'a>` 标注（第 10 章） |
| `with_capacity` 之后 `len` 不是 0 | `len` 是元素个数，不是容量 | 分清 `len()` 与 `capacity()` |

## 7.15 练习

1. 写 `fn average(values: &[f64]) -> Option<f64>`，空切片返回 `None`，否则返回平均值。
2. 写 `fn unique_sorted(values: &[i32]) -> Vec<i32>`，返回排序并去重后的新 `Vec`（不要修改传入的切片）。
3. 写 `fn count_chars_map(text: &str) -> HashMap<char, usize>`，统计每个字符出现的次数（想想为什么用 `char` 当键而不是 `String`）。
4. 写 `fn top_n(scores: &HashMap<String, u32>, n: usize) -> Vec<(String, u32)>`，按分数从高到低返回前 `n` 名；分数相同时按名字升序。
5. 给定 `let mut v = vec![3.0, 1.0, 2.0];`，解释 `v.sort()` 为什么编不过，并用 `sort_by` 写出能编过的版本（再加一句：如果数据里可能出现 `NaN`，你会怎么处理）。

（第 1、2 题是 7.1 和 7.5 示例的直接变体，这里不再单独给答案。）

### 参考答案

::: details 第 3 题

```rust
use std::collections::HashMap;

fn count_chars_map(text: &str) -> HashMap<char, usize> {
    let mut counts = HashMap::new();
    for c in text.chars() {
        *counts.entry(c).or_insert(0) += 1;
    }
    counts
}

fn main() {
    let counts = count_chars_map("hello");
    let mut items: Vec<(char, usize)> = Vec::new();
    for (c, n) in &counts {
        items.push((*c, *n));
    }
    items.sort();
    println!("{items:?}");   // [('e', 1), ('h', 1), ('l', 2), ('o', 1)]
}
```

用 `char` 而不是 `String` 当键：`char` 是 `Copy` 的定长类型，哈希和比较都便宜；用 `String` 每个键都要单独分配一次堆内存。而且 `char` 天然按「字符」切分，不会出现把多字节字符拆坏的问题。

注意 `items.sort()` 需要 `(char, usize)` 实现 `Ord`——元组会按第一个字段、再按第二个字段比较，所以输出是按字符排好序的。

:::

::: details 第 4 题

```rust
use std::collections::HashMap;

fn top_n(scores: &HashMap<String, u32>, n: usize) -> Vec<(String, u32)> {
    let mut items: Vec<(String, u32)> = Vec::new();
    for (name, score) in scores {
        items.push((name.clone(), *score));
    }
    // 分数从高到低；分数相同再按名字升序
    items.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    items.truncate(n);
    items
}

fn main() {
    let mut scores = HashMap::new();
    scores.insert(String::from("ada"), 90);
    scores.insert(String::from("linus"), 75);
    scores.insert(String::from("grace"), 99);
    println!("{:?}", top_n(&scores, 2));
    // [("grace", 99), ("ada", 90)]
}
```

两个细节：`name.clone()` 是因为要返回拥有所有权的 `String`（返回值不能借 `scores`，否则又回到案例 7 的生命周期问题）；`sort_by` 里的比较顺序写成 `b.1.cmp(&a.1)` 就是「降序」，后面的 `.then(...)` 决定并列时的次序——**排序要求必须是全序**，否则结果不稳定。

:::

::: details 第 5 题

`sort()` 要求元素实现 `Ord`（全序）。`f64` 只有 `PartialOrd`，因为 `NaN` 和任何值比较都是 `false`，包括它自己，无法满足全序的要求——所以编译器直接拒绝。

```rust
fn main() {
    let mut v = vec![3.0, 1.0, 2.0];
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    println!("{v:?}");   // [1.0, 2.0, 3.0]
}
```

如果数据里可能有 `NaN`，`partial_cmp` 会返回 `None`，上面的 `unwrap()` 会 panic。这时要么用 `unwrap_or(Ordering::Equal)` 把 `NaN` 当成「相等」让 `Vec` 自己决定位置，要么事先清洗数据（比如把 `NaN` 过滤掉），要么改用 `f64::total_cmp`（它给浮点定了一个确定的全序，包含 `NaN`）。

:::

## 7.16 小结

- `Vec<T>` 是默认的「一串东西」：末尾 `push`/`pop` 均摊 O(1)，头部 `insert`/`remove` 是 O(n)。

- 下标 `v[i]` 越界会 panic，`v.get(i)` 返回 `Option`；**来源不可信的下标一律用 `get`**。

- 遍历有 `&v`（只读）、`&mut v`（改元素）、按值（移动）三种；边遍历边改长度会被借用检查拦住，用「先收集后修改」或 `retain`。

- `len` 是元素个数，`capacity` 是已分配空间；能预估规模就用 `with_capacity` 少搬几次家。

- `String` 本质是 UTF-8 字节序列：`len()` 是字节数，`chars().count()` 是字符数，不能按下标取字符，按字节切片可能 panic。

- 拼接优先 `format!`（不消费操作数）；`+` 会消费左边的 `String`。

- 参数收 `&str` 时 `String` 和字面量都能传，靠的是 `Deref` 带来的 deref coercion。

- `HashMap` 的 `get` 返回 `Option`、`map[k]` 会 panic；`entry().or_insert()` 是「没有就插入、有就更新」的标准写法。

- `HashMap` 不保证遍历顺序，要稳定顺序就先排序；键必须实现 `Hash + Eq`，自定义类型用 `derive` 一行搞定。

- 集合上撞到的报错多半还是第 5 章那几类：`E0502`（借用冲突）、`E0382`（移动后使用）、`E0106`（生命周期标注）。

下一章讲**错误处理**：`Result<T, E>`、`?` 运算符、自定义错误类型，以及什么时候该 `panic!`、什么时候该返回错误。
