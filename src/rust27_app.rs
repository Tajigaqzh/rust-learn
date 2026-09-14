//! 第 27 章配套代码：综合实战——任务管理系统。
//!
//! 运行方式：`cargo run`（输出接在第 26 章后面）。
//!
//! 这一章把前面学过的东西串成一个最小可用的应用：
//!
//! | 层 | 负责什么 | 来自哪一章 |
//! | --- | --- | --- |
//! | 配置 | TOML 文件 + 环境变量覆盖 | 第 20 章 |
//! | 命令行 | `clap` 解析子命令、生成帮助与错误信息 | 第 21 章 |
//! | 服务 | 编排业务动作、决定错误类型 | 本章 |
//! | 仓储 | SQLite 增删改查与迁移 | 第 26 章 |
//! | 展示 | 纯文本 / JSON 渲染、退出码 | 本章 |
//!
//! 演示不会去读真实的 `std::env::args()`，而是用固定的参数数组调用
//! [`parse_args`]——这样 `cargo run` 的演示和 `cargo test` 走的是同一条路径。

use crate::rust26_database::{
    Stats, Task, TaskError, add_task, complete_task, delete_task, init, list_open_tasks,
    list_tasks, search_tasks, stats,
};
use clap::{Arg, ArgAction, Command as ClapCommand};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// 输出格式。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Format {
    /// 给人看的纯文本。
    #[default]
    Plain,
    /// 给程序看的 JSON。
    Json,
}

/// 应用配置：可以由 TOML 文件提供，再被环境变量覆盖。
///
/// `deny_unknown_fields` 让拼错的键直接报错，而不是被静默忽略——
/// 配置里最贵的 bug 就是「写了但没生效」。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct AppConfig {
    /// 数据库文件路径；`None` 表示用内存库（演示与测试用）。
    #[serde(default)]
    pub database: Option<PathBuf>,
    /// 默认输出格式。
    #[serde(default)]
    pub format: Format,
}

/// 应用错误：把「用法错误」和「业务/数据库错误」分开，才能映射成不同退出码。
#[derive(Debug)]
pub enum AppError {
    /// 命令行用法错误（参数缺失、子命令不存在……）。
    Usage(String),
    /// 业务或数据库失败，直接复用第 26 章的错误类型。
    Task(TaskError),
    /// 内部错误（例如序列化失败），属于「不该发生但必须处理」的一类。
    Internal(String),
}

impl std::fmt::Display for AppError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppError::Usage(message) => write!(formatter, "用法错误：{message}"),
            AppError::Task(error) => write!(formatter, "{error}"),
            AppError::Internal(message) => write!(formatter, "内部错误：{message}"),
        }
    }
}

impl std::error::Error for AppError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            AppError::Task(error) => Some(error),
            _ => None,
        }
    }
}

impl From<TaskError> for AppError {
    fn from(error: TaskError) -> Self {
        AppError::Task(error)
    }
}

/// 退出码约定：0 成功、2 用法错误、1 业务失败、70 内部错误。
///
/// 退出码是 CLI 契约的一部分：脚本和 CI 靠它判断「该重试还是该修参数」。
pub fn exit_code(error: &AppError) -> i32 {
    match error {
        AppError::Usage(_) => 2,
        AppError::Task(_) => 1,
        AppError::Internal(_) => 70,
    }
}

/// 解析后的子命令。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Add { title: String },
    List { all: bool },
    Done { id: i64 },
    Delete { id: i64 },
    Search { keyword: String },
}

/// 全局选项。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Options {
    pub format: Format,
    pub database: Option<PathBuf>,
}

/// 一次完整的命令行输入。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cli {
    pub options: Options,
    pub action: Action,
}

/// 命令行定义：子命令 + 两个全局选项。
fn build_cli() -> ClapCommand {
    ClapCommand::new("tasks")
        .about("任务管理（第 27 章综合实战）")
        .arg(
            Arg::new("format")
                .long("format")
                .value_parser(["plain", "json"])
                .default_value("plain")
                .global(true)
                .help("输出格式"),
        )
        .arg(
            Arg::new("db")
                .long("db")
                .global(true)
                .help("SQLite 文件路径；省略时用内存库"),
        )
        .subcommand(
            ClapCommand::new("add")
                .about("添加任务")
                .arg(Arg::new("title").required(true).help("任务标题")),
        )
        .subcommand(
            ClapCommand::new("list").about("列出任务").arg(
                Arg::new("all")
                    .long("all")
                    .action(ArgAction::SetTrue)
                    .help("连已完成的任务一起列出"),
            ),
        )
        .subcommand(
            ClapCommand::new("done").about("标记完成").arg(
                Arg::new("id")
                    .required(true)
                    .value_parser(clap::value_parser!(i64))
                    .help("任务 id"),
            ),
        )
        .subcommand(
            ClapCommand::new("delete").about("删除任务").arg(
                Arg::new("id")
                    .required(true)
                    .value_parser(clap::value_parser!(i64))
                    .help("任务 id"),
            ),
        )
        .subcommand(
            ClapCommand::new("search")
                .about("按关键字搜索")
                .arg(Arg::new("keyword").required(true).help("关键字")),
        )
}

