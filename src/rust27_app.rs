//! 第 27 章：综合实战任务管理系统。
use crate::rust26_database::{add_task, complete_task, init, list_tasks};
use rusqlite::Connection;

pub fn render_tasks(tasks: &[crate::rust26_database::Task]) -> String {
    tasks
        .iter()
        .map(|task| {
            format!(
                "[{}] {}: {}",
                if task.done { 'x' } else { ' ' },
                task.id,
                task.title
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn run_task_flow() -> rusqlite::Result<String> {
    let conn = Connection::open_in_memory()?;
    init(&conn)?;
    let id = add_task(&conn, "完成综合实战")?;
    complete_task(&conn, id)?;
    Ok(render_tasks(&list_tasks(&conn)?))
}

pub fn task_app_demo() {
    println!("\n========== rust27_app: 任务管理系统 ==========");
    println!("{}", run_task_flow().expect("任务流程失败"));
    println!("========== 任务管理系统演示结束 ==========");
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rendering_is_stable() {
        let c = Connection::open_in_memory().unwrap();
        init(&c).unwrap();
        let id = add_task(&c, "demo").unwrap();
        complete_task(&c, id).unwrap();
        assert_eq!(render_tasks(&list_tasks(&c).unwrap()), "[x] 1: demo");
    }

    #[test]
    fn flow_returns_rendered_tasks() {
        assert_eq!(run_task_flow().unwrap(), "[x] 1: 完成综合实战");
    }
}
