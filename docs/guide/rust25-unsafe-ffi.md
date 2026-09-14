# 第 25 章 · unsafe 与 FFI

`unsafe` 不是「关掉检查」，而是把少数**编译器无法证明**的约束交给程序员，
再用安全 API 把它包起来。FFI（Foreign Function Interface）则是这套思路最典型
的用武之地：语言边界上的一切——布局、长度、生命周期、所有权、错误——
都得由接口契约写清楚。

本章配套代码分几块，全部在本仓库里真实可跑：

| 部分 | 位置 | 怎么跑 |
| --- | --- | --- |
| 演示模块（裸指针、安全封装、panic 隔离） | `src/rust25_unsafe_ffi/mod.rs` | `cargo run` |
| 导出的 C ABI 动态库目标 | `src/lib.rs` | `cargo build --release --lib` |
| C 侧调用示例 | `examples/ffi/math.c`、`caller.c`、`Makefile` | `cc` 编译运行（见 25.8） |
| Python `ctypes` 调用 | `examples/ffi/caller.py` | `python examples/ffi/caller.py` |
| Go 调用 | `examples/ffi/caller.go` | `go run examples/ffi/caller.go` |

`src/lib.rs` 之所以能被这些语言调用，靠的是清单里的一行配置与函数上的一个属性：

```toml
[lib]
crate-type = ["cdylib", "rlib"]   # cdylib 给 C ABI 调用者，rlib 给同包的二进制目标
```

```rust
#[unsafe(no_mangle)]              // 别把符号名按 Rust 规则改名，C 才能按名字找到
pub extern "C" fn multiply(left: c_int, right: c_int) -> c_int { ... }
```

## 25.1 `unsafe` 到底关掉了什么

`unsafe` 是一个**显式的责任移交声明**。它只打开下面这五件事：

| `unsafe` 才能做的事 | 说明 |
| --- | --- |
| 解引用裸指针 | `*const T` / `*mut T` 没有有效性、对齐、生命周期保证 |
| 调用 `unsafe fn` | 前置条件写在文档里，由调用者自己保证 |
| 读写可变 `static` | 没有同步机制，可能直接制造数据竞争 |
| 访问 `union` 字段 | 编译器不知道当前哪个变体是有效的 |
| 调用 `extern` 外部函数 | 对面的行为不在 Rust 的检查范围里 |

反过来，`unsafe` **不会**关闭借用检查、类型检查、生命周期检查，也不会让
`Option` 自动变成空指针。写 `unsafe` 时常见的错误是「以为它能糊弄编译器」：
它做不到，它只是把某些检查从编译器挪到了你身上。

## 25.2 裸指针的三条硬规则

引用保证了有效性、对齐和生命周期；裸指针一条都不保证。创建裸指针通常是安全的，
解引用必须放进 `unsafe`：

```rust
let value = 42;
let pointer = &value as *const i32;         // 创建：安全
let copied = unsafe { *pointer };           // 解引用：unsafe
```

解引用之前必须自己证明三件事：

1. **有效性**：指针来自仍然存活的对象，不是空指针、不是已释放的地址；
2. **对齐与初始化**：地址满足 `T` 的对齐要求，且已经初始化为合法的 `T`；
3. **别名与可变性**：通过 `*mut T` 写入时不能与已有引用冲突，也不能制造数据竞争。

本章的 `read_at` 把「越界」这件事挡在 `unsafe` 之外，于是只剩一行危险操作：

```rust
pub fn read_at(values: &[i32], index: usize) -> Option<i32> {
    let pointer = values.as_ptr();
    if index >= values.len() {
        return None;
    }
    // SAFETY: 上面的长度检查保证 index < values.len()，切片在调用期间一直存活，
    // 因此 `add(index)` 仍在同一块分配内，且该地址已初始化为合法的 i32。
    Some(unsafe { *pointer.add(index) })
}
```

实测输出（`cargo run` 的第 2 节）：

```text
--- 2. 安全封装把越界挡在边界检查里 ---
    read_at([10, 20, 30], 0) = Some(10)
    read_at([10, 20, 30], 2) = Some(30)
    read_at([10, 20, 30], 3) = None（越界返回 None，而不是 UB）
```

`pointer.add(index)` **不是**边界检查，它只是地址算术；越界 `add` 加上解引用就是
未定义行为（UB）。对应的四个边界用例都写进了单元测试：

```rust
#[test]
fn read_at_covers_every_boundary() {
    let values = [10, 20, 30];
    assert_eq!(read_at(&values, 0), Some(10)); // 第一个合法索引
    assert_eq!(read_at(&values, 2), Some(30)); // 最后一个合法索引
    assert_eq!(read_at(&values, 3), None);     // 正好越界
    assert_eq!(read_at(&[], 0), None);         // 空切片
}
```

> 为什么 `read_at` 明明可以用 `values.get(index)`？因为它要演示的是**封装**：
> 真实项目里出现 unsafe，通常是因为要调用外部库，或者性能测量证明边界检查
> 确实是热点。能用安全 API 表达时，就别写 unsafe。

