//! 第 29 章配套代码：类型系统进阶。
//!
//! 覆盖 const 泛型、GAT（泛型关联类型）、trait 中的 `impl Trait`
//! 和高阶生命周期 `for<'a>` 四个主题。
//!
//! 运行方式：`cargo run`；只跑本章的测试：`cargo test rust29_typesystem`。

use std::fmt;

// ---------------------------------------------------------------------------
// 29.2 const 泛型：让数组长度参与类型
// ---------------------------------------------------------------------------

/// 用 const 泛型参数把「行数、列数」编码进类型。
///
/// `Matrix<2, 3>` 和 `Matrix<3, 2>` 是**两个不同的类型**，
/// 维度错误在编译期就能暴露，不用等到运行时 panic。
#[derive(Debug, PartialEq)]
struct Matrix<const R: usize, const C: usize> {
    data: [[f64; C]; R],
}

impl<const R: usize, const C: usize> Matrix<R, C> {
    /// 全零矩阵。`[[0.0; C]; R]` 之所以能写，正是因为 C 和 R 是编译期常量。
    fn zeros() -> Self {
        Self {
            data: [[0.0; C]; R],
        }
    }

    /// 用闭包按坐标生成元素，类似 `std::array::from_fn` 的二维版。
    fn from_coords(f: impl Fn(usize, usize) -> f64) -> Self {
        let mut data = [[0.0; C]; R];
        for (r, row) in data.iter_mut().enumerate() {
            for (c, cell) in row.iter_mut().enumerate() {
                *cell = f(r, c);
            }
        }
        Self { data }
    }
}

/// 矩阵乘法：`(R×K) * (K×C) -> R×C`。
///
/// 注意泛型参数表怎么读：`R`、`K`、`C` 都是 const 参数（usize 值），
/// `K` 同时出现在两个操作数里——类型系统因此天然要求「左列数 = 右行数」，
/// 形状不匹配直接编译不过。这就是 const 泛型带来的编译期形状检查。
fn mat_mul<const R: usize, const K: usize, const C: usize>(
    a: &Matrix<R, K>,
    b: &Matrix<K, C>,
) -> Matrix<R, C> {
    let mut out = Matrix::<R, C>::zeros();
    for r in 0..R {
        for c in 0..C {
            let mut sum = 0.0;
            for k in 0..K {
                sum += a.data[r][k] * b.data[k][c];
            }
            out.data[r][c] = sum;
        }
    }
    out
}

/// const 泛型最常见的用途：给「任意长度的数组」写一个实现，而不是给
/// `[T; 0]`、`[T; 1]`、`[T; 2]`……各写一遍。
///
/// 标准库自己就是这么干的——`impl<T, const N: usize> IntoIterator
/// for [T; N]` 一行覆盖所有数组长度。
fn array_len_is_part_of_type() {
    // `[i32; 3]` 和 `[i32; 4]` 是不同类型，但下面这个泛型函数对两者都适用。
    fn sum<const N: usize>(arr: [i32; N]) -> i32 {
        arr.iter().sum()
    }
    let a: [i32; 3] = [1, 2, 3];
    let b: [i32; 4] = [1, 2, 3, 4];
    debug_assert_eq!(sum(a), 6);
    debug_assert_eq!(sum(b), 10);
}

// ---------------------------------------------------------------------------
// 29.3 const 泛型的限制
// ---------------------------------------------------------------------------

/// const 参数能做的事很少：只能参与类型、参与常量表达式，
/// **不能**做任意运算或基于它做条件分支。
///
/// 例如「只在 N > 0 时提供 get 方法」就做不到——
/// `if N > 0` 不是常量求值上下文里允许的 impl 条件（还没有
/// 特化 / const generics 全面版）。这正是当前 const 泛型的边界。
const fn const_param_limits() {
    // 在常量上下文里可以对 const 参数做算术，生成新的常量：
    const DOUBLE: usize = 3 * 2;
    let _arr: [u8; DOUBLE] = [0; DOUBLE];
    // 但没法写「impl<const N: usize> Foo where N > 0」这样的条件实现。
}

// ---------------------------------------------------------------------------
// 29.4 GAT：带泛型参数的关联类型
// ---------------------------------------------------------------------------

/// 借用型迭代器：每次 `next` 借出一个元素，而不是把元素搬出来。
///
/// 标准 `Iterator::Item` 是不带生命周期的关联类型，所以 `Iterator`
/// 必须**拥有**每个产出的值。想迭代一个自己没有所有权的东西
/// （比如迭代器内部复用缓冲区），标准 `Iterator` 就不够用了——
/// 这正是 GAT 存在的理由：`type Item<'a>` 让关联类型随调用处的
/// 生命周期变化。
trait LendingIterator {
    type Item<'a>
    where
        Self: 'a;

    fn next(&mut self) -> Option<Self::Item<'_>>;
}

