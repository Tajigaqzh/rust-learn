# 第 25 章 · unsafe 与 FFI

Rust 的 `unsafe` 不是“关闭所有检查”，而是把少数无法由编译器证明的约束交给程序员。正确做法是把不安全操作缩小到最小范围，再用安全 API 暴露出来。本章代码位于 `src/rust25_unsafe_ffi/mod.rs`。

## 25.1 裸指针

引用保证了有效性、对齐和生命周期；裸指针 `*const T` / `*mut T` 不提供这些保证。创建裸指针通常是安全的，但解引用必须位于 `unsafe` 块：

```rust
let value = 42;
let pointer = &value as *const i32;
let copied = unsafe { *pointer };
```

解引用前必须证明指针非空、指向正确类型、满足对齐要求，并且指向的对象仍然存活。

### 指针的三条硬规则

1. **有效性**：指针必须来自仍存活的对象，不能是空指针或已经释放的地址。
2. **对齐与初始化**：`*const T` 必须满足 `T` 的对齐要求，且目标内存已经初始化为合法的 `T`。
3. **别名与可变性**：通过 `*mut T` 写入时，不能同时存在违反 Rust 别名规则的引用；不能用裸指针绕过借用检查器制造数据竞争。

下面的写法只借用数组，不保存指针，因此数组在解引用时仍然存活：

```rust
fn read_at(values: &[i32], index: usize) -> Option<i32> {
    let pointer = values.as_ptr();
    if index >= values.len() {
        return None;
    }
    // SAFETY: 上面的检查保证 index 在切片范围内，values 保证内存存活且对齐。
    Some(unsafe { *pointer.add(index) })
}
```

不要把 `pointer.add(index)` 当成边界检查；`add` 本身不会验证长度，越界解引用属于未定义行为（UB）。

## 25.2 把 unsafe 封装起来

本章的 `first_or_zero` 只在确认切片非空后调用 `get_unchecked(0)`。调用者只面对安全签名，边界检查集中在一个地方。每个 `unsafe` 块前都应写 `SAFETY` 注释，说明不变量由哪段代码保证。

优先使用安全标准库 API；只有在性能测量证明必要，或操作本身无法用安全 API 表达时，才引入 unsafe。`cargo clippy` 能发现一部分不必要或可疑的 unsafe 用法。

### `unsafe fn` 与 `unsafe` 块

`unsafe fn` 把前置条件交给调用者，适合表达“调用者必须已经证明某件事”的底层原语；安全包装函数则应在进入 unsafe 块前完成所有检查。Rust 2024 要求在 `unsafe fn` 内部仍用显式 `unsafe { ... }` 包住每个危险操作，这让审查范围更清楚。

```rust
/// # Safety
/// `ptr` 必须非空、对齐，并指向一个已初始化的 `u32`。
unsafe fn read_raw(ptr: *const u32) -> u32 {
    // SAFETY: 由函数调用者保证文档中的前置条件。
    unsafe { *ptr }
}
```

常见 UB 来源包括越界、释放后使用、错误转型（`transmute`）、未对齐读取、违反 `Send`/`Sync` 约束，以及跨线程同时读写。UB 不一定立即崩溃，优化后可能表现为完全不可预测的结果。

## 25.3 FFI 与 C ABI

FFI（Foreign Function Interface）让 Rust 与 C 等语言交互。声明外部函数时使用 C ABI：

```rust
unsafe extern "C" {
    fn c_function(input: *const std::ffi::c_char) -> i32;
}
```

调用前要确认头文件契约：指针是否允许为空、字符串是否以 NUL 结尾、长度由谁提供、内存由谁分配和释放，以及函数是否线程安全。跨边界传递 Rust 的引用、`String` 或 trait 对象通常是不正确的；应使用 C 兼容类型和明确的生命周期协议。

实际项目常用 `#[unsafe(no_mangle)] pub extern "C" fn ...` 导出函数，再用 `bindgen` 根据 C 头文件生成声明。生成代码应隔离在专门模块中，并对外提供经过校验的安全包装。

### C 兼容类型与布局

FFI 接口只使用拥有稳定 C 布局的类型：`std::ffi::c_int`、`c_char`、`c_void`、`#[repr(C)] struct` 和指针/长度组合。Rust 默认 `struct` 布局不保证字段顺序，不能直接当作 C 结构体传递。