## 25.3 把 unsafe 封在最小范围里

`first_or_zero` 只在确认切片非空后调用 `get_unchecked(0)`，调用者只面对安全签名：

```rust
pub fn first_or_zero(bytes: &[u8]) -> u8 {
    if bytes.is_empty() {
        0
    } else {
        // SAFETY: 上面的分支保证长度至少为 1，索引 0 必然在切片范围内。
        unsafe { *bytes.get_unchecked(0) }
    }
}
```

实测行为：`first_or_zero([]) = 0`、`first_or_zero([10, 20, 30]) = 10`。

三条纪律，写 unsafe 时反复对照：

1. **每个 `unsafe` 块前写 `SAFETY` 注释**，说明不变量由哪段代码保证。评审的人
   应该能顺着注释验证，而不是靠对作者的信任；
2. **危险操作只留一行**：能提前检查的检查完再进 `unsafe`，让审查范围最小；
3. **对外只暴露安全 API**：`unsafe fn` 只在「前置条件无法在函数内部检查」时才是
   正确选择，例如接收外部指针的底层原语。

要求调用者满足前置条件的函数，用 `unsafe fn` 加 `# Safety` 文档表达：

```rust
/// # Safety
///
/// `ptr` 必须非空、对齐，并指向一个已初始化的 `u32`。
unsafe fn read_raw(ptr: *const u32) -> u32 {
    // SAFETY: 由调用者保证上面文档里的前置条件。
    unsafe { *ptr }
}
```

Rust 2024 要求即使在 `unsafe fn` 内部，也要用显式 `unsafe { ... }` 包住危险操作，
这样「哪些行真正危险」在代码里一眼可见。

常见 UB 来源：越界访问、释放后使用、`transmute` 转型错误、未对齐读取、
违反 `Send`/`Sync` 约束、跨线程同时读写。UB 不一定立刻崩溃——优化之后可能
表现为完全不可预测的结果，这也是它比「崩溃」可怕的地方。

## 25.4 结构体布局：`#[repr(C)]` 和写进类型里的生命周期

Rust 默认的 `struct` 布局**不保证字段顺序**，编译器可以重排字段以节省空间，
所以它不能直接当 C 结构体传。跨语言的结构体必须用 `#[repr(C)]`：

```rust
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct BufferView<'a> {
    pub data: *const u8,
    pub len: usize,
    borrow: PhantomData<&'a [u8]>,   // 零大小字段：C 看不到它
}
```

这个结构体演示了三个要点：

- **布局**：两个字段的偏移与 C 的 `struct { const uint8_t *data; size_t len; }`
  一致。实测（x86_64）：

```text
--- 3. #[repr(C)] 结构体 + 长度一起传 ---
    size_of::<BufferView>() = 16，offset_of!(len) = 8
    view.len = 3，view.as_slice() = [1, 2, 3]
    空视图的 as_slice() = []
```

- **长度必须单独传**：C 不知道 Rust 切片的长度，`len` 不是可选项，而是契约的一部分；
- **生命周期写进类型**：`PhantomData<&'a [u8]>` 让视图不能比它借用的切片活得更久。
  正因为这个约束存在，`as_slice` 才可以是安全函数：

```rust
pub fn as_slice(&self) -> &'a [u8] {
    if self.len == 0 {
        return &[];                     // 空视图根本不读指针
    }
    // SAFETY: data/len 只能由 from_slice 从 &'a [u8] 生成（字段私有），
    // 因此这段内存在 'a 内有效、已初始化，长度就是 len。
    unsafe { std::slice::from_raw_parts(self.data, self.len) }
}
```

字段 `borrow` 是私有的，外部无法构造出「指针和长度对不上」的视图——**让非法状态
无法构造**，比事后检查更省事（第 6 章「用类型表达约束」在 FFI 里的应用）。

布局还有一个更容易踩的同类问题：**不要跨语言传 `String`、`Vec`、`&str` 或 trait
对象**。它们的内部结构、分配器和所有权规则都是 Rust 私有的，C 侧既不能读也不能
正确销毁。跨边界只传 C 兼容类型：

| Rust 参数 | C 侧形式 | 约定 |
| --- | --- | --- |
| `c_int`、`c_uint` 等 | `int`、`unsigned int` | 用 `std::ffi::c_*`，不要猜平台宽度 |
| `*const T` + `usize` | `const T*` + `size_t` | 长度单独传，调用期间有效 |
| `*mut T` + `usize` | `T*` + `size_t` | 明确谁写、谁拥有、谁释放 |
| `*const c_char` | `const char*` | 必须 NUL 结尾；编码不保证是 UTF-8 |
| `#[repr(C)] struct` | `struct` | 字段顺序和对齐固定 |
| `Option<NonNull<T>>` | 可空指针 | 只有这种「有 ABI 保证」的类型才有空指针语义 |