/// 一个诚实的用例：迭代器内部**复用同一个缓冲区**，
/// 每次产出 `&'a str` 都借自它自己。标准 Iterator 做不到这件事
/// （产出引用会要求迭代器本身活得比借用久， borrow checker 拒绝）。
struct WordSplitter<'s> {
    rest: &'s str,
}

impl<'s> WordSplitter<'s> {
    fn new(s: &'s str) -> Self {
        Self { rest: s }
    }
}

impl LendingIterator for WordSplitter<'_> {
    type Item<'a>
        = &'a str
    where
        Self: 'a;

    fn next(&mut self) -> Option<&'_ str> {
        let rest = self.rest.trim_start();
        if rest.is_empty() {
            return None;
        }
        let end = rest.find(char::is_whitespace).unwrap_or(rest.len());
        self.rest = &rest[end..];
        Some(&rest[..end])
    }
}

// ---------------------------------------------------------------------------
// 29.5 trait 中的 impl Trait（RPITIT）与 use<> 捕获
// ---------------------------------------------------------------------------

/// Rust 1.75 起，trait 方法可以直接返回 `impl Trait`，
/// 不再需要为每个返回类型定义关联类型。
///
/// 对比第 9.10 章的写法：那时候 trait 只能用关联类型 `type Output`
/// 表达「返回类型由实现者决定」；RPITIT 把决定权反过来——
/// 由**方法签名**决定，实现者返回任意满足约束的具体类型。
trait Style {
    fn render(&self) -> impl fmt::Display;
}

struct Plain(String);
struct Shouty(String);

impl Style for Plain {
    fn render(&self) -> impl fmt::Display {
        // 实现者可以返回任何 Display 类型，不必和别的实现相同。
        &self.0
    }
}

impl Style for Shouty {
    fn render(&self) -> impl fmt::Display {
        // 这个实现返回的是「新构造的 String」，和上面那个不同型。
        format!("{}!!!", self.0)
    }
}

// ---------------------------------------------------------------------------
// 29.6 高阶生命周期 for<'a>
// ---------------------------------------------------------------------------

/// 「接受任意生命周期的引用」的函数指针类型。
///
/// 普通泛型 `fn takes<F, 'a>(f: F)` 里的 `'a` 是**调用者选定**的：
/// 每个具体生命周期实例化出一个不同的 F 类型。
/// 而 `for<'a>` 说的是「对**所有**生命周期 'a 都成立」——
/// 一个类型，塞进同一个槽位，覆盖所有调用。
///
/// 你其实早就见过它：`fn(&str) -> usize` 作为参数时，编译器推导出的
/// 就是 `for<'a> fn(&'a str) -> usize`。真正需要手写 `for<'a>` 的场景，
/// 是闭包和 trait 对象。
fn apply_to_any_lifetime(f: fn(&str) -> &str, input: &str) -> &str {
    // f 的类型是 for<'a> fn(&'a str) -> &'a str：
    // 不管 input 的生命周期多短，f 都能接受。
    f(input)
}

/// first_word 的签名天然满足 HRTB：引用进、引用出，且两个生命周期绑定。
fn first_word(s: &str) -> &str {
    s.split_whitespace().next().unwrap_or("")
}

/// 演示用：把满足 HRTB 的闭包装箱成 trait 对象。
///
/// `Box<dyn for<'a> Fn(&'a str) -> &'a str>` 读作：
/// 「装箱的闭包，对任意生命周期 'a，接受 &'a str 返回 &'a str」。
/// 省略 for<'a> 直接写 `Box<dyn Fn(&str) -> &str>` 时编译器
/// 会自动加上高阶 bound——所以日常少写，但报错信息里全是它。
fn boxed_hrtb_closure() -> Box<dyn for<'a> Fn(&'a str) -> &'a str> {
    // 闭包什么都没捕获（前缀是字面量），所以对任意生命周期都安全。
    Box::new(|s: &str| s.strip_prefix(">>>").unwrap_or(s).trim())
}

// ---------------------------------------------------------------------------
// 演示入口
// ---------------------------------------------------------------------------

