# 学习路线与章节规划

这套笔记按「先看得见、再讲原理」的思路组织：每一章有一个明确的主题，配一份可以直接 `cargo run` 的代码。

代码放在练习工程的 `src/` 下，按 `rustNN_主题` 命名。例如第 1 章的模块就是 `src/rust01_print/mod.rs`，在 `src/main.rs` 里调用。

## 阅读建议

1. 先读章节内容，把例子敲一遍，再看输出和你预期是否一致。
2. 每章末尾有练习，先自己做，再展开答案对照。
3. 卡在编译错误上时，先读错误信息里的 `help:` 那几行，Rust 编译器的提示质量很高。

## 章节规划

| 章节 | 主题 | 你会学到 | 配套代码 |
| --- | --- | --- | --- |
| 第 1 章 | [打印输出](./rust01-print) | `print!` 家族、格式说明符、`Debug` 与 `Display`、stdout/stderr 与缓冲 | `src/rust01_print/` |
| 第 2 章 | [变量与基本类型](./rust02-variables) | 不可变绑定与遮蔽、整数与浮点、溢出处理、`char` 与字符串、元组数组切片、类型转换 | `src/rust02_variables/` |
| 第 3 章 | [函数与表达式](./rust03-functions) | 函数签名、尾表达式、表达式与语句的区别、提前返回、函数指针、`const fn`、报错读法 | `src/rust03_functions/` |
| 第 4 章 | [控制流](./rust04-control-flow) | `if` / `loop` / `while` / `for`、带返回值的 `loop`、循环标签 | `src/rust04_control_flow/` |
| 第 5 章 | 所有权与借用 | 移动、克隆、借用规则、切片、`String` 与 `&str` 的取舍 | 规划中 |
| 第 6 章 | 结构体与枚举 | `struct`、`enum`、`impl`、`match` 与 `Option` | 规划中 |
| 第 7 章 | 常用集合 | `Vec`、`String`、`HashMap` 的常用操作与坑 | 规划中 |
| 第 8 章 | 错误处理 | `Option` / `Result`、`?` 运算符、自定义错误、`panic!` 的边界 | 规划中 |
| 第 9 章 | 泛型与 trait | 泛型函数、trait 定义与实现、trait bound、trait 对象 | 规划中 |
| 第 10 章 | 生命周期 | 借用检查器如何推理、显式标注、`'static` 的真实含义 | 规划中 |
| 第 11 章 | 闭包与迭代器 | 三种闭包 trait、迭代器适配器、惰性求值 | 规划中 |
| 第 12 章 | 智能指针 | `Box`、`Rc`/`Arc`、`RefCell`、`Deref` 与内部可变性 | 规划中 |
| 第 13 章 | 并发编程 | `thread::spawn`、`mpsc` 通道、`Mutex` 与 `Arc`、`Send` 与 `Sync` | 规划中 |
| 第 14 章 | 模块、包与测试 | `mod`、`pub`、工作空间、单元测试与集成测试 | 规划中 |
| 第 15 章 | 宏 | `macro_rules!`、重复匹配、过程宏能做什么 | 规划中 |
| 第 16 章 | async/await | `Future` 模型、`.await`、运行时与阻塞陷阱 | 规划中 |

## 本地跑文档站

```bash
pnpm install
pnpm docs:dev     # 打开 http://localhost:5173
pnpm docs:build   # 构建静态站点到 docs/.vitepress/dist
```
