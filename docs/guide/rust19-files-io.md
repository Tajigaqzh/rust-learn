# 第 19 章 · 文件、路径与 IO

这一章是程序真正开始接触外部世界的地方：读写文件、处理路径、标准输入输出。两条原则贯穿全章：

1. **路径用 `Path` / `PathBuf`，不要用字符串拼**——手写分隔符在 Windows 和 Linux 上行为不同。
2. **IO 操作全都返回 `Result`，必须处理错误**——磁盘满了、文件被占用、权限不足，都是常态（第 8 章的错误处理在这里天天用到）。

配套代码在 `src/rust19_files_io/mod.rs`。所有磁盘操作都在**临时目录**里进行，不会污染项目目录。本章用到两个 crate：

```toml
[dependencies]
tempfile = "3"     # 临时目录 / 临时文件，离开作用域自动删除
walkdir = "2"      # 递归遍历目录
```

## 19.1 `Path` 与 `PathBuf`

它们的关系和 `&str` / `String` 一样（第 5 章）：

| 类型 | 类比 | 含义 |
| --- | --- | --- |
| `Path` | `&str` | 借用的路径切片 |
| `PathBuf` | `String` | 拥有所有权的路径，可增长 |

```rust
use std::path::{Path, PathBuf};

let path = PathBuf::from("notes/2026/september.md");
path.file_name();          // Some("september.md")
path.extension();          // Some("md")
path.parent();             // Some("notes/2026")
path.with_extension("txt"); // notes/2026/september.txt
path.is_absolute();        // false
```

实测：

```text
    路径 = notes/2026/september.md
    file_name = "september.md"
    extension = "md"
    parent = "notes/2026"
    with_extension("txt") = notes/2026/september.txt
    is_absolute = false
    starts_with("notes") = true
    join = notes\2026\september.md
```

几个要点：

- **这些操作不碰磁盘**，纯粹是字符串层面的解析。所以你可以对「还不存在的路径」做 `parent()`、`file_name()`。
- `file_name()`、`parent()` 返回 `Option`：根目录没有父目录，`..` 也没有文件名。**该处理就处理，不要无脑 `unwrap`**。
- `join` 负责拼分隔符：在 Windows 上实测输出 `notes\2026\september.md`，在 Linux 上会是 `notes/2026/september.md`，**代码本身不用改**。
- 想拿到字符串：`path.display()`（用于打印，能容忍非 UTF-8）、`path.to_str()`（返回 `Option<&str>`，非 UTF-8 时是 `None`）、`to_string_lossy()`（有损转换，替换掉非法字节）。

## 19.2 路径与文件的几个坑

**① 路径不一定是 UTF-8。** Linux 上文件名可以是任意字节序列，所以 Rust 用 `OsStr` / `OsString` 表示路径，而 `String` 装不下所有路径。要不要 `to_str().unwrap()` 取决于场景：**展示给人看用 `display()`，参与逻辑判断用 `Path` 本身**（`path == other_path` 不需要转字符串）。

**② 别用 `exists()` 做「先检查再操作」。**

```rust
// 有竞态（TOCTOU：检查完到操作之间文件可能被删/被建）
if !path.exists() {
    fs::write(&path, data)?;
}

// 更好的做法：直接操作，用错误类型判断
match fs::read_to_string(&path) {
    Ok(text) => println!("{text}"),
    Err(error) if error.kind() == io::ErrorKind::NotFound => println!("文件不存在"),
    Err(error) => return Err(error.into()),
}
```

`exists()` 还有个更隐蔽的问题：**它把「不存在」和「权限不足」都变成 `false`**——目录没权限时你会以为文件不存在。

**③ 相对路径是相对于「当前工作目录」**，而当前工作目录不一定是程序所在目录（比如从别的路径执行 `cargo run`）。需要绝对路径时用 `std::env::current_dir()` 或 `path.canonicalize()`（后者还要求路径真实存在）。