## 25.5 字符串与切片的边界

`CString` 用来生成 NUL 结尾的 C 字符串，`CStr` 用来借用 C 传进来的字符串。
实测输出：

```text
--- 4. C 字符串：NUL 结尾与编码 ---
    to_c_string("中文 rust") 的字节数 = Some(11)（汉字各占 3 字节）
    to_c_string("a\0b") 是否失败 = true
    borrowed_byte_len(NULL) = None
```

三条不能想当然的规则：

- **内部 NUL 必须报错**，不能截断。`CString::new("a\0b")` 返回 `Err(NulError)`，
  因为 C 侧只认第一个 NUL，悄悄截断会让两边看到不同的字符串；
- **长度是字节数，不是字符数**。`"中文 rust"` 是 11 个字节（两个汉字各 3 字节、
  空格 1 字节、`rust` 4 字节），如果按字符数算就会传错长度；
- **C 返回的字符串只能借用**：`CStr::from_ptr` 不取得所有权，不要 `free` 它，
  也不要假设它是 UTF-8；要转成 `String` 就用 `to_string_lossy` 或显式处理
  非法编码。

```rust
pub fn borrowed_byte_len(pointer: *const c_char) -> Option<usize> {
    if pointer.is_null() {
        return None;
    }
    // SAFETY: 空指针是唯一能在函数内部检查的前置条件，这里已经排除；
    // 非空时由调用者保证它指向 NUL 结尾的有效字符串，且调用期间存活。
    Some(unsafe { CStr::from_ptr(pointer) }.to_bytes().len())
}
```

切片同理：`slice.as_ptr()` 配 `slice.len()`，并且要保证**调用期间**切片一直存活、
没有并发可变访问。谁分配谁释放——不要用 Rust 的 `Box::from_raw` 去释放 C 用
`malloc` 拿到的内存，反过来也一样。

## 25.6 导出 C API：错误码、out 参数与调用约定

`src/lib.rs` 里导出了三个函数，它们的契约都写在文档注释里，也应当同步写进 C 头文件：

```rust
/// # Safety
///
/// `data` 为空指针时 `len` 必须是 0（否则返回 ERR_NULL，属于契约内的错误输入）；
/// `data` 非空时必须指向至少 `len` 个已初始化的可读字节，且本次调用期间有效。
///
/// 契约：
/// - `data.is_null()` 时只有 `len == 0` 合法，此时返回 0；
/// - `len > MAX_BYTES` 时返回 ERR_TOO_LONG；
/// - `data` 非空时由调用者保证它起 `len` 个字节可读，且本次调用期间有效；
/// - 本函数不保存指针，也不释放任何内存。
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
```

注意它是 **`unsafe` 函数**：空指针和长度上限能在函数内部检查，但「这块内存真的
可读」没法在函数内部证明——只要参数是裸指针，可读性就只能由调用者保证。
`clippy` 会用 `not_unsafe_ptr_arg_deref` 提示这一点：公开函数解引用裸指针却没标
`unsafe`，等于把一段事实上不安全的契约伪装成安全 API。所以 Rust 侧调用它时
同样要写 `unsafe` 块和 `SAFETY` 注释：

```rust
// SAFETY: payload 是本地数组，长度 3、调用期间有效，满足契约。
let sum = unsafe { sum_bytes(payload.as_ptr(), payload.len()) };
```

`cargo run` 的第 5 节实测输出（本模块通过 `rust_learn::…` 复用了库目标里的导出函数）：

```text
--- 5. 调用导出的 C API（同一个包里的库目标） ---
    multiply(6, 7) = 42
    sum_bytes(payload) = 6
    checksum(payload) = 6（1+2+3 = 6，与 sum_bytes 相同）
    sum_bytes(NULL, 0) = 0（空指针只在长度为 0 时合法）
    sum_bytes(NULL, 4) = -1（就是 ERR_NULL = -1，不崩溃）
    sum_bytes(payload, MAX_BYTES + 1) = -2（就是 ERR_TOO_LONG = -2，上限 1048576 字节）
```

这里有几个约定值得抄进自己的项目：

| 约定 | 为什么 |
| --- | --- |
| 空指针语义写进文档：`NULL + 0` 合法，`NULL + 非零` 返回错误码 | 否则「空切片」这种正常输入没有合法表达 |
| 长度有上限（本章 1 MiB） | C 的 `size_t` 可以传进荒唐的长度，Rust 侧必须先挡住 |
| 错误用**负的错误码**而不是 panic | panic 穿过 `extern "C"` 边界会直接 abort（见 25.7） |
| `extern "C"` 显式写在函数上 | 调用约定决定参数怎么传、返回值怎么取，不能省略 |
| 返回 `u32` 校验和时说明「0 同时表示空输入和参数非法」 | 有歧义就要在契约里写清楚，或改用错误码 + out 参数 |

需要同时返回「结果」和「状态」时，用 **out 参数**：

