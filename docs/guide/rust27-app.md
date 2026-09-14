# 第 27 章 · 综合实战：任务管理系统

前面 26 章是「一个主题一章」，这一章把其中几样拼成一个真的能用的东西：
一个命令行任务管理工具。它不追求功能多，而是把**分层、错误类型、配置、
退出码、测试**这几件工程上的事情一次做齐。

配套代码只有一个文件，但里面按职责分了层：

| 层 | 负责什么 | 依赖 |
| --- | --- | --- |
| 配置 `AppConfig` | TOML 文件 + 环境变量覆盖 | `serde` + `toml`（第 20 章） |
| 命令行 | `clap` 子命令、帮助、错误信息 | `clap`（第 21 章） |
| 服务 `TaskService` | 编排业务动作、持有数据库连接 | 第 26 章的函数 |
| 仓储 | SQL、迁移、约束、类型映射 | `rusqlite`（第 26 章） |
| 展示 | 纯文本 / JSON、退出码 | `serde_json`、本章 |

```bash
cargo run                                              # 跑本仓库的完整演示
cargo test --bin rust-learn-demo rust27_app                 # 只跑本章的 10 条测试
```

## 27.1 分层的意义：谁负责什么

分层不是为了「看起来专业」，而是为了让每一层**单独可测**：

```text
参数数组 ──parse_args──▶ Cli{options, action}
                              │
                              ▼
                        execute(service, cli)
                              │
                    ┌─────────┴─────────┐
                    ▼                   ▼
             TaskService            render_tasks
           （业务 + 数据库）        （纯文本 / JSON）
```

对应到代码里，就是几句话：

- **`parse_args` 只认字符串切片**，不读 `std::env::args()`，所以测试和演示能
  用任意参数调用它；
- **`TaskService` 是唯一知道数据库在哪里的类型**，`execute` 只调它的方法，
  一行 SQL 都不写；
- **`render_tasks` 是纯函数**，给定任务列表和格式就有确定输出，最好测；
- **`AppError` 把三类失败分开**，于是退出码可以精确定义。

## 27.2 配置：TOML 打底，环境变量覆盖

```rust
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
```

`deny_unknown_fields` 是这里最值钱的一个属性：配置里写错一个键名时**直接报错**，
而不是静默忽略、让你在生产上排查「为什么我配了却没生效」。

叠加顺序只有两步，但顺序本身就是契约：

```rust
pub fn resolve_config(
    toml_text: &str,
    env_database: Option<&str>,
    env_format: Option<&str>,
) -> Result<AppConfig, AppError> {
    let mut config: AppConfig = toml::from_str(toml_text)?;   // ① 文件打底
    if let Some(value) = env_database { config.database = Some(PathBuf::from(value)); }
    if let Some(value) = env_format { config.format = parse_format(value)?; } // ② 环境变量覆盖
    Ok(config)
}
```

| 来源 | 优先级 | 典型用途 |
| --- | --- | --- |
| 默认值（`#[serde(default)]`） | 最低 | 让配置可以只写关心的字段 |
| TOML 文件 | 中 | 本地开发、随镜像分发的默认配置 |
| 环境变量 | 最高 | 容器 / CI / K8s 注入，改一个变量就能换环境 |

演示里的实测输出：

```text
--- 1. 配置分层：TOML + 环境变量覆盖 ---
    TOML 里的配置：AppConfig { database: Some("tasks.sqlite"), format: Plain }
    环境变量覆盖后：AppConfig { database: Some("ci.sqlite"), format: Json }
    非法格式被拒绝：用法错误：TASKS_FORMAT 只支持 plain / json，收到 xml
```

注意第三行：**格式不认识时要报错，不要悄悄回退到默认值**。这类「静默降级」
在配置系统里是最难查的 bug 来源之一。

## 27.3 命令行：可测试的 `clap` 用法

