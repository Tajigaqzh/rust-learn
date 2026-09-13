# 第 12 章 · 智能指针

第 5 章的所有权规则有一个前提：**一个值只有一个所有者**。但真实程序里有两类需求它满足不了：

1. **一个值需要多个所有者**：图结构、树的双向链接、多处共享的配置、被多个模块引用的缓存。
2. **在不可变引用里修改数据**：带缓存的函数、计数器、观察者列表。

这一章的工具就是为这两种需求准备的：

| 类型 | 解决什么 | 代价 |
| --- | --- | --- |
| `Box<T>` | 把值放到堆上，仍然只有一个所有者 | 一次堆分配 |
| `Rc<T>` | 多个所有者（单线程） | 维护引用计数 |
| `Arc<T>` | 多个所有者（多线程） | 原子引用计数，略慢 |
| `RefCell<T>` | 内部可变性（运行时检查借用） | 借用错误从编译期推迟到运行期 |
| `Weak<T>` | 不增加所有权计数的「弱引用」 | 使用时必须 `upgrade()` |

它们都叫「智能指针」：像指针一样指向别处的数据，但**额外带着所有权和生命周期管理的能力**。

本章配套代码在 `src/rust12_smart_pointers/mod.rs`，执行 `cargo run` 可以看到全部输出。

## 12.1 智能指针是什么

普通引用 `&T` 只借用数据，不负责释放。智能指针则**拥有**它指向的数据，并负责在合适的时候释放。

其实你已经用过两个了：`String` 和 `Vec<T>` 都是智能指针——它们在栈上存「指针 + 长度 + 容量」，堆上存真正的数据，离开作用域时自动释放（第 5 章的 `Drop`）。

这一章要看的几个新角色，都是在「拥有」这件事上做文章：

```text
Box<T>      ── 唯一所有者，数据在堆上
Rc<T>       ── 多个所有者，靠计数决定何时释放
Arc<T>      ── 多线程版的 Rc
RefCell<T>  ── 数据本身不可变，但内部可以在运行时借出来改
Weak<T>     ── 指向 Rc 管理的对象，但不增加引用计数
```

它们都实现了 `Deref`（所以能像引用一样用）和 `Drop`（所以会自动释放）。这两个 trait 是智能指针的「标配」，先看它们。

## 12.2 `Box<T>`：把值放到堆上

`Box::new(v)` 把 `v` 放到堆上，返回一个「拥有它」的指针：

```rust
let boxed = Box::new(5);
println!("{boxed}");         // 5
println!("{}", *boxed + 1);  // 6，用 * 解引用
```

实测 `Box::new(5) = 5`、`*boxed = 5`。`Box` 的用处主要有三个：

1. **递归类型**：类型大小必须在编译期确定，递归定义需要一层间接（下一节）。
2. **转移大块数据**：把一个很大的结构体放进 `Box`，函数传参时移动的只是一个指针。
3. **装 trait 对象**：第 9 章的 `Box<dyn Error>`、`Box<dyn Summary>`。

`Box` 的额外能力就这么点，它**没有**引用计数、也没有内部可变性——只是一个「放在堆上、独占所有权」的盒子。

## 12.3 递归类型必须间接化

想定义一个链表：

```rust
enum List {
    Cons(i32, List),
    Nil,
}
```

```text
error[E0072]: recursive type `List` has infinite size
 --> src/main.rs:1:1
  |
1 | enum List {
  | ^^^^^^^^^
2 |     Cons(i32, List),
  |               ---- recursive without indirection
  |
help: insert some indirection (e.g., a `Box`, `Rc`, or `&`) to break the cycle
  |
2 |     Cons(i32, Box<List>),
  |               ++++    +
```

原因很直白：`List` 里含一个 `List`，那个 `List` 里又含一个……大小无限，编译器没法给它分配空间。`Box` 把「下一个节点」放到堆上，栈上就只剩一个固定大小的指针：

```rust
#[derive(Debug)]
enum List {
    Cons(i32, Box<List>),
    Nil,
}
```

实测打印出来是 `Cons(1, Cons(2, Cons(3, Nil)))`，`sum = 6`、`to_vec = [1, 2, 3]`。

注意 `push` 的写法：

```rust
fn push(self, value: i32) -> Self {
    List::Cons(value, Box::new(self))   // self 被移动进新节点
}
```

头插需要把整个旧链表移动进新节点——这在 Rust 里是零成本的（只移动一个指针），性能上和 C 的链表头插一样。