```rust
/// 错误码：结果指针为空 / 输入指针为空 / 长度超限。
const ERR_CODE_POINTER: u32 = 1;
const ERR_CODE_NULL: u32 = 2;
const ERR_CODE_TOO_LONG: u32 = 3;

/// 返回 0 表示成功，此时 `*written` 是实际处理长度；非 0 是错误码。
#[unsafe(no_mangle)]
pub extern "C" fn checksum_into(data: *const u8, len: usize, written: *mut usize) -> u32 {
    if written.is_null() {
        return ERR_CODE_POINTER;
    }
    // SAFETY: 先检查空指针，再由调用者保证 written 指向可写的 usize。
    unsafe { *written = 0 };
    // ... 写入校验和与长度 ...
}
```

out 参数必须**先检查空指针再写**，否则一次误传就会踩到 UB；这也是为什么示例里
第一个分支就是 `written.is_null()`。错误码常量与返回值的含义要一起写进头文件。

### `unsafe extern` 声明与导出

调用 C 函数时声明外部符号，Rust 2024 要求整个块写成 `unsafe extern "C"`：

```rust
unsafe extern "C" {
    fn c_function(input: *const std::ffi::c_char) -> i32;
}
```

导出则用 `#[unsafe(no_mangle)] pub extern "C" fn ...`。声明只描述调用约定，
**不会帮你找到或编译库**——库从哪来是构建问题，见 25.8。

## 25.7 panic 不能穿过 FFI 边界

Rust 1.81 起，panic 一旦逃出 `extern "C"` 函数，默认行为是直接 abort。原因是
对面的语言不知道什么叫「展开栈」，强行穿过去只会让两边都处于未定义状态。
正确做法是在 Rust 内部拦住它：

```rust
fn divide_strict(left: c_int, right: c_int) -> c_int {
    assert!(right != 0, "除数不能为 0");   // 除数为 0 时 panic
    left / right
}

#[unsafe(no_mangle)]
pub extern "C" fn safe_divide(left: c_int, right: c_int) -> c_int {
    match catch_unwind(|| divide_strict(left, right)) {
        Ok(value) => value,
        Err(_) => ERR_DIVIDE,             // 换成错误码返回
    }
}
```

实测输出：

```text
--- 6. panic 不穿过 FFI 边界 ---
    safe_divide(10, 2) = 5
    safe_divide(1, 0) = -3（就是 ERR_DIVIDE = -3，进程继续运行）
    未包装的实现会 panic：除数不能为 0
    （panic 若穿过 extern "C" 边界，进程默认直接 abort；所以在 Rust 里拦住）
```

第三行是演示故意调用「没包装」的实现并捕获载荷得到的：`catch_unwind` 返回
`Err(payload)`，把载荷 `downcast` 一下就能拿到消息文本。演示里还临时换掉了
全局 panic hook 让输出保持干净：

```rust
fn quiet<T>(body: impl FnOnce() -> T) -> std::thread::Result<T> {
    let previous = take_hook();
    set_hook(Box::new(|_| {}));
    let outcome = catch_unwind(AssertUnwindSafe(body));
    set_hook(previous);
    outcome
}
```

**真实项目不要动全局 hook**：panic 信息正是日志和监控要看的信号。这里换掉它
只是为了让本章的演示输出可读，函数的文档注释里也写清楚了这一点。

还要注意 profile 的影响：如果按第 24 章 24.5 那样设置了
`panic = "abort"`，`catch_unwind` 就拦不住任何东西了——进程会直接结束。
「用错误码代替 panic」和「用 abort 减小产物体积」是互相冲突的两个选择，
交叉编译到 C 侧集成时尤其要先确定这一点。

## 25.8 从 Rust 调用 C：一个完整的最小流程

假设 C 库提供一个加法函数。头文件和实现：

```c
/* examples/ffi/math.h */
#ifndef RUST_LEARN_MATH_H
#define RUST_LEARN_MATH_H

int add_ints(int left, int right);

#endif
```

```c
/* examples/ffi/math.c */
#include "math.h"

int add_ints(int left, int right) {
    return left + right;
}
```

调用方 `examples/ffi/caller.c` 就是普通的 C 程序：

```c
#include <stdio.h>
#include "math.h"

int main(void) {
    printf("%d\n", add_ints(20, 22));
    return 0;
}
```

**实测**（本机装了 msys2 的 gcc，`cc` 指向它）：

```text
$ cc examples/ffi/math.c examples/ffi/caller.c -o %TEMP%\ffi-caller.exe
$ %TEMP%\ffi-caller.exe
42
```

Linux/macOS 上的命令完全一样，把输出路径换掉即可；Windows 用 MSVC 时是
`cl examples/ffi/math.c examples/ffi/caller.c`。目录里的 `Makefile` 把这三步
包成 `make` / `make run` / `make clean`（Windows 的 MinGW 环境通常用
`mingw32-make`）：