## 19.3 临时目录：测试和演示的标配

在项目目录里造测试文件是坏习惯：污染仓库、可能覆盖用户数据、忘记清理。`tempfile` 提供了开箱即用的方案：

```rust
use tempfile::tempdir;

let temp = tempdir().unwrap();     // 创建一个临时目录
let base = temp.path();             // 拿到它的路径
// ... 在里面随便写 ...
// 函数结束时 temp 被 drop，目录连同内容一起被递归删除
```

实测 `tempdir() 创建成功，exists = true`，而**程序退出后目录自动消失**——本章配套代码就靠它做到「演示了完整的文件操作，却不在你电脑上留垃圾」。

配套的 `tempfile::NamedTempFile` 适合「一个临时文件」的场景，`tempfile::tempdir()` 适合「一整套目录结构」。测试里用它们还有个好处：**测试可以并行跑，不会互相覆盖文件**。

## 19.4 读写文件

三个最常用的函数：

```rust
use std::fs;

fs::write(&path, "内容")?;              // 写：覆盖已有内容
let text: String = fs::read_to_string(&path)?;   // 读：要求内容是 UTF-8
let bytes: Vec<u8> = fs::read(&path)?;           // 读：按字节，适合二进制
```

实测：

```text
    写入 3 行，读回 30 字节
    行数 = 3，第一行 = "第一行"
    按字节读回 30 字节，前 3 个字节 = [231, 172, 172]
```

最后一行是「第」字的 UTF-8 编码（`E7 AC AC` = 231 172 172）——**按字节读和按字符串读，拿到的是同一份数据的两种视角**（第 7 章 7.6 讲的那套）。

什么时候用哪个？

| 场景 | 用什么 |
| --- | --- |
| 小文本文件，一次性读完 | `fs::read_to_string` |
| 二进制（图片、压缩包） | `fs::read` |
| 大文件、要逐行处理 | `File::open` + `BufReader`（下一节） |
| 只想读一部分 | `File::open` + `seek` + `take` |

**`read_to_string` 要求文件是合法 UTF-8**，遇到二进制内容会返回 `InvalidData` 错误——这也是它的优点：不会悄悄给你一个乱码字符串。

## 19.5 追加与 `OpenOptions`

`fs::write` 会**覆盖**文件。想追加、想只在文件不存在时创建，要用 `OpenOptions`：

```rust
use std::fs::OpenOptions;
use std::io::Write;

let mut file = OpenOptions::new().append(true).open(&path)?;
writeln!(file, "第四行")?;
```

实测追加之后，行数从 3 变成 4。

`OpenOptions` 的组合：

| 方法 | 含义 |
| --- | --- |
| `read(true)` / `write(true)` | 允许读 / 写 |
| `append(true)` | 追加模式（写入总在末尾） |
| `truncate(true)` | 打开时清空 |
| `create(true)` | 不存在就创建 |
| `create_new(true)` | **必须不存在**才创建，否则报 `AlreadyExists` |

`create_new(true)` 是「原子地创建独占文件」的标准做法（比如写 pid 文件、锁文件）——比「先 `exists()` 再创建」可靠得多。

注意 `OpenOptions::new()` 默认什么都不允许：**至少要写 `.write(true)` 或 `.append(true)`**，否则打开会失败。

## 19.6 缓冲：`BufReader` 与 `BufWriter`

每次读写都调用一次操作系统，代价很高。缓冲把许多次小操作合并成少数几次大操作：

```rust
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};

// 读：一行一行来
let reader = BufReader::new(File::open(&path)?);
for line in reader.lines() {
    let line = line?;          // 每行都是 io::Result<String>
    println!("{line}");
}

// 写：攒着一起写
let mut writer = BufWriter::new(File::create(&output)?);
for index in 0..3 {
    writeln!(writer, "第 {index} 行")?;
}
writer.flush()?;               // 别忘了
```

实测：

