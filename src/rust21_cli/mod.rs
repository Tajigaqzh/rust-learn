//! 第 21 章配套代码：命令行工具。
//!
//! 运行方式：`cargo run`，输出接在第 20 章后面。
//!
//! 这里的命令行解析用的是 clap 的 **builder API**（`Command::new(...)`）。
//! 更常见的 derive API 需要 `clap` 的 `derive` feature，正文 21.3 会给出对照。

use clap::{Arg, ArgAction, Command};
use tracing::{debug, info, warn};
use tracing_subscriber::fmt;

/// 统计结果。
#[derive(Debug, PartialEq)]
struct Counts {
    lines: usize,
    words: usize,
    chars: usize,
}

/// 统计文本的行数、词数、字符数。
fn count(text: &str) -> Counts {
    Counts {
        lines: text.lines().count(),
        words: text.split_whitespace().count(),
        chars: text.chars().count(),
    }
}

/// 按指定格式渲染统计结果。
fn render(counts: &Counts, format: &str) -> String {
    match format {
        "json" => format!(
            "{{\"lines\":{},\"words\":{},\"chars\":{}}}",
            counts.lines, counts.words, counts.chars
        ),
        _ => format!(
            "{} 行，{} 个词，{} 个字符",
            counts.lines, counts.words, counts.chars
        ),
    }
}

/// 构建命令行定义：参数、选项、标志与子命令。
fn build_cli() -> Command {
    Command::new("wc-lite")
        .version("1.0.0")
        .about("统计文本的行数 / 词数 / 字符数")
        .arg(
            Arg::new("file")
                .help("要统计的文件；省略时从标准输入读")
                .required(false),
        )
        .arg(
            Arg::new("lines")
                .short('l')
                .long("lines")
                .action(ArgAction::SetTrue)
                .help("只统计行数"),
        )
        .arg(
            Arg::new("format")
                .long("format")
                .value_parser(["plain", "json"])
                .default_value("plain")
                .help("输出格式"),
        )
        .arg(
            Arg::new("verbose")
                .short('v')
                .long("verbose")
                .action(ArgAction::SetTrue)
                .help("打开详细日志"),
        )
        .subcommand(Command::new("info").about("显示构建信息"))
}

/// 演示参数解析、帮助信息、错误输出与日志。
pub fn cli_demo() {
    println!("\n========== rust21_cli: 命令行工具 ==========");

    // 1. 解析参数
    println!("\n--- 1. 解析参数 ---");
    let matches = build_cli()
        .try_get_matches_from(["wc-lite", "--lines", "--format", "json", "notes.txt"])
        .unwrap();
    println!("    file    = {:?}", matches.get_one::<String>("file"));
    println!("    --lines = {}", matches.get_flag("lines"));
    println!("    --format= {:?}", matches.get_one::<String>("format"));
    println!("    --verbose = {}", matches.get_flag("verbose"));

    // 2. 默认值
    println!("\n--- 2. 默认值 ---");
    let defaults = build_cli().try_get_matches_from(["wc-lite"]).unwrap();
    println!(
        "    什么都没传时：format = {:?}，file = {:?}",
        defaults.get_one::<String>("format"),
        defaults.get_one::<String>("file")
    );

    // 3. 子命令
    println!("\n--- 3. 子命令 ---");
    let with_sub = build_cli()
        .try_get_matches_from(["wc-lite", "info"])
        .unwrap();
    println!("    subcommand = {:?}", with_sub.subcommand_name());

    // 4. 帮助信息
    println!("\n--- 4. --help 的输出 ---");
    let help = build_cli()
        .try_get_matches_from(["wc-lite", "--help"])
        .unwrap_err();
    println!("{help}");

    // 5. 错误：未知参数与非法取值
    println!("--- 5. 参数写错时 ---");
    let unknown = build_cli()
        .try_get_matches_from(["wc-lite", "--nope"])
        .unwrap_err();
    println!("{unknown}");
    let invalid = build_cli()
        .try_get_matches_from(["wc-lite", "--format", "xml"])
        .unwrap_err();
    println!("{invalid}");
    println!("    （clap 默认用退出码 2 表示「参数错误」）");

    // 6. 真正跑一遍业务
    println!("\n--- 6. 跑一遍：统计文本 ---");
    let text = "rust is fast\nsafe and productive\n";
    let counts = count(text);
    println!("    纯文本格式：{}", render(&counts, "plain"));
    println!("    JSON 格式：{}", render(&counts, "json"));

    // 7. 日志
    println!("\n--- 7. 日志：分级 + 结构化字段 ---");
    fmt()
        .without_time()
        .with_target(false)
        .with_max_level(tracing::Level::INFO)
        .init();
    info!("开始统计文本");
    debug!("这条 debug 日志默认不显示（级别不够）");
    warn!(file = "notes.txt", "文件比较大，可能有点慢");
    println!("    （日志走 stderr，不污染 stdout 的数据输出）");

    // 8. 退出码
    println!("\n--- 8. 退出码 ---");
    println!("    0：成功（main 正常返回或 ExitCode::SUCCESS）");
    println!("    1：运行时错误（返回 Result::Err 时标准库会给出 1）");
    println!("    2：参数错误（clap 的约定，和大多数 Unix 工具一致）");
    println!("    130：被 Ctrl-C 中断（128 + SIGINT）");

    println!("\n========== 命令行工具演示结束 ==========");
}
