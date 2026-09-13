//! 第 20 章配套代码：序列化与配置。
//!
//! 运行方式：`cargo run`，输出接在第 19 章后面。

use serde::{Deserialize, Serialize};

/// 应用配置。
///
/// 三个 serde 属性各有用途：
/// - `rename_all = "kebab-case"`：字段名转成 `app-name` 这种形式；
/// - `deny_unknown_fields`：配置里出现不认识的字段就报错（避免拼错键被忽略）；
/// - 字段上的 `default`：缺少该字段时用默认值补上。
#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
struct Config {
    app_name: String,
    #[serde(default = "default_port")]
    port: u16,
    #[serde(default)]
    debug: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    log_path: Option<String>,
}

fn default_port() -> u16 {
    8080
}

/// 后端类型：用 `tag` 让枚举在 JSON / TOML 里带上类型标记。
#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "lowercase")]
enum Backend {
    Memory,
    File { path: String },
}

/// 用环境变量覆盖配置里的端口（模拟「环境变量优先级最高」）。
fn apply_env(mut config: Config, env_port: Option<String>) -> Result<Config, String> {
    if let Some(raw) = env_port {
        config.port = raw
            .trim()
            .parse()
            .map_err(|error| format!("PORT 不是合法端口：{error}"))?;
    }
    Ok(config)
}

/// 演示 serde 的序列化、反序列化、配置分层与错误信息。
pub fn serde_demo() {
    println!("\n========== rust20_serde: 序列化与配置 ==========");

    // 1. JSON 序列化
    println!("\n--- 1. JSON 序列化 ---");
    let config = Config {
        app_name: String::from("rust-learn"),
        port: 8080,
        debug: true,
        log_path: None,
    };
    println!("    紧凑：{}", serde_json::to_string(&config).unwrap());
    println!("    美化：\n{}", serde_json::to_string_pretty(&config).unwrap());
    println!("    （log-path 是 None，被 skip_serializing_if 跳过了）");

    // 2. 往返
    println!("\n--- 2. 往返一致 ---");
    let json = serde_json::to_string(&config).unwrap();
    let back: Config = serde_json::from_str(&json).unwrap();
    println!("    反序列化后与原值相等 = {}", back == config);

    // 3. TOML 配置与默认值
    println!("\n--- 3. TOML 配置：缺省值 ---");
    let minimal = "app-name = \"tiny\"\n";
    let from_minimal: Config = toml::from_str(minimal).unwrap();
    println!("    只写 app-name 时：{from_minimal:?}");

    let file_text = "app-name = \"rust-learn\"\nport = 3000\ndebug = false\n";
    let from_file: Config = toml::from_str(file_text).unwrap();
    println!("    配置文件里：port = {}", from_file.port);
    println!("    序列化回 TOML：\n{}", toml::to_string_pretty(&from_file).unwrap());

    // 4. 配置分层：默认值 < 文件 < 环境变量
    println!("\n--- 4. 配置分层 ---");
    let merged = apply_env(from_file, Some(String::from("9000"))).unwrap();
    println!("    环境变量覆盖后 port = {}", merged.port);
    let bad = apply_env(merged, Some(String::from("abc")));
    println!("    非法环境变量 = {bad:?}");

    // 5. 动态 JSON
    println!("\n--- 5. 动态 JSON：json! 与 Value ---");
    let value = serde_json::json!({
        "name": "ada",
        "age": 36,
        "tags": ["rust", "systems"],
    });
    println!("    json! = {value}");
    println!("    value[\"name\"] = {}", value["name"]);
    println!("    value[\"tags\"][1] = {}", value["tags"][1]);
    println!("    不存在的键返回 Null，不会 panic = {}", value["missing"].is_null());

    // 6. 带标签的枚举
    println!("\n--- 6. 带标签的枚举 ---");
    let memory = Backend::Memory;
    let file = Backend::File {
        path: String::from("/var/data.db"),
    };
    println!("    Memory -> {}", serde_json::to_string(&memory).unwrap());
    println!("    File   -> {}", serde_json::to_string(&file).unwrap());
    let parsed: Backend =
        serde_json::from_str(r#"{"kind":"file","path":"/tmp/x.db"}"#).unwrap();
    println!("    解析回来 = {parsed:?}");

    // 7. 错误信息
    println!("\n--- 7. 解析失败时能看到什么 ---");
    let cases: [(&str, &str); 4] = [
        ("缺少逗号", r#"{ "app-name": "x" "port": 1 }"#),
        ("类型不对", r#"{ "app-name": "x", "port": "abc" }"#),
        (
            "未知字段",
            r#"{ "app-name": "x", "port": 1, "extra": 2 }"#,
        ),
        ("缺少必填字段", r#"{ "port": 1 }"#),
    ];
    for (label, text) in cases {
        match serde_json::from_str::<Config>(text) {
            Ok(_) => println!("    {label}：居然解析成功了"),
            Err(error) => println!("    {label}：{error}"),
        }
    }
    match toml::from_str::<Config>("app-name = \"x\"\nport = \"abc\"\n") {
        Ok(_) => println!("    TOML 类型错：居然解析成功了"),
        Err(error) => println!("    TOML 类型错：\n{error}"),
    }

    println!("\n========== 序列化与配置演示结束 ==========");
}