```bash
cd examples/ffi
make        # 编译 caller
make run    # 编译并运行，输出 42
make clean  # 删除本地 C 产物
```

### Rust 侧编译并链接这个 C 文件

用 `cc` crate 在构建期编译 C 源码（`Cargo.toml` 增加
`[build-dependencies] cc = "1"`）：

```rust
// build.rs
fn main() {
    cc::Build::new()
        .file("native/math.c")
        .include("native")
        .compile("native_math");
}
```

```rust
unsafe extern "C" {
    fn add_ints(left: std::ffi::c_int, right: std::ffi::c_int) -> std::ffi::c_int;
}

fn add(left: i32, right: i32) -> i32 {
    // SAFETY: build.rs 保证链接了 add_ints；i32 与 C int 的 ABI 由接口固定。
    unsafe { add_ints(left, right) as i32 }
}
```

链接已经预编译好的系统库时，在 `build.rs` 里打印链接参数：

```rust
println!("cargo:rustc-link-search=native=/opt/mylib/lib");
println!("cargo:rustc-link-lib=static=mylib");
```

跨平台项目要根据 `TARGET` 选择 `.a`、`.lib` 还是 `.so`/`.dll`，并在 CI 里装好
对应编译器。动态链接还要考虑运行时搜索路径：发布包不能只在开发机上找得到
那个动态库。

> 本仓库**没有**引入 `cc` crate（这会新增构建依赖和 `Cargo.lock` 变更），
> 所以上面这两段代码是用法示例，未在本仓库执行。本章实际验证过的 C 侧流程是
> 上一小节那段 `cc` 命令。

## 25.9 用 Python `ctypes` 调用 Rust

先构建动态库，再运行脚本（两步都实测过）：

```bash
cargo build --release --lib
python examples/ffi/caller.py
```

```text
multiply(6, 7) = 42
sum_bytes(b'rust') = 462
checksum(b'rust') = 462
sum_bytes(NULL, 4) = -1
```

脚本的关键部分：

```python
import ctypes, platform
from pathlib import Path

root = Path(__file__).resolve().parents[2]
if platform.system() == "Windows":
    library = root / "target" / "release" / "rust_learn.dll"
elif platform.system() == "Darwin":
    library = root / "target" / "release" / "librust_learn.dylib"
else:
    library = root / "target" / "release" / "librust_learn.so"

lib = ctypes.CDLL(str(library))
lib.multiply.argtypes = [ctypes.c_int, ctypes.c_int]
lib.multiply.restype = ctypes.c_int
print(lib.multiply(6, 7))            # 42
```

三个必须遵守的细节：

- **`argtypes` / `restype` 不能省**。省略时 `ctypes` 按默认规则猜，返回值宽度
  错了会读到垃圾数据，指针被当整数传更是直接崩；
- **缓冲区必须活到调用返回**。`(ctypes.c_ubyte * n).from_buffer_copy(payload)`
  必须绑定到变量；如果 Rust 需要异步保存数据，只能自己复制一份并另配释放函数；
- **路径按平台选**。`target/release/` 下的产物名在不同系统不一样
  （`.dll` / `.so` / `.dylib`），脚本里用 `platform.system()` 分支处理。

`ctypes` 适合少量稳定的 C ABI 函数；复杂对象、异常映射和高频调用应该用
`pyo3` + `maturin` 这类框架，由框架负责 Python 对象转换。无论哪种方式，
都要避免让 Rust panic 穿过 Python 边界，并为每个导出函数固定 ABI 与错误协议。

## 25.10 用 Go 调用 Rust

Go 侧同样可以按 C ABI 调用动态库。本仓库的 `examples/ffi/caller.go` 只用标准库
的 `syscall.NewLazyDLL`，在 Windows 上加载 DLL 并取过程地址，**不需要 cgo**：

```go
dll := syscall.NewLazyDLL("target/release/rust_learn.dll")

multiply := dll.NewProc("multiply")
product, _, _ := multiply.Call(6, 7)
fmt.Println("multiply(6, 7) =", int32(product))

sumBytes := dll.NewProc("sum_bytes")
payload := []byte("rust")
sum, _, _ := sumBytes.Call(uintptr(unsafe.Pointer(&payload[0])), uintptr(len(payload)))
```

实测输出：

```text
$ go run examples/ffi/caller.go
multiply(6, 7) = 42
sum_bytes(b"rust") = 462
sum_bytes(NULL, 4) = -1
```

Go 这边和 Python 的注意点完全一致：过程名必须和 `#[unsafe(no_mangle)]` 的符号
名一致，参数按 C ABI 的宽度传，切片要用 `&payload[0]` 取地址并单独传长度，
并且保证 Go 的切片在调用期间不被 GC 回收（`Call` 期间切片仍在作用域内即可）。

在 Linux/macOS 上更常见的写法是 cgo：

<!-- staged-check-disable commented-code -- cgo 的语法就是把 C 声明写在块注释里，这里是文档示例，不是被注释掉的代码 -->

