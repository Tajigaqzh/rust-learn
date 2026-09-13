//! 第 25 章：unsafe 与 FFI。

/// 使用明确的不变量封装一个需要 unsafe 的切片读取。
fn first_or_zero(bytes: &[u8]) -> u8 {
    if bytes.is_empty() {
        0
    } else {
        // SAFETY: 非空分支保证索引 0 在切片范围内。
        unsafe { *bytes.get_unchecked(0) }
    }
}

/// 演示裸指针、unsafe 封装和 C ABI 外部函数声明。
pub fn unsafe_ffi_demo() {
    println!("\n========== rust25_unsafe_ffi: unsafe 与 FFI ==========");

    let values = [10_u8, 20, 30];
    let ptr = values.as_ptr();
    println!("\n--- 1. 裸指针只在 unsafe 中解引用 ---");
    println!("    ptr = {ptr:p}, *ptr = {}", unsafe { *ptr });
    println!("    first_or_zero([]) = {}", first_or_zero(&[]));
    println!(
        "    first_or_zero([10, 20, 30]) = {}",
        first_or_zero(&values)
    );

    println!("\n--- 2. FFI 边界 ---");
    println!("    extern \"C\" 声明用于调用 C ABI；跨语言指针必须明确所有权、长度和生命周期");
    println!("    本章只展示安全封装，不链接平台相关的外部库");

    println!("\n--- 3. 安全清单 ---");
    println!("    缩小 unsafe 范围，写出 SAFETY 不变量，并用安全函数包裹它");
    println!("    FFI 输入先校验空指针、长度、编码和释放责任");
    println!("    cargo clippy --all-targets -- -D warnings 检查潜在问题");

    println!("\n========== unsafe 与 FFI 演示结束 ==========");
}

#[cfg(test)]
mod tests {
    use super::first_or_zero;

    #[test]
    fn safe_wrapper_handles_empty_and_non_empty_slices() {
        assert_eq!(first_or_zero(&[]), 0);
        assert_eq!(first_or_zero(&[7, 8]), 7);
    }
}
