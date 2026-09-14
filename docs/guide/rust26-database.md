# 第 26 章 · 数据库操作

程序要把数据活过进程退出，就得有个地方存。SQLite 是最适合「一个可执行文件配
一个数据文件」这种场景的数据库：没有服务端进程，整个引擎编译进程序里，
依然支持事务、约束和索引。

本章用 `rusqlite` 连接 SQLite，把它当一门需要认真对待的技术来学：

1. **连接与迁移**：schema 怎么版本化，怎样重复执行也不出错；
2. **CRUD 与参数化查询**：为什么绝不能拼接 SQL 字符串；
3. **类型映射**：SQLite 只有五种存储类型，怎么映射回 Rust 的 `bool` / `i64`；
4. **事务**：批量写入失败时如何保证「要么全成，要么全不成」；
5. **索引与查询计划**：怎么确认一条查询真的走了索引；
6. **测试隔离**：每个测试一个内存库，文件库怎么验证。

配套代码在本仓库里真实可跑：

| 部分 | 位置 | 怎么跑 |
| --- | --- | --- |
| 演示模块（迁移 / CRUD / 事务 / 查询计划） | `src/rust26_database/mod.rs` | `cargo run` |
| 本章单元测试 | 同上文件末尾 `#[cfg(test)]` | `cargo test --bin rust-learn-demo rust26_database` |
| 第 27 章的整合示例 | `src/rust27_app.rs` | `cargo run`（复用本章的 CRUD） |

依赖只有一行，关键在 `bundled`：

```toml
[dependencies]
rusqlite = { version = "0.32", features = ["bundled"] }
```

`bundled` 会把 SQLite 源码一起编译进程序。好处是**用户机器上不需要预装 SQLite**，
交叉编译和 CI 也更可控；代价是首次编译要等一会儿（要编 C 代码），
并且交叉编译到别的平台时需要准备对应平台的 C 工具链（第 24 章 24.7 提过这一点）。

## 26.1 连接：内存库、文件库与生命周期

```rust
use rusqlite::Connection;

let conn = Connection::open_in_memory()?;   // 只在内存里，进程退出即消失
let conn = Connection::open("tasks.sqlite")?; // 文件库，数据留在磁盘上
```

| 方式 | 适合 | 注意 |
| --- | --- | --- |
| `open_in_memory()` | 示例、单元测试 | 多连接之间**不共享**数据；连接关闭即清空 |
| `open(path)` | 真实应用 | 路径属于运行期配置，应从配置文件或环境变量读取（第 20 章） |
| `Connection::open_with_flags(...)` | 只读打开、创建策略 | 需要显式传 `OpenFlags`，适合迁移工具或只读分析脚本 |

`Connection` 不实现 `Sync`：它代表一条连接，**不要在多线程之间共享**。
需要并发时通常是「每个线程一条连接」配合理的锁策略与 `busy_timeout`，
细节见 26.10。

## 26.2 迁移：把 schema 版本化

真实项目不可能一次把表结构定死。把迁移写成一串「只追加」的脚本，
版本号记在 SQLite 自己的 `PRAGMA user_version` 里：

```rust
/// 下标 + 1 就是 schema 版本。
const MIGRATIONS: &[&str] = &[
    // v1：任务表。SQLite 没有布尔类型，done 用 INTEGER 加 CHECK 表示。
    "CREATE TABLE IF NOT EXISTS tasks (
        id INTEGER PRIMARY KEY,
        title TEXT NOT NULL UNIQUE,
        done INTEGER NOT NULL DEFAULT 0 CHECK (done IN (0, 1))
    );",
    // v2：新增优先级，老数据默认 2。
    "ALTER TABLE tasks ADD COLUMN priority INTEGER NOT NULL DEFAULT 2;",
    // v3：按完成状态过滤时走索引，而不是全表扫描。
    "CREATE INDEX IF NOT EXISTS idx_tasks_done ON tasks (done);",
];

pub fn migrate(conn: &Connection) -> Result<usize, TaskError> {
    let current = schema_version(conn)? as usize;
    let mut applied = 0;
    for (index, script) in MIGRATIONS.iter().enumerate().skip(current) {
        conn.execute_batch(script)?;
        conn.pragma_update(None, "user_version", (index + 1) as i64)?;
        applied += 1;
    }
    Ok(applied)
}
```

