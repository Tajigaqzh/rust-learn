//! 第 26 章配套代码：SQLite 数据库操作（`rusqlite` + bundled SQLite）。
//!
//! 运行方式：`cargo run`；只跑本章的测试：`cargo test rust26_database`。
//!
//! 演示全程使用内存数据库（`Connection::open_in_memory`），进程退出即消失；
//! 文件数据库的差别、以及"重新打开后迁移不会重复执行"这件事见文档 26.2。

use rusqlite::{Connection, ErrorCode, Row, params};

/// 一行任务，对应 `tasks` 表里的一行。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Task {
    pub id: i64,
    pub title: String,
    pub done: bool,
    /// 优先级：1 最高，3 最低。
    pub priority: i64,
}

/// 一次统计查询的结果快照。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stats {
    pub total: i64,
    pub done: i64,
    pub open: i64,
}

/// 业务校验错误与 SQLite 错误的统一出口。
///
/// 分开两类是有意的：调用方可能想「空标题就在界面上提示用户」，
/// 而「磁盘满、锁等待」需要重试或告警。
#[derive(Debug)]
pub enum TaskError {
    /// 标题去掉首尾空白之后为空。
    EmptyTitle,
    /// 优先级不在 1..=3 之间。
    InvalidPriority(i64),
    /// 底层 SQLite 错误：约束冲突、IO、SQL 语法等。
    Sqlite(rusqlite::Error),
}

impl TaskError {
    /// 这个错误是不是约束冲突（例如标题重复、`CHECK` 不通过）。
    pub fn is_constraint_violation(&self) -> bool {
        matches!(
            self,
            TaskError::Sqlite(rusqlite::Error::SqliteFailure(error, _))
                if error.code == ErrorCode::ConstraintViolation
        )
    }
}

impl std::fmt::Display for TaskError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskError::EmptyTitle => write!(formatter, "任务标题不能为空"),
            TaskError::InvalidPriority(value) => {
                write!(formatter, "优先级 {value} 不在 1..=3 之间")
            }
            TaskError::Sqlite(error) => write!(formatter, "数据库错误：{error}"),
        }
    }
}

impl std::error::Error for TaskError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            TaskError::Sqlite(error) => Some(error),
            _ => None,
        }
    }
}

impl From<rusqlite::Error> for TaskError {
    fn from(error: rusqlite::Error) -> Self {
        TaskError::Sqlite(error)
    }
}

/// 迁移脚本：下标 + 1 就是 schema 版本，记录在 `PRAGMA user_version` 里。
///
/// 规则只有一条：**只往后追加，不改历史项**。已经部署过的数据库只会执行
/// 「比它新」的脚本，改动历史脚本会让新旧库的结构悄悄分叉。
const MIGRATIONS: &[&str] = &[
    // v1：任务表。SQLite 没有布尔类型，done 用 INTEGER 加 CHECK 表示。
    "CREATE TABLE IF NOT EXISTS tasks (
        id INTEGER PRIMARY KEY,
        title TEXT NOT NULL UNIQUE,
        done INTEGER NOT NULL DEFAULT 0 CHECK (done IN (0, 1))
    );",
    // v2：新增优先级，老数据默认 2。范围校验放在 Rust 侧（见 validate_priority）。
    "ALTER TABLE tasks ADD COLUMN priority INTEGER NOT NULL DEFAULT 2;",
    // v3：按完成状态过滤时走索引，而不是全表扫描。
    "CREATE INDEX IF NOT EXISTS idx_tasks_done ON tasks (done);",
];

/// 读取当前 schema 版本。
pub fn schema_version(conn: &Connection) -> Result<i64, TaskError> {
    Ok(conn.query_row("PRAGMA user_version", [], |row| row.get(0))?)
}

/// 顺序执行尚未应用的迁移，返回本次实际执行了几条。
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

/// 建表并把 schema 升到最新版本；可重复执行。
pub fn init(conn: &Connection) -> Result<(), TaskError> {
    migrate(conn).map(|_| ())
}