```rust
fn build_cli() -> ClapCommand {
    ClapCommand::new("tasks")
        .about("任务管理（第 27 章综合实战）")
        .arg(Arg::new("format").long("format")
            .value_parser(["plain", "json"]).default_value("plain").global(true))
        .arg(Arg::new("db").long("db").global(true))
        .subcommand(ClapCommand::new("add").arg(Arg::new("title").required(true)))
        .subcommand(ClapCommand::new("list").arg(Arg::new("all").long("all").action(ArgAction::SetTrue)))
        .subcommand(ClapCommand::new("done").arg(Arg::new("id").required(true)
            .value_parser(clap::value_parser!(i64))))
        // delete / search 同上
        // …
}

pub fn parse_args(args: &[&str]) -> Result<Cli, AppError> {
    let matches = build_cli()
        .try_get_matches_from(args.iter().copied())
        .map_err(|error| AppError::Usage(first_line(&error.to_string())))?;
    // 把 clap 的 Matches 翻译成自己的 Cli / Action
    // …
}
```

三个设计决定值得抄走：

1. **函数签名收 `&[&str]`**。真实入口负责把 `std::env::args()` 传进来，测试和
   演示传自己的数组——`try_get_matches_from` 本来就是为这种用法设计的。
2. **不把 `clap::Matches` 传到业务层**。`Matches` 是「命令行库的类型」，
   翻译成自己的 `Action` 之后，`execute` 完全不依赖 clap；
3. **错误信息只取第一行**。clap 的完整报错带用法提示，塞进日志字段太长，
   所以用 `first_line` 截断。

实测输出：

```text
--- 2. 命令行解析（用固定参数，不读真实 argv） ---
    解析结果：Cli { options: Options { format: Json, database: None }, action: Add { title: "写周报" } }
    非法 id 被拒绝：用法错误：error: invalid value 'abc' for '<id>': invalid digit found in string
```

`value_parser(clap::value_parser!(i64))` 让 clap 在解析阶段就把「不是整数」
挡下来，业务层拿到的 id 一定是合法数字——**把校验放在能最早发现错误的地方**。

## 27.4 服务层：连接的生命周期

```rust
pub struct TaskService {
    conn: Connection,
    database: Option<PathBuf>,
}

impl TaskService {
    pub fn open_file(path: &Path) -> Result<Self, TaskError> {
        let conn = Connection::open(path).map_err(TaskError::from)?;
        init(&conn)?;                       // 启动即迁移到最新 schema（第 26 章）
        Ok(Self { conn, database: Some(path.to_path_buf()) })
    }

    pub fn from_config(config: &AppConfig) -> Result<Self, TaskError> {
        match &config.database {
            Some(path) => Self::open_file(path),
            None => Self::open_in_memory(),
        }
    }

    pub fn add(&self, title: &str) -> Result<i64, TaskError> { add_task(&self.conn, title) }
    // list / complete / delete / search / stats …
}
```

几个要点：

- **连接在服务里**，方法用 `&self`：SQLite 的读接口只需要共享引用，
  唯一需要 `&mut self` 的是「开事务」（第 26 章的 `add_tasks_transactional`）；
- **迁移在打开时执行**，所以「先建表再插数据」这件事不需要调用方记得；
- **服务不打印任何东西**，它只返回数据或错误。打印是展示层的事，
  这样同一份服务既能给 CLI 用，也能给 HTTP handler 或测试用。

演示里对服务的完整调用链：

```text
--- 3. 服务编排：一条命令一个动作 ---
    $ tasks add 完成综合实战
    已添加 #1：完成综合实战
    $ tasks add 写测试
    已添加 #2：写测试
    $ tasks done 2
    已完成 #2
    $ tasks list --all
    [ ] 1: 完成综合实战
    [x] 2: 写测试
```

## 27.5 输出格式与退出码

纯文本给人看，JSON 给程序看。两者共用同一份数据，只是渲染方式不同：

```rust
/// 输出视图：只把要暴露给外部的字段交给 serde。
#[derive(Debug, Serialize)]
struct TaskView<'a> {
    id: i64,
    title: &'a str,
    done: bool,
    priority: i64,
}

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
```