实测输出（`cargo run` 的第 1 节）：

```text
--- 1. 连接与版本化迁移 ---
    本次执行 3 条迁移，schema 版本 = 3
    再调用一次 migrate = 0 条（已经是最新版本，不会重复建表）
```

同一条逻辑在**文件库**上同样成立：第一次打开执行 3 条迁移，重新打开时版本已经
是 3，`migrate` 什么都不做。这条性质写在测试里：

```rust
#[test]
fn file_backed_database_keeps_data_and_version() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("tasks.sqlite");

    {
        let conn = Connection::open(&path).unwrap();
        assert_eq!(migrate(&conn).unwrap(), MIGRATIONS.len());
        add_task(&conn, "持久化").unwrap();
    } // 连接在这里关闭

    let conn = Connection::open(&path).unwrap();
    assert_eq!(migrate(&conn).unwrap(), 0, "重开时版本已经是最新的");
    assert_eq!(titles(&list_tasks(&conn).unwrap()), ["持久化"]);
}
```

迁移的三条纪律：

1. **只往后追加，不改历史脚本**。已经部署的库只会执行「比它新」的脚本，
   改动历史项会让新旧库悄悄分叉；
2. **每一步都要能重复执行**：建表用 `IF NOT EXISTS`，加索引用 `IF NOT EXISTS`，
   加列这种不能重复的操作由版本号保证只跑一次；
3. **不要依赖手工改表**。迁移是代码，要进仓库、要走评审、要跟着发布走。

## 26.3 约束：把规则放在哪一层

建表时能声明的规则，就别留给应用层去记：

| 约束 | 作用 | 本章的用法 |
| --- | --- | --- |
| `PRIMARY KEY` | 唯一标识，SQLite 里 `INTEGER PRIMARY KEY` 即 rowid | `id` |
| `NOT NULL` | 禁止空值 | `title`、`done` |
| `UNIQUE` | 值唯一（会自动建索引） | `title` 不允许重复 |
| `CHECK` | 值必须满足条件 | `done IN (0, 1)` |
| `DEFAULT` | 缺省值 | `done = 0`、`priority = 2` |

范围校验则是**两层都做**的例子：`priority` 的 1..=3 写在 Rust 侧
（`validate_priority`），没有写进 `CHECK`。原因很实际：这一列是 v2 迁移用
`ALTER TABLE ... ADD COLUMN` 加的，写 `CHECK` 会限制老数据的兼容性；
而且业务规则的报错要给用户看得懂的中文提示，Rust 侧的 `TaskError::InvalidPriority`
比一条 `CHECK constraint failed: tasks` 更友好。

反过来，`title` 的唯一性放在数据库：它是**并发下的最终裁决者**，而且 `UNIQUE`
顺带给你一个可用于查询的索引。两层配合的实测效果：

```text
--- 2. 插入：参数绑定 + 业务校验 ---
    id = 1（默认优先级 2）、id = 2（优先级 1）
    空标题在进 SQL 之前就被拒绝：任务标题不能为空
    重复标题被数据库 UNIQUE 约束拒绝：is_constraint_violation = true
```

## 26.4 CRUD 与参数化查询

先看反面教材。**永远不要**这样写：

```rust
// 危险：用户输入直接拼进 SQL
let sql = format!("SELECT id FROM tasks WHERE title = '{keyword}'");
```

如果 `keyword` 是 `' OR '1'='1`，整条语句的语义就被改写了（SQL 注入）。
正确写法是把值当作**参数**绑定，SQL 文本永远不变：