/// 把参数数组解析成 [`Cli`]。
///
/// 参数写成切片而不是直接读 `std::env::args()`：**同一份解析逻辑既能被
/// 程序入口调用，也能被测试和本章演示调用**，而且不依赖进程状态。
pub fn parse_args(args: &[&str]) -> Result<Cli, AppError> {
    let matches = build_cli()
        .try_get_matches_from(args.iter().copied())
        .map_err(|error| AppError::Usage(first_line(&error.to_string())))?;

    let format = match matches.get_one::<String>("format").map(String::as_str) {
        Some("json") => Format::Json,
        _ => Format::Plain,
    };
    let database = matches.get_one::<String>("db").map(PathBuf::from);

    let (name, sub) = matches.subcommand().ok_or_else(|| {
        AppError::Usage(String::from(
            "缺少子命令，可用：add / list / done / delete / search",
        ))
    })?;

    let missing = |field: &str| AppError::Usage(format!("缺少参数 {field}"));
    let action = match name {
        "add" => Action::Add {
            title: sub
                .get_one::<String>("title")
                .cloned()
                .ok_or_else(|| missing("title"))?,
        },
        "list" => Action::List {
            all: sub.get_flag("all"),
        },
        "done" => Action::Done {
            id: *sub.get_one::<i64>("id").ok_or_else(|| missing("id"))?,
        },
        "delete" => Action::Delete {
            id: *sub.get_one::<i64>("id").ok_or_else(|| missing("id"))?,
        },
        "search" => Action::Search {
            keyword: sub
                .get_one::<String>("keyword")
                .cloned()
                .ok_or_else(|| missing("keyword"))?,
        },
        other => return Err(AppError::Usage(format!("未知子命令：{other}"))),
    };

    Ok(Cli {
        options: Options { format, database },
        action,
    })
}

/// 取多行错误信息的第一行，避免把 clap 的用法提示整个塞进日志字段。
fn first_line(text: &str) -> String {
    text.lines().next().unwrap_or(text).trim().to_string()
}

/// 配置分层：先读 TOML，再用环境变量覆盖。
///
/// 环境变量优先级最高，是容器和 CI 里最常见的注入方式；
/// 参数格式错误时返回错误，而不是悄悄用默认值。
pub fn resolve_config(
    toml_text: &str,
    env_database: Option<&str>,
    env_format: Option<&str>,
) -> Result<AppConfig, AppError> {
    let mut config: AppConfig = toml::from_str(toml_text)
        .map_err(|error| AppError::Usage(format!("配置解析失败：{error}")))?;

    if let Some(value) = env_database {
        config.database = Some(PathBuf::from(value));
    }
    if let Some(value) = env_format {
        config.format = match value.trim().to_ascii_lowercase().as_str() {
            "plain" => Format::Plain,
            "json" => Format::Json,
            other => {
                return Err(AppError::Usage(format!(
                    "TASKS_FORMAT 只支持 plain / json，收到 {other}"
                )));
            }
        };
    }
    Ok(config)
}

/// 服务层：持有数据库连接，编排业务动作。
///
/// 它是唯一知道「数据存在哪里」的类型；CLI 层只拿到它的方法，不碰 SQL。
pub struct TaskService {
    conn: Connection,
    database: Option<PathBuf>,
}

impl TaskService {
    /// 打开内存库（演示与测试用）。
    pub fn open_in_memory() -> Result<Self, TaskError> {
        let conn = Connection::open_in_memory().map_err(TaskError::from)?;
        init(&conn)?;
        Ok(Self {
            conn,
            database: None,
        })
    }

    /// 打开文件库，并在启动时把 schema 迁移到最新版本（第 26 章）。
    pub fn open_file(path: &Path) -> Result<Self, TaskError> {
        let conn = Connection::open(path).map_err(TaskError::from)?;
        init(&conn)?;
        Ok(Self {
            conn,
            database: Some(path.to_path_buf()),
        })
    }

