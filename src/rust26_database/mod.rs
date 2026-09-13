//! 第 26 章：SQLite 数据库操作。
use rusqlite::{Connection, Result, params};

#[derive(Debug, PartialEq)]
pub struct Task {
    pub id: i64,
    pub title: String,
    pub done: bool,
}

pub fn init(conn: &Connection) -> Result<()> {
    conn.execute_batch("CREATE TABLE IF NOT EXISTS tasks (id INTEGER PRIMARY KEY, title TEXT NOT NULL, done INTEGER NOT NULL DEFAULT 0);")
}
pub fn add_task(conn: &Connection, title: &str) -> Result<i64> {
    conn.execute("INSERT INTO tasks (title) VALUES (?1)", params![title])?;
    Ok(conn.last_insert_rowid())
}

/// 在一个事务中批量插入任务；任意一次插入失败都会回滚全部变更。
pub fn add_tasks_transactional(conn: &mut Connection, titles: &[&str]) -> Result<Vec<i64>> {
    let tx = conn.transaction()?;
    let mut ids = Vec::with_capacity(titles.len());
    for title in titles {
        tx.execute("INSERT INTO tasks (title) VALUES (?1)", params![title])?;
        ids.push(tx.last_insert_rowid());
    }
    tx.commit()?;
    Ok(ids)
}
pub fn list_tasks(conn: &Connection) -> Result<Vec<Task>> {
    let mut stmt = conn.prepare("SELECT id,title,done FROM tasks ORDER BY id")?;
    stmt.query_map([], |r| {
        Ok(Task {
            id: r.get(0)?,
            title: r.get(1)?,
            done: r.get::<_, i64>(2)? != 0,
        })
    })?
    .collect()
}
pub fn complete_task(conn: &Connection, id: i64) -> Result<bool> {
    Ok(conn.execute("UPDATE tasks SET done=1 WHERE id=?1", params![id])? == 1)
}
pub fn delete_task(conn: &Connection, id: i64) -> Result<bool> {
    Ok(conn.execute("DELETE FROM tasks WHERE id=?1", params![id])? == 1)
}
pub fn search_tasks(conn: &Connection, keyword: &str) -> Result<Vec<Task>> {
    let pattern = format!("%{keyword}%");
    let mut stmt =
        conn.prepare("SELECT id,title,done FROM tasks WHERE title LIKE ?1 ORDER BY id")?;
    stmt.query_map([pattern], |r| {
        Ok(Task {
            id: r.get(0)?,
            title: r.get(1)?,
            done: r.get::<_, i64>(2)? != 0,
        })
    })?
    .collect()
}
pub fn database_demo() {
    println!("\n========== rust26_database: SQLite 数据库操作 ==========");
    let mut conn = Connection::open_in_memory().expect("打开内存数据库失败");
    init(&conn).expect("建表失败");
    let id = add_tasks_transactional(&mut conn, &["学习 Rust 数据库"]).expect("插入失败")[0];
    complete_task(&conn, id).expect("更新失败");
    println!("任务：{:?}", list_tasks(&conn).expect("查询失败"));
    println!("搜索：{:?}", search_tasks(&conn, "Rust").expect("搜索失败"));
    println!("删除成功：{}", delete_task(&conn, id).expect("删除失败"));
    println!("========== 数据库操作演示结束 ==========");
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn crud_works() {
        let c = Connection::open_in_memory().unwrap();
        init(&c).unwrap();
        let id = add_task(&c, "demo").unwrap();
        assert_eq!(
            list_tasks(&c).unwrap()[0],
            Task {
                id,
                title: "demo".into(),
                done: false
            }
        );
        assert!(complete_task(&c, id).unwrap());
        assert!(list_tasks(&c).unwrap()[0].done);
        assert!(delete_task(&c, id).unwrap());
    }

    #[test]
    fn search_filters_titles() {
        let c = Connection::open_in_memory().unwrap();
        init(&c).unwrap();
        add_task(&c, "read book").unwrap();
        add_task(&c, "write code").unwrap();
        assert_eq!(search_tasks(&c, "book").unwrap().len(), 1);
    }

    #[test]
    fn transaction_inserts_batch() {
        let mut c = Connection::open_in_memory().unwrap();
        init(&c).unwrap();
        let ids = add_tasks_transactional(&mut c, &["one", "two"]).unwrap();
        assert_eq!(ids.len(), 2);
        assert_eq!(list_tasks(&c).unwrap().len(), 2);
    }
}