```rust
pub fn add_task_with_priority(
    conn: &Connection,
    title: &str,
    priority: i64,
) -> Result<i64, TaskError> {
    let title = validate_title(title)?;
    let priority = validate_priority(priority)?;
    conn.execute(
        "INSERT INTO tasks (title, priority) VALUES (?1, ?2)",
        params![title, priority],
    )?;
    Ok(conn.last_insert_rowid())
}
```

参数化的两个额外好处：引号、换行、表情符号都能原样存取，**不需要手工转义**；
SQLite 还能复用已编译的语句计划。这条性质也写进了测试：

```rust
assert!(
    search_tasks(&conn, "' OR 1=1 --").unwrap().is_empty(),
    "参数绑定会把引号当普通字符"
);
```

本章的 CRUD 一览：

| 函数 | SQL | 返回值 |
| --- | --- | --- |
| `add_task` / `add_task_with_priority` | `INSERT` | 新行的 `id` |
| `list_tasks` | `SELECT ... ORDER BY id` | 全部任务 |
| `list_open_tasks` | `SELECT ... WHERE done = 0 ORDER BY priority, id` | 未完成任务 |
| `list_by_priority` | `SELECT ... WHERE priority = ?1` | 指定优先级 |
| `search_tasks` | `SELECT ... WHERE title LIKE ?1` | 关键字命中 |
| `complete_task` / `rename_task` / `set_priority` | `UPDATE` | 是否真的改到一行 |
| `delete_task` | `DELETE` | 是否真的删掉一行 |
| `add_tasks_transactional` | 事务内批量 `INSERT` | 新 id 列表 |
| `stats` | `SELECT COUNT(*), SUM(done)` | `Stats` 快照 |

`LIKE` 的通配符要当心：`%` 和 `_` 在 `LIKE` 里有特殊含义，用户搜 `100%` 时
`%` 会变成「任意字符串」。要精确匹配就得转义（`ESCAPE '\'`）。本章的示例
只用它演示参数绑定，实际项目要按需求决定是否需要转义。

## 26.5 类型映射：从一行数据到 Rust 类型

SQLite 的存储类型只有 INTEGER、REAL、TEXT、BLOB、NULL 五种，
没有布尔、没有日期、没有枚举。映射回来时，这些转换都由你决定：

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Task {
    pub id: i64,
    pub title: String,
    pub done: bool,
    pub priority: i64,
}

fn task_from_row(row: &Row<'_>) -> rusqlite::Result<Task> {
    Ok(Task {
        id: row.get(0)?,
        title: row.get(1)?,
        // SQLite 没有 bool：约定非零即完成。
        done: row.get::<_, i64>(2)? != 0,
        priority: row.get(3)?,
    })
}
```

三个要点：

- **按列号取值，就必须和 `SELECT` 的顺序一致**。列多起来之后很容易错位，
  生产代码通常改用 `row.get("title")` 的形式按列名取；
- **`ORDER BY` 不是可选项**。不加排序时 SQL 不保证顺序，测试就会时好时坏；
- **整数宽度要显式**：`row.get::<_, i64>(2)` 而不是让编译器猜，SQLite 的
  `INTEGER` 是 64 位有符号整数，超出 `i32` 是完全可能的。

实测输出：

```text
--- 3. 查询：类型映射与顺序 ---
    Task { id: 1, title: "学习 Rust 数据库", done: false, priority: 2 }
    Task { id: 2, title: "写测试", done: false, priority: 1 }
    Task { id: 3, title: "整理笔记", done: false, priority: 3 }
    未完成（按优先级）= ["写测试", "学习 Rust 数据库", "整理笔记"]
    标题含 "Rust" = ["学习 Rust 数据库"]
    优先级 3 = ["整理笔记"]