```text
    lines = ["第一行", "第二行", "第三行", "第四行"]
    out.txt 大小 = 30 字节
```

三个必须记住的点：

1. **`BufWriter` 必须 `flush()`**（或者让它正常 `drop`）。缓冲没写满时数据还在内存里，程序崩溃就丢了。上面的代码显式 `flush()` 是最稳妥的写法。
2. **`lines()` 会把换行符去掉**，而且每行是 `io::Result<String>`——`?` 或 `unwrap` 处理掉。
3. **`lines()` 遇到非法 UTF-8 会报错**。处理二进制数据时用 `read_until(b'\n', &mut buf)` 按字节读。

性能差别有多大？对于「逐行读一个几十万行的文件」，不加缓冲的版本会发起几十万次系统调用，而缓冲版本只有几千次——**差距通常在几十倍以上**。

## 19.7 标准输入与输出

读标准输入最常见的是「逐行处理」：

```rust
use std::io::{self, BufRead};

let stdin = io::stdin();
for line in stdin.lock().lines() {
    let line = line?;
    println!("{}", line.chars().count());
}
```

用管道喂给它三行输入，实测输出：

```text
    读到 3 行，共 12 个字符
```

（`alpha` 5 个 + `beta` 4 个 + `中文行` 3 个 = 12 个字符。）

几个细节：

- **`stdin.lock()` 很重要**：不加锁的话每行都要重新加解锁一次，循环里差别明显。
- `stdin.read_line(&mut buf)` 是「读一行到已有的 `String` 里」，**不会清空缓冲区**——复用时记得先 `buf.clear()`，这是个经典 bug。
- 输出侧：`println!` 每次都加锁，**循环里大量输出时用 `io::stdout().lock()`** 配合 `writeln!` 会快很多。
- 错误信息走 `eprintln!`（stderr），这样重定向 stdout 时日志和错误不会混在一起（第 1 章 1.7 讲过）。

## 19.8 目录操作与递归遍历

```rust
use std::fs;

fs::create_dir(&dir)?;        // 只能创建一级，父目录不存在会失败
fs::create_dir_all(&dir)?;    // 递归创建，父目录不存在也能建（更常用）

for entry in fs::read_dir(&dir)? {
    let entry = entry?;                       // 每一项也是 Result
    println!("{}", entry.file_name().to_string_lossy());
}
```

实测 `read_dir(顶层) = ["note.txt", "out.txt", "sub"]`。

两个注意点：

- **`read_dir` 的顺序不保证**。需要稳定输出就先收集再 `sort`（本章示例都这么做了）。
- 每一项是 `io::Result<DirEntry>`：**遍历过程中某个条目可能因为权限等原因读失败**，所以里面还有一层 `Result`。

想看子目录里的文件，就得用 `walkdir` 递归遍历：

```rust
use walkdir::WalkDir;

let files: Vec<_> = WalkDir::new(base)
    .into_iter()
    .filter_map(Result::ok)                    // 跳过出错的条目
    .filter(|entry| entry.file_type().is_file())  // 只要文件
    .map(|entry| entry.into_path())
    .collect();
```

实测：

```text
    所有文件 = ["note.txt", "out.txt", "sub/a.txt", "sub/deep/b.txt"]
```

`WalkDir` 比手写递归好在：**它迭代、可配置深度、能跟随或拒绝符号链接、错误处理明确**（`filter_map(Result::ok)` 表示「跳过读不了的条目」）。

## 19.9 元数据与删除

```rust
let meta = fs::metadata(&path)?;
meta.is_file();          // 是不是普通文件
meta.is_dir();           // 是不是目录
meta.len();              // 字节数（对目录的含义由平台决定，别依赖）
meta.modified();         // Result<SystemTime>，第 17 章的时间类型

fs::remove_file(&path)?;         // 删文件
fs::remove_dir(&empty_dir)?;     // 删空目录
fs::remove_dir_all(&dir)?;       // 递归删目录（危险！）
```

