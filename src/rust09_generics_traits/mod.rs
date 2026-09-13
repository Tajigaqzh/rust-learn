//! 第 9 章配套代码：泛型与 trait。
//!
//! 运行方式：`cargo run`，输出接在第 8 章后面。

use std::fmt;
use std::ops::Add;

/// 泛型函数：只要求 `T` 能比大小、能按值取走。
fn largest<T: PartialOrd + Copy>(list: &[T]) -> T {
    let mut largest = list[0];
    for &item in list {
        if item > largest {
            largest = item;
        }
    }
    largest
}

/// 泛型结构体：坐标的数值类型由使用者决定。
#[derive(Debug, Clone, Copy, PartialEq)]
struct Point<T> {
    x: T,
    y: T,
}

impl<T: fmt::Display> Point<T> {
    /// 泛型方法：约束写在 `impl` 上，整个块里的方法共享。
    fn show(&self) -> String {
        format!("({}, {})", self.x, self.y)
    }
}

/// 只给 `Point<f64>` 实现的方法：别的 `Point<T>` 没有。
impl Point<f64> {
    fn mixup(&self, other: Point<f64>) -> Point<f64> {
        Point {
            x: self.x,
            y: other.y,
        }
    }
}

/// 运算符重载：给 `Point` 实现 `+`。
impl<T: Add<Output = T>> Add for Point<T> {
    type Output = Point<T>;

    fn add(self, other: Point<T>) -> Point<T> {
        Point {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }
}

/// trait：描述「能做什么」，不关心「是什么」。
trait Summary {
    fn summarize(&self) -> String;

    /// 默认方法：实现者只管 `summarize` 就行。
    fn preview(&self) -> String {
        format!("[摘要] {}", self.summarize())
    }
}

/// 新闻：实现 `Summary`，用默认的 `preview`。
struct NewsArticle {
    title: String,
    author: String,
}

impl Summary for NewsArticle {
    fn summarize(&self) -> String {
        format!("《{}》，作者 {}", self.title, self.author)
    }
}

/// 推文：实现 `Summary`，并覆盖默认的 `preview`。
struct Tweet {
    user: String,
    content: String,
}

impl Summary for Tweet {
    fn summarize(&self) -> String {
        format!("@{}: {}", self.user, self.content)
    }

    fn preview(&self) -> String {
        let text: String = self.content.chars().take(12).collect();
        format!("[推文] {text}...")
    }
}

/// 参数写 `impl Trait`：短，适合只有一个泛型参数时。
fn notify(item: &impl Summary) -> String {
    format!("通知：{}", item.preview())
}

/// 等价的泛型写法，方便写多个约束。
fn notify_all<T: Summary>(items: &[T]) -> Vec<String> {
    let mut notices = Vec::new();
    for item in items {
        notices.push(item.preview());
    }
    notices
}

/// 约束多的时候用 `where` 子句，签名更好读。
fn describe_pair<A, B>(a: &A, b: &B) -> String
where
    A: Summary,
    B: fmt::Display,
{
    format!("{} / {}", a.preview(), b)
}

/// 返回 `impl Trait`：调用方只知道「它实现了 Summary」。
fn make_article() -> impl Summary {
    NewsArticle {
        title: String::from("Rust 所有权"),
        author: String::from("Ada"),
    }
}

/// 关联类型：trait 里先留一个类型占位，实现时才确定。
trait Container {
    type Item;

    fn first(&self) -> Option<&Self::Item>;
    fn len(&self) -> usize;
}

impl<T> Container for Vec<T> {
    type Item = T;

    fn first(&self) -> Option<&T> {
        self.as_slice().first()
    }

