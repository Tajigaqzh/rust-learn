//! 工作区成员之二：只做命令行入口，业务规则复用 `core-lib`。

use core_lib::normalize_title;

fn main() {
    let title = normalize_title("  写   Rust   ");
    println!("{title}");
}