```rust
#[repr(C)]
pub struct BufferView {
    pub data: *const u8,
    pub len: usize,
}
```

`BufferView` 不拥有内存，C 调用者必须保证 `data` 在调用期间有效。若接口需要转移所有权，应同时提供对应的释放函数，且分配和释放必须使用同一个分配器。

### 导出一个安全的 C API

导出函数要处理空指针、长度溢出和错误码，不能让 Rust panic 穿过 FFI 边界：

```rust
#[unsafe(no_mangle)]
pub extern "C" fn sum_bytes(data: *const u8, len: usize) -> i32 {
    if data.is_null() && len != 0 {
        return -1;
    }
    // SAFETY: 空指针只在 len == 0 时允许；非空时由调用者保证 len 个字节可读。
    let bytes = unsafe { std::slice::from_raw_parts(data, len) };
    bytes.iter().map(|&byte| byte as i32).sum()
}
```

生产接口还应限制 `len` 的最大值、使用 `catch_unwind` 防止 panic 外泄，并在 C 头文件中写明返回码和线程安全性。

### 从 Rust 调用 C 函数：完整最小流程

假设 C 库提供一个相加函数。先写头文件和实现：

```c
/* native/math.h */
#ifndef MATH_H
#define MATH_H
int add_ints(int left, int right);
#endif
```

```c
/* native/math.c */
#include "math.h"
int add_ints(int left, int right) { return left + right; }
```

仓库中对应的完整文件位于 `examples/ffi/math.h`、`examples/ffi/math.c` 和 `examples/ffi/caller.c`。在 Linux/macOS 上可用 `cc examples/ffi/math.c examples/ffi/caller.c -o /tmp/ffi-caller && /tmp/ffi-caller` 编译运行；Windows 可使用 Visual Studio Developer Command Prompt 的 `cl examples/ffi/math.c examples/ffi/caller.c`。这些文件是独立的 C 示例，不会改变主项目的 Rust 构建流程。

目录还提供了 `examples/ffi/Makefile`，在安装 GNU Make 和 C 编译器后可执行：

```bash
cd examples/ffi
make        # 编译 caller
make run    # 编译并运行，输出 42
make clean  # 删除本地 C 产物
```

Windows 的 MinGW 环境通常使用 `mingw32-make`；使用 MSVC 时仍可直接运行前面的 `cl` 命令。Makefile 只负责独立的 C 示例，Rust 动态库仍由根目录的 Cargo 管理。

使用 `cc` crate 在构建时编译 C 文件（`Cargo.toml` 增加 `cc = "1"`，放在 `[build-dependencies]`）：

```rust
// build.rs
fn main() {
    cc::Build::new()
        .file("native/math.c")
        .include("native")
        .compile("native_math");
}
```

Rust 侧声明并调用：

```rust
unsafe extern "C" {
    fn add_ints(left: std::ffi::c_int, right: std::ffi::c_int) -> std::ffi::c_int;
}

fn add(left: i32, right: i32) -> i32 {
    // SAFETY: build.rs 确保链接了 add_ints；i32 与 C int 的 ABI 约定由接口固定。
    unsafe { add_ints(left, right) as i32 }
}
```

`extern "C"` 只描述调用约定，不会自动找到或编译库。静态库可由 `cc` 生成，也可以在 `build.rs` 中打印 `cargo:rustc-link-search` 和 `cargo:rustc-link-lib` 来链接系统库：

```rust
println!("cargo:rustc-link-search=native=/opt/mylib/lib");
println!("cargo:rustc-link-lib=static=mylib");
```

跨平台项目应根据 `TARGET` 环境变量选择 `.a`、`.lib` 或 `.so/.dll`，并在 CI 中安装对应 C 编译器。动态库还涉及运行时搜索路径，发布包不能只在开发机上能找到它。

### FFI 参数怎么传

