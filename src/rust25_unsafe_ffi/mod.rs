//! 第 25 章配套代码：unsafe 与 FFI。
//!
//! 运行方式：`cargo run`。
//!
//! 导出给 C 的符号写在库目标 `src/lib.rs`（`crate-type = ["cdylib", "rlib"]`）里，
//! 本模块通过 `rust_learn::…` 复用它们，所以一次 `cargo run` 就能看到
//! 「Rust 导出 C API → Rust 自己调用」的完整链路；Python `ctypes` 的调用方式
//! 见文档 25.4 与 `examples/ffi/caller.py`。

pub use rust_learn::{ERR_NULL, ERR_TOO_LONG, MAX_BYTES, checksum, multiply, sum_bytes};
use std::ffi::{CStr, CString, c_char, c_int};
use std::marker::PhantomData;
use std::panic::{AssertUnwindSafe, catch_unwind, set_hook, take_hook};

/// 错误码：除数为 0。
pub const ERR_DIVIDE: c_int = -3;

/// 安全封装：空切片返回 0，否则用 `get_unchecked` 读第一个字节。
///
/// 边界检查集中在函数入口，`unsafe` 只剩一行。
pub fn first_or_zero(bytes: &[u8]) -> u8 {
    if bytes.is_empty() {
        0
    } else {
        // SAFETY: 上面的分支保证长度至少为 1，索引 0 必然在切片范围内。
        unsafe { *bytes.get_unchecked(0) }
    }
}

/// 安全封装：只借用切片、不保存指针，越界返回 `None`。
pub fn read_at(values: &[i32], index: usize) -> Option<i32> {
    let pointer = values.as_ptr();
    if index >= values.len() {
        return None;
    }
    // SAFETY: 上面的长度检查保证 index < values.len()，切片在调用期间一直存活，
    // 因此 `add(index)` 仍在同一块分配内，且该地址已初始化为合法的 i32。
    Some(unsafe { *pointer.add(index) })
}

/// `#[repr(C)]` 的只读字节视图，布局等价于 C 的
/// `struct { const uint8_t *data; size_t len; }`。
///
/// `PhantomData<&'a [u8]>` 是零大小字段：C 侧看不到它，但它把借用生命周期
/// 写进了类型，视图不可能比原切片活得更久，于是 [`BufferView::as_slice`]
/// 可以是安全函数，而不是又一个 `unsafe fn`。
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct BufferView<'a> {
    /// 起始地址：由 [`BufferView::from_slice`] 从切片生成。
    pub data: *const u8,
    /// 字节数。C 不知道 Rust 切片的长度，长度必须单独传递。
    pub len: usize,
    borrow: PhantomData<&'a [u8]>,
}

impl<'a> BufferView<'a> {
    /// 从切片借用一段视图；不复制数据，也不取得所有权。
    pub fn from_slice(bytes: &'a [u8]) -> Self {
        Self {
            data: bytes.as_ptr(),
            len: bytes.len(),
            borrow: PhantomData,
        }
    }

    /// 把视图还原成切片；`len == 0` 时直接返回空切片，根本不读指针。
    pub fn as_slice(&self) -> &'a [u8] {
        if self.len == 0 {
            return &[];
        }
        // SAFETY: data/len 只能由 from_slice 从 &'a [u8] 生成（字段私有，
        // 外部无法构造出「指针与长度不一致」的视图），因此这段内存在 'a 内
        // 有效、已初始化，长度就是 len。
        unsafe { std::slice::from_raw_parts(self.data, self.len) }
    }
}

/// 把 Rust 字符串转成 C 兼容的 NUL 结尾字符串。
///
/// 内部含 NUL 时返回 `Err`，而不是悄悄截断——C 侧只认第一个 NUL。
pub fn to_c_string(text: &str) -> Result<CString, std::ffi::NulError> {
    CString::new(text)
}

/// 借用 C 传进来的字符串并读出字节长度；空指针返回 `None`。
///
/// 这里读的是**字节数**而不是字符数：C 字符串不保证是 UTF-8。
pub fn borrowed_byte_len(pointer: *const c_char) -> Option<usize> {
    if pointer.is_null() {
        return None;
    }
    // SAFETY: 空指针是唯一能在函数内部检查的前置条件，这里已经排除；
    // 非空时由调用者按契约保证它指向 NUL 结尾的有效字符串，且调用期间存活。
    Some(unsafe { CStr::from_ptr(pointer) }.to_bytes().len())
}

/// 除数为 0 时 panic 的内部实现。
///
/// 它故意保留「会 panic」的形态，用来演示 panic 一旦穿过 `extern "C"` 边界
/// 会发生什么（Rust 1.81 起默认直接 abort）。
fn divide_strict(left: c_int, right: c_int) -> c_int {
    assert!(right != 0, "除数不能为 0");
    left / right
}

