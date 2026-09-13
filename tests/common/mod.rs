//! 集成测试共享的辅助代码。
//!
//! `tests/` 下每个 `.rs` 文件都是一个独立 crate，所以公共代码要放在子目录里，
//! 用 `mod common;` 引入——**放在 `tests/common/mod.rs` 而不是 `tests/common.rs`**，
//! 这样 Cargo 不会把它当成一个测试文件去跑。

use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use walkdir::WalkDir;

/// 建一个临时目录，把 `(相对路径, 内容)` 列表写进去。
///
/// 返回 `TempDir` 的持有者（别丢掉，drop 时目录会被删除）和根路径。
pub fn make_fixture(files: &[(&str, &str)]) -> (TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("创建临时目录失败");
    let root = dir.path().to_path_buf();

    for (relative, content) in files {
        let path = root.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("创建目录失败");
        }
        fs::write(&path, content).expect("写文件失败");
    }

    (dir, root)
}

/// 统计目录下所有文件的字符数总和（递归）。
pub fn total_chars(root: &Path) -> usize {
    WalkDir::new(root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .filter_map(|entry| fs::read_to_string(entry.path()).ok())
        .map(|text| text.chars().count())
        .sum()
}
