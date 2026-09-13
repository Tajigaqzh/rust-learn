//! 第 22 章配套代码：网络与 HTTP。
//!
//! 运行方式：`cargo run`，输出接在第 21 章后面。
//!
//! 全部演示都在**本机回环地址**（127.0.0.1）上进行：自己起服务器、自己发请求，
//! 不依赖外网，也不受网络环境影响。

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::time::Duration;

/// 起一个「回显」TCP 服务器：收到一行就回一行。
fn spawn_echo_server() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap().to_string();

    thread::spawn(move || {
        if let Ok((mut stream, _)) = listener.accept() {
            let clone = stream.try_clone().unwrap();
            let mut reader = BufReader::new(clone);
            let mut line = String::new();
            if reader.read_line(&mut line).is_ok() {
                let _ = writeln!(stream, "echo: {}", line.trim());
                let _ = stream.flush();
            }
        }
    });

    addr
}

/// 起一个最小的 HTTP 服务器：只回一个 JSON，`delay` 用来演示超时。
fn spawn_http_server(delay: Option<Duration>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();

    thread::spawn(move || {
        if let Ok((mut stream, _)) = listener.accept() {
            let mut reader = BufReader::new(stream.try_clone().unwrap());

            // 读请求行和头部
            let mut request_line = String::new();
            let _ = reader.read_line(&mut request_line);
            let mut content_length = 0usize;
            loop {
                let mut line = String::new();
                if reader.read_line(&mut line).unwrap_or(0) == 0 || line.trim().is_empty() {
                    break;
                }
                if let Some(value) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                    content_length = value.trim().parse().unwrap_or(0);
                }
            }

            // 读请求体（如果有）
            let mut body = vec![0u8; content_length];
            if content_length > 0 {
                let _ = reader.read_exact(&mut body);
            }

            if let Some(delay) = delay {
                thread::sleep(delay);
            }

            // 用 serde_json 构造响应体，保证引号被正确转义
            let payload = serde_json::json!({
                "method": request_line.split_whitespace().next().unwrap_or("?"),
                "body": String::from_utf8_lossy(&body).trim(),
            })
            .to_string();

            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                payload.len(),
                payload
            );
            let _ = stream.write_all(response.as_bytes());
            let _ = stream.flush();
        }
    });

    format!("http://{addr}/")
}

/// 带重试的 GET：失败时最多重试 `attempts` 次。
fn retry_get(url: &str, attempts: u32) -> Result<u16, reqwest::Error> {
    let mut last_error = None;
    for attempt in 1..=attempts {
        match reqwest::blocking::get(url) {
            Ok(response) => return Ok(response.status().as_u16()),
            Err(error) => {
                println!("        第 {attempt} 次失败：{error}");
                last_error = Some(error);
                thread::sleep(Duration::from_millis(20));
            }
        }
    }
    Err(last_error.unwrap())
}

/// 演示原始 TCP、HTTP 请求、超时、重试与并发。
pub fn network_demo() {
    println!("\n========== rust22_network: 网络与 HTTP ==========");

    // 1. 原始 TCP
    println!("\n--- 1. 原始 TCP：连上、发字节、读字节 ---");
    let addr = spawn_echo_server();
    let mut stream = TcpStream::connect(&addr).unwrap();
    stream.write_all(b"hello network\n").unwrap();
    stream.flush().unwrap();
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let mut reply = String::new();
    reader.read_line(&mut reply).unwrap();
    println!("    服务器回应：{}", reply.trim());
    println!("    （TCP 只是「可靠的双向字节流」，HTTP 是建立在它之上的文本协议）");

    // 2. 手写一次 HTTP 请求
    println!("\n--- 2. 手写 HTTP 请求 ---");
    let url = spawn_http_server(None);
    let host = url
        .trim_start_matches("http://")
        .trim_end_matches('/')
        .to_string();
    let mut stream = TcpStream::connect(&host).unwrap();
    stream
        .write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
        .unwrap();
    let mut raw = String::new();
    stream.read_to_string(&mut raw).unwrap();
    println!("    收到 {} 字节", raw.len());
    println!("    状态行 = {}", raw.lines().next().unwrap_or(""));
    println!(
        "    响应体 = {}",
        raw.split("\r\n\r\n").nth(1).unwrap_or("")
    );

    // 3. reqwest：GET
    println!("\n--- 3. reqwest：GET 请求 ---");
    let url = spawn_http_server(None);
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_millis(500))
        .build()
        .unwrap();
    let response = client.get(&url).send().unwrap();
    println!("    状态 = {}", response.status());
    println!("    响应体 = {}", response.text().unwrap());

    // 4. reqwest：POST JSON
    println!("\n--- 4. reqwest：POST JSON ---");
    let url = spawn_http_server(None);
    let payload = serde_json::json!({ "name": "ada", "age": 36 });
    let echoed: serde_json::Value = client
        .post(&url)
        .json(&payload)
        .send()
        .unwrap()
        .json()
        .unwrap();
    println!("    服务器回显 = {echoed}");
    println!("    （请求体里的引号被正确转义，这说明 JSON 是逐字节传输的）");

    // 5. 超时
    println!("\n--- 5. 超时 ---");
    // 客户端超时是 500ms，这里故意让服务器慢 700ms
    let url = spawn_http_server(Some(Duration::from_millis(700)));
    match client.get(&url).send() {
        Ok(response) => println!("    居然成功了：{}", response.status()),
        Err(error) => println!("    请求失败：is_timeout = {}，{error}", error.is_timeout()),
    }

    // 6. 连接失败与重试
    println!("\n--- 6. 连接失败与重试 ---");
    let error = TcpStream::connect("127.0.0.1:1").unwrap_err();
    println!("    连接不存在的主机端口：kind = {:?}", error.kind());
    match retry_get("http://127.0.0.1:1/", 3) {
        Ok(status) => println!("    重试成功：{status}"),
        Err(error) => println!("    重试 3 次仍然失败：{error}"),
    }

    // 7. 并发请求
    println!("\n--- 7. 并发请求 ---");
    let url = spawn_http_server(None);
    let handle = thread::spawn(move || {
        let response = reqwest::blocking::get(&url).unwrap();
        response.status().as_u16()
    });
    println!("    子线程请求的状态码 = {}", handle.join().unwrap());
    println!("    （每个线程一个阻塞客户端；高并发场景应该用异步客户端）");

    println!("\n--- 8. 现实中的异步版本 ---");
    println!("    reqwest 也提供 async 版：client.get(url).send().await?");
    println!("    需要配合 tokio 运行时（第 16 章），并支持连接池、超时、重试中间件");
    println!("    （本仓库为了离线可编译，只用了 blocking feature；外网请求未在本书验证）");

    println!("\n========== 网络与 HTTP 演示结束 ==========");
}
