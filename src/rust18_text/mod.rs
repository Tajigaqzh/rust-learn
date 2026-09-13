//! 第 18 章配套代码：文本处理与正则。
//!
//! 运行方式：`cargo run`，输出接在第 17 章后面。

use regex::Regex;
use std::borrow::Cow;
use std::sync::LazyLock;
use unicode_segmentation::UnicodeSegmentation;

/// 正则编译很贵，用 `LazyLock` 编译一次、全局复用。
static DATE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(\d{4})-(\d{2})-(\d{2})").unwrap());

/// 日志行：`INFO 14:30:00 服务启动`。
static LOG_LINE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(?P<level>INFO|WARN|ERROR)\s+(?P<time>\d{2}:\d{2}:\d{2})\s+(?P<msg>.*)$")
        .unwrap()
});

/// 连续空白。
static WHITESPACE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s+").unwrap());

/// 清洗文本：去首尾空白、把连续空白压成一个空格。
///
/// 返回 `Cow`：不需要清洗时直接借用原串，避免多余的内存分配。
fn normalize_whitespace(text: &str) -> Cow<'_, str> {
    let trimmed = text.trim();
    let already_clean = trimmed == text && !text.contains("  ");
    if already_clean {
        Cow::Borrowed(text) // 本来就干净：直接借用，零分配
    } else {
        Cow::Owned(WHITESPACE.replace_all(trimmed, " ").into_owned())
    }
}

/// 从日志行里提取（等级、时间、消息）。
fn parse_log_line(line: &str) -> Option<(String, String, String)> {
    let caps = LOG_LINE.captures(line)?;
    Some((
        caps.name("level")?.as_str().to_string(),
        caps.name("time")?.as_str().to_string(),
        caps.name("msg")?.as_str().to_string(),
    ))
}

/// 三种「长度」：字符数、字节数、字素数。
fn counts(text: &str) -> (usize, usize, usize) {
    (
        text.chars().count(),
        text.len(),
        text.graphemes(true).count(),
    )
}

/// 演示 Unicode、`Cow`、正则匹配与提取。
pub fn text_demo() {
    println!("\n========== rust18_text: 文本处理与正则 ==========");

    // 1. 三种长度
    println!("\n--- 1. 字符、字节、字素 ---");
    let ascii = "abc";
    let cjk = "中文abc";
    let emoji = "👨‍👩‍👧‍👦";
    for (name, text) in [("abc", ascii), ("中文abc", cjk), ("一家四口", emoji)] {
        let (chars, bytes, graphemes) = counts(text);
        println!("    {name}: 字符 {chars} / 字节 {bytes} / 字素 {graphemes}");
    }
    println!("    （emoji 由 7 个字符组成，但对用户来说是 1 个字）");

    // 2. Unicode 大小写
    println!("\n--- 2. 大小写与 Unicode ---");
    println!("    \"straße\".to_uppercase() = {}", "straße".to_uppercase());
    println!("    \"中文\".to_uppercase() = {}", "中文".to_uppercase());
    println!("    （大小写转换是按 Unicode 规则做的，可能改变长度）");

    // 3. 空白清洗与 Cow
    println!("\n--- 3. 清洗空白：能借用就不分配 ---");
    let dirty = "  rust   is   fun  ";
    let clean = normalize_whitespace(dirty);
    println!("    清洗结果 = [{clean}]");
    let already_clean = "rust is fun";
    let borrowed = normalize_whitespace(already_clean);
    println!(
        "    本来就干净的输入：是否复用原串 = {}",
        matches!(borrowed, Cow::Borrowed(_))
    );

    // 4. 正则基础
    println!("\n--- 4. 正则基础 ---");
    let text = "日期：2026-09-13，下次：2027-01-05";
    println!("    is_match = {}", DATE.is_match(text));
    println!("    第一个匹配 = {:?}", DATE.find(text).unwrap().as_str());
    for caps in DATE.captures_iter(text) {
        println!("    捕获组：年 {} 月 {} 日 {}", &caps[1], &caps[2], &caps[3]);
    }

    // 5. 命名捕获组与替换
    println!("\n--- 5. 命名捕获组与替换 ---");
    let named = Regex::new(r"(?P<y>\d{4})-(?P<m>\d{2})-(?P<d>\d{2})").unwrap();
    let replaced = named.replace_all("今天是 2026-09-13", "$d/$m/$y");
    println!("    replace_all = {replaced}");

    // 6. 切分
    println!("\n--- 6. 用正则切分 ---");
    let separator = Regex::new(r"\s*,\s*").unwrap();
    let parts: Vec<&str> = separator.split("a, b ,c").collect();
    println!("    split = {parts:?}");

    // 7. 实战：解析日志
    println!("\n--- 7. 实战：解析日志行 ---");
    let lines = [
        "INFO 14:30:00 服务启动",
        "WARN 14:30:05 磁盘使用率 85%",
        "ERROR 14:31:12 连接数据库失败",
        "这不是日志行",
    ];
    for line in lines {
        match parse_log_line(line) {
            Some((level, time, message)) => println!("    {time} [{level}] {message}"),
            None => println!("    跳过无法解析的行：{line}"),
        }
    }

    // 8. 转义与不支持的语法
    println!("\n--- 8. 转义与不支持的语法 ---");
    println!("    把普通文本当模式用：regex::escape(\"a.b*c\") = {}", regex::escape("a.b*c"));
    let lookahead = Regex::new(r"\d+(?=px)").unwrap_err();
    println!("    前瞻不支持：{}", lookahead.to_string().lines().last().unwrap());
    let backref = Regex::new(r"(\w+)\1").unwrap_err();
    println!("    反向引用不支持：{}", backref.to_string().lines().last().unwrap());
    println!("    （Rust 的 regex 走的是「线性时间」路线，故意不支持这些）");

    println!("\n========== 文本处理与正则演示结束 ==========");
}