```

## 26.6 业务校验与统一错误类型

`rusqlite::Error` 只描述数据库层面发生了什么。「标题为空」「优先级越界」是业务
规则，两者混在一起会让上层难以决定怎么响应。本章用一个枚举把两类错误分开：

```rust
#[derive(Debug)]
pub enum TaskError {
    EmptyTitle,
    InvalidPriority(i64),
    Sqlite(rusqlite::Error),
}

impl TaskError {
    /// 这个错误是不是约束冲突（例如标题重复、CHECK 不通过）。
    pub fn is_constraint_violation(&self) -> bool {
        matches!(
            self,
            TaskError::Sqlite(rusqlite::Error::SqliteFailure(error, _))
                if error.code == ErrorCode::ConstraintViolation
        )
    }
}

impl From<rusqlite::Error> for TaskError {
    fn from(error: rusqlite::Error) -> Self {
        TaskError::Sqlite(error)
    }
}
```

有了 `From` 实现，`?` 就能直接从 `rusqlite::Result` 转到本章的错误类型；
再加上 `Display` 与 `Error::source`（第 8 章的自定义错误套路），上层既能打印
中文提示，也能顺着 `source()` 找到底层原因。

对应测试断言失败时的实际错误：

```rust
#[test]
fn duplicate_titles_hit_the_unique_constraint() {
    let conn = memory_db();
    add_task(&conn, "写代码").unwrap();

    let error = add_task(&conn, "写代码").unwrap_err();
    assert!(error.is_constraint_violation(), "实际错误：{error}");

    // 首尾空白会被 trim，所以 " 写代码 " 和 "写代码" 是同一个标题。
    assert!(add_task(&conn, " 写代码 ").unwrap_err().is_constraint_violation());
    assert_eq!(list_tasks(&conn).unwrap().len(), 1);
}
```

「校验失败的输入不会留下数据」也单独有一条测试：

```rust
assert!(matches!(add_task(&conn, "   "), Err(TaskError::EmptyTitle)));
assert!(matches!(add_task_with_priority(&conn, "x", 0), Err(TaskError::InvalidPriority(0))));
assert_eq!(stats(&conn).unwrap(), Stats { total: 0, done: 0, open: 0 });
```

## 26.7 事务：要么全成，要么全不成

批量写入必须包在事务里，否则中途失败会留下半批数据：

```rust
pub fn add_tasks_transactional(
    conn: &mut Connection,
    titles: &[&str],
) -> Result<Vec<i64>, TaskError> {
    let transaction = conn.transaction()?;
    let mut ids = Vec::with_capacity(titles.len());
    for title in titles {
        ids.push(add_task(&transaction, title)?);   // 失败就提前返回
    }
    transaction.commit()?;                          // 成功才提交
    Ok(ids)
}
```

三个细节：

- **`&mut Connection` 不是摆设**：`transaction()` 需要独占借用，SQLite 也要求
  同一时刻只有一条写事务；
- **`&Transaction` 能当 `&Connection` 用**：`Transaction` 实现了 `Deref`，
  所以上面调用的 `add_task` 不需要为事务再写一遍；
- **提前返回即回滚**：`?` 直接返回时 `transaction` 被 drop，未提交的事务自动
  回滚。想显式回滚可以调用 `transaction.rollback()`。

实测输出（第二条标题和已有任务重复，整批回滚）：

```text
--- 5. 事务：要么全成，要么全不成 ---
    批量插入成功：[4, 5]
    含重复标题的批量插入：is_constraint_violation = true
    回滚后 total = 4，和插入前的 4 相同：true
    （第一条 "新任务" 也一起回滚了，批量写入不会留下半成品）