## 12.4 `Deref`：让自定义类型像引用一样用

`Box<T>` 能写出 `*boxed`，靠的是 `Deref` trait：

```rust
impl<T> Deref for Wrapper<T> {
    type Target = T;
    fn deref(&self) -> &T { &self.value }
}
```

实现之后，`Wrapper<String>` 就能直接调用 `String` 的方法：

```rust
let wrapper = Wrapper { value: String::from("hello") };
wrapper.len();            // 5，自动解引用到 String
wrapper.to_uppercase();   // HELLO
```

实测输出 `wrapper.len() = 5`、`wrapper.to_uppercase() = HELLO`。

更常见的是 **deref coercion**（自动解引用转换）：`&Box<String>` 会自动变成 `&str`：

```rust
let boxed_string = Box::new(String::from("boxed"));
let as_str: &str = &boxed_string;     // 连着解引用两层：Box -> String -> str
```

这就是第 5 章那个「`&String` 为什么能当 `&str` 传」的原理，也是 `&Vec<T>` 能传给收 `&[T]` 的函数、`&Rc<String>` 能当 `&str` 用的原因。**deref coercion 是编译器在函数调用和赋值处自动插入的转换**，让你不必手写 `&*boxed_string`。

一个提醒：`Deref` 只该用于「确实是某种包装/指针」的类型。`struct Meters(f64)` 实现 `Deref<Target = f64>` 会让 `meters + 1.0` 这种本该报错的运算悄悄通过——**新类型（newtype）的价值就在于不让它自动退化成内部类型**（第 6 章 6.5 的结论）。

## 12.5 `Drop` 与「不能同时 `Copy` 和 `Drop`」

第 5 章讲过 `Drop`：值离开作用域时自动调用。两个补充点：

**释放在离开作用域时逆序进行**：

```text
    块结束前
    释放 second
    释放 first
    块已经结束
```

**实现了 `Drop` 的类型，不能把字段移动出去**：

```rust
struct Guard(String);
impl Drop for Guard { /* ... */ }

let guard = Guard(String::from("x"));
let inner = guard.0;      // 报错
```

```text
error[E0509]: cannot move out of type `Guard`, which implements the `Drop` trait
  -->
11 |     let inner = guard.0;
   |                 ^^^^^^^
   |                 cannot move out of here
   |
help: consider borrowing here
   |
11 |     let inner = &guard.0;
   |                 +
help: consider cloning the value if the performance cost is acceptable
   |
11 |     let inner = guard.0.clone();
   |                        ++++++++
```

原因：`Drop` 会在值销毁时运行，如果字段已经被移动走，`drop` 里访问它就会出问题。想拿出来就借用（`&guard.0`）或者克隆。

顺带解释一个经典规则：**`Copy` 和 `Drop` 不能同时实现**。`Copy` 意味着赋值时随意复制，如果同时有 `Drop`，那么复制出来的每一份都会在销毁时执行一次析构——同一块资源被释放多次，正是第 5 章说的双重释放。编译器直接在类型定义处就禁止了这种组合。

## 12.6 `Rc<T>`：共享所有权

`Rc`（reference counting，引用计数）让一个值可以有多个所有者。每次 `Rc::clone` 只是把计数 +1，**不复制底层数据**：

```rust
use std::rc::Rc;

let shared = Rc::new(vec![1, 2, 3]);
Rc::strong_count(&shared);        // 1
{
    let clone_a = Rc::clone(&shared);
    let clone_b = Rc::clone(&shared);
    Rc::strong_count(&shared);    // 3
}
// 出了块，两个克隆被丢弃，计数回到 1
```

实测计数变化正是 `1 → 3 → 1`，而且三份引用打印出来都是 `[1, 2, 3]`——它们指向同一块数据。

计数归零时数据才被释放。这里的 clone 也值得注意：**`Rc::clone(&shared)` 和 `shared.clone()` 效果一样，但社区习惯写前者**，因为它在提示读者「这只是加个计数，不是深拷贝」。

两条重要限制：

**1. 默认拿不到可变内容。** 因为可能有很多份引用同时存在，允许其中一份改内容就破坏了借用规则：

```rust
let shared = Rc::new(String::from("hello"));
let moved = *shared;
```