/// 导出给 C 的除法：把 panic 拦在 Rust 内部，用错误码返回。
///
/// 契约：`right != 0` 时返回商，否则返回 [`ERR_DIVIDE`]。
#[unsafe(no_mangle)]
pub extern "C" fn safe_divide(left: c_int, right: c_int) -> c_int {
    match catch_unwind(|| divide_strict(left, right)) {
        Ok(value) => value,
        Err(_) => ERR_DIVIDE,
    }
}

/// 在「静音 panic 输出」的前提下执行可能 panic 的代码。
///
/// 只用于让演示和测试输出保持干净：默认的 panic hook 会往 stderr 写信息，
/// 而演示想自己决定怎么呈现。**真实项目不要动全局 hook**——那正是日志和
/// 监控要看到的信号。
fn quiet<T>(body: impl FnOnce() -> T) -> std::thread::Result<T> {
    let previous = take_hook();
    set_hook(Box::new(|_| {}));
    let outcome = catch_unwind(AssertUnwindSafe(body));
    set_hook(previous);
    outcome
}

/// 从 panic 载荷里取出消息文本，供演示打印。
fn panic_message(payload: &(dyn std::any::Any + Send)) -> &str {
    payload
        .downcast_ref::<&'static str>()
        .copied()
        .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
        .unwrap_or("（非字符串 panic 载荷）")
}