实测：

```text
    删除前存在 = true，删除后存在 = false
    note.txt: 是文件 = true，大小 = 40 字节
    是否有修改时间 = true
```

**`remove_dir_all` 要格外小心**：路径来自用户输入或配置时，一个错误的路径就能删掉整棵目录树。生产代码里要么校验路径在允许的根目录之内，要么用「移动到回收站/临时目录再删」的两步策略。

## 19.10 错误处理：认 `ErrorKind`，别认错误消息

`io::Error` 的关键方法是 `kind()`，它返回一个 `ErrorKind` 枚举：

```rust
use std::io::ErrorKind;

match fs::read_to_string(&path) {
    Ok(text) => println!("{text}"),
    Err(error) if error.kind() == ErrorKind::NotFound => println!("文件不存在"),
    Err(error) => eprintln!("读取失败：{error}"),
}
```

实测两种常见情况：

```text
    读不存在的文件：kind = NotFound
    重复创建目录：kind = AlreadyExists
    （错误消息随系统语言变化，判断类型要用 kind()）
```

**最后一行是这一节的重点**：同一行代码在不同语言的系统上会给出不同的消息——本章实测的机器是中文 Windows，所以消息是「系统找不到指定的文件。 (os error 2)」，换成英文系统就是 `The system cannot find the file specified. (os error 2)`。**任何按消息文本 `contains("not found")` 的判断都会在别的机器上失效。**

常见的 `ErrorKind`：

| kind | 典型场景 |
| --- | --- |
| `NotFound` | 文件 / 目录不存在 |
| `PermissionDenied` | 没权限（注意 `exists()` 会把这种情况也报成 `false`） |
| `AlreadyExists` | 创建时目标已存在（`create_new`、`create_dir`） |
| `InvalidData` | 按 UTF-8 读，但内容不是合法 UTF-8 |
| `UnexpectedEof` | 数据比预期短（解析二进制格式时常见） |
| `WouldBlock` | 非阻塞 IO 暂时没有数据（第 13 章 `try_lock` 见过） |

返回值统一用 `io::Result<T>`（等价于 `Result<T, io::Error>`），函数签名一眼就能看出「这里会做 IO」：

```rust
fn load_config(path: &Path) -> std::io::Result<String> {
    std::fs::read_to_string(path)
}
```

需要给错误加上下文（「哪个文件出的错」）时，用 `map_err` 包一层，或者用第 8 章提到的 `anyhow` 的 `.with_context()`——这也正是 19.2 里那个 `match` 想表达的：**错误信息要能定位到具体对象**。

## 19.11 三个真实报错怎么读

**案例 1：读文件时忘了 `mut`（`E0596`）**

```rust
use std::fs::File;
use std::io::Read;

let file = File::open("note.txt").unwrap();
let mut content = String::new();
file.read_to_string(&mut content).unwrap();
```

```text
error[E0596]: cannot borrow `file` as mutable, as it is not declared as mutable
 --> src/main.rs:7:5
  |
7 |     file.read_to_string(&mut content).unwrap();
  |     ^^^^ cannot borrow as mutable
  |
help: consider changing this to be mutable
  |
5 |     let mut file = File::open("note.txt").unwrap();
  |         +++
```

为什么会这样？**`Read::read_to_string` 需要 `&mut self`**——读取会推进「当前读到哪」的位置，所以文件句柄必须是可变的。`help` 直接给出了补 `mut` 的位置。

**案例 2：忘了引入 `Write` trait（`E0599`）**

```rust
use std::fs::File;

let mut file = File::create("out.txt").unwrap();
writeln!(file, "hello").unwrap();
```

```text
error[E0599]: cannot write into `File`
    --> src/main.rs:5:14
     |
   5 |     writeln!(file, "hello").unwrap();
     |              ^^^^
     |
note: must implement `io::Write`, `fmt::Write`, or have a `write_fmt` method
     = help: items from traits can only be used if the trait is in scope
help: trait `Write` which provides `write_fmt` is implemented but not in scope; perhaps you want to import it
     |
   1 + use std::io::Write;
```

