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
lib.multiply.argtypes = [ctypes.c_int, ctypes.c_int]
lib.multiply.restype = ctypes.c_int

print(lib.multiply(6, 7))
