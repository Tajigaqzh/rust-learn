//! 第 25 章的 FFI 动态库目标。
//!
//! 清单里写的是 `[lib] crate-type = ["cdylib", "rlib"]`：`cdylib` 产出可被
//! C ABI 调用者加载的动态库，`rlib` 让同一个包里的二进制目标（`cargo run`）
//! 也能直接复用这里的函数。
//!
//! 构建：`cargo build --release --lib`，Windows 产物是
//! `target/release/rust_learn.dll`，Linux 是 `librust_learn.so`，
//! macOS 是 `librust_learn.dylib`；调用示例见 `examples/ffi/caller.py`。

use std::ffi::{c_int, c_uchar};

/// [`sum_bytes`] 与 [`checksum`] 允许的最大输入长度。
///
/// 上限本身是契约的一部分：没有它，C 侧的 `size_t` 可以传进一个荒唐的长度，
/// 而 Rust 会在 `from_raw_parts` 里读越界内存。
pub const MAX_BYTES: usize = 1 << 20;

/// 错误码：`data == NULL` 却传了非零长度。
pub const ERR_NULL: c_int = -1;

/// 错误码：长度超过 [`MAX_BYTES`]。
pub const ERR_TOO_LONG: c_int = -2;

/// 可由 Python `ctypes` 或其他 C ABI 调用者使用的乘法函数。
///
/// 溢出时饱和到 `c_int` 的边界，而不是 panic——panic 不能穿过 FFI 边界。
#[unsafe(no_mangle)]
pub extern "C" fn multiply(left: c_int, right: c_int) -> c_int {
    left.saturating_mul(right)
}

/// 逐字节求和，错误以负的错误码返回。
///
/// `NULL` 与长度上限在函数内部检查，但**内存可读性没法在函数内部证明**，
/// 所以它仍是 `unsafe fn`：调用者必须满足下面的契约（也应同步写进 C 头文件）：
///
/// - `data.is_null()` 时只有 `len == 0` 合法，此时返回 0；
/// - `len > MAX_BYTES` 时返回 [`ERR_TOO_LONG`]；
/// - `data` 非空时由调用者保证它起 `len` 个字节可读，且本次调用期间有效；
/// - 本函数不保存指针，也不释放任何内存。
///
/// # Safety
///
/// `data` 为空指针时 `len` 必须是 0（否则返回 [`ERR_NULL`]——这是契约内的错误
/// 输入，不是未定义行为）；`data` 非空时必须指向至少 `len` 个已初始化的可读
/// 字节，且在这次调用期间保持有效；`len` 必须不超过 [`MAX_BYTES`]。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sum_bytes(data: *const c_uchar, len: usize) -> c_int {
    if data.is_null() {
        return if len == 0 { 0 } else { ERR_NULL };
    }
    if len > MAX_BYTES {
        return ERR_TOO_LONG;
    }
    // SAFETY: 上面已经排除空指针并限制了长度上限；剩下的可读性与生命周期
    // 由调用者按上面的契约保证。
    let bytes = unsafe { std::slice::from_raw_parts(data, len) };
    bytes.iter().map(|&byte| c_int::from(byte)).sum()
}

/// 32 位校验和，空指针与长度契约和 [`sum_bytes`] 相同。
///
/// 注意返回 `0` 同时表示「空输入」和「参数非法」：需要区分时请换成
/// 错误码 + out 参数的组合（见第 25 章正文）。
///
/// # Safety
///
/// 与 [`sum_bytes`] 相同：`data` 为空指针时 `len` 必须是 0；`data` 非空时必须
/// 指向至少 `len` 个已初始化的可读字节，且在这次调用期间保持有效。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn checksum(data: *const c_uchar, len: usize) -> u32 {
    if data.is_null() || len > MAX_BYTES {
        return 0;
    }
    // SAFETY: 同 sum_bytes：指针非空、长度受限，可读性由调用者保证。
    let bytes = unsafe { std::slice::from_raw_parts(data, len) };
    bytes.iter().map(|&byte| u32::from(byte)).sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ptr;

    #[test]
    fn multiply_saturates_instead_of_panicking() {
        assert_eq!(multiply(6, 7), 42);
        assert_eq!(multiply(c_int::MAX, 2), c_int::MAX);
        assert_eq!(multiply(c_int::MIN, 2), c_int::MIN);
    }

    #[test]
    fn sum_and_checksum_follow_the_null_and_length_contract() {
        let bytes = [1_u8, 2, 3];
        // SAFETY: bytes 是本地数组，指针有效且长度为 3，满足契约。
        unsafe {
            assert_eq!(sum_bytes(bytes.as_ptr(), bytes.len()), 6);
            assert_eq!(checksum(bytes.as_ptr(), bytes.len()), 6);

            assert_eq!(sum_bytes(ptr::null(), 0), 0); // NULL + 0 是合法的
            assert_eq!(sum_bytes(ptr::null(), 1), ERR_NULL); // NULL + 非零长度是错误
            assert_eq!(checksum(ptr::null(), 1), 0);

            assert_eq!(sum_bytes(bytes.as_ptr(), MAX_BYTES + 1), ERR_TOO_LONG);
            assert_eq!(checksum(bytes.as_ptr(), MAX_BYTES + 1), 0);
        }
    }
}