```text
error[E0507]: cannot move out of an `Rc`
 --> src/main.rs:5:17
  |
5 |     let moved = *shared;
  |                 ^^^^^^^ move occurs because value has type `String`, which does not implement the `Copy` trait
  |
help: consider cloning the value if the performance cost is acceptable
  |
5 |     let moved = shared.clone();
  |                        ++++++++
```

想改内容，只有当**引用计数为 1**（也就是没有别人共享）时才行：

```rust
let mut only_one = Rc::new(10);
if let Some(value) = Rc::get_mut(&mut only_one) {
    *value += 5;               // 此时才能拿到 &mut
}
```

实测改成 `15`。有多份引用时 `get_mut` 返回 `None`——它不会 panic，但你也改不了。

**2. `Rc` 不是线程安全的。** 多个线程同时改计数会导致数据竞争，所以 `Rc` 不能跨线程（见 12.10）。

## 12.7 `RefCell<T>`：内部可变性

只有一个所有者、但想在「不可变引用」里改数据时，用 `RefCell`：

```rust
use std::cell::RefCell;

let counter = RefCell::new(0);      // 注意：绑定不是 mut
*counter.borrow_mut() += 1;         // 却可以改
*counter.borrow_mut() += 1;
println!("{}", counter.borrow());   // 2
```

实测 `不可变绑定里的值被改了 = 2`。

**原理**：`RefCell` 把借用检查从编译期挪到了运行期。`borrow()` 返回一个 `Ref` 守卫（相当于 `&T`），`borrow_mut()` 返回 `RefMut` 守卫（相当于 `&mut T`）；守卫在作用域内时，`RefCell` 会记录「当前有几种借用」。规则还是第 5 章那三条，只是违规时**不是编译错误，而是运行时 panic**：

```rust
let cell = RefCell::new(5);
let first = cell.borrow_mut();
let second = cell.borrow();       // 运行时 panic
```

```text
thread 'main' panicked at src/main.rs:6:23:
RefCell already mutably borrowed
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
```

什么时候值得用 `RefCell`？典型场景是**方法签名只有 `&self`，却需要修改内部状态**：缓存（第一次算完存起来）、访问计数、观察者列表。用它的代价是「借用错误从编译期推迟到运行期」，所以：

- 能用 `&mut` 解决就别用 `RefCell`；
- 借用守卫（`Ref` / `RefMut`）要**尽快用完**，别跨长逻辑持有；
- 一旦 panic，程序就结束了，测试里要覆盖到这些路径。

`RefCell` 和 `Rc` 一样不是线程安全的；多线程里有对应的 `Mutex` / `RwLock`（第 13 章）。

## 12.8 `Rc<RefCell<T>>`：共享且可改

单独看，`Rc` 能共享不能改，`RefCell` 能改不能共享。组合起来就是「多个所有者 + 都能改」：

```rust
let shared_state = Rc::new(RefCell::new(vec![1]));
let handle = Rc::clone(&shared_state);
handle.borrow_mut().push(2);

println!("{:?}", shared_state.borrow());   // [1, 2]
```

实测：通过克隆出来的句柄修改，原来的引用也看得到 `[1, 2]`。

这个组合在「图结构」「共享配置」「回调注册表」里很常见。用的时候要小心两件事：**同时借用冲突会在运行时 panic**（比如一边 `borrow()` 遍历、一边 `borrow_mut()` 插入），以及**引用计数为 0 之前谁都不能释放它**——如果互相持有，就会形成循环。

## 12.9 `Weak<T>`：打破循环引用

`Rc` 有一个著名问题：**两个对象互相 `Rc` 对方，计数永远不会归零，内存就泄漏了**。

```rust
struct Node {
    children: Vec<Rc<Node>>,
    parent: Rc<Node>,        // 父持有子、子也持有父 —— 计数永远是 1，谁都释放不了
}
```

解决办法是让其中一边用 **`Weak<T>`**：它指向 `Rc` 管理的数据，但**不增加强引用计数**。

```rust
struct Node {
    name: &'static str,
    children: RefCell<Vec<Rc<Node>>>,
    parent: RefCell<Weak<Node>>,        // 父用弱引用
}

// 建立关系
*child.parent.borrow_mut() = Rc::downgrade(parent);   // Rc -> Weak
parent.children.borrow_mut().push(Rc::clone(child));
```

用的时候要 `upgrade()` 把 `Weak` 变回 `Option<Rc<T>>`——因为目标可能已经被释放了：

```rust
if let Some(parent) = child.parent.borrow().upgrade() {
    println!("从 child 找回 parent = {}", parent.name);
}
```

实测：

