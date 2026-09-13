//! 第 8 章配套代码：错误处理。
//!
//! 运行方式：`cargo run`，输出接在第 7 章后面。

use std::error::Error;
use std::fmt;
use std::num::ParseIntError;

/// 自定义错误类型：把「可能出的几种错」写成枚举。
#[derive(Debug)]
enum ConfigError {
    MissingKey(String),
    NotANumber { key: String, source: ParseIntError },
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::MissingKey(key) => write!(f, "缺少配置项 `{key}`"),
            ConfigError::NotANumber { key, source } => {
                write!(f, "配置项 `{key}` 不是数字：{source}")
            }
        }
    }
}

impl Error for ConfigError {
    /// 指出这一层的错误是由哪个底层错误引起的。
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            ConfigError::NotANumber { source, .. } => Some(source),
            ConfigError::MissingKey(_) => None,
        }
    }
}

impl From<ParseIntError> for ConfigError {
    fn from(source: ParseIntError) -> Self {
        ConfigError::NotANumber {
            key: String::from("value"),
            source,
        }
    }
}

/// 从形如 `key = value` 的多行文本里取值；找不到就返回错误。
fn read_field<'a>(text: &'a str, key: &str) -> Result<&'a str, ConfigError> {
    for line in text.lines() {
        if let Some((k, v)) = line.split_once('=') {
            if k.trim() == key {
                return Ok(v.trim());
            }
        }
    }
    Err(ConfigError::MissingKey(key.to_string()))
}

/// `?` 会把 `ParseIntError` 自动转成 `ConfigError`（因为实现了 `From`）。
fn parse_port(text: &str) -> Result<u16, ConfigError> {
    let raw = read_field(text, "port")?;
    let port: u16 = raw.parse()?;
    Ok(port)
}

/// 可恢复的错误用 `Result` 返回，而不是 panic。
fn divide(a: i32, b: i32) -> Result<i32, String> {
    if b == 0 {
        Err(String::from("除数不能为 0"))
    } else {
        Ok(a / b)
    }
}

/// 统一用 `Box<dyn Error>` 当错误类型，签名简单，适合应用程序入口。
fn double(text: &str) -> Result<i32, Box<dyn Error>> {
    let n: i32 = text.trim().parse()?;
    Ok(n * 2)
}

/// 用 `Option` 表达「可能没有」，再决定要不要升级成错误。
fn find_user(users: &[&str], name: &str) -> Option<usize> {
    users.iter().position(|user| *user == name)
}

/// 演示 `Result`、`?`、自定义错误与 panic 的边界。
pub fn errors_demo() {
    println!("\n========== rust08_errors: 错误处理 ==========");

    // 1. 可恢复错误用 Result 表达
    println!("\n--- 1. 用 Result 返回错误 ---");
    println!("    divide(10, 2) = {:?}", divide(10, 2));
    println!("    divide(10, 0) = {:?}", divide(10, 0));

    // 2. match：两种结果都处理
    println!("\n--- 2. match 处理 Result ---");
    match divide(9, 3) {
        Ok(value) => println!("    9 / 3 = {value}"),
        Err(message) => println!("    出错了：{message}"),
    }
    match divide(9, 0) {
        Ok(value) => println!("    9 / 0 = {value}"),
        Err(message) => println!("    出错了：{message}"),
    }

    // 3. unwrap 家族
    println!("\n--- 3. unwrap 家族 ---");
    let ok: Result<i32, String> = Ok(7);
    let err: Result<i32, String> = Err(String::from("坏掉了"));
    println!("    unwrap = {}", ok.clone().unwrap());
    println!("    unwrap_or 兜底 = {}", err.clone().unwrap_or(-1));
    println!(
        "    unwrap_or_else 兜底 = {}",
        err.clone().unwrap_or_else(|message| message.len() as i32)
    );
    println!("    is_ok / is_err = {} / {}", ok.is_ok(), err.is_err());
    println!("    （err.unwrap() 会 panic：called `Result::unwrap()` on an `Err` value）");

    // 4. ? 运算符：一路传播
    println!("\n--- 4. ? 运算符传播错误 ---");
    let good = "host = localhost\nport = 8080";
    let bad_number = "host = localhost\nport = abc";
    let missing = "host = localhost";
    println!("    合法配置：{:?}", parse_port(good));
    println!("    端口不是数字：{}", parse_port(bad_number).unwrap_err());
    println!("    缺少端口：{}", parse_port(missing).unwrap_err());

    // 5. 自定义错误的层级
    println!("\n--- 5. 自定义错误与 source ---");
    let error = parse_port(bad_number).unwrap_err();
    println!("    错误描述 = {error}");
    println!("    Debug 输出 = {error:?}");
    match error.source() {
        Some(source) => println!("    底层错误 = {source}"),
        None => println!("    没有底层错误"),
    }

    // 6. Box<dyn Error>
    println!("\n--- 6. Box<dyn Error>：一种更省事的签名 ---");
    println!("    double(\" 21 \") = {:?}", double(" 21 "));
    println!(
        "    double(\"abc\") = {:?}",
        double("abc").map_err(|e| e.to_string())
    );

    // 7. Option 与 Result 互转
    println!("\n--- 7. Option 与 Result 互转 ---");
    let users = ["ada", "linus"];
    println!("    find_user(ada) = {:?}", find_user(&users, "ada"));
    let found = find_user(&users, "grace").ok_or("没有这个人");
    println!("    ok_or 之后 = {:?}", found);
    let found2 =
        find_user(&users, "grace").ok_or_else(|| format!("名单 {} 里没有 grace", users.len()));
    println!("    ok_or_else 之后 = {:?}", found2);
    println!("    Ok(3).ok() = {:?}", Ok::<i32, String>(3).ok());

    // 8. 被忽略的 Result
    println!("\n--- 8. 忽略 Result 会被警告 ---");
    println!("    （写 \"42\".parse::<i32>(); 会得到 unused `Result` that must be used，");
    println!("      确实想忽略就显式写 let _ = ...；这条警告是 #[must_use] 的功劳）");

    // 9. 什么时候该 panic
    println!("\n--- 9. panic 的边界 ---");
    println!("    divide(1, 0) 返回 Err，调用方可以重试或提示用户；");
    println!("    而 v[99]、\"abc\".parse::<i32>().unwrap() 这类是 panic，程序直接结束。");
    println!("    经验法则：可预期的失败用 Result，程序自身写错了才 panic。");

    println!("\n========== 错误处理演示结束 ==========");
}