报错说得很清楚：**方法来自 trait，trait 不在作用域里就用不了**。这也解释了为什么本章的 `use` 列表里总有 `std::io::{BufRead, BufReader, BufWriter, Write}`——`BufRead` 和 `Write` 不是用到类型才引入的，而是**用到方法才引入的**。

（同理：`lines()` 来自 `BufRead`，忘记引入会报「no method named `lines`」而不是「BufReader 未定义」。）

**案例 3：运行时错误（`ErrorKind`）**

```text
    读不存在的文件：kind = NotFound
    重复创建目录：kind = AlreadyExists
```

注意这两行是**运行结果**，不是编译错误——路径是否存在只有运行时才知道。这也是为什么本章反复强调：**IO 错误必须处理，编译器帮不了你**。

## 19.12 常见坑速查

| 现象 | 原因 | 解法 |
| --- | --- | --- |
| Windows 上路径好用，Linux 跑不了（或反过来） | 手写了分隔符 | 用 `PathBuf::join` |
| 文件名是乱码 | 路径不是 UTF-8，被强行 `to_str().unwrap()` | 用 `display()` / `to_string_lossy()`，或 `OsStr` |
| 判断「文件存在」后操作却失败 | TOCTOU 竞态，或权限不足被当成不存在 | 直接操作，匹配 `ErrorKind` |
| 读文件报「cannot borrow as mutable」 | 忘了 `let mut file` | 加 `mut` |
| 写文件报「cannot write into File」 | 没 `use std::io::Write` | 引入 trait |
| `writeln!` 之后文件内容不全 | `BufWriter` 没 flush | 显式 `flush()` |
| 逐行读大文件很慢 | 没加缓冲，每行一次系统调用 | `BufReader` |
| `read_line` 复用缓冲区时内容叠加 | `read_line` 追加而不是替换 | 每次先 `buf.clear()` |
| `read_to_string` 读图片报错 | 内容不是合法 UTF-8 | 二进制用 `fs::read` |
| 目录遍历结果顺序每次不同 | `read_dir` 顺序不保证 | 收集后 `sort` |
| 循环里打印很慢 | `println!` 每次加解锁 | `io::stdout().lock()` |
| 测试之间互相干扰 | 用了固定文件名 | `tempfile::tempdir()` |
| 误删了重要目录 | `remove_dir_all` 传错路径 | 校验路径在允许范围内 |

## 19.13 练习

1. 写 `fn copy_file(src: &Path, dst: &Path) -> io::Result<u64>`，用 `BufReader` + `BufWriter` + `io::copy` 复制文件并返回复制的字节数。
2. 写 `fn count_lines(path: &Path) -> io::Result<usize>`，用 `BufReader` 逐行统计行数（不要一次读进内存）。
3. 写 `fn find_by_extension(dir: &Path, extension: &str) -> Vec<PathBuf>`，用 `walkdir` 递归找出指定扩展名的文件，结果排序。
4. 写 `fn append_line(path: &Path, line: &str) -> io::Result<()>`：把一行追加到文件末尾，**文件不存在时自动创建**。
5. 回答两个问题：(a) 为什么「先 `exists()` 再操作」是坏习惯？(b) 为什么判断错误类型要用 `error.kind()` 而不是看错误消息？

（第 2、5 题是 19.6 和 19.10 讨论过的内容，这里不再单独给答案。）

### 参考答案

::: details 第 1 题

```rust
use std::fs::File;
use std::io::{self, BufReader, BufWriter, Write};
use std::path::Path;

fn copy_file(src: &Path, dst: &Path) -> io::Result<u64> {
    let mut reader = BufReader::new(File::open(src)?);
    let mut writer = BufWriter::new(File::create(dst)?);
    let copied = io::copy(&mut reader, &mut writer)?;
    writer.flush()?;
    Ok(copied)
}
```

