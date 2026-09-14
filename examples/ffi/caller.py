"""用 ctypes 调用 Rust 导出的 C ABI 动态库（第 25 章）。

先构建动态库，再从仓库根目录运行本脚本：

    cargo build --release --lib
    python examples/ffi/caller.py
"""

import ctypes
import platform
from pathlib import Path

root = Path(__file__).resolve().parents[2]
if platform.system() == "Windows":
    library = root / "target" / "release" / "rust_learn.dll"
elif platform.system() == "Darwin":
    library = root / "target" / "release" / "librust_learn.dylib"
else:
    library = root / "target" / "release" / "librust_learn.so"

lib = ctypes.CDLL(str(library))

# 1) 只传值类型：参数和返回值宽度必须显式声明，不能靠默认的 int 猜。
lib.multiply.argtypes = [ctypes.c_int, ctypes.c_int]
lib.multiply.restype = ctypes.c_int
print("multiply(6, 7) =", lib.multiply(6, 7))

# 2) 传字节缓冲区：指针 + 长度成对出现，buffer 必须活到调用返回。
payload = b"rust"
buffer = (ctypes.c_ubyte * len(payload)).from_buffer_copy(payload)

lib.sum_bytes.argtypes = [ctypes.POINTER(ctypes.c_ubyte), ctypes.c_size_t]
lib.sum_bytes.restype = ctypes.c_int
print("sum_bytes(b'rust') =", lib.sum_bytes(buffer, len(payload)))

lib.checksum.argtypes = [ctypes.POINTER(ctypes.c_ubyte), ctypes.c_size_t]
lib.checksum.restype = ctypes.c_uint32
print("checksum(b'rust') =", lib.checksum(buffer, len(payload)))

# 3) 错误码约定：NULL + 非零长度返回 -1，进程不会崩。
print("sum_bytes(NULL, 4) =", lib.sum_bytes(None, 4))