    /// 按配置建服务：有路径就开文件库，否则用内存库。
    pub fn from_config(config: &AppConfig) -> Result<Self, TaskError> {
        match &config.database {
            Some(path) => Self::open_file(path),
            None => Self::open_in_memory(),
        }
    }

    /// 数据库位置，用于启动日志。
    pub fn database(&self) -> Option<&Path> {
        self.database.as_deref()
    }

    pub fn add(&self, title: &str) -> Result<i64, TaskError> {
        add_task(&self.conn, title)
    }

    pub fn complete(&self, id: i64) -> Result<bool, TaskError> {
        complete_task(&self.conn, id)
    }

    pub fn delete(&self, id: i64) -> Result<bool, TaskError> {
        delete_task(&self.conn, id)
    }

    /// `all = false` 时只列未完成任务。
    pub fn list(&self, all: bool) -> Result<Vec<Task>, TaskError> {
        if all {
            list_tasks(&self.conn)
        } else {
            list_open_tasks(&self.conn)
        }
    }

    pub fn search(&self, keyword: &str) -> Result<Vec<Task>, TaskError> {
        search_tasks(&self.conn, keyword)
    }

    pub fn stats(&self) -> Result<Stats, TaskError> {
        stats(&self.conn)
    }
}

/// 输出视图：只把要暴露给外部的字段交给 serde。
///
/// 直接给内部类型加上 `Serialize` 很省事，但会让「数据库字段」和
/// 「对外契约」永久绑在一起：以后加一列，接口就悄悄变了。
#[derive(Debug, Serialize)]
struct TaskView<'a> {
    id: i64,
    title: &'a str,
    done: bool,
    priority: i64,
}

impl TaskView<'_> {
    fn from_task(task: &Task) -> TaskView<'_> {
        TaskView {
            id: task.id,
            title: &task.title,
            done: task.done,
            priority: task.priority,
        }
    }
}