实测对一个 `"hello\nworld\n"`（12 字节）的文件，返回 `Ok(12)`，目标文件内容完全一致。

`io::copy` 是标准库提供的「搬运」函数：**它按块复制，不会把整个文件读进内存**，处理大文件也没问题。注意末尾的 `flush()`——`io::copy` 只保证把数据写进 `BufWriter` 的缓冲区，最终落盘还要靠 `flush`。

:::

::: details 第 3 题

```rust
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

fn find_by_extension(dir: &Path, extension: &str) -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = WalkDir::new(dir)
        .into_iter()
        .filter_map(Result::ok)                       // 跳过读不了的条目
        .filter(|entry| entry.file_type().is_file())  // 只要文件，不要目录
        .filter(|entry| entry.path().extension().is_some_and(|e| e == extension))
        .map(|entry| entry.into_path())
        .collect();
    found.sort();
    found
}
```

实测在一个包含 `a.txt`、`sub/b.txt`、`sub/c.md` 的目录里找 `txt`，返回 `["a.txt", "dst.txt", "src.txt", "sub/b.txt"]`（含测试时创建的其他 txt 文件）。

两个细节：`entry.file_type().is_file()` 必须判断，否则目录名里含 `.txt` 时也会被选中；`sort()` 是因为**遍历顺序不保证**，排序后测试才能稳定断言。

:::

::: details 第 4 题

```rust
use std::fs::OpenOptions;
use std::io::{self, Write};
use std::path::Path;

fn append_line(path: &Path, line: &str) -> io::Result<()> {
    let mut file = OpenOptions::new()
        .create(true)     // 不存在就创建
        .append(true)     // 追加而不是覆盖
        .open(path)?;
    writeln!(file, "{line}")
}
```

实测连续追加两行后，文件内容是 `"第一条\n第二条\n"`。

这个组合（`create(true) + append(true)`）就是「日志文件」的标准写法：**文件不存在自动创建，存在就追加**，而且不需要先判断存在性（避开了 19.2 的竞态问题）。

:::

## 19.14 小结

- 路径用 `Path` / `PathBuf`（`&str` / `String` 的关系），常用操作 `file_name` / `parent` / `extension` / `join` / `with_extension`；`join` 自动处理平台分隔符。

- 路径不一定是 UTF-8：逻辑判断用 `Path`，展示用 `display()` 或 `to_string_lossy()`。

- **别用「先 `exists()` 再操作」**：有竞态、也分不清「不存在」和「权限不足」；直接操作并匹配 `ErrorKind` 更可靠。

- 测试和演示用 `tempfile::tempdir()`，离开作用域自动递归删除，不污染项目目录。

- 读写小文件用 `fs::write` / `fs::read_to_string` / `fs::read`；大文件用 `File` + `BufReader` / `BufWriter`。

- 追加、只创建不覆盖等需求用 `OpenOptions`；`create_new(true)` 是原子创建的标准做法。

- **缓冲读写是性能关键**：`BufWriter` 记得 `flush()`；`stdin.lock()` / `stdout().lock()` 能显著减少加锁开销。

- `read_dir` 顺序不保证（要排序），每一项是 `io::Result<DirEntry>`；递归遍历用 `walkdir`。

- `remove_dir_all` 很危险，路径来自外部时务必校验范围。

- **错误处理认 `ErrorKind` 不认消息**：消息随系统语言变化，`NotFound` / `PermissionDenied` / `AlreadyExists` / `InvalidData` 才是稳定的判断依据。

- 方法来自 trait 就必须引入 trait：`Read`、`BufRead`、`Write` 忘记 `use` 时报的是「no method / cannot write into」，而不是「类型不存在」。

下一章讲**序列化与配置**：用 `serde` + `serde_json` / `toml` 把结构体与 JSON、TOML 互转，处理配置文件、环境变量与默认值。