    fn len(&self) -> usize {
        Vec::len(self)
    }
}

/// 一组 trait 对象：可以装不同类型的值。
fn print_all(items: &[Box<dyn Summary>]) {
    for item in items {
        println!("    {}", item.summarize());
    }
}

/// 演示泛型函数、泛型结构体、trait、trait 对象与关联类型。
pub fn generics_traits_demo() {
    println!("\n========== rust09_generics_traits: 泛型与 trait ==========");

    // 1. 泛型函数
    println!("\n--- 1. 泛型函数 ---");
    println!("    largest(&[3, 9, 4]) = {}", largest(&[3, 9, 4]));
    println!("    largest(&['a', 'z', 'm']) = {}", largest(&['a', 'z', 'm']));
    println!(
        "    largest(&[\"apple\", \"pear\", \"fig\"]) = {}",
        largest(&["apple", "pear", "fig"])
    );

    // 2. 泛型结构体与泛型方法
    println!("\n--- 2. 泛型结构体与泛型方法 ---");
    let int_point = Point { x: 3, y: 4 };
    let float_point = Point { x: 1.5, y: 2.5 };
    println!("    int_point = {}", int_point.show());
    println!("    float_point = {}", float_point.show());

    // 3. 只为特定类型实现的方法
    println!("\n--- 3. 只为某种类型实现的方法 ---");
    let mixed = float_point.mixup(Point { x: 9.0, y: 9.0 });
    println!("    mixup 之后 = {}", mixed.show());
    println!("    （int_point 没有 mixup：那个 impl 块只写给 Point<f64>）");

    // 4. 运算符重载
    println!("\n--- 4. 运算符重载 ---");
    let sum = int_point + Point { x: 10, y: 20 };
    println!("    int_point + Point {{ 10, 20 }} = {}", sum.show());

    // 5. trait：实现与默认方法
    println!("\n--- 5. trait：定义行为 ---");
    let article = NewsArticle {
        title: String::from("Rust 所有权"),
        author: String::from("Ada"),
    };
    let tweet = Tweet {
        user: String::from("rustlang"),
        content: String::from("所有权规则很好用，借用检查器帮我抓了个 bug"),
    };
    println!("    article.summarize() = {}", article.summarize());
    println!("    tweet.summarize()   = {}", tweet.summarize());
    println!("    article.preview()   = {}", article.preview());
    println!("    tweet.preview()     = {}", tweet.preview());

    // 6. 三种约束写法
    println!("\n--- 6. 三种约束写法 ---");
    println!("    notify(&article) = {}", notify(&article));
    let articles = vec![
        NewsArticle {
            title: String::from("所有权"),
            author: String::from("Ada"),
        },
        NewsArticle {
            title: String::from("借用检查器"),
            author: String::from("Grace"),
        },
    ];
    for notice in notify_all(&articles) {
        println!("    notify_all -> {notice}");
    }
    println!(
        "    describe_pair(&article, &42) = {}",
        describe_pair(&article, &42)
    );

    // 7. 返回 impl Trait
    println!("\n--- 7. 返回 impl Trait ---");
    let made = make_article();
    println!("    make_article().preview() = {}", made.preview());

    // 8. trait 对象：混装不同类型
    println!("\n--- 8. trait 对象：一个容器装多种类型 ---");
    let items: Vec<Box<dyn Summary>> = vec![
        Box::new(NewsArticle {
            title: String::from("泛型"),
            author: String::from("Linus"),
        }),
        Box::new(Tweet {
            user: String::from("ada"),
            content: String::from("trait 对象可以混装"),
        }),
    ];
    print_all(&items);

    // 9. 关联类型
    println!("\n--- 9. 关联类型 ---");
    let numbers = vec![10, 20, 30];
    println!("    Container::first(&numbers) = {:?}", Container::first(&numbers));
    println!("    Container::len(&numbers) = {}", Container::len(&numbers));
    let empty: Vec<i32> = Vec::new();
    println!("    空 Vec 的 first = {:?}", Container::first(&empty));

    // 10. 两种分发方式
    println!("\n--- 10. 泛型 vs trait 对象 ---");
    println!("    泛型是静态分发：编译期为每种具体类型生成一份代码，调用无额外开销，体积更大");
    println!("    dyn 是动态分发：运行时查虚表，能装不同类型，多一次间接跳转");
    println!("    选择：类型编译期就确定用泛型；需要「一个容器装多种类型」才用 dyn");

    println!("\n========== 泛型与 trait 演示结束 ==========");
}