**为什么不直接给 `Task` 加 `Serialize`？** 因为那样「数据库列」和「对外契约」
就被永久绑在一起了：以后加一列（比如内部用的重试次数），接口就会悄悄多一个字段。
用一个专门的视图类型，改动是显式的。

实测输出（`tasks --format json list` 在只列未完成时）：

```text
--- 4. 只列未完成 + JSON 输出 ---
    [ ] 1: 完成综合实战
    [
      {
        "id": 1,
        "title": "完成综合实战",
        "done": false,
        "priority": 2
      }
    ]
```

退出码是 CLI 对外的另一份契约：

| 退出码 | 含义 | 触发例子 |
| --- | --- | --- |
| `0` | 成功 | 正常返回 |
| `1` | 业务 / 数据库失败 | 空标题、数据库报错 |
| `2` | 用法错误 | 参数写错、子命令不存在（clap 的约定） |
| `70` | 内部错误 | 序列化失败这类「不该发生」的情况 |

```rust
pub fn exit_code(error: &AppError) -> i32 {
    match error {
        AppError::Usage(_) => 2,
        AppError::Task(_) => 1,
        AppError::Internal(_) => 70,
    }
}
```

实测的错误路径输出：

```text
--- 5. 错误路径：业务错误与用法错误 ---
    $ tasks add
    退出码 1：任务标题不能为空
    $ tasks delete 9999
    #9999 不存在
    $ tasks search 不存在
    没有匹配 "不存在" 的任务
    统计 = Stats { total: 2, done: 1, open: 1 }
```

注意「删除不存在的任务」返回的是 **成功**（退出码 0）而不是错误：对 `delete`
来说结果是确定的（删完之后它确实不存在），这叫**幂等**。真正需要报错的是
「参数不合法」和「系统故障」——把这两类和「业务上没找到」区分开，
脚本才好写重试逻辑。

## 27.6 持久化：配置给的路径说了算

演示最后把内存库换成文件库，并在关闭连接后重新打开：

```text
--- 6. 持久化：配置给路径，重开后数据还在 ---
    数据库 = <临时目录>\rust-learn-ch27-<pid>.sqlite
    已添加 #1：落盘的任务
    重开后的列表：
    [ ] 1: 落盘的任务
    已清理临时数据库：<临时目录>\rust-learn-ch27-<pid>.sqlite
```

三件事同时被验证了：

1. `TaskService::from_config` 按配置选择内存库还是文件库；
2. 重新打开时 `init` 会读 `PRAGMA user_version`，已经是最新版本就什么都不做
   （第 26 章的迁移幂等性）；
3. 演示用临时目录并在结束时删除文件，**测试和演示都不在仓库里留垃圾**。

生产环境当然不该用 `%TEMP%`：路径应该来自配置（第 27.2 节），并且要考虑
备份、权限和磁盘配额——这些属于第 28 章的部署话题。

## 27.7 测试闭环

本章 10 条测试，按层次分布在「纯函数 → 服务 → 文件」三档：

