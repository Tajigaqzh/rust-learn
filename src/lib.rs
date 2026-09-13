//! 第 25 章 FFI 动态库目标。

use std::ffi::c_int;

/// 可由 Python `ctypes` 或其他 C ABI 调用者使用的乘法函数。
#[unsafe(no_mangle)]
pub extern "C" fn multiply(left: c_int, right: c_int) -> c_int {
    left.saturating_mul(right)
}
