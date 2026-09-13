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
| 第 5 章 | [所有权与借用](./rust05-ownership) | 移动、克隆、借用规则、切片、`String` 与 `&str` 的取舍 | `src/rust05_ownership/` |
| 第 6 章 | [结构体与枚举](./rust06-structs-enums) | `struct`、`enum`、`impl`、`match` 与 `Option` | `src/rust06_structs_enums/` |
| 第 7 章 | [常用集合](./rust07-collections) | `Vec`、`String`、`HashMap` 的常用操作与坑 | `src/rust07_collections/` |
| 第 8 章 | [错误处理](./rust08-errors) | `Option` / `Result`、`?` 运算符、自定义错误、`panic!` 的边界 | `src/rust08_errors/` |
| 第 9 章 | [泛型与 trait](./rust09-generics-traits) | 泛型函数、trait 定义与实现、trait bound、trait 对象、关联类型 | `src/rust09_generics_traits/` |
| 第 10 章 | [生命周期](./rust10-lifetimes) | 借用检查器如何推理、显式标注、`'static` 的真实含义 | `src/rust10_lifetimes/` |
| 第 11 章 | [闭包与迭代器](./rust11-closures-iterators) | 三种闭包 trait、迭代器适配器、惰性求值、自定义迭代器 | `src/rust11_closures_iterators/` |
| 第 12 章 | [智能指针](./rust12-smart-pointers) | `Box`、`Rc`/`Arc`、`RefCell`、`Weak`、`Deref` 与内部可变性 | `src/rust12_smart_pointers/` |
| 第 13 章 | [并发编程](./rust13-concurrency) | `thread::spawn`、`mpsc` 通道、`Mutex` 与 `Arc`、`Send` 与 `Sync` | `src/rust13_concurrency/` |
| 第 14 章 | [模块、包与测试](./rust14-modules-tests) | `mod`、`pub`、工作空间、单元测试、集成测试与文档测试 | `src/rust14_modules_tests/`、`tests/` |
| 第 15 章 | [宏](./rust15-macros) | `macro_rules!`、片段说明符、重复匹配、卫生性、过程宏概览 | `src/rust15_macros/` |
| 第 16 章 | [async/await](./rust16-async) | `Future` 模型、`.await`、手写最小运行时、阻塞陷阱与 tokio 概览 | `src/rust16_async/` |
| 第 17 章 | 日期与时间 | `SystemTime` / `Instant` / `Duration`、时间戳与格式化、`chrono` / `time`、时区陷阱 | 规划中 |
| 第 18 章 | 文本处理与正则 | Unicode 与 UTF-8 进阶、字符串性能与 `Cow`、`regex` crate、文本清洗 | 规划中 |
| 第 19 章 | 文件、路径与 IO | `Path` / `PathBuf`、读写文件、`BufReader` / `BufWriter`、stdin/stdout、遍历目录 | 规划中 |
| 第 20 章 | 序列化与配置 | `serde`、`serde_json`、TOML、环境变量、配置合并 | 规划中 |
| 第 21 章 | 命令行工具 | `env::args`、`clap`、退出码、错误输出、日志（`tracing` / `env_logger`） | 规划中 |
| 第 22 章 | 网络与 HTTP | `TcpStream`、`reqwest`、超时与重试、和 async 的配合 | 规划中 |
| 第 23 章 | 测试进阶与基准 | 单元 / 集成 / 文档测试、断言与 mock、`criterion` 基准 | 规划中 |
| 第 24 章 | Cargo 深入与发布 | features、workspace、依赖管理、交叉编译、发布 crate 与二进制 | 规划中 |
| 第 25 章 | unsafe 与 FFI | 裸指针、`unsafe` 块与安全抽象、`extern "C"`、bindgen 概览 | 规划中 |
| 第 26 章 | 综合实战项目 | 写一个完整的 CLI 工具，把前面所有章节串起来 | 规划中 |

## 本地跑文档站

```bash
pnpm install
pnpm docs:dev     # 打开 http://localhost:5173
pnpm docs:build   # 构建静态站点到 docs/.vitepress/dist
```