/// 标题去掉首尾空白后不能为空。
fn validate_title(title: &str) -> Result<&str, TaskError> {
    let trimmed = title.trim();
    if trimmed.is_empty() {
        return Err(TaskError::EmptyTitle);
    }
    Ok(trimmed)
}

/// 优先级约定为 1..=3（1 最高）。
fn validate_priority(priority: i64) -> Result<i64, TaskError> {
    if (1..=3).contains(&priority) {
        Ok(priority)
    } else {
        Err(TaskError::InvalidPriority(priority))
    }
}

/// 把一行结果映射成 `Task`；列顺序必须和 SQL 里的 `SELECT` 一致。
fn task_from_row(row: &Row<'_>) -> rusqlite::Result<Task> {
    Ok(Task {
        id: row.get(0)?,
        title: row.get(1)?,
        // SQLite 没有 bool：约定非零即完成。
        done: row.get::<_, i64>(2)? != 0,
        priority: row.get(3)?,
    })
}

/// 列出全部任务；顺序由 `ORDER BY` 固定，不依赖数据库的实现细节。
fn select_tasks(
    conn: &Connection,
    sql: &str,
    params: &[&dyn rusqlite::ToSql],
) -> Result<Vec<Task>, TaskError> {
    let mut statement = conn.prepare(sql)?;
    let tasks = statement
        .query_map(params, task_from_row)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(tasks)
}

/// 列出全部任务。
pub fn list_tasks(conn: &Connection) -> Result<Vec<Task>, TaskError> {
    select_tasks(
        conn,
        "SELECT id, title, done, priority FROM tasks ORDER BY id",
        &[],
    )
}

/// 只列出未完成的任务；`done = 0` 会走 v3 建的索引。
pub fn list_open_tasks(conn: &Connection) -> Result<Vec<Task>, TaskError> {
    select_tasks(
        conn,
        "SELECT id, title, done, priority FROM tasks WHERE done = 0 ORDER BY priority, id",
        &[],
    )
}

/// 按优先级列出任务（1 最高）。
pub fn list_by_priority(conn: &Connection, priority: i64) -> Result<Vec<Task>, TaskError> {
    let priority = validate_priority(priority)?;
    select_tasks(
        conn,
        "SELECT id, title, done, priority FROM tasks WHERE priority = ?1 ORDER BY id",
        &[&priority],
    )
}

/// 关键字搜索：通配符由参数绑定，绝不拼接用户输入。
pub fn search_tasks(conn: &Connection, keyword: &str) -> Result<Vec<Task>, TaskError> {
    let pattern = format!("%{keyword}%");
    select_tasks(
        conn,
        "SELECT id, title, done, priority FROM tasks WHERE title LIKE ?1 ORDER BY id",
        &[&pattern],
    )
}

/// 插入任务（默认优先级 2），返回新行的 id。
pub fn add_task(conn: &Connection, title: &str) -> Result<i64, TaskError> {
    add_task_with_priority(conn, title, 2)
}

/// 插入任务并指定优先级。
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

/// 在一个事务里批量插入；任意一条失败都会回滚整批。
///
/// `Transaction` 通过 `Deref` 提供 `&Connection`，所以上面对 `&Connection`
/// 写的函数不用为事务再写一遍。
pub fn add_tasks_transactional(
    conn: &mut Connection,
    titles: &[&str],
) -> Result<Vec<i64>, TaskError> {
    let transaction = conn.transaction()?;
    let mut ids = Vec::with_capacity(titles.len());
    for title in titles {
        ids.push(add_task(&transaction, title)?);
    }
    transaction.commit()?;
    Ok(ids)
}

/// 标记完成；只有真正从「未完成」变成「完成」才返回 `true`。
///
/// 条件里带上 `done = 0` 是有意的：SQLite 的 `execute` 返回的是**被语句
/// 影响的行数**，重复执行 `SET done = 1` 同样会算作影响一行。加上这个条件，
/// 重复调用就会返回 `false`，调用方据此判断「这次是不是真的改了状态」。
pub fn complete_task(conn: &Connection, id: i64) -> Result<bool, TaskError> {
    Ok(conn.execute(
        "UPDATE tasks SET done = 1 WHERE id = ?1 AND done = 0",
        params![id],
    )? == 1)
}