/// 纯文本渲染：`[x] 1: 标题`。
pub fn render_plain(tasks: &[Task]) -> String {
    if tasks.is_empty() {
        return String::from("（没有任务）");
    }
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

/// 按指定格式渲染任务列表。
pub fn render_tasks(tasks: &[Task], format: Format) -> Result<String, AppError> {
    match format {
        Format::Plain => Ok(render_plain(tasks)),
        Format::Json => {
            let views: Vec<TaskView<'_>> = tasks.iter().map(TaskView::from_task).collect();
            serde_json::to_string_pretty(&views)
                .map_err(|error| AppError::Internal(format!("序列化失败：{error}")))
        }
    }
}

/// 执行一条命令，返回要打印给用户的文本。
pub fn execute(service: &TaskService, cli: &Cli) -> Result<String, AppError> {
    match &cli.action {
        Action::Add { title } => {
            let id = service.add(title)?;
            Ok(format!("已添加 #{id}：{title}"))
        }
        Action::List { all } => render_tasks(&service.list(*all)?, cli.options.format),
        Action::Done { id } => {
            if service.complete(*id)? {
                Ok(format!("已完成 #{id}"))
            } else {
                Ok(format!("#{id} 不存在或已经是完成状态"))
            }
        }
        Action::Delete { id } => {
            if service.delete(*id)? {
                Ok(format!("已删除 #{id}"))
            } else {
                Ok(format!("#{id} 不存在"))
            }
        }
        Action::Search { keyword } => {
            let tasks = service.search(keyword)?;
            if tasks.is_empty() {
                return Ok(format!("没有匹配 \"{keyword}\" 的任务"));
            }
            render_tasks(&tasks, cli.options.format)
        }
    }
}

/// 演示用的临时数据库路径：放在系统临时目录，跑完删掉。
fn demo_database_path() -> PathBuf {
    std::env::temp_dir().join(format!("rust-learn-ch27-{}.sqlite", std::process::id()))
}

/// 给多行输出统一加缩进，让演示的层次看得出来。
fn indent(text: &str) -> String {
    text.lines()
        .map(|line| format!("    {line}"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// 把「参数数组 → 服务 → 输出」这条链路跑一遍，并打印结果或错误码。
fn run_line(service: &TaskService, args: &[&str]) -> Result<String, AppError> {
    let cli = parse_args(args)?;
    execute(service, &cli)
}

/// 演示配置分层、命令行解析、服务编排、输出格式与退出码。
pub fn task_app_demo() {
    println!("\n========== rust27_app: 任务管理系统 ==========");

    println!("\n--- 1. 配置分层：TOML + 环境变量覆盖 ---");
    let toml_text = r#"
database = "tasks.sqlite"
format = "plain"
"#;
    let config = resolve_config(toml_text, None, None).expect("配置解析失败");
    println!("    TOML 里的配置：{config:?}");
    let overridden =
        resolve_config(toml_text, Some("ci.sqlite"), Some("json")).expect("环境变量覆盖失败");
    println!("    环境变量覆盖后：{overridden:?}");
    match resolve_config(toml_text, None, Some("xml")) {
        Ok(config) => println!("    不该成功：{config:?}"),
        Err(error) => println!("    非法格式被拒绝：{error}"),
    }

    println!("\n--- 2. 命令行解析（用固定参数，不读真实 argv） ---");
    let cli = parse_args(&["tasks", "--format", "json", "add", "写周报"]).expect("解析失败");
    println!("    解析结果：{cli:?}");
    match parse_args(&["tasks", "done", "abc"]) {
        Ok(cli) => println!("    不该成功：{cli:?}"),
        Err(error) => println!("    非法 id 被拒绝：{error}"),
    }

    println!("\n--- 3. 服务编排：一条命令一个动作 ---");
    let service = TaskService::open_in_memory().expect("打开内存库失败");
    for args in [
        vec!["tasks", "add", "完成综合实战"],
        vec!["tasks", "add", "写测试"],
        vec!["tasks", "done", "2"],
        vec!["tasks", "list", "--all"],
    ] {
        let echo = args.join(" ").trim_end().to_string();
        match run_line(&service, &args) {
            Ok(text) => println!("    $ {echo}\n{}", indent(&text)),
            Err(error) => println!(
                "    $ {echo}\n    失败（退出码 {}）：{error}",
                exit_code(&error)
            ),
        }
    }

    println!("\n--- 4. 只列未完成 + JSON 输出 ---");
    println!(
        "{}",
        indent(&run_line(&service, &["tasks", "list"]).expect("列出失败"))
    );
    println!(
        "{}",
        indent(&run_line(&service, &["tasks", "--format", "json", "list"]).expect("JSON 输出失败"))
    );

    println!("\n--- 5. 错误路径：业务错误与用法错误 ---");
    for args in [
        vec!["tasks", "add", "   "],
        vec!["tasks", "delete", "9999"],
        vec!["tasks", "search", "不存在"],
    ] {
        let echo = args.join(" ").trim_end().to_string();
        match run_line(&service, &args) {
            Ok(text) => println!("    $ {echo}\n{}", indent(&text)),
            Err(error) => println!("    $ {echo}\n    退出码 {}：{error}", exit_code(&error)),
        }
    }
    println!("    统计 = {:?}", service.stats().expect("统计失败"));

    println!("\n--- 6. 持久化：配置给路径，重开后数据还在 ---");
    let path = demo_database_path();
    let _ = std::fs::remove_file(&path);
    let config = AppConfig {
        database: Some(path.clone()),
        format: Format::Plain,
    };
    {
        let service = TaskService::from_config(&config).expect("按配置打开文件库失败");
        println!(
            "    数据库 = {}",
            service.database().unwrap_or(Path::new("(内存)")).display()
        );
        println!(
            "{}",
            indent(&run_line(&service, &["tasks", "add", "落盘的任务"]).expect("添加失败"))
        );
    } // 连接在这里关闭
    let reopened = TaskService::from_config(&config).expect("按配置重开失败");
    println!(
        "    重开后的列表：\n{}",
        indent(&run_line(&reopened, &["tasks", "list", "--all"]).expect("列出失败"))
    );
    drop(reopened);
    let _ = std::fs::remove_file(&path); // 演示不留垃圾文件
    println!("    已清理临时数据库：{}", path.display());

    println!("\n========== 任务管理系统演示结束 ==========");
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn service() -> TaskService {
        TaskService::open_in_memory().unwrap()
    }

    #[test]
    fn config_prefers_environment_over_toml() {
        let toml_text = "database = \"tasks.sqlite\"\nformat = \"plain\"\n";
        let from_toml = resolve_config(toml_text, None, None).unwrap();
        assert_eq!(from_toml.database, Some(PathBuf::from("tasks.sqlite")));
        assert_eq!(from_toml.format, Format::Plain);

        let from_env = resolve_config(toml_text, Some("ci.sqlite"), Some("JSON")).unwrap();
        assert_eq!(from_env.database, Some(PathBuf::from("ci.sqlite")));
        assert_eq!(from_env.format, Format::Json);

        // 空 TOML 也能用：字段都有默认值
        assert_eq!(
            resolve_config("", None, None).unwrap(),
            AppConfig::default()
        );
    }

    #[test]
    fn config_rejects_unknown_keys_and_bad_values() {
        let unknown = "unknown-key = 1\n";
        assert!(matches!(
            resolve_config(unknown, None, None),
            Err(AppError::Usage(_))
        ));
        assert!(matches!(
            resolve_config("", None, Some("xml")),
            Err(AppError::Usage(_))
        ));
    }

    #[test]
    fn cli_parsing_covers_subcommands_and_global_options() {
        let cli = parse_args(&["tasks", "--db", "x.sqlite", "add", "写周报"]).unwrap();
        assert_eq!(cli.options.database, Some(PathBuf::from("x.sqlite")));
        assert_eq!(cli.options.format, Format::Plain);
        assert_eq!(
            cli.action,
            Action::Add {
                title: String::from("写周报")
            }
        );

        // 全局选项放在子命令后面同样生效
        let cli = parse_args(&["tasks", "list", "--all", "--format", "json"]).unwrap();
        assert_eq!(cli.action, Action::List { all: true });
        assert_eq!(cli.options.format, Format::Json);
    }

    #[test]
    fn cli_reports_usage_errors() {
        assert!(matches!(parse_args(&["tasks"]), Err(AppError::Usage(_)))); // 缺子命令
        assert!(matches!(
            parse_args(&["tasks", "done", "abc"]),
            Err(AppError::Usage(_))
        )); // id 不是整数
        assert!(matches!(
            parse_args(&["tasks", "--format", "xml", "list"]),
            Err(AppError::Usage(_))
        )); // 非法格式
    }

    #[test]
    fn service_runs_the_full_flow() {
        let service = service();
        let first = service.add("完成综合实战").unwrap();
        let second = service.add("写测试").unwrap();

        assert!(service.complete(second).unwrap());
        assert!(!service.complete(second).unwrap(), "重复标记应返回 false");
        assert_eq!(service.list(false).unwrap().len(), 1, "默认只列未完成任务");
        assert_eq!(service.list(true).unwrap().len(), 2);
        assert!(service.delete(first).unwrap());
        assert!(!service.delete(first).unwrap());
        assert_eq!(
            service.stats().unwrap(),
            Stats {
                total: 1,
                done: 1,
                open: 0
            }
        );
    }

    #[test]
    fn execute_renders_every_action() {
        let service = service();
        let add = parse_args(&["tasks", "add", "写周报"]).unwrap();
        assert_eq!(execute(&service, &add).unwrap(), "已添加 #1：写周报");

        let done = parse_args(&["tasks", "done", "1"]).unwrap();
        assert_eq!(execute(&service, &done).unwrap(), "已完成 #1");

        let again = execute(&service, &done).unwrap();
        assert_eq!(again, "#1 不存在或已经是完成状态");

        let search = parse_args(&["tasks", "search", "周报"]).unwrap();
        assert_eq!(execute(&service, &search).unwrap(), "[x] 1: 写周报");

        let missing = parse_args(&["tasks", "search", "找不到"]).unwrap();
        assert_eq!(
            execute(&service, &missing).unwrap(),
            "没有匹配 \"找不到\" 的任务"
        );
    }

    #[test]
    fn plain_rendering_handles_empty_and_unicode() {
        assert_eq!(render_plain(&[]), "（没有任务）");
        let service = service();
        service.add("读《Rust 程序设计语言》").unwrap();
        assert_eq!(
            render_plain(&service.list(true).unwrap()),
            "[ ] 1: 读《Rust 程序设计语言》"
        );
    }

    #[test]
    fn json_rendering_uses_a_dedicated_view() {
        let service = service();
        service.add("写周报").unwrap();
        let text = render_tasks(&service.list(true).unwrap(), Format::Json).unwrap();
        let value: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(
            value,
            json!([{ "id": 1, "title": "写周报", "done": false, "priority": 2 }])
        );
    }

    #[test]
    fn exit_codes_distinguish_usage_business_and_internal() {
        assert_eq!(exit_code(&AppError::Usage(String::from("bad"))), 2);
        assert_eq!(exit_code(&AppError::Task(TaskError::EmptyTitle)), 1);
        assert_eq!(exit_code(&AppError::Internal(String::from("boom"))), 70);
    }

    #[test]
    fn file_backed_service_keeps_data_after_reopen() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tasks.sqlite");

        {
            let service = TaskService::open_file(&path).unwrap();
            service.add("落盘的任务").unwrap();
        }
        let reopened = TaskService::open_file(&path).unwrap();
        assert_eq!(
            render_plain(&reopened.list(true).unwrap()),
            "[ ] 1: 落盘的任务"
        );
    }
}