```text
$ cargo test --bin rust-learn-demo rust27_app
running 10 tests
test rust27_app::tests::cli_parsing_covers_subcommands_and_global_options ... ok
test rust27_app::tests::cli_reports_usage_errors ... ok
test rust27_app::tests::config_prefers_environment_over_toml ... ok
test rust27_app::tests::config_rejects_unknown_keys_and_bad_values ... ok
test rust27_app::tests::execute_renders_every_action ... ok
test rust27_app::tests::exit_codes_distinguish_usage_business_and_internal ... ok
test rust27_app::tests::file_backed_service_keeps_data_after_reopen ... ok
test rust27_app::tests::json_rendering_uses_a_dedicated_view ... ok
test rust27_app::tests::plain_rendering_handles_empty_and_unicode ... ok
test rust27_app::tests::service_runs_the_full_flow ... ok
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

| 测试目标 | 用什么替代外部依赖 | 覆盖的边界 |
| --- | --- | --- |
| 配置 | 直接传 TOML 字符串 + 环境变量参数 | 缺省值、覆盖、非法值、未知键 |
| 命令行 | 传参数数组 | 子命令、全局选项位置、非法 id、非法格式 |
| 服务 | 内存库 | 重复完成、删除不存在、只列未完成 |
| 渲染 | 直接构造字符串断言 | 空列表、Unicode 标题、JSON 结构 |
| 退出码 | 直接比较数字 | 三类错误各一条 |
| 持久化 | `tempfile::tempdir()` | 重开后数据与 schema 版本 |

这套测试里最值得注意的是**没有任何一处需要真实终端或真实用户输入**：
`parse_args` 收参数切片、`resolve_config` 收字符串、服务收连接——
依赖全从外面传进来，测试自然就好写（第 23 章讲的依赖注入，在综合项目里的样子）。

## 27.8 常见坑速查

| 现象 | 原因 | 解法 |
| --- | --- | --- |
| 配置写了但没生效 | 键名拼错，被静默忽略 | 加 `deny_unknown_fields`，让解析直接报错 |
| 环境变量没覆盖掉文件里的值 | 覆盖顺序写反了 | 固定为「默认 → 文件 → 环境变量」，并写测试 |
| 业务层到处 `unwrap` | 错误类型没定义 | 定义 `AppError`，用 `?` 传播 |
| 退出码永远是 0 | `main` 里吞掉了错误 | 用 `ExitCode`/`process::exit` 返回错误码 |
| JSON 输出多了内部字段 | 直接给领域类型加了 `Serialize` | 用专门的视图类型（`TaskView`） |
| 输出被日志污染 | 日志写 stdout，和结果混在一起 | 日志走 stderr，结果走 stdout（第 21 章） |
| 测试之间互相影响 | 共用了一个数据库或文件 | 每个测试一个内存库或临时目录 |
| 演示在仓库里留下文件 | 用了固定路径 | 放临时目录并在结束时删除 |
| 数据库路径硬编码 | 路径没做成配置 | 从 TOML / 环境变量读，服务只管用 |
| 「删除不存在」被当成失败 | 把幂等操作和错误混为一谈 | 想清楚语义，返回「删了 0 行」而不是报错 |

## 27.9 练习

1. 给 `Action` 加一个 `Rename { id, title }`，接上第 26 章的 `rename_task`，
   并为「重命名成功」「id 不存在」各写一条测试。
2. 给 `AppConfig` 加一个 `default-priority` 字段（默认 2），让 `add` 用它，
   并验证「TOML 里改成 1 → 新任务是高优先级」。
3. 让 `main` 真正用上这套代码：读取真实 `std::env::args()`，用 `ExitCode` 返回
   `exit_code(&error)`；参数错误时把用法提示写到 stderr。
4. 把 `execute` 拆成「服务层返回结构化结果（例如 `Outcome` 枚举）」+「渲染层
   负责转文本」，并说明这样拆之后 JSON 输出需要改哪些地方。
5. 回答两个问题：(a) 为什么 `parse_args` 收参数切片而不是直接读
   `std::env::args()`？(b) 为什么给 `Task` 直接加 `Serialize` 是个坏主意？

（第 5 题是 27.3 与 27.5 讨论过的内容，这里不再单独给答案。）

### 参考答案

::: details 第 1 题

```rust
// Action 里加一个变体
Action::Rename { id: i64, title: String },

// clap 侧
ClapCommand::new("rename")
    .about("重命名任务")
    .arg(Arg::new("id").required(true).value_parser(clap::value_parser!(i64)))
    .arg(Arg::new("title").required(true)),