| Rust 参数 | C 侧形式 | 约定 |
| --- | --- | --- |
| `c_int`、`c_uint` 等 | `int`、`unsigned int` | 使用 `std::ffi::c_*`，不要猜平台宽度 |
| `*const T` + `usize` | `const T*` + `size_t` | C 不知道 Rust 切片长度，长度必须单独传递 |
| `*mut T` + `usize` | `T*` + `size_t` | 明确 C 是否允许修改，以及谁拥有内存 |
| `*const c_char` | `const char*` | 必须是 NUL 结尾；编码不自动等于 UTF-8 |
| `#[repr(C)] struct` | `struct` | 字段顺序和对齐固定，不能直接传 Rust 默认布局 |
| `Option<NonNull<T>>` | 可空指针 | 仅对 ABI 有保证的类型使用，接口文档写明 NULL 语义 |

传递切片时通常写成 `slice.as_ptr()` 和 `slice.len()`；C 只能在调用期间使用这段内存，Rust 侧必须保证切片仍存活且没有并发可变访问。传递字符串时用 `CString::new` 检查内部 NUL，再调用 `as_ptr()`；C 返回的字符串用 `CStr::from_ptr` 借用，不能直接释放或转成拥有值，除非 C 明确交出释放责任。

### FFI 注意事项

- **ABI 与布局**：函数声明、整数宽度、结构体 `#[repr(C)]`、枚举表示和调用约定必须与头文件完全一致；声明错一个类型都可能导致未定义行为。
- **生命周期**：裸指针不延长对象寿命。异步或保存回调指针时，必须设计显式的创建/销毁 API。
- **所有权**：谁分配谁释放是最稳妥的规则。不要用 Rust 的 `Box::from_raw` 释放由 C `malloc` 得到的指针，反之亦然。
- **空指针与长度**：`NULL + 0` 是否允许要写入契约；Rust 构造 slice 前必须先检查空指针和长度。
- **panic 与异常**：Rust panic 不得穿过 C 边界；C++ exception 也不能穿过 Rust 函数。用错误码、out 参数或 `Result` 包装。
- **线程安全**：C 库的全局状态、回调和句柄是否可跨线程使用必须查文档；不要仅凭 Rust 类型推断 C 代码安全。
- **回调函数**：使用 `extern "C" fn`，并让用户数据通过 `*mut c_void` 传递；注销回调前确保 C 不会再调用它。
- **工具链与发布**：`bindgen` 生成的绑定应固定头文件版本；构建脚本要处理目标平台、编译器和动态库部署。

## 25.4 Python `ctypes` 调用 Rust

FFI 不只用于 C。Python 的 `ctypes` 可以加载遵循 C ABI 的动态库，因此可以直接调用 Rust 导出的 `extern "C"` 函数。关键仍然是：两边必须使用完全一致的参数类型和内存契约。

先把 Rust 包配置为动态库：

```toml
# Cargo.toml
[lib]
crate-type = ["cdylib"]
```

库目标中导出只使用 C 兼容类型的函数：

```rust
use std::ffi::c_int;

#[unsafe(no_mangle)]
pub extern "C" fn multiply(left: c_int, right: c_int) -> c_int {
    left.saturating_mul(right)
}
```

运行 `cargo build --release` 后，Windows 产物通常是 `target/release/rust_learn.dll`，Linux 是 `librust_learn.so`，macOS 是 `librust_learn.dylib`。Python 侧声明返回值和参数类型：

```python
import ctypes
import sys

library_name = {
    "win32": "target/release/rust_learn.dll",
    "darwin": "target/release/librust_learn.dylib",
}.get(sys.platform, "target/release/librust_learn.so")

lib = ctypes.CDLL(library_name)
lib.multiply.argtypes = [ctypes.c_int, ctypes.c_int]
lib.multiply.restype = ctypes.c_int

print(lib.multiply(6, 7))  # 42
```

同样的可运行脚本保存在 `examples/ffi/caller.py`。先按后文配置 `cdylib` 并执行 `cargo build --release`，再从仓库根目录运行 `python examples/ffi/caller.py`。脚本会根据当前操作系统选择 `.dll`、`.so` 或 `.dylib`。

`argtypes` 和 `restype` 不能省略：如果 Python 按错误宽度读取返回值，或者把指针当整数传递，程序可能直接崩溃。这个例子只传值类型，不跨边界传递 Rust `String`、`Vec` 或引用。

### Python 传递字节缓冲区