```

对应测试只断言一件事——回滚之后数据没有被污染：

```rust
#[test]
fn transaction_rolls_back_the_whole_batch() {
    let mut conn = memory_db();
    add_task(&conn, "已存在").unwrap();

    let failed = add_tasks_transactional(&mut conn, &["新任务", "已存在"]);
    assert!(failed.is_err(), "整批应当失败");
    assert_eq!(titles(&list_tasks(&conn).unwrap()), ["已存在"], "前一条插入必须一起回滚");
}
```

**事务里不要做慢操作**：网络请求、等待用户输入、大文件读写都会长时间持有
SQLite 的写锁，让其他写者超时。事务应该只包住一组数据库操作。

## 26.8 `UPDATE` 的影响行数：一个容易搞错的语义

`conn.execute` 返回的是「被语句影响的行数」，而不是「值真的变了」。看一下
下面这条语句在同一个任务上执行两次会怎样：

```sql
UPDATE tasks SET done = 1 WHERE id = ?1;   -- 第二次也返回 1
```

第二次执行时 `done` 本来就是 1，SQLite 仍然报告影响了 1 行——于是「标记完成」
这个函数会两次返回 `true`，调用方根本分不清「这次真的改了」还是「本来就是这样」。
解决方法是把条件写进 `WHERE`：

```rust
/// 只有真正从「未完成」变成「完成」才返回 true。
pub fn complete_task(conn: &Connection, id: i64) -> Result<bool, TaskError> {
    Ok(conn.execute(
        "UPDATE tasks SET done = 1 WHERE id = ?1 AND done = 0",
        params![id],
    )? == 1)
}
```

实测输出：

```text
--- 4. 更新、删除与统计 ---
    complete_task(1) = true
    rename_task(2, "写更多测试") = true
    set_priority(1, 3) = true
    不存在的 id 返回 false：complete_task(9999) = false
    统计 = Stats { total: 3, done: 1, open: 2 }
    delete_task(2) = true，剩余 待办 = ["学习 Rust 数据库", "整理笔记"]
```

同样的思路可以用在「幂等接口」上：`rename_task` 用 `WHERE id = ?` 返回是否命中，
`delete_task` 用「删掉几行」判断是否真的存在。返回 `bool` 的接口要**在文档里
写清是什么语义**，否则调用方迟早会误用。

统计用的 SQL 也顺手避开了一个坑：

```rust
let mut statement = conn.prepare("SELECT COUNT(*), COALESCE(SUM(done), 0) FROM tasks")?;
```

空表时 `SUM(done)` 返回 `NULL` 而不是 0，直接取成 `i64` 会报错，
所以用 `COALESCE` 兜底。`COUNT` 则天然返回 0，不需要处理。

## 26.9 索引与查询计划

索引不是「加上就快」，得看查询有没有用上。SQLite 提供了直接看计划的办法：

```rust
pub fn query_plan(conn: &Connection, sql: &str) -> Result<Vec<String>, TaskError> {
    let mut statement = conn.prepare(&format!("EXPLAIN QUERY PLAN {sql}"))?;
    let plan = statement
        .query_map([], |row| row.get::<_, String>(3))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(plan)
}
```

实测输出：

```text
--- 6. 查询计划：索引有没有生效 ---
    done = 0   -> ["SEARCH tasks USING COVERING INDEX idx_tasks_done (done=?)"]
    title LIKE -> ["SCAN tasks USING COVERING INDEX sqlite_autoindex_tasks_1"]
    （SEARCH 才是用索引定位；SCAN 表示逐行扫描，% 开头的 LIKE 只能 SCAN）