// execute 里
Action::Rename { id, title } => {
    if service.rename(*id, title)? {
        Ok(format!("已重命名 #{id} 为 {title}"))
    } else {
        Ok(format!("#{id} 不存在"))
    }
}
```

服务侧加一个方法复用第 26 章的 `rename_task`：

```rust
pub fn rename(&self, id: i64, title: &str) -> Result<bool, TaskError> {
    crate::rust26_database::rename_task(&self.conn, id, title)
}
```

测试可以这样写：

```rust
#[test]
fn rename_reports_missing_id() {
    let service = service();
    let id = service.add("旧名字").unwrap();
    assert!(service.rename(id, "新名字").unwrap());
    assert_eq!(service.list(true).unwrap()[0].title, "新名字");
    assert!(!service.rename(9999, "无所谓").unwrap());
}
```

注意 `rename` 和 `delete` 一样用 `bool` 表示「是否真的改到了行」，
和「操作是否失败」区分开——这是第 26 章 26.8 讨论过的那条语义。

:::

::: details 第 2 题

```rust
pub struct AppConfig {
    #[serde(default)]
    pub database: Option<PathBuf>,
    #[serde(default)]
    pub format: Format,
    /// 新任务的默认优先级：1 最高，3 最低。
    #[serde(default = "default_priority")]
    pub default_priority: i64,
}

fn default_priority() -> i64 {
    2
}
```

服务里改用带优先级的插入（第 26 章的 `add_task_with_priority`），
并把配置传进构造函数；测试断言：

```rust
let config = resolve_config("default-priority = 1\n", None, None).unwrap();
assert_eq!(config.default_priority, 1);
```

一个小提醒：优先级范围校验已经在第 26 章的 `validate_priority` 里了，
配置层不用重复实现，只要在启动时跑一次「试插一条再回滚」或者直接信任服务层
返回的 `TaskError` 即可——**校验逻辑只留一份**。

:::

::: details 第 3 题

```rust
fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let refs: Vec<&str> = args.iter().map(String::as_str).collect();

    let cli = match parse_args(&refs) {
        Ok(cli) => cli,
        Err(error) => {
            eprintln!("{error}");                 // 用法提示走 stderr
            return std::process::ExitCode::from(exit_code(&error) as u8);
        }
    };

    let config = resolve_config("", std::env::var("TASKS_DB").ok().as_deref(),
        std::env::var("TASKS_FORMAT").ok().as_deref());
    // …用 config 建服务、执行命令，把 Ok 的文本打到 stdout，Err 走 stderr
    std::process::ExitCode::SUCCESS
}
```

要点是**成功信息走 stdout、错误和日志走 stderr**：前者是数据，可以被管道和
重定向消费；后者是给人看的诊断信息。`exit_code` 把 `AppError` 映射成数字，
调用方（shell 脚本、CI）据此决定重试还是报警。

:::

## 27.10 小结

- 分层要落到「谁能被单独测试」上：`parse_args` 收参数切片、`resolve_config`
  收字符串、`TaskService` 持有连接、`render_tasks` 是纯函数。
- 配置按「默认 → TOML → 环境变量」叠加，写错键名要报错（`deny_unknown_fields`），
  非法值不要静默降级。
- 命令行解析留在最外层，业务层不依赖 `clap` 的类型；能用 `value_parser` 挡住的
  错误，不要留到业务层再判断。
- 错误分三类（用法 / 业务 / 内部）并映射到不同退出码：`2` / `1` / `70`，
  `0` 表示成功。幂等操作「没找到」不算失败。
- 输出用专门的视图类型，别把领域类型的字段直接暴露成 API。
- 服务打开连接时就执行迁移，所以「内存库 → 文件库」只是配置里的一个字段之差。
- 本章 10 条测试覆盖配置、解析、服务、渲染、退出码和文件持久化，
  全程不需要真实终端、真实网络或真实用户输入。
- 本仓库实测：`cargo test --bin rust-learn-demo rust27_app` 10 条全通过；
  `cargo run` 能看到配置覆盖、命令编排、JSON 输出、错误退出码和重开文件库后的数据。

下一章是**第 28 章 部署、监控与性能优化**：把程序交付出去之后，怎么知道它还活着、
快不快、有没有变慢。