/// 重命名任务；不存在的 id 返回 `false`。
pub fn rename_task(conn: &Connection, id: i64, title: &str) -> Result<bool, TaskError> {
    let title = validate_title(title)?;
    Ok(conn.execute(
        "UPDATE tasks SET title = ?1 WHERE id = ?2",
        params![title, id],
    )? == 1)
}

/// 调整优先级；不存在的 id 返回 `false`。
pub fn set_priority(conn: &Connection, id: i64, priority: i64) -> Result<bool, TaskError> {
    let priority = validate_priority(priority)?;
    Ok(conn.execute(
        "UPDATE tasks SET priority = ?1 WHERE id = ?2",
        params![priority, id],
    )? == 1)
}

/// 删除任务；不存在的 id 返回 `false`。
pub fn delete_task(conn: &Connection, id: i64) -> Result<bool, TaskError> {
    Ok(conn.execute("DELETE FROM tasks WHERE id = ?1", params![id])? == 1)
}

/// 汇总总数、已完成和未完成；空表时 `SUM` 是 NULL，用 `COALESCE` 兜底。
pub fn stats(conn: &Connection) -> Result<Stats, TaskError> {
    let mut statement = conn.prepare("SELECT COUNT(*), COALESCE(SUM(done), 0) FROM tasks")?;
    let (total, done) =
        statement.query_row([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)))?;
    Ok(Stats {
        total,
        done,
        open: total - done,
    })
}

/// 读 `EXPLAIN QUERY PLAN` 的 detail 列，用来确认查询到底走没走索引。
///
/// 这是调试工具：`sql` 必须是代码里写死的语句，不能拼用户输入
/// （`EXPLAIN` 之后的 SQL 同样会被执行计划器解析）。
pub fn query_plan(conn: &Connection, sql: &str) -> Result<Vec<String>, TaskError> {
    let mut statement = conn.prepare(&format!("EXPLAIN QUERY PLAN {sql}"))?;
    let plan = statement
        .query_map([], |row| row.get::<_, String>(3))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(plan)
}

/// 把任务列表压成标题列表，让演示输出短一些。
fn titles(tasks: &[Task]) -> Vec<&str> {
    tasks.iter().map(|task| task.title.as_str()).collect()
}

