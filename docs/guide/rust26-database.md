# 第 26 章 · 数据库操作

本章使用 `rusqlite` 和 bundled SQLite，示范连接、建表、CRUD、参数化查询、事务和测试隔离。

## 26.1 连接与建表

`Connection::open_in_memory()` 适合示例和测试；生产环境使用文件路径，并通过配置注入。`init` 使用 `CREATE TABLE IF NOT EXISTS`，可重复执行。

## 26.2 CRUD 与参数化查询

`add_task`、`list_tasks`、`complete_task`、`delete_task` 和 `search_tasks` 覆盖常用 CRUD。始终使用 `params!` 绑定参数，不要拼接 SQL 字符串，以避免 SQL 注入并正确处理引号。

## 26.3 事务与错误处理

`add_tasks_transactional` 将批量写操作放进事务：`let tx = conn.transaction()?; ... tx.commit()?;`。事务提交失败时自动回滚。数据库函数返回 `rusqlite::Result`，由上层决定显示错误还是重试。

## 26.4 测试隔离与迁移

测试使用独立内存数据库，避免共享状态。生产项目应把 schema 迁移作为版本化文件，部署时按顺序执行，不能依赖手工改表。

```bash
cargo test --locked
cargo run
```

## 26.5 小结

数据库代码的核心是明确连接生命周期、参数化 SQL、事务边界和迁移策略。

## 26.6 从一行数据到 Rust 类型

`Task` 将表中的三列转换为 Rust 字段。SQLite 没有原生布尔类型，因此 `done` 以 `INTEGER` 保存，读取时把非零值转换为 `true`。查询时显式列出字段并指定 `ORDER BY id`，这样输出顺序不会依赖数据库实现细节。

```rust
let mut statement = conn.prepare(
    "SELECT id, title, done FROM tasks WHERE title LIKE ?1 ORDER BY id",
)?;
let tasks = statement
    .query_map(["%Rust%"], |row| {
        Ok(Task {
            id: row.get(0)?,
            title: row.get(1)?,
            done: row.get::<_, i64>(2)? != 0,
        })
    })?
    .collect::<rusqlite::Result<Vec<_>>>()?;
```

## 26.7 事务失败会发生什么

`add_tasks_transactional` 接收 `&mut Connection`，先创建事务，再逐条插入，最后调用 `commit`。如果中途返回错误，事务被丢弃并回滚，调用者不会得到半批数据。事务中不要执行网络请求或等待用户输入，否则会长时间持有 SQLite 锁。

```rust
let ids = add_tasks_transactional(&mut conn, &["one", "two"])?;
assert_eq!(ids.len(), 2);
```

## 26.8 常见错误与练习

- 使用 `format!` 拼接用户输入会产生注入风险；始终使用 `params!`。
- 共享一个全局连接会让测试互相污染；测试应各自创建内存数据库。
- 没有 `ORDER BY` 的查询不保证顺序。
- 把迁移逻辑散落在启动代码中，最终难以回滚；应使用编号迁移文件。

练习：为任务增加优先级字段并写迁移；实现不存在 ID 时返回 `false` 的重命名操作；让空标题在事务中触发错误并验证整批回滚。