```

怎么读这两行：

| 关键字 | 含义 |
| --- | --- |
| `SEARCH` | 用索引定位，命中行数与匹配量相关 |
| `SCAN` | 从头扫到尾，行数越多越慢 |
| `USING INDEX idx_tasks_done` | 用上了 v3 迁移里建的索引 |
| `USING COVERING INDEX` | 查询要的列都在索引里，连表都不用回查 |

两条实践结论：

- **`WHERE done = 0` 走索引**，因为 v3 迁移建了 `idx_tasks_done`；
- **`title LIKE '%Rust%'` 只能 `SCAN`**。以 `%` 开头的模式没有可定位的前缀，
  B 树索引帮不上忙。真要做全文搜索，应该用 SQLite 的 FTS5 扩展，或者把数据
  交给专门的搜索引擎。

索引也有成本：**每个索引都会拖慢写入并占空间**。判断标准很简单，加索引前先跑
一次 `EXPLAIN QUERY PLAN`，加上之后再跑一次；真正慢的查询才值得建索引。
第 28 章的性能优化流程也是这套逻辑。

## 26.10 测试隔离、并发与连接管理

测试最简单可靠的做法是**每个测试一个内存库**：

```rust
/// 每个测试都自己开一个内存库，测试之间不共享状态。
fn memory_db() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    init(&conn).unwrap();
    conn
}
```

`cargo test` 默认并行跑测试，共享一个连接（更别说共享一个文件库）会互相污染，
表现就是「单跑能过、一起跑随机挂」。用内存库还能顺带解决「测试跑完留下垃圾文件」
的问题——不过需要验证文件库行为时，就用 `tempfile` 建临时目录（第 19、23 章
的套路）：

```rust
let dir = tempfile::tempdir().unwrap();
let path = dir.path().join("tasks.sqlite");
```

本章的 8 条测试全部通过：

```text
$ cargo test --bin rust-learn-demo rust26_database
running 8 tests
test rust26_database::tests::crud_roundtrip ... ok
test rust26_database::tests::duplicate_titles_hit_the_unique_constraint ... ok
test rust26_database::tests::file_backed_database_keeps_data_and_version ... ok
test rust26_database::tests::migrations_are_tracked_and_idempotent ... ok
test rust26_database::tests::query_plan_shows_index_usage ... ok
test rust26_database::tests::search_order_and_stats ... ok
test rust26_database::tests::transaction_rolls_back_the_whole_batch ... ok
test rust26_database::tests::validation_errors_never_reach_the_database ... ok
test result: ok. 8 passed; 0 failed; ...
```

### 多线程与多连接

SQLite 的并发模型是「多个读者 + 一个写者」。几条实用规则：

| 场景 | 做法 |
| --- | --- |
| 多线程访问同一个文件库 | 每个线程一条连接，不要 `Arc<Connection>` 共享 |
| 偶尔写锁冲突 | 设置 `busy_timeout`，让 SQLite 自己等一会儿再重试 |
| 读多写少、跨进程 | 打开 WAL：`PRAGMA journal_mode = WAL`，读不再阻塞写 |
| 队列型的写入 | 用一个专门的写线程串行化，比到处加锁简单 |
| 内存库共享 | 需要共享时用 `file:name?mode=memory&cache=shared` 命名内存库 |

```rust
// 连接建立后立刻设置，避免「database is locked」直接失败
conn.busy_timeout(std::time::Duration::from_secs(5))?;
```

`busy_timeout` 只是让冲突时等待，不解决设计问题：事务太长、写太频繁时，
该改的是访问模式（缩短事务、合并写入、批量提交）。

## 26.11 常见坑速查

| 现象 | 原因 | 解法 |
| --- | --- | --- |
| `database is locked` | 另一条连接正持有写锁 | 设置 `busy_timeout`，缩短事务，必要时开 WAL |
| 查询结果顺序变来变去 | 没写 `ORDER BY` | 显式排序；测试也别依赖默认顺序 |
| 参数值被当成 SQL 执行 | 用 `format!` 拼了用户输入 | 一律用 `params![...]` 绑定 |
| 布尔值取不出来 | SQLite 没有 `bool` 类型 | 用 `INTEGER` 存，读取时 `!= 0` |
| 大整数溢出 | 用了 `i32` 取 SQLite 的 `INTEGER` | 用 `i64`（SQLite 的 INTEGER 是 64 位） |
| 空表的求和不报错但取值为空 | `SUM` 对空集返回 `NULL` | 用 `COALESCE(SUM(x), 0)` |
| 批量插入失败后留下半批数据 | 没有用事务 | 用 `conn.transaction()` 包住整批 |
| 重复「标记完成」都返回 true | `UPDATE` 返回的是影响行数 | 条件里带上 `AND done = 0` |
| 迁移在老库上崩 | 改了历史脚本或没写 `IF NOT EXISTS` | 只追加脚本，每步都可重复执行 |
| 测试单跑能过、一起跑失败 | 共享了连接或文件库 | 每个测试一个内存库 |
| 加了索引反而更慢 | 写入成本变高、查询没用到索引 | 用 `EXPLAIN QUERY PLAN` 确认，再决定保留 |
| 把整个表读进内存再过滤 | 用 `SELECT *` + 循环筛选 | 让 SQL 用 `WHERE` / `LIMIT` 过滤，只取需要的列 |

## 26.12 练习

1. 给任务表加一个 `due_date TEXT`（ISO-8601 日期）列作为 v4 迁移，并实现
   `list_due_before(conn, date) -> Result<Vec<Task>, TaskError>`，用字符串比较
   完成筛选，然后解释为什么 ISO-8601 可以直接做字典序比较。
2. 实现 `clear_completed(conn) -> Result<usize, TaskError>`：在一个事务里删除所有
   已完成任务并返回删除行数；写测试验证「没有已完成任务时返回 0」。
3. 为 `priority` 增加 `CHECK (priority BETWEEN 1 AND 3)` 作为 v4 迁移（提示：
   SQLite 不能给已有表加 `CHECK`，需要「建新表 → 搬数据 → 删旧表」三步），
   说明这样做相比当前「只在 Rust 侧校验」的优缺点。
4. 给 `stats` 加一个 `by_priority` 字段，用 `GROUP BY priority` 一次查出来，
   并写测试覆盖空表的情形。
5. 回答两个问题：(a) 为什么 `open_in_memory()` 更适合单元测试？
   (b) `UPDATE ... WHERE id = ?` 和 `UPDATE ... WHERE id = ? AND done = 0`
   在「返回是否真的改变」这个语义上有什么差别？

（第 5 题是 26.1 与 26.8 讨论过的内容，这里不再单独给答案。）

### 参考答案

::: details 第 1 题

```rust
// v4：到期日，用 TEXT 存 ISO-8601（YYYY-MM-DD）。
"ALTER TABLE tasks ADD COLUMN due_date TEXT;",
```

```rust
pub fn list_due_before(conn: &Connection, date: &str) -> Result<Vec<Task>, TaskError> {
    select_tasks(
        conn,
        "SELECT id, title, done, priority FROM tasks
         WHERE due_date IS NOT NULL AND due_date < ?1
         ORDER BY due_date, id",
        &[&date],
    )
}
```

ISO-8601 的日期是**定长、零填充、从大到小排列**的（`2026-09-14`），
所以字典序和真实时间顺序一致：`2026-09-14 < 2026-10-01` 既是字符串比较结果，
也是时间先后结果。这也是为什么日志和数据库里推荐用它，而不是 `9/14/2026`
这类格式——后者比较时先比月份，`10` 会排在 `9` 前面。

:::

::: details 第 2 题

```rust
pub fn clear_completed(conn: &mut Connection) -> Result<usize, TaskError> {
    let transaction = conn.transaction()?;
    let removed = transaction.execute("DELETE FROM tasks WHERE done = 1", [])?;
    transaction.commit()?;
    Ok(removed)
}

