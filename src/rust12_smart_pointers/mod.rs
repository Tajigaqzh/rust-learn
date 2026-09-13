//! 第 12 章配套代码：智能指针。
//!
//! 运行方式：`cargo run`，输出接在第 11 章后面。

use std::cell::RefCell;
use std::ops::Deref;
use std::rc::{Rc, Weak};
use std::sync::Arc;
use std::thread;

/// 递归类型：`Box` 提供「大小确定的间接层」。
#[derive(Debug)]
enum List {
    Cons(i32, Box<List>),
    Nil,
}

impl List {
    fn new() -> Self {
        List::Nil
    }

    /// 头插：`self` 被移动进新的节点里，返回新的表头。
    fn push(self, value: i32) -> Self {
        List::Cons(value, Box::new(self))
    }

    fn sum(&self) -> i32 {
        match self {
            List::Cons(value, next) => value + next.sum(),
            List::Nil => 0,
        }
    }

    fn to_vec(&self) -> Vec<i32> {
        match self {
            List::Cons(value, next) => {
                let mut rest = next.to_vec();
                rest.insert(0, *value);
                rest
            }
            List::Nil => Vec::new(),
        }
    }
}

/// 自定义智能指针：实现 `Deref` 之后就能像引用一样用。
struct Wrapper<T> {
    value: T,
}

impl<T> Deref for Wrapper<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.value
    }
}

/// 用来观察 `Drop` 的调用时机。
struct Tracked(&'static str);

impl Drop for Tracked {
    fn drop(&mut self) {
        println!("    释放 {}", self.0);
    }
}

/// 树节点：子节点用 `Rc`，父节点用 `Weak`，避免互相持有造成循环引用。
struct Node {
    name: &'static str,
    children: RefCell<Vec<Rc<Node>>>,
    parent: RefCell<Weak<Node>>,
}

impl Node {
    fn new(name: &'static str) -> Rc<Node> {
        Rc::new(Node {
            name,
            children: RefCell::new(Vec::new()),
            parent: RefCell::new(Weak::new()),
        })
    }

    fn add_child(parent: &Rc<Node>, child: &Rc<Node>) {
        *child.parent.borrow_mut() = Rc::downgrade(parent);
        parent.children.borrow_mut().push(Rc::clone(child));
    }
}

/// 演示 `Box`、`Deref`、`Drop`、`Rc`、`RefCell`、`Weak` 与 `Arc`。
pub fn smart_pointers_demo() {
    println!("\n========== rust12_smart_pointers: 智能指针 ==========");

    // 1. Box
    println!("\n--- 1. Box：把值放到堆上 ---");
    let boxed = Box::new(5);
    println!("    Box::new(5) = {boxed}，解引用 *boxed = {}", *boxed);
    let list = List::new().push(3).push(2).push(1);
    println!("    递归 List = {list:?}");
    println!("    sum = {}，展开 = {:?}", list.sum(), list.to_vec());

    // 2. Deref
    println!("\n--- 2. Deref：让自定义类型像引用一样用 ---");
    let wrapper = Wrapper {
        value: String::from("hello"),
    };
    println!(
        "    wrapper.len() = {}（自动解引用到 String）",
        wrapper.len()
    );
    println!("    wrapper.to_uppercase() = {}", wrapper.to_uppercase());
    let boxed_string = Box::new(String::from("boxed"));
    let as_str: &str = &boxed_string; // deref coercion：&Box<String> -> &str
    println!("    &Box<String> 当 &str 用 = {as_str}");

    // 3. Drop
    println!("\n--- 3. Drop：离开作用域自动释放 ---");
    {
        let _first = Tracked("first");
        let _second = Tracked("second");
        println!("    块结束前");
    }
    println!("    块已经结束");

    // 4. Rc
    println!("\n--- 4. Rc：共享所有权 ---");
    let shared = Rc::new(vec![1, 2, 3]);
    println!("    创建后 strong_count = {}", Rc::strong_count(&shared));
    {
        let clone_a = Rc::clone(&shared);
        let clone_b = Rc::clone(&shared);
        println!(
            "    克隆两次之后 strong_count = {}",
            Rc::strong_count(&shared)
        );
        println!("    三份指向同一块数据 = {clone_a:?} / {clone_b:?}");
    }
    println!(
        "    离开块之后 strong_count = {}",
        Rc::strong_count(&shared)
    );

    // 5. Rc 的内容默认不可变
    println!("\n--- 5. Rc 的内容默认改不了 ---");
    let mut only_one = Rc::new(10);
    if let Some(value) = Rc::get_mut(&mut only_one) {
        *value += 5; // 只有一份引用时才拿得到可变引用
    }
    println!("    只有一份引用时可以改：{only_one}");

    // 6. RefCell
    println!("\n--- 6. RefCell：内部可变性 ---");
    let counter = RefCell::new(0);
    *counter.borrow_mut() += 1;
    *counter.borrow_mut() += 1;
    println!("    不可变绑定里的值被改了 = {}", counter.borrow());
    println!("    （借用检查从编译期挪到运行期：冲突时 panic）");

    // 7. Rc<RefCell<T>>
    println!("\n--- 7. Rc<RefCell<T>>：共享且可改 ---");
    let shared_state = Rc::new(RefCell::new(vec![1]));
    let handle = Rc::clone(&shared_state);
    handle.borrow_mut().push(2);
    println!(
        "    通过克隆出来的句柄修改，原引用也看得到 = {:?}",
        shared_state.borrow()
    );

    // 8. Weak
    println!("\n--- 8. Weak：打破循环引用 ---");
    let root = Node::new("root");
    let child = Node::new("child");
    Node::add_child(&root, &child);
    println!(
        "    root  strong = {} / weak = {}",
        Rc::strong_count(&root),
        Rc::weak_count(&root)
    );
    println!(
        "    child strong = {} / weak = {}",
        Rc::strong_count(&child),
        Rc::weak_count(&child)
    );
    if let Some(parent) = child.parent.borrow().upgrade() {
        println!("    从 child 找回 parent = {}", parent.name);
    }
    println!("    （parent 存的是 Weak，不会形成互相持有的环）");

    // 9. Arc
    println!("\n--- 9. Arc：跨线程共享 ---");
    let data = Arc::new(vec![10, 20, 30]);
    let mut handles = Vec::new();
    for index in 0..3 {
        let shared_data = Arc::clone(&data);
        handles.push(thread::spawn(move || shared_data[index]));
    }
    let results: Vec<i32> = handles
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .collect();
    println!("    三个线程各取一个 = {results:?}");
    println!("    （线程之间只能共享 Arc，Rc 不是线程安全的）");

    println!("\n========== 智能指针演示结束 ==========");
}