指针参数必须配合长度，并约定调用期间的所有权：

```rust
use std::ffi::c_uchar;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn checksum(data: *const c_uchar, len: usize) -> u32 {
    if data.is_null() && len != 0 {
        return 0;
    }
    // SAFETY: 调用者保证 data 指向至少 len 个可读字节，且调用期间保持有效。
    let bytes = unsafe { std::slice::from_raw_parts(data, len) };
    bytes.iter().map(|&byte| byte as u32).sum()
}
```

```python
payload = b"rust"
lib.checksum.argtypes = [ctypes.POINTER(ctypes.c_ubyte), ctypes.c_size_t]
lib.checksum.restype = ctypes.c_uint32
buffer = (ctypes.c_ubyte * len(payload)).from_buffer_copy(payload)
print(lib.checksum(buffer, len(payload)))
```

Rust 不会持有 `buffer`，Python 必须让它存活到调用返回。若 Rust 需要异步保存数据，不能保存这个指针；应复制到 Rust 自己拥有的内存，并提供明确的释放函数。

### 与 Python C 扩展的区别

`ctypes` 适合少量稳定的 C ABI 函数，不需要编写 Python 扩展模块。复杂对象、异常映射和高频调用通常使用 `pyo3`/`maturin`，由框架负责 Python 对象转换。无论选择哪种方式，都应避免让 Rust panic 穿过 Python 边界，并为每个导出函数固定 ABI 和错误协议。

### 字符串边界

`CString` 用于生成以 NUL 结尾的 C 字符串；`CStr` 用于借用 C 传入的字符串。不要把任意 `&str` 的指针直接交给 C，也不要假设 C 字符串一定是 UTF-8。转换失败时返回错误码或显式的 `Result`，而不是强行 `unwrap`。

```rust
use std::ffi::{CStr, c_char};

unsafe fn c_string_length(input: *const c_char) -> Option<usize> {
    if input.is_null() { return None; }
    // SAFETY: 调用者保证 input 指向 NUL 结尾的有效 C 字符串。
    Some(unsafe { CStr::from_ptr(input) }.to_bytes().len())
}
```

## 25.4 构建一个可审查的 unsafe 模块

把底层代码集中到一个模块，公开 API 保持安全，并让测试覆盖每条前置条件。推荐的目录结构是：

```text
src/ffi/
├── mod.rs          # 安全公开 API
├── raw.rs          # 最小化 unsafe 与 extern 声明
└── bindings.rs     # bindgen 生成文件（不手工编辑）
```

审查时逐个回答：数据从哪里来、谁拥有它、何时失效、长度如何获得、哪个线程访问、错误如何返回、谁负责释放。任何一个问题答不上来，都不应扩大 unsafe 范围。

## 25.5 验证与清单

```bash
cargo fmt --check
cargo test --all-features --locked
cargo clippy --all-targets --all-features -- -D warnings
```

- unsafe 范围是否最小？
- 每个块是否有可验证的 `SAFETY` 不变量？
- FFI 的 ABI、布局、编码、线程和释放责任是否写入文档？
- 是否用测试覆盖空指针、长度边界和错误路径？

建议再使用 Miri 检查未定义行为（需要 nightly 工具链）：

```bash
cargo +nightly miri test
```

Miri 不能替代普通测试，也不能验证真实 C 库的所有行为，但能发现越界、悬垂指针、未对齐访问和部分别名违规。

## 25.6 练习

1. 为 `read_at` 增加负向测试：空切片、索引等于长度、最后一个合法索引。
2. 为 `sum_bytes` 设计 C 侧契约：`data == NULL` 时哪些 `len` 合法？错误码如何表示？
3. 写一个 `#[repr(C)]` 的配置结构体，说明每个字段的所有权、编码和 ABI 类型，并解释为什么不能直接使用 `String`。
4. 使用 `cargo clippy --all-targets --all-features -- -D warnings` 检查本章代码，记录每个 lint 的原因。

## 25.7 小结

`unsafe` 代表责任转移，不代表可以忽略规则；FFI 则要求把语言边界上的内存和所有权契约写清楚。把危险操作封装在小函数里，再用测试和文档守住不变量，是 Rust 项目中最可维护的做法。
