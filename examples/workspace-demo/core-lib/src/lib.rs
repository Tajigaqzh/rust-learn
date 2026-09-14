//! 工作区成员之一：只放业务规则，不依赖任何第三方 crate。
//!
//! 这样 `cli-bin` 可以复用规则，测试也可以只针对这个小 crate 跑：
//! `cargo test -p core-lib`。

/// 把用户输入的任务标题规范化：去掉首尾空白，把连续空白压成一个空格。
pub fn normalize_title(input: &str) -> String {
    input.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::normalize_title;

    #[test]
    fn trims_and_collapses_whitespace() {
        assert_eq!(normalize_title("  写   Rust   "), "写 Rust");
        assert_eq!(normalize_title(""), "");
    }
}
