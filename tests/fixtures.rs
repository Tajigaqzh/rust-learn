//! 用共享夹具做集成测试。
//!
//! 这些测试会真的读写磁盘，但**全部发生在临时目录里**，测试之间互不干扰。

mod common;

use common::{make_fixture, total_chars};

#[test]
fn counts_chars_in_fixture() {
    let (_dir, root) = make_fixture(&[
        ("a.txt", "abc"),
        ("sub/b.txt", "中文"),
    ]);

    // "abc" 3 个字符 + "中文" 2 个字符
    assert_eq!(total_chars(&root), 5);
}

#[test]
fn fixtures_are_isolated_between_tests() {
    let (_dir, root) = make_fixture(&[("only.txt", "x")]);
    assert_eq!(total_chars(&root), 1);
}

#[test]
fn fixture_directories_are_cleaned_up() {
    let (dir, root) = make_fixture(&[("temp.txt", "内容")]);
    let path = root.clone();
    assert!(path.exists());

    drop(dir); // 模拟测试结束
    assert!(!path.exists(), "TempDir 被 drop 后目录应当被删除");
}
