//! 第 19 章配套代码：文件、路径与 IO。
//!
//! 运行方式：`cargo run`，输出接在第 18 章后面。
//!
//! 所有磁盘操作都在**临时目录**里进行：`tempfile::TempDir` 在离开作用域时
//! 自动递归删除，不会在项目里留下垃圾文件。

use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use tempfile::tempdir;
use walkdir::WalkDir;

/// `Path` / `PathBuf` 的常用操作：这些只是字符串层面的处理，不碰磁盘。
fn path_demo() {
    println!("\n--- 1. Path 与 PathBuf ---");

    let path = PathBuf::from("notes/2026/september.md");
    println!("    路径 = {}", path.display());
    println!("    file_name = {:?}", path.file_name().unwrap());
    println!("    extension = {:?}", path.extension().unwrap());
    println!("    parent = {:?}", path.parent().unwrap());
    println!(
        "    with_extension(\"txt\") = {}",
        path.with_extension("txt").display()
    );
    println!("    is_absolute = {}", path.is_absolute());
    println!("    starts_with(\"notes\") = {}", path.starts_with("notes"));

    let dir = PathBuf::from("notes").join("2026");
    println!("    join = {}", dir.join("september.md").display());
    println!("    （Windows 和 Linux 的分隔符不同，PathBuf 会自己处理）");
}

/// 读写文件、缓冲、目录操作：全部在临时目录里演示。
fn io_demo() {
    let temp = tempdir().unwrap();
    let base = temp.path();
    println!("\n--- 2. 临时目录 ---");
    println!("    tempdir() 创建成功，exists = {}", base.exists());
    println!("    （离开作用域时自动删除，所以演示不会污染项目目录）");

    // 写文件
    println!("\n--- 3. 写文件与读文件 ---");
    let note = base.join("note.txt");
    fs::write(&note, "第一行\n第二行\n第三行\n").unwrap();
    let content = fs::read_to_string(&note).unwrap();
    println!("    写入 3 行，读回 {} 字节", content.len());
    println!(
        "    行数 = {}，第一行 = {:?}",
        content.lines().count(),
        content.lines().next().unwrap()
    );

    let bytes = fs::read(&note).unwrap();
    println!(
        "    按字节读回 {} 字节，前 3 个字节 = {:?}",
        bytes.len(),
        &bytes[..3]
    );

    // 追加
    println!("\n--- 4. 追加内容 ---");
    let mut file = OpenOptions::new().append(true).open(&note).unwrap();
    writeln!(file, "第四行").unwrap();
    drop(file);
    let content = fs::read_to_string(&note).unwrap();
    println!("    追加后行数 = {}", content.lines().count());

    // 缓冲读取
    println!("\n--- 5. BufReader：逐行读 ---");
    let reader = BufReader::new(File::open(&note).unwrap());
    let lines: Vec<String> = reader.lines().map(|line| line.unwrap()).collect();
    println!("    lines = {lines:?}");

    // 缓冲写入
    println!("\n--- 6. BufWriter：批量写 ---");
    let output = base.join("out.txt");
    let mut writer = BufWriter::new(File::create(&output).unwrap());
    for index in 0..3 {
        writeln!(writer, "第 {index} 行").unwrap();
    }
    writer.flush().unwrap();
    println!(
        "    out.txt 大小 = {} 字节",
        fs::metadata(&output).unwrap().len()
    );

    // 目录
    println!("\n--- 7. 目录操作 ---");
    fs::create_dir_all(base.join("sub/deep")).unwrap();
    fs::write(base.join("sub/a.txt"), "a").unwrap();
    fs::write(base.join("sub/deep/b.txt"), "bb").unwrap();
    let mut names: Vec<String> = fs::read_dir(base)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    println!("    read_dir(顶层) = {names:?}");

    // 递归遍历
    println!("\n--- 8. WalkDir：递归遍历 ---");
    let mut files: Vec<String> = WalkDir::new(base)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .map(|entry| {
            entry
                .path()
                .strip_prefix(base)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/")
        })
        .collect();
    files.sort();
    println!("    所有文件 = {files:?}");

    // 删除
    println!("\n--- 9. 删除 ---");
    let doomed = base.join("doomed.txt");
    fs::write(&doomed, "临时").unwrap();
    let existed_before = doomed.exists();
    fs::remove_file(&doomed).unwrap();
    println!(
        "    删除前存在 = {existed_before}，删除后存在 = {}",
        doomed.exists()
    );

    // 错误处理
    println!("\n--- 10. 错误处理：看 ErrorKind，别匹配消息 ---");
    let missing = fs::read_to_string(base.join("nope.txt")).unwrap_err();
    println!("    读不存在的文件：kind = {:?}", missing.kind());
    let already = fs::create_dir(base.join("sub")).unwrap_err();
    println!("    重复创建目录：kind = {:?}", already.kind());
    println!("    （错误消息随系统语言变化，判断类型要用 kind()）");

    // 元数据
    println!("\n--- 11. 元数据 ---");
    let meta = fs::metadata(&note).unwrap();
    println!(
        "    note.txt: 是文件 = {}，大小 = {} 字节",
        meta.is_file(),
        meta.len()
    );
    println!("    是否有修改时间 = {}", meta.modified().is_ok());
}

/// 路径相关的常见坑。
fn pitfalls_demo() {
    println!("\n--- 12. 几个坑 ---");
    println!(
        "    ① 路径可能不是 UTF-8：用 OsStr/OsString，展示时用 to_string_lossy() 或 display()"
    );
    println!("    ② 拼接路径用 join，不要手写分隔符（Windows 是反斜杠）");
    println!("    ③ 判断存在用 Path::exists，但它不区分「不存在」和「权限不足」");
    println!("    ④ 大文件别一次 read_to_string，用 BufReader 逐行处理");
    println!("    ⑤ 缓冲写入要 flush（或用 drop），否则末尾可能没落盘");
    println!("    ⑥ 临时文件用 tempfile，别在项目目录里造文件");

    let path = Path::new("data/report.csv");
    println!(
        "    示例：{:?} 的父目录是 {:?}",
        path,
        path.parent().unwrap()
    );
}

/// 演示路径、文件读写、目录遍历与错误处理。
pub fn files_io_demo() {
    println!("\n========== rust19_files_io: 文件、路径与 IO ==========");
    path_demo();
    io_demo();
    pitfalls_demo();
    println!("\n========== 文件、路径与 IO 演示结束 ==========");
}