```text
    root  strong = 1 / weak = 1
    child strong = 2 / weak = 0
    从 child 找回 parent = root
```

读法：`root` 被 `child.parent` 里的 `Weak` 指了一次（weak = 1），但没有人强持有它额外的计数；`child` 被 `root.children` 里的 `Rc` 持有（strong = 2，另一份是局部变量 `child`）。

Rust 里的「内存泄漏」是**安全**的（不会像 C 那样产生未定义行为），但它仍然是 bug：内存不会归还给系统。**用 `Rc` 建图时，问一句「会不会形成环」，会的话就有一边该用 `Weak`。**

## 12.10 `Arc<T>`：跨线程共享

`Arc` 是 atomic 版的 `Rc`，计数用原子操作，因此可以安全地跨线程：

```rust
use std::sync::Arc;
use std::thread;

let data = Arc::new(vec![10, 20, 30]);
let mut handles = Vec::new();
for index in 0..3 {
    let shared_data = Arc::clone(&data);
    handles.push(thread::spawn(move || shared_data[index]));
}
let results: Vec<i32> = handles.into_iter().map(|h| h.join().unwrap()).collect();
```

实测三个线程分别取到 `[10, 20, 30]` 对应的元素。

注意结构：**`Arc::clone` 在 `move` 之前完成**，闭包拿走的是克隆出来的 `Arc`；主线程的 `data` 仍然有效，等所有线程结束后计数归零、数据释放。

把 `Arc` 换成 `Rc` 就会编译失败：

```text
error[E0277]: `Rc<String>` cannot be sent between threads safely
   --> src/main.rs:6:32
    |
6 |     let handle = thread::spawn(move || shared.len());
    |                  ------------- -------^^^^^^^^^^^^^
    |                  |             |
    |                  |             `Rc<String>` cannot be sent between threads safely
    |                  |             within this `{closure@src/main.rs:6:32: 6:39}`
    |
    = help: within `{closure@src/main.rs:6:32: 6:39}`, the trait `Send` is not implemented for `Rc<String>`
note: required by a bound in `spawn`
    |
128 |     F: Send + 'static,
    |        ^^^^ required by this bound in `spawn`
```

报错里那个 `F: Send + 'static` 就是 `thread::spawn` 的约束（第 9 章讲过怎么读 trait bound）：闭包必须能安全地送到别的线程，而且不能借用局部数据。这正是第 13 章的主题。

`Arc` 只解决「多个线程共享**只读**数据」。要在线程间**修改**，还需要 `Mutex` / `RwLock`，最常见的组合是 `Arc<Mutex<T>>`。

## 12.11 怎么选

这张表可以当决策树用：

| 需求 | 选择 |
| --- | --- |
| 递归类型、大值、trait 对象 | `Box<T>` |
| 单一所有者 + 内部可变 | `RefCell<T>`（单线程） |
| 多个所有者、只读共享 | `Rc<T>`（单线程）/ `Arc<T>`（多线程） |
| 多个所有者 + 可改 | `Rc<RefCell<T>>`（单线程）/ `Arc<Mutex<T>>`（多线程） |
| 图/树的双向引用 | 一边用 `Rc`，另一边用 `Weak` |
| 只是借用，不需要所有权 | 用 `&T` / `&mut T`，别上智能指针 |

最后一行最重要：**智能指针是最后手段，不是第一选择**。Rust 的默认姿势是借用；只有当「所有权必须共享」或「必须绕过借用检查在运行时改」时，才掏出这一章的工具。滥用 `Rc<RefCell<T>>` 会把编译期的借用检查换成运行期的 panic，还容易写出循环引用。

## 12.12 五个报错与 panic 速查

这一章的报错都在前面出现过，这里汇总成一张表：

| 错误 | 触发场景 | 修法 |
| --- | --- | --- |
| `E0072: recursive type has infinite size` | 递归类型缺少间接层 | 把递归字段改成 `Box<List>` / `Rc<List>` |
| `E0507: cannot move out of an Rc` | 想把 `Rc` 里的值移动出来 | 借用（`&*rc`）或 `.clone()` |
| `E0509: cannot move out of type which implements Drop` | 从实现 `Drop` 的类型里移动字段 | 借用或克隆该字段 |
| `RefCell already mutably borrowed`（运行时 panic） | `borrow()` 与 `borrow_mut()` 守卫同时存在 | 缩短守卫的生命周期，用完立刻 `drop` |
| `E0277: Rc<T> cannot be sent between threads safely` | 把 `Rc` 送进 `thread::spawn` | 换成 `Arc<T>` |