/// 演示裸指针、安全封装、`#[repr(C)]` 布局、C 字符串与 panic 隔离。
pub fn unsafe_ffi_demo() {
    println!("\n========== rust25_unsafe_ffi: unsafe 与 FFI ==========");

    println!("\n--- 1. 裸指针只在 unsafe 中解引用 ---");
    let values = [10_i32, 20, 30];
    let pointer = values.as_ptr();
    println!("    values.as_ptr() = {pointer:p}");
    // SAFETY: pointer 来自上面仍然存活、且已初始化的数组，读第一个元素合法。
    println!("    解引用首元素 = {}", unsafe { *pointer });
    println!("    first_or_zero([]) = {}", first_or_zero(&[]));
    println!(
        "    first_or_zero([10, 20, 30]) = {}",
        first_or_zero(&[10, 20, 30])
    );

    println!("\n--- 2. 安全封装把越界挡在边界检查里 ---");
    println!("    read_at([10, 20, 30], 0) = {:?}", read_at(&values, 0));
    println!("    read_at([10, 20, 30], 2) = {:?}", read_at(&values, 2));
    println!(
        "    read_at([10, 20, 30], 3) = {:?}（越界返回 None，而不是 UB）",
        read_at(&values, 3)
    );

    println!("\n--- 3. #[repr(C)] 结构体 + 长度一起传 ---");
    let payload = [1_u8, 2, 3];
    let view = BufferView::from_slice(&payload);
    println!(
        "    size_of::<BufferView>() = {}，offset_of!(len) = {}",
        std::mem::size_of::<BufferView<'_>>(),
        std::mem::offset_of!(BufferView<'_>, len)
    );
    println!(
        "    view.len = {}，view.as_slice() = {:?}",
        view.len,
        view.as_slice()
    );
    println!(
        "    空视图的 as_slice() = {:?}",
        BufferView::from_slice(&[]).as_slice()
    );

    println!("\n--- 4. C 字符串：NUL 结尾与编码 ---");
    match to_c_string("中文 rust") {
        Ok(text) => println!(
            "    to_c_string(\"中文 rust\") 的字节数 = {:?}（汉字各占 3 字节）",
            borrowed_byte_len(text.as_ptr())
        ),
        Err(error) => println!("    转换失败：{error}"),
    }
    println!(
        "    to_c_string(\"a\\0b\") 是否失败 = {}",
        to_c_string("a\0b").is_err()
    );
    println!(
        "    borrowed_byte_len(NULL) = {:?}",
        borrowed_byte_len(std::ptr::null())
    );

    println!("\n--- 5. 调用导出的 C API（同一个包里的库目标） ---");
    println!("    multiply(6, 7) = {}", multiply(6, 7));
    // SAFETY: payload 是上面的本地数组，长度 3、调用期间有效，满足
    // 「非空指针指向至少 len 个可读字节」的契约。
    let (sum, checked, null_zero) = unsafe {
        (
            sum_bytes(payload.as_ptr(), payload.len()),
            checksum(payload.as_ptr(), payload.len()),
            sum_bytes(std::ptr::null(), 0),
        )
    };
    println!("    sum_bytes(payload) = {sum}");
    println!("    checksum(payload) = {checked}（1+2+3 = 6，与 sum_bytes 相同）");
    println!("    sum_bytes(NULL, 0) = {null_zero}（空指针只在长度为 0 时合法）");
    // SAFETY: 这两次调用故意传错参数（NULL + 非零长度、长度超限），但函数在
    // 解引用之前就把它们变成错误码返回，不会读到非法内存。
    let (null_error, too_long) = unsafe {
        (
            sum_bytes(std::ptr::null(), 4),
            sum_bytes(payload.as_ptr(), MAX_BYTES + 1),
        )
    };
    println!("    sum_bytes(NULL, 4) = {null_error}（就是 ERR_NULL = {ERR_NULL}，不崩溃）");
    println!(
        "    sum_bytes(payload, MAX_BYTES + 1) = {too_long}（就是 ERR_TOO_LONG = {ERR_TOO_LONG}，上限 {MAX_BYTES} 字节）"
    );

    println!("\n--- 6. panic 不穿过 FFI 边界 ---");
    println!("    safe_divide(10, 2) = {}", safe_divide(10, 2));
    // 这里的 quiet 只是为了不让默认 panic hook 往 stderr 写信息，
    // safe_divide 自己已经用 catch_unwind 把 panic 转成了错误码。
    let division = quiet(|| safe_divide(1, 0)).unwrap_or(ERR_DIVIDE);
    println!("    safe_divide(1, 0) = {division}（就是 ERR_DIVIDE = {ERR_DIVIDE}，进程继续运行）");
    match quiet(|| divide_strict(1, 0)) {
        Ok(value) => println!("    不该走到这里：{value}"),
        Err(payload) => println!("    未包装的实现会 panic：{}", panic_message(&*payload)),
    }
    println!("    （panic 若穿过 extern \"C\" 边界，进程默认直接 abort；所以在 Rust 里拦住）");

    println!("\n--- 7. 审查清单 ---");
    println!("    每个 unsafe 块前写 SAFETY，说明不变量由哪段代码保证");
    println!("    unsafe 范围最小化：边界检查放外面，危险操作只留一行");
    println!("    空指针、长度上限、编码、所有权与释放责任都要写进接口契约");
    println!("    cargo clippy --all-targets --all-features -- -D warnings");

    println!("\n========== unsafe 与 FFI 演示结束 ==========");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::{offset_of, size_of};
    use std::ptr;

    #[test]
    fn safe_wrapper_handles_empty_and_non_empty_slices() {
        assert_eq!(first_or_zero(&[]), 0);
        assert_eq!(first_or_zero(&[7, 8]), 7);
    }

    #[test]
    fn read_at_covers_every_boundary() {
        let values = [10, 20, 30];
        assert_eq!(read_at(&values, 0), Some(10)); // 第一个合法索引
        assert_eq!(read_at(&values, 2), Some(30)); // 最后一个合法索引
        assert_eq!(read_at(&values, 3), None); // 正好越界
        assert_eq!(read_at(&[], 0), None); // 空切片
    }

    #[test]
    fn buffer_view_roundtrips_and_keeps_c_layout() {
        let bytes = [1_u8, 2, 3];
        let view = BufferView::from_slice(&bytes);
        assert_eq!(view.len, 3);
        assert_eq!(view.as_slice(), &bytes);

        // 空视图不读指针，所以这一句不需要 unsafe。
        assert!(BufferView::from_slice(&[]).as_slice().is_empty());

        // repr(C) + 两个 usize 宽字段：C 侧看到的偏移与 Rust 一致，
        // PhantomData 是零大小字段，不影响布局。
        assert_eq!(size_of::<BufferView<'_>>(), 2 * size_of::<usize>());
        assert_eq!(offset_of!(BufferView<'_>, len), size_of::<usize>());
    }

    #[test]
    fn c_strings_reject_interior_nul_and_count_bytes() {
        assert!(to_c_string("rust").is_ok());
        assert!(to_c_string("a\0b").is_err()); // C 只认第一个 NUL，必须报错

        let text = CString::new("中文").unwrap();
        assert_eq!(borrowed_byte_len(text.as_ptr()), Some(6)); // 2 个汉字 × 3 字节
        assert_eq!(borrowed_byte_len(ptr::null()), None);
    }

    #[test]
    fn exported_api_follows_the_null_and_length_contract() {
        let bytes = [1_u8, 2, 3];
        // SAFETY: bytes 在作用域内有效；NULL 只与非零长度搭配用来验证错误码。
        unsafe {
            assert_eq!(sum_bytes(bytes.as_ptr(), bytes.len()), 6);
            assert_eq!(sum_bytes(ptr::null(), 0), 0); // NULL + 0 合法
            assert_eq!(sum_bytes(ptr::null(), 4), ERR_NULL); // NULL + 非零长度是错误
            assert_eq!(sum_bytes(bytes.as_ptr(), MAX_BYTES + 1), ERR_TOO_LONG);
            assert_eq!(checksum(bytes.as_ptr(), bytes.len()), 6);
        }
    }

    #[test]
    fn panic_is_converted_to_an_error_code() {
        assert_eq!(safe_divide(10, 2), 5);
        // 下面这行内部会 panic，再被 catch_unwind 转成错误码；quiet 只是让
        // 测试输出不被打断（默认 hook 会把 panic 信息写进 stderr）。
        assert_eq!(quiet(|| safe_divide(1, 0)).unwrap(), ERR_DIVIDE);
        assert_eq!(quiet(|| safe_divide(7, 1)).unwrap(), 7);
    }
}
