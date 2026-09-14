//go:build windows

// 用 Go 调用 Rust 编译出的 C ABI 动态库（第 25 章）。
//
// 先构建动态库，再从仓库根目录运行：
//
//	cargo build --release --lib
//	go run examples/ffi/caller.go
//
// 这里只用标准库的 syscall.NewLazyDLL：Windows 上加载 DLL 并取过程地址，
// 不需要 cgo，也就不需要额外的 C 工具链。
package main

import (
	"fmt"
	"syscall"
	"unsafe"
)

func main() {
	// 相对路径以当前工作目录为准：在仓库根目录运行时能直接找到。
	dll := syscall.NewLazyDLL("target/release/rust_learn.dll")

	multiply := dll.NewProc("multiply")
	product, _, _ := multiply.Call(6, 7)
	fmt.Println("multiply(6, 7) =", int32(product))

	// 指针 + 长度成对传递；Go 侧必须让切片活到调用返回。
	sumBytes := dll.NewProc("sum_bytes")
	payload := []byte("rust")
	sum, _, _ := sumBytes.Call(uintptr(unsafe.Pointer(&payload[0])), uintptr(len(payload)))
	fmt.Printf("sum_bytes(b\"rust\") = %d\n", int32(sum))

	// 错误码约定：NULL + 非零长度返回 -1，进程不会崩。
	null, _, _ := sumBytes.Call(0, 4)
	fmt.Println("sum_bytes(NULL, 4) =", int32(null))
}