#[test]
fn clearing_completed_is_idempotent() {
    let mut conn = memory_db();
    let id = add_task(&conn, "已完成").unwrap();
    complete_task(&conn, id).unwrap();
    add_task(&conn, "未完成").unwrap();

    assert_eq!(clear_completed(&mut conn).unwrap(), 1);
    assert_eq!(clear_completed(&mut conn).unwrap(), 0);   // 再删一次没有行受影响
    assert_eq!(titles(&list_tasks(&conn).unwrap()), ["未完成"]);
}
```

单条 `DELETE` 本身就在隐式事务里，这里显式包一层是为了演示**返回行数**这个语义：
「删除成功」和「删掉了几行」是两件事，接口返回 `usize` 让调用方拿到确切信息。

:::

::: details 第 3 题

```sql
-- v4：把 priority 的范围约束交给数据库（SQLite 不支持 ADD CONSTRAINT）
CREATE TABLE tasks_new (
    id INTEGER PRIMARY KEY,
    title TEXT NOT NULL UNIQUE,
    done INTEGER NOT NULL DEFAULT 0 CHECK (done IN (0, 1)),
    priority INTEGER NOT NULL DEFAULT 2 CHECK (priority BETWEEN 1 AND 3)
);
INSERT INTO tasks_new (id, title, done, priority)
    SELECT id, title, done, priority FROM tasks;