两个规律：**涉及「间接层」的问题（递归、trait 对象）报 `E0072`**；**涉及「共享所有权」的问题要么是 `E0507`/`E0509`（想拿走不属于你的值），要么是运行期的借用冲突**。最后一条最值得警惕——它不在编译期报错，而是等你跑到那一行才崩。

## 12.13 常见坑速查

| 现象 | 原因 | 解法 |
| --- | --- | --- |
| 递归类型报「无限大小」 | 类型里直接嵌套自己 | 加 `Box` / `Rc` |
| `Rc::clone` 以为是深拷贝 | 名字叫 clone，其实只加计数 | 想复制数据用 `(*rc).clone()` |
| 想改 `Rc` 里的值改不了 | 共享状态下不允许可变借用 | 唯一所有者时用 `Rc::get_mut`，否则用 `Rc<RefCell<T>>` |
| 程序偶尔 panic「already mutably borrowed」 | `RefCell` 运行时借用冲突 | 缩短 `borrow()` / `borrow_mut()` 守卫的生命 |
| 内存一直不释放 | `Rc` 之间形成了环 | 一边改成 `Weak`，用 `upgrade()` 取用 |
| `Weak` 用之前忘了 `upgrade` | `Weak` 不能直接访问数据 | `upgrade()` 返回 `Option<Rc<T>>` |
| `Rc` 放进线程报 `Send` 不满足 | `Rc` 的计数不是原子的 | 换 `Arc` |
| 多线程里改共享数据编译不过 | `Arc` 只管共享不管同步 | 用 `Arc<Mutex<T>>`（第 13 章） |
| 给新类型实现了 `Deref` 之后类型失去保护 | 自动退化成内部类型 | 只在确实要「当包装用」时实现 `Deref` |
| 到处 `Rc<RefCell<T>>`，代码越来越乱 | 用智能指针代替了设计 | 先问「是不是该用借用/拥有型数据」 |

## 12.14 练习

1. 用 `Box` 实现递归二叉树：`enum Tree { Leaf(i32), Node(Box<Tree>, Box<Tree>) }`，写 `fn sum(&self) -> i32`。
2. 定义 `struct Stack<T> { items: Vec<T> }`，实现 `Deref<Target = [T]>`，让它能直接用 `len()` / `first()` / 切片语法；再说明为什么给 `struct Meters(f64)` 实现 `Deref<Target = f64>` 反而是坏主意。
3. 用 `Rc` 共享一个 `Vec`，克隆出两份，打印 `strong_count`；丢掉其中一份后再打印一次。
4. 用 `Rc<RefCell<Vec<String>>>` 建一个共享列表，让两个「句柄」分别往里加字符串，最后打印内容。
5. 用 `Rc` + `Weak` 搭一个 parent / child 结构，验证：child 能 `upgrade()` 出 parent，且 parent 的 `weak_count` 是 1。

（第 3 题是 12.6 示例的直接变体，这里不再单独给答案。）

### 参考答案

::: details 第 1 题

```rust
#[derive(Debug)]
enum Tree {
    Leaf(i32),
    Node(Box<Tree>, Box<Tree>),
}

impl Tree {
    fn sum(&self) -> i32 {
        match self {
            Tree::Leaf(value) => *value,
            Tree::Node(left, right) => left.sum() + right.sum(),
        }
    }
}

fn main() {
    let tree = Tree::Node(
        Box::new(Tree::Node(Box::new(Tree::Leaf(1)), Box::new(Tree::Leaf(2)))),
        Box::new(Tree::Leaf(3)),
    );
    println!("{}", tree.sum());   // 6
}
```

`Node` 的两个字段都是 `Box<Tree>`：如果写成 `Tree`，又是 `E0072` 的无限大小。和链表的区别在于二叉树有**两个**递归分支，所以两边都要装箱。

:::

::: details 第 2 题

```rust
use std::ops::Deref;

struct Stack<T> {
    items: Vec<T>,
}

impl<T> Deref for Stack<T> {
    type Target = [T];

    fn deref(&self) -> &[T] {
        &self.items
    }
}

fn main() {
    let stack = Stack { items: vec![1, 2, 3] };
    println!("len = {}", stack.len());            // 3
    println!("first = {:?}", stack.first());      // Some(1)
    println!("slice = {:?}", &stack[1..]);        // [2, 3]
    println!("重复元素? {}", stack.contains(&2));  // true
}
```