/// 演示迁移、CRUD、参数化查询、事务回滚和查询计划。
pub fn database_demo() {
    println!("\n========== rust26_database: SQLite 数据库操作 ==========");
    let mut conn = Connection::open_in_memory().expect("打开内存数据库失败");

    println!("\n--- 1. 连接与版本化迁移 ---");
    let applied = migrate(&conn).expect("迁移失败");
    println!(
        "    本次执行 {applied} 条迁移，schema 版本 = {}",
        schema_version(&conn).expect("读取版本失败")
    );
    println!(
        "    再调用一次 migrate = {} 条（已经是最新版本，不会重复建表）",
        migrate(&conn).expect("迁移失败")
    );

    println!("\n--- 2. 插入：参数绑定 + 业务校验 ---");
    let rust_id = add_task(&conn, "学习 Rust 数据库").expect("插入失败");
    let test_id = add_task_with_priority(&conn, "写测试", 1).expect("插入失败");
    add_task_with_priority(&conn, "整理笔记", 3).expect("插入失败");
    println!("    id = {rust_id}（默认优先级 2）、id = {test_id}（优先级 1）");
    match add_task(&conn, "   ") {
        Ok(id) => println!("    不该插入成功：{id}"),
        Err(error) => println!("    空标题在进 SQL 之前就被拒绝：{error}"),
    }
    match add_task(&conn, "学习 Rust 数据库") {
        Ok(id) => println!("    不该插入成功：{id}"),
        Err(error) => println!(
            "    重复标题被数据库 UNIQUE 约束拒绝：is_constraint_violation = {}",
            error.is_constraint_violation()
        ),
    }

    println!("\n--- 3. 查询：类型映射与顺序 ---");
    for task in list_tasks(&conn).expect("查询失败") {
        println!("    {task:?}");
    }
    println!(
        "    未完成（按优先级）= {:?}",
        titles(&list_open_tasks(&conn).expect("查询失败"))
    );
    println!(
        "    标题含 \"Rust\" = {:?}",
        titles(&search_tasks(&conn, "Rust").expect("搜索失败"))
    );
    println!(
        "    优先级 3 = {:?}",
        titles(&list_by_priority(&conn, 3).expect("查询失败"))
    );

    println!("\n--- 4. 更新、删除与统计 ---");
    println!(
        "    complete_task({rust_id}) = {}",
        complete_task(&conn, rust_id).expect("更新失败")
    );
    println!(
        "    rename_task({test_id}, \"写更多测试\") = {}",
        rename_task(&conn, test_id, "写更多测试").expect("更新失败")
    );
    println!(
        "    set_priority({rust_id}, 3) = {}",
        set_priority(&conn, rust_id, 3).expect("更新失败")
    );
    println!(
        "    不存在的 id 返回 false：complete_task(9999) = {}",
        complete_task(&conn, 9999).expect("更新失败")
    );
    println!("    统计 = {:?}", stats(&conn).expect("统计失败"));
    println!(
        "    delete_task({test_id}) = {}，剩余 待办 = {:?}",
        delete_task(&conn, test_id).expect("删除失败"),
        titles(&list_tasks(&conn).expect("查询失败"))
    );

    println!("\n--- 5. 事务：要么全成，要么全不成 ---");
    let ids = add_tasks_transactional(&mut conn, &["备份脚本", "写周报"]).expect("事务失败");
    println!("    批量插入成功：{ids:?}");
    let before = stats(&conn).expect("统计失败").total;
    let failed = add_tasks_transactional(&mut conn, &["新任务", "学习 Rust 数据库"]);
    let after = stats(&conn).expect("统计失败").total;
    println!(
        "    含重复标题的批量插入：is_constraint_violation = {}",
        failed
            .as_ref()
            .err()
            .map(TaskError::is_constraint_violation)
            .unwrap_or(false)
    );
    println!(
        "    回滚后 total = {after}，和插入前的 {before} 相同：{}",
        after == before
    );
    println!("    （第一条 \"新任务\" 也一起回滚了，批量写入不会留下半成品）");

    println!("\n--- 6. 查询计划：索引有没有生效 ---");
    println!(
        "    done = 0   -> {:?}",
        query_plan(&conn, "SELECT id FROM tasks WHERE done = 0").expect("查询计划失败")
    );
    println!(
        "    title LIKE -> {:?}",
        query_plan(&conn, "SELECT id FROM tasks WHERE title LIKE '%Rust%'").expect("查询计划失败")
    );
    println!("    （SEARCH 才是用索引定位；SCAN 表示逐行扫描，% 开头的 LIKE 只能 SCAN）");

    println!("\n========== 数据库操作演示结束 ==========");
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 每个测试都自己开一个内存库，测试之间不共享状态。
    fn memory_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        init(&conn).unwrap();
        conn
    }

    #[test]
    fn migrations_are_tracked_and_idempotent() {
        let conn = Connection::open_in_memory().unwrap();
        assert_eq!(schema_version(&conn).unwrap(), 0); // 新库还没有版本号
        assert_eq!(migrate(&conn).unwrap(), MIGRATIONS.len()); // 全部脚本执行一遍
        assert_eq!(schema_version(&conn).unwrap(), MIGRATIONS.len() as i64);
        assert_eq!(migrate(&conn).unwrap(), 0); // 再跑一次什么都不做
    }

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

    #[test]
    fn crud_roundtrip() {
        let conn = memory_db();
        let id = add_task(&conn, "demo").unwrap();
        assert_eq!(
            list_tasks(&conn).unwrap()[0],
            Task {
                id,
                title: "demo".into(),
                done: false,
                priority: 2,
            }
        );
        assert!(complete_task(&conn, id).unwrap());
        assert!(!complete_task(&conn, id).unwrap(), "第二次没有改到行");
        assert!(list_tasks(&conn).unwrap()[0].done);
        assert!(set_priority(&conn, id, 1).unwrap());
        assert_eq!(list_tasks(&conn).unwrap()[0].priority, 1);
        assert!(rename_task(&conn, id, "  改个名字  ").unwrap());
        assert_eq!(list_tasks(&conn).unwrap()[0].title, "改个名字");
        assert!(delete_task(&conn, id).unwrap());
        assert!(!delete_task(&conn, id).unwrap());
        assert!(list_tasks(&conn).unwrap().is_empty());
    }

    #[test]
    fn validation_errors_never_reach_the_database() {
        let conn = memory_db();
        assert!(matches!(add_task(&conn, "   "), Err(TaskError::EmptyTitle)));
        assert!(matches!(
            add_task_with_priority(&conn, "x", 0),
            Err(TaskError::InvalidPriority(0))
        ));
        assert!(matches!(
            add_task_with_priority(&conn, "x", 4),
            Err(TaskError::InvalidPriority(4))
        ));
        assert!(matches!(
            set_priority(&conn, 1, 9),
            Err(TaskError::InvalidPriority(9))
        ));
        assert_eq!(
            stats(&conn).unwrap(),
            Stats {
                total: 0,
                done: 0,
                open: 0
            }
        );
    }

    #[test]
    fn duplicate_titles_hit_the_unique_constraint() {
        let conn = memory_db();
        add_task(&conn, "写代码").unwrap();

        let error = add_task(&conn, "写代码").unwrap_err();
        assert!(error.is_constraint_violation(), "实际错误：{error}");

        // 首尾空白会被 trim，所以 " 写代码 " 和 "写代码" 是同一个标题。
        assert!(
            add_task(&conn, " 写代码 ")
                .unwrap_err()
                .is_constraint_violation()
        );
        assert_eq!(list_tasks(&conn).unwrap().len(), 1);
    }

    #[test]
    fn transaction_rolls_back_the_whole_batch() {
        let mut conn = memory_db();
        add_task(&conn, "已存在").unwrap();

        let failed = add_tasks_transactional(&mut conn, &["新任务", "已存在"]);
        assert!(failed.is_err(), "整批应当失败");
        assert_eq!(
            titles(&list_tasks(&conn).unwrap()),
            ["已存在"],
            "前一条插入必须一起回滚"
        );
    }

    #[test]
    fn search_order_and_stats() {
        let conn = memory_db();
        let rust_book = add_task_with_priority(&conn, "读 Rust 书", 1).unwrap();
        let rust_code = add_task(&conn, "写 Rust 代码").unwrap();
        add_task_with_priority(&conn, "买菜", 3).unwrap();

        assert_eq!(
            titles(&search_tasks(&conn, "Rust").unwrap()),
            ["读 Rust 书", "写 Rust 代码"]
        );
        assert!(
            search_tasks(&conn, "' OR 1=1 --").unwrap().is_empty(),
            "参数绑定会把引号当普通字符"
        );
        assert_eq!(titles(&list_by_priority(&conn, 3).unwrap()), ["买菜"]);
        assert_eq!(
            titles(&list_open_tasks(&conn).unwrap()),
            ["读 Rust 书", "写 Rust 代码", "买菜"]
        );

        complete_task(&conn, rust_book).unwrap();
        complete_task(&conn, rust_code).unwrap();
        assert_eq!(
            stats(&conn).unwrap(),
            Stats {
                total: 3,
                done: 2,
                open: 1
            }
        );
    }

    #[test]
    fn query_plan_shows_index_usage() {
        let conn = memory_db();
        let indexed = query_plan(&conn, "SELECT id FROM tasks WHERE done = 0").unwrap();
        assert!(
            indexed.iter().any(|line| line.contains("idx_tasks_done")),
            "done = 0 应该走索引，实际计划：{indexed:?}"
        );

        let scanned = query_plan(&conn, "SELECT id FROM tasks WHERE title LIKE '%Rust%'").unwrap();
        assert!(
            scanned.iter().any(|line| line.contains("SCAN")),
            "前缀通配符的 LIKE 只能全表扫描，实际计划：{scanned:?}"
        );
    }
}