DROP TABLE tasks;
ALTER TABLE tasks_new RENAME TO tasks;
```

优缺点：

| | 数据库 `CHECK` | Rust 侧校验 |
| --- | --- | --- |
| 保护范围 | 所有写入路径（含手工 SQL、导入脚本） | 只有走这段代码的路径 |
| 报错信息 | `CHECK constraint failed: tasks` | 可定制的中文提示 |
| 迁移成本 | 要重建表，老数据必须先满足约束 | 无 |
| 性能 | 每次写入多一次校验 | 可忽略 |

成熟项目通常**两层都要**：应用层负责给用户友好提示，数据库层负责兜底。
只有当你确定这个库会被别的程序直接写时，才必须优先保证数据库层的约束。
重建表迁移时要注意：先关外键（`PRAGMA foreign_keys = OFF`），迁移完再打开，
并且整个过程放在一个事务里。

:::

## 26.13 小结

- 连接分内存库和文件库：内存库适合示例与测试（连接之间不共享），文件库的路径
  属于运行期配置；`Connection` 不要跨线程共享。
- 迁移靠「只追加的脚本 + `PRAGMA user_version`」实现，每步都要能重复执行；
  重开文件库时版本号还在，`migrate` 不会再跑一遍。
- 约束尽量交给数据库（`NOT NULL` / `UNIQUE` / `CHECK` / `DEFAULT`），
  业务提示放在 Rust 侧，两层配合。
- SQL 永远用 `params!` 绑定参数，绝不拼接用户输入；`LIKE` 的 `%` / `_`
  是通配符，需要精确匹配时要转义。
- SQLite 只有五种存储类型：`done` 用 `INTEGER` 存、读取时 `!= 0`，
  整数用 `i64`，查询一定要写 `ORDER BY`。
- 用自定义错误枚举把「业务校验错误」和「SQLite 错误」分开，`From` + `?`
  让调用代码保持干净，`is_constraint_violation` 让上层识别约束冲突。
- 批量写入包在事务里，`&mut Connection` 换独占借用，`&Transaction` 可以当
  `&Connection` 用；提前返回即回滚。
- `UPDATE` 返回的是影响行数，不是「值变了没有」；要判断真实变化就把条件写进
  `WHERE`。空表求和要 `COALESCE`。
- 索引要看 `EXPLAIN QUERY PLAN` 的 `SEARCH` / `SCAN`，加索引前先确认查询
  真的用得上——索引也拖慢写入。
- 测试用「每个测试一个内存库」保证隔离，需要文件行为时用 `tempfile`；
  并发场景靠「每线程一条连接 + `busy_timeout` + 短事务」，必要时开 WAL。
- 本仓库实测：3 条迁移可重复执行、8 条测试全部通过、`done = 0` 走
  `idx_tasks_done` 索引而 `%Rust%` 只能 `SCAN`。

下一章是**第 27 章 综合实战**：把本章的数据库、前几章的配置与错误处理串成一个
最小的任务管理系统，并讨论从单文件到分层架构的拆分方式。