/// 第 29 章演示：const 泛型 → 限制 → GAT → RPITIT → for<'a>。
pub fn typesystem_demo() {
    println!("\n========== rust29_typesystem: 类型系统进阶 ==========");

    println!("\n--- 1. const 泛型：维度进类型，错误进编译期 ---");
    let a = Matrix::<2, 3>::from_coords(|r, c| (r * 3 + c) as f64);
    let b = Matrix::<3, 2>::from_coords(|r, c| (r * 2 + c + 1) as f64);
    let c = mat_mul(&a, &b);
    println!("Matrix<2,3> * Matrix<3,2> = Matrix<2,2>:");
    for row in c.data {
        println!("  {row:?}");
    }
    // 下面这行是编译错误（Matrix<2,3> 和 Matrix<2,2> 形状不匹配），
    // 保留成注释供读者取消注释体验：
    // let bad = mat_mul(&a, &a); // expected `&Matrix<3, _>`, found `&Matrix<2, 3>`
    array_len_is_part_of_type();
    println!("数组长度也是类型的一部分：sum([i32;3]) 和 sum([i32;4]) 用同一个 impl。");

    println!("\n--- 2. const 泛型的限制 ---");
    const_param_limits();
    println!("const 参数可以在常量上下文参与运算（[u8; 3*2]），");
    println!("但不能做条件 impl（`where N > 0` 还不支持）。");

    println!("\n--- 3. GAT：让迭代器借出元素而不是搬出元素 ---");
    let mut splitter = WordSplitter::new("  hello   world  rust ");
    let mut count = 0;
    while let Some(word) = splitter.next() {
        // 每个词借自 splitter 自身：只能当场用，不能攒起来。
        print!("{word:?} ");
        count += 1;
    }
    println!("（共 {count} 个词）");
    // 收集为什么编译不过：每个 word 都借自 splitter，Vec 同时持有
    // 多个借用 = 多次活跃的 &mut splitter。这正是 lending iterator
    // 与标准 Iterator 的本质差别——借出的东西攒不到一起。
    //
    // let mut words = Vec::new();
    // while let Some(word) = splitter.next() { words.push(word); }
    //      ^ error[E0499]: cannot borrow `splitter` as mutable more than once
    println!("  借出的元素只能当场用，收集会报 E0499（注释里有现场）。");
    println!("  标准 Iterator 的 Item 没有生命周期参数，必须产出拥有的值；");
    println!("  `type Item<'a>` 解除了这个限制。");

    println!("\n--- 4. trait 中的 impl Trait（RPITIT）---");
    let plain = Plain("hello".to_string());
    let shouty = Shouty("hello".to_string());
    println!("Plain::render()  = {}", plain.render());
    println!("Shouty::render() = {}", shouty.render());
    println!("两个实现返回的是不同的具体类型，但签名只承诺 Display。");

    println!("\n--- 5. for<'a>：对任意生命周期都成立的函数/闭包 ---");
    let input = ">>> tokio";
    let trimmed = boxed_hrtb_closure();
    println!(
        "apply_to_any_lifetime(first_word, \"  async rust\") = {:?}",
        apply_to_any_lifetime(first_word, "  async rust")
    );
    println!(
        "Box<dyn for<'a> Fn(&'a str) -> &'a str>(\"{input}\") = {:?}",
        trimmed(input)
    );
    println!("普通泛型的 'a 由调用者选定；for<'a> 是「对所有 'a 成立」，一个类型覆盖全部调用。");

    println!("\n========== rust29_typesystem 演示结束 ==========");
}

// ---------------------------------------------------------------------------
// 测试
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matrix_dimensions_are_checked_at_compile_time() {
        let a = Matrix::<2, 3>::from_coords(|r, c| (r + c) as f64);
        let b = Matrix::<3, 1>::from_coords(|r, _| (r + 1) as f64);
        let out = mat_mul(&a, &b);
        // 手算验证一处即可：out[0][0] = 0*1 + 1*2 + 2*3
        assert_eq!(out.data[0][0], 0.0 * 1.0 + 1.0 * 2.0 + 2.0 * 3.0);
        assert_eq!(out.data[1][0], 1.0 * 1.0 + 2.0 * 2.0 + 3.0 * 3.0);
    }

    #[test]
    fn zeros_uses_const_params_in_array_syntax() {
        let m = Matrix::<4, 5>::zeros();
        assert_eq!(m.data.len(), 4);
        assert!(m.data.iter().all(|row| row.iter().all(|&v| v == 0.0)));
    }

    #[test]
    fn lending_iterator_yields_borrowed_words() {
        let mut it = WordSplitter::new(" a  bb ccc ");
        assert_eq!(it.next(), Some("a"));
        assert_eq!(it.next(), Some("bb"));
        assert_eq!(it.next(), Some("ccc"));
        assert_eq!(it.next(), None);
    }

    #[test]
    fn rpitit_implementations_return_different_types() {
        let plain = Plain("x".to_string());
        let shouty = Shouty("x".to_string());
        // 两个 render() 的返回类型不同，但都能 format!，因为签名承诺 Display。
        assert_eq!(plain.render().to_string(), "x");
        assert_eq!(shouty.render().to_string(), "x!!!");
    }

    #[test]
    fn hrtb_accepts_any_lifetime() {
        assert_eq!(apply_to_any_lifetime(first_word, " one two "), "one");
        let f = boxed_hrtb_closure();
        assert_eq!(f(">>> keep"), "keep");
        assert_eq!(f("no-prefix"), "no-prefix");
    }
}