```go
/*
#cgo LDFLAGS: -L${SRCDIR}/../../target/release -lrust_learn
#include <stdint.h>
int32_t multiply(int32_t left, int32_t right);
*/
import "C"
```

<!-- staged-check-enable commented-code -->

cgo 需要 C 编译器和能匹配的动态库命名（`lib*.so` 会被自动找到，Windows 上通常
要额外提供导入库），所以本仓库选择用 `syscall` 而不是 cgo。

其他语言也是同一套逻辑。以 C# 的 P/Invoke 为例（**本机没有 .NET SDK，实测
`dotnet` 未安装，这段片段没有编译验证**）：

```csharp
using System.Runtime.InteropServices;

internal static partial class Native
{
    [LibraryImport("rust_learn.dll")]
    internal static partial int Multiply(int left, int right);

    [LibraryImport("rust_learn.dll")]
    internal static partial int SumBytes(byte[] data, nuint len);
}
```

无论哪种语言，契约都一样：**过程名、参数宽度、指针与长度、编码、返回码、
线程安全和释放责任**，一条都不能少。

## 25.11 线程安全、回调与释放责任

FFI 项目最容易出事的不是语法，而是生命周期。审查时逐个回答这些问题：

| 问题 | 典型错误 | 正确做法 |
| --- | --- | --- |
| 这块内存谁拥有？ | Rust 用 `Box::from_raw` 释放 C `malloc` 的内存 | 谁分配谁释放，两边用同一个分配器 |
| 指针能活多久？ | 把临时 `CString` 的指针存进 C 结构体 | 保存指针就必须让所有者活得更久，或改成复制 |
| 会不会跨线程？ | 假设「Rust 类型是 `Send`，所以 C 库也能多线程用」 | 查 C 库文档，句柄/全局状态要自己加锁或限定线程 |
| 回调什么时候注销？ | C 侧还在回调，Rust 侧已经把资源 drop 了 | 提供成对的注册/注销 API，注销后再释放用户数据 |
| 错误怎么回去？ | 让 panic 或 C++ 异常穿过边界 | 错误码、out 参数或 `Result` 包装 |

回调必须用 `extern "C" fn`，用户数据通过 `*mut c_void` 传进去：

```rust
type Callback = extern "C" fn(user_data: *mut std::ffi::c_void, value: c_int);

unsafe extern "C" {
    fn register_callback(callback: Callback, user_data: *mut std::ffi::c_void);
    fn unregister_callback();
}
```

「注销之后再释放用户数据」是回调 API 的黄金顺序，写反了就是释放后使用。

## 25.12 把 unsafe 收进一个可审查的模块

推荐的目录结构：公开 API 保持安全，危险代码集中且最小：

```text
src/ffi/
├── mod.rs          # 安全公开 API，只做参数校验与错误转换
├── raw.rs          # 最小化 unsafe 与 extern 声明
└── bindings.rs     # bindgen 生成，不手工编辑
```

`bindgen` 可以根据 C 头文件生成声明，生成文件要固定头文件版本、单独放一个模块，
并对外只暴露校验过的安全包装。

本章代码对应的审查清单（`cargo run` 的第 7 节也会打印）：

- 每个 `unsafe` 块前是否有能验证的 `SAFETY` 不变量？
- `unsafe` 范围是否最小？能提前检查的是否都检查完了？
- 空指针、长度上限、编码、调用约定、结构体布局是否写进契约？
- 所有权与释放责任是否明确？分配和释放是否用同一个分配器？
- panic / 异常是否被拦住？错误码是否覆盖所有错误路径？
- 线程安全与回调注销规则是否写清楚？

## 25.13 验证与工具

```bash
cargo fmt --check
cargo test --all-features --locked
cargo clippy --all-targets --all-features -- -D warnings
```

> 三条命令在本仓库都是通过的（整仓 `clippy -D warnings` 曾经因为第 6–23 章的
> 历史 lint 失败，后来逐条处理干净了）。对 FFI 代码来说 `clippy` 尤其值得跑：
> 它会提示「公开函数解引用裸指针却没标 `unsafe`」这类契约问题——本章的
> `sum_bytes` / `checksum` 就是因为这条提示才改成 `unsafe extern "C" fn` 的。

本仓库实测的相关测试（`cargo test` 输出节选）：

```text
$ cargo test --lib
running 2 tests
test tests::multiply_saturates_instead_of_panicking ... ok
test tests::sum_and_checksum_follow_the_null_and_length_contract ... ok
test result: ok. 2 passed; 0 failed; ...

$ cargo test --bin rust-learn-demo rust25_unsafe_ffi
running 6 tests
test rust25_unsafe_ffi::tests::buffer_view_roundtrips_and_keeps_c_layout ... ok
test rust25_unsafe_ffi::tests::c_strings_reject_interior_nul_and_count_bytes ... ok
test rust25_unsafe_ffi::tests::exported_api_follows_the_null_and_length_contract ... ok
test rust25_unsafe_ffi::tests::panic_is_converted_to_an_error_code ... ok
test rust25_unsafe_ffi::tests::read_at_covers_every_boundary ... ok
test rust25_unsafe_ffi::tests::safe_wrapper_handles_empty_and_non_empty_slices ... ok
test result: ok. 6 passed; 0 failed; ...
```