`Stack` 的语义就是「一叠东西」，让它表现得像切片是合理的——这也正是标准库给 `Vec<T>` 实现 `Deref<Target = [T]>` 的理由。

而 `struct Meters(f64)` 的新类型只有一个目的：**让「米」和「秒」不能相加**。实现 `Deref<Target = f64>` 之后，`meters + 1.0` 会自动解引用成一个裸 `f64` 加法，类型保护就失效了。**新类型的价值在于不透明，`Deref` 的价值在于透明——两者方向相反。**

:::

::: details 第 4 题

```rust
use std::cell::RefCell;
use std::rc::Rc;

fn main() {
    let shared: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));
    let handle_a = Rc::clone(&shared);
    let handle_b = Rc::clone(&shared);

    handle_a.borrow_mut().push(String::from("来自 A"));
    handle_b.borrow_mut().push(String::from("来自 B"));

    println!("{:?}", shared.borrow());                    // ["来自 A", "来自 B"]
    println!("strong_count = {}", Rc::strong_count(&shared)); // 3
}
```

实测输出 `["来自 A", "来自 B"]` 和 `strong_count = 3`。注意每次 `borrow_mut()` 的守卫都在**同一行语句结束时**就被丢弃了，所以两次修改不会冲突——这正是 12.7 说的「守卫要尽快释放」。

:::

::: details 第 5 题

```rust
use std::cell::RefCell;
use std::rc::{Rc, Weak};

struct Node {
    name: &'static str,
    parent: RefCell<Weak<Node>>,
}

fn main() {
    let parent = Rc::new(Node { name: "parent", parent: RefCell::new(Weak::new()) });
    let child = Rc::new(Node {
        name: "child",
        parent: RefCell::new(Rc::downgrade(&parent)),
    });

    println!(
        "parent strong = {}, weak = {}",
        Rc::strong_count(&parent),
        Rc::weak_count(&parent)
    );
    if let Some(found) = child.parent.borrow().upgrade() {
        println!("child 的父节点 = {}", found.name);
    }
}
```

实测 `parent strong = 1, weak = 1` 和 `child 的父节点 = parent`。`parent` 的强引用只有局部变量这一份（strong = 1），`child.parent` 里那个 `Weak` 只贡献了 weak 计数——所以即使 child 一直被持有，parent 该释放时仍会释放，不会形成环。

:::

## 12.15 小结

- 智能指针在「指针」之上加了所有权管理：`Box` 独占、`Rc` / `Arc` 共享、`RefCell` 运行时借用、`Weak` 弱引用。

- `Box<T>` 是最简单的一个：堆上的唯一所有者，用于递归类型、转移大值和装 trait 对象。

- 递归类型必须有一层间接（`Box` / `Rc` / `&`），否则报 `E0072`。

- `Deref` 让自定义类型能像引用一样用，也是 deref coercion（`&String` → `&str`、`&Vec<T>` → `&[T]`）的原理；但新类型**不该**实现 `Deref`，那会毁掉它的类型保护。

- 实现了 `Drop` 的类型不能把字段移动出去（`E0509`）；`Copy` 和 `Drop` 不能共存（否则双重释放）。

- `Rc<T>` 让值有多个所有者，`Rc::clone` 只增加计数；内容默认不可变，只有 `strong_count == 1` 时能用 `Rc::get_mut` 拿到 `&mut`。

- `RefCell<T>` 提供内部可变性，把借用检查推迟到运行期：违规时 panic（`already mutably borrowed`），所以守卫要尽快释放。

- `Rc<RefCell<T>>` = 共享 + 可改，代价是运行期风险和潜在的循环引用。

- 双向引用必须有一边用 `Weak`，用 `Rc::downgrade` 创建、`upgrade()` 取用，否则计数永不归零、内存泄漏。

- `Arc<T>` 是线程安全的 `Rc`，跨线程共享只读数据用它；要跨线程修改就得上 `Arc<Mutex<T>>`（第 13 章）。

- 智能指针是最后手段：先考虑借用和拥有型数据，只有在「必须共享所有权」或「必须在运行期绕过借用检查」时才用它们。

下一章讲**并发编程**：`thread::spawn`、`mpsc` 通道、`Mutex` 与 `Arc`、`Send` 与 `Sync`——这一章埋下的 `Send` 约束会在那里彻底讲清楚。
