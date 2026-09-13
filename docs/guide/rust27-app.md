# 第 27 章 · 综合实战：任务管理系统

本章把前面的知识串成一个最小系统：使用 Rust 所有权和结构体表达任务，SQLite 持久化，错误通过 `Result` 传播，CLI 层负责展示，日志和配置可在后续扩展。

当前演示在 `src/rust27_app.rs` 中使用内存数据库完成“添加—完成—列出”闭环；`run_task_flow` 返回 `Result<String>`，便于 CLI 层处理错误。生产版可将 `Connection::open_in_memory()` 替换为配置文件中的路径，并用 `clap` 增加 `add/list/done` 子命令。

建议的分层结构：`cli` 解析命令，`service` 编排业务，`repository` 访问数据库，`model` 保存类型。这样每层都能独立测试。

## 练习

1. 增加删除任务和按关键字过滤。
2. 将数据库路径从 TOML/环境变量读取。
3. 为命令增加 JSON 输出和错误退出码。

## 27.1 业务流程的错误边界

`run_task_flow` 把数据库错误作为 `rusqlite::Result<String>` 返回，调用者可以决定输出到终端、转换成退出码，或记录结构化日志。演示入口使用 `expect` 只是为了让章节输出保持简洁；面向用户的 CLI 应避免对文件路径、参数和数据库操作无条件 `unwrap`。

```rust
match run_task_flow() {
    Ok(text) => println!("{text}"),
    Err(error) => {
        eprintln!("任务流程失败：{error}");
        std::process::exit(1);
    }
}
```

## 27.2 从单文件到分层架构

当功能增加时，建议把当前模块拆成四层：`model` 只保存 `Task` 和过滤条件；`repository` 负责 SQL；`service` 校验标题、编排事务；`cli` 解析 `add/list/done/delete` 子命令并决定输出格式。每层都应只依赖下一层的抽象，避免 CLI 直接拼 SQL。

## 27.3 测试闭环

纯函数 `render_tasks` 应覆盖空列表、未完成任务和 Unicode 标题；流程测试使用独立内存数据库，验证“添加—完成—列出”的完整路径。集成测试再运行真实二进制，检查退出码和稳定标记，不要断言时间戳或日志顺序。

```bash
cargo test rust27_app -- --nocapture
```

## 27.4 持久化与并发

演示的 `open_in_memory()` 在进程退出后丢失数据。生产版可改为 `Connection::open(path)`，路径从 TOML 或环境变量读取，并在启动时执行迁移。多个线程写 SQLite 时需要明确连接所有权、设置 busy timeout，并尽量缩短事务；不要把一个可变 `Connection` 随意跨线程共享。

## 27.5 练习扩展

实现 `delete` 和 `search` 子命令；增加 `--format json` 并保证日志不污染 stdout；定义应用错误枚举，将参数错误、业务错误和数据库错误映射到不同退出码。