测试覆盖的正是契约里的每一条边界：空切片、最后一个合法索引、越界、
`NULL + 0`、`NULL + 非零`、长度超限、内部 NUL、非法 UTF-8 长度，
以及「panic 被换成错误码」这条路径。

再往前走一步可以用 Miri 检查未定义行为（需要 nightly）：

```bash
cargo +nightly miri test
```

Miri 能发现越界、悬垂指针、未对齐访问和一部分别名违规，但它**不能**验证真实
C 库的行为，也替代不了普通测试。本仓库当前工具链是 stable，未执行这一步。

## 25.14 常见坑速查

| 现象 | 原因 | 解法 |
| --- | --- | --- |
| 调用 C 函数时编译报错 | Rust 2024 要求 `unsafe extern "C" { ... }` | 给声明块加上 `unsafe` |
| 运行时找不到符号 | 忘了 `#[unsafe(no_mangle)]`，符号被改名 | 导出函数加上该属性，或对绑定用 `#[link_name]` |
| C 侧读到的结构体字段错位 | 用了默认布局的 `struct` | 加 `#[repr(C)]`，并核对字段类型宽度 |
| 传字符串过去被截断 | 内部有 NUL，或没传 NUL 结尾的 `CString` | 用 `CString::new` 检查，失败就返回错误 |
| C 侧收到乱码 | 假设 C 字符串是 UTF-8 | 明确编码，必要时显式转换并返回错误 |
| 进程直接 abort，没有错误信息 | panic 穿过了 `extern "C"` 边界 | 在导出函数里 `catch_unwind`，转成错误码 |
| `catch_unwind` 突然不生效 | profile 里设了 `panic = "abort"` | 要么去掉 abort，要么全程用错误码 |
| Python 侧读到垃圾值或崩溃 | 省略了 `argtypes` / `restype` | 显式声明每个参数和返回值类型 |
| 传递的缓冲区偶尔变成乱码 | Python/Go 侧缓冲区提前被回收 | 让缓冲区活到调用返回；Rust 需要留存就复制 |
| Linux 上找不到动态库 | 只设了编译期路径，没有运行时搜索路径 | 设置 `LD_LIBRARY_PATH` / `rpath`，或安装到系统路径 |
| `cargo clippy` 报不必要 unsafe | 有安全 API 可以表达同样的逻辑 | 换成安全写法，把 unsafe 留给真正需要的场景 |

## 25.15 练习

1. 为 `read_at` 增加一个 `read_at_mut`（返回 `Option<&mut i32>`），要求不写
   `unsafe` 就能实现，并解释为什么这次不需要。
2. 给导出 API 加一个 `sum_view(view: BufferView) -> c_int`：按 `#[repr(C)]`
   结构体传参，验证它和 `sum_bytes(view.data, view.len)` 结果一致。
3. 写一个 `checksum_into(data, len, written) -> u32` 的完整实现（含 out 参数），
   并为「`written` 为空指针」「`data` 为空但长度非零」「正常输入」各写一个测试。
4. 把 25.11 的回调示例写成可运行的小程序：Rust 侧提供 `register/unregister`，
   用 `*mut c_void` 传用户数据，并解释为什么注销必须在释放之前。
5. 用 `examples/ffi/caller.py` 的写法给 `multiply` 传一个超出 `c_int` 范围的值，
   观察 `ctypes` 的行为，再解释为什么 Rust 侧要用 `saturating_mul`。

（第 1、5 题是 25.2 与 25.6 讨论过的内容，这里不再单独给答案。）

### 参考答案

::: details 第 2 题

```rust
/// # Safety
///
/// 与 `sum_bytes` 相同：视图指向的内存必须至少 `view.len` 字节可读，
/// 且在本次调用期间有效。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sum_view(view: BufferView<'_>) -> c_int {
    // SAFETY: 可读性由本函数的调用者按上面的契约保证。
    unsafe { sum_bytes(view.data, view.len) }
}
```

`BufferView` 是 `#[repr(C)]` 且只含 C 兼容字段，因此可以按值跨边界传递；
`PhantomData` 是零大小字段，不影响布局。实测它与 `sum_bytes` 结果一致
（`[1, 2, 3]` 都是 6）。注意契约不变：**视图指向的内存仍由调用方保证有效**，
按值传结构体并不会让数据跟着复制过去——所以它也得是 `unsafe fn`。

:::

::: details 第 3 题

```rust
// 错误码常量沿用 25.6 的定义。
/// 返回 0 表示成功，`*written` 是实际处理的字节数；非 0 是错误码。
///
/// # Safety
///
/// `written` 非空时必须指向可写的 `usize`；`data` 非空时必须指向至少 `len` 个
/// 可读字节，且本次调用期间有效。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn checksum_into(data: *const u8, len: usize, written: *mut usize) -> u32 {
    if written.is_null() {
        return ERR_CODE_POINTER;
    }
    // SAFETY: 空指针已经排除，调用者保证 written 指向可写的 usize。
    unsafe { *written = 0 };

    if data.is_null() {
        return if len == 0 { 0 } else { ERR_CODE_NULL };
    }
    if len > MAX_BYTES {
        return ERR_CODE_TOO_LONG;
    }
    // SAFETY: 指针非空、长度受限，可读性由调用者保证。
    let bytes = unsafe { std::slice::from_raw_parts(data, len) };
    // SAFETY: 同上；调用期间没有其他引用指向这块内存。
    unsafe { *written = len };
    bytes.iter().map(|&byte| u32::from(byte)).sum()
}

#[test]
fn out_parameter_reports_written_length() {
    let bytes = [1_u8, 2, 3];
    let mut written = 0usize;
    // SAFETY: bytes 与 written 都是本地变量，在调用期间有效。
    unsafe {
        let sum = checksum_into(bytes.as_ptr(), bytes.len(), &mut written);
        assert_eq!((sum, written), (6, 3));

        assert_eq!(
            checksum_into(bytes.as_ptr(), bytes.len(), std::ptr::null_mut()),
            ERR_CODE_POINTER
        );
        assert_eq!(checksum_into(std::ptr::null(), 4, &mut written), ERR_CODE_NULL);
    }
}
```

要点有两个：**先把 `written` 清零**（错误路径也让调用方拿到确定的值），
以及**每个 `unsafe` 块各自说明不变量**，而不是在函数开头写一句笼统的注释。

:::

::: details 第 4 题

```rust
use std::ffi::c_void;

type Callback = extern "C" fn(user_data: *mut c_void, value: c_int);

/// 保存回调，直到 `unregister_callback` 被调用。
struct Registry {
    callback: Option<Callback>,
    user_data: *mut c_void,
}

impl Registry {
    fn register(&mut self, callback: Callback, user_data: *mut c_void) {
        self.callback = Some(callback);
        self.user_data = user_data;
    }

    /// 必须先清空回调，再让调用方释放 user_data。
    fn unregister(&mut self) {
        self.callback = None;
        self.user_data = std::ptr::null_mut();
    }

    fn fire(&self, value: c_int) {
        if let Some(callback) = self.callback {
            // SAFETY: user_data 由注册方保证在注销前一直有效。
            callback(self.user_data, value);
        }
    }
}
```

顺序不能反：**注销在前、释放在后**。反过来的话，C 侧可能拿着已经释放的
`user_data` 再回调一次，那就是典型的释放后使用（use after free），
而且往往只在压力测试里偶发。

:::

## 25.16 小结

- `unsafe` 只打开五件事（解引用裸指针、调用 `unsafe fn`、访问可变 `static`、
  访问 `union` 字段、调用外部函数），借用检查、类型检查和生命周期检查照旧。
- 裸指针解引用前必须证明有效性、对齐与初始化、别名与可变性三条；
  `add` / `offset` 不是边界检查。
- 把 `unsafe` 缩到最小并用安全函数包起来，每个块写 `SAFETY` 注释；
  能用安全 API 表达就別写 unsafe。
- FFI 只传 C 兼容类型：`c_*`、指针 + 长度、`#[repr(C)]` 结构体、
  `CString` / `CStr`；永远不要传 `String`、`Vec`、`&str`、trait 对象。
- 用 `PhantomData` 把生命周期写进结构体，可以让 `as_slice` 这样的函数保持安全；
  字段私有能防止构造出非法状态。
- 空指针语义、长度上限、编码、所有权与释放责任、线程安全、回调注销，
  这些都要写进契约——C 头文件里也要写一份。
- panic 不能穿过 `extern "C"`（默认 abort）；用 `catch_unwind` 或错误码 + out
  参数把它拦在 Rust 里，并注意 `panic = "abort"` 会让 `catch_unwind` 失效。
- 调用方向也成立：C 用 `cc` + `build.rs` 编译链接，Python 用 `ctypes`，
  Go 用 `syscall.NewLazyDLL`，三者的契约完全一样。
- 本仓库实测链路：`cargo build --release --lib` 产出 `rust_learn.dll`，
  `python examples/ffi/caller.py` 与 `go run examples/ffi/caller.go` 都能拿到
  `multiply = 42`、`sum_bytes = 462`、`sum_bytes(NULL, 4) = -1`。

下一章是**第 26 章 数据库操作**：用 `rusqlite` 连接 SQLite，做建表迁移、
参数化 CRUD、事务与回滚、类型映射、查询计划和测试隔离。
