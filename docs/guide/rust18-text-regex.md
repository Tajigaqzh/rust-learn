# 第 18 章 · 文本处理与正则

文本处理看起来是「小事」，但坑分三层：

| 层次 | 典型问题 |
| --- | --- |
| 编码 | `len()` 是字节数不是字数；按字节截断会把字符切成两半 |
| Unicode 规则 | 大小写转换可能改变长度；看起来一样的字符串可能不相等 |
| 正则 | 语法与 PCRE 不同（不支持前瞻）；性能取决于怎么用 |

第 5 章和第 7 章已经讲过 UTF-8 的基础（`&str` 与 `String`、按字节切片的 panic），这一章补齐剩下的部分。

配套代码在 `src/rust18_text/mod.rs`，本章用到两个 crate，已经加进 `Cargo.toml`：

```toml
[dependencies]
regex = "1"
unicode-segmentation = "1"
```

## 18.1 三种「长度」：字节、字符、字素

同一个字符串有三种数法，而且**结果经常不一样**：

| 数法 | 方法 | 含义 |
| --- | --- | --- |
| 字节 | `s.len()` | UTF-8 编码后的字节数 |
| 字符 | `s.chars().count()` | Unicode 码点（`char`）的个数 |
| 字素 | `s.graphemes(true).count()` | 用户感知的「一个字」 |

实测：

```text
    abc: 字符 3 / 字节 3 / 字素 3
    中文abc: 字符 5 / 字节 9 / 字素 5
    一家四口: 字符 7 / 字节 25 / 字素 1
```

第三行最值得注意：`👨‍👩‍👧‍👦`（一家四口的 emoji）由**7 个码点**、**25 个字节**组成，但用户在屏幕上看到的是**1 个字**。它由四个 emoji 加三个零宽连接符（ZWJ）拼成——`chars()` 会把它数成 7，`graphemes()` 才是 1。

选择标准：

| 需求 | 用什么 |
| --- | --- |
| 存储大小、网络传输、缓冲区 | 字节（`len()`） |
| 按「字符」遍历或截取（中文、标点） | `chars()` |
| 按用户感知的「一个字」处理（emoji、组合音标） | `graphemes()`（`unicode-segmentation`） |
| 截断显示（比如「最多 20 个字」） | **字素**，否则可能把 emoji 切成两半 |

## 18.2 Unicode 大小写与「看起来一样但不相等」

大小写转换不是「ASCII 那 26 个字母」的事：

```text
    "straße".to_uppercase() = STRASSE
    "中文".to_uppercase() = 中文
    （大小写转换是按 Unicode 规则做的，可能改变长度）
```

实测 `"straße"` 有 6 个字符，转大写后变成 7 个字符（`ß` 变成 `SS`）。所以：

- **不要假设 `to_uppercase()` 不改变长度**；
- 大小写不敏感的比较，标准做法是两边都做 `to_lowercase()` 再比（注意 `to_lowercase` 也可能改变长度，比如 `İ`）；
- 真正「语言感知」的大小写规则（土耳其语的 `i`/`İ`）需要专门处理。

另一个更隐蔽的坑是**规范化**（normalization）：

```rust
let composed = "é";             // U+00E9，一个码点
let decomposed = "e\u{301}";    // e + 组合尖音符，两个码点
```

实测：

```text
    composed: chars = 1, bytes = 2
    decomposed: chars = 2, bytes = 3
    两者相等吗 = false
    （屏幕上看完全一样）
```

**屏幕上看起来一模一样的两个字符串，可以用不同的字节序列表示，因此 `==` 会返回 `false`。** 处理用户输入、去重、搜索时如果遇到「明明一样却不相等」，这就是原因。标准解法是先做 Unicode 规范化（NFC），这需要 `unicode-normalization` crate 的 `nfc()`——本项目没有引入它，但知道这个问题存在，能省下你半天排查时间。

## 18.3 拼接与 `Cow`：什么时候真的需要分配

第 7 章讲过拼接的三种方式（`push_str` / `+` / `format!`）。这里补一个更精细的工具：**`Cow<'a, str>`**（Copy-on-Write）——「要么借用，要么拥有」：

```rust
use std::borrow::Cow;

fn normalize_whitespace(text: &str) -> Cow<'_, str> {
    let trimmed = text.trim();
    let already_clean = trimmed == text && !text.contains("  ");
    if already_clean {
        Cow::Borrowed(text)      // 不需要修改：直接借用，零分配
    } else {
        Cow::Owned(WHITESPACE.replace_all(trimmed, " ").into_owned())
    }
}
```

实测：

```text
    清洗结果 = [rust is fun]
    本来就干净的输入：是否复用原串 = true
```

第二行是重点：输入「本来就干净」时返回的是 `Cow::Borrowed`，**没有发生任何堆分配**。这在文本处理里很有价值——大量输入不需要清洗，为它们各分配一个 `String` 是浪费。

`Cow` 的用法：

| 场景 | 建议 |
| --- | --- |
| 函数可能修改、也可能原样返回入参 | 返回 `Cow<'_, str>` |
| 调用方只需要 `&str` | 用 `Cow` 避免不必要的分配 |
| 每次都要修改 | 直接返回 `String`，别绕弯 |

判断 `Cow` 里是借用还是拥有：`matches!(value, Cow::Borrowed(_))`。

## 18.4 正则入门

Rust 的正则是 `regex` crate，核心 API 只有几个：

```rust
use regex::Regex;

let re = Regex::new(r"(\d{4})-(\d{2})-(\d{2})").unwrap();   // 编译：返回 Result

re.is_match(text);          // 有没有匹配
re.find(text);              // 第一个匹配，返回 Option<Match>
re.captures(text);          // 第一个匹配 + 捕获组
re.captures_iter(text);     // 遍历所有匹配
```

注意三点：

- **正则字符串习惯用原始字符串 `r"..."`**，否则 `\d` 要写成 `"\\d"`。
- `Regex::new` 返回 `Result`，因为模式可能非法——**编译失败是运行时的错，不是编译期的**（见 18.10）。所以要么 `unwrap()`（模式是写死的常量，确定合法），要么把错误交给调用方（模式来自用户输入）。
- 编译一次、反复使用。**别把 `Regex::new` 放进循环**（见 18.8）。

实测：

```text
    is_match = true
    第一个匹配 = "2026-09-13"
    捕获组：年 2026 月 09 日 13
    捕获组：年 2027 月 01 日 05
```

常用的模式语法：

| 语法 | 含义 | 例子 |
| --- | --- | --- |
| `.` | 任意字符（默认不含换行） | `a.c` 匹配 `abc` |
| `\d` / `\w` / `\s` | 数字 / 单词字符 / 空白 | `\d{3}` 匹配三位数字 |
| `[...]` | 字符类 | `[a-z_]`、`[^0-9]` |
| `*` / `+` / `?` | 0+ / 1+ / 0 或 1 次 | `\d+` |
| `{n}` / `{n,m}` | 恰好 n 次 / n 到 m 次 | `\d{4}` |
| `(...)` | 捕获组 | `(\d{4})-(\d{2})` |
| `(?:...)` | 非捕获组 | `(?:abc)+` |
| `^` / `$` | 行首 / 行尾 | `^\d+$` |
| `*?` / `+?` | 非贪婪 | `<.*?>` 只匹配到第一个 `>` |
| `(?P<name>...)` | 命名捕获组 | `(?P<year>\d{4})` |

## 18.5 命名捕获组与替换

用编号取捕获组（`&caps[1]`）在多组时很容易数错。命名捕获组可读性好得多：

```rust
let named = Regex::new(r"(?P<y>\d{4})-(?P<m>\d{2})-(?P<d>\d{2})").unwrap();
let replaced = named.replace_all("今天是 2026-09-13", "$d/$m/$y");
```

实测 `replace_all = 今天是 13/09/2026`。

替换串里有几个约定：

| 写法 | 含义 |
| --- | --- |
| `$0` / `$1` | 整个匹配 / 第 n 个捕获组 |
| `$name` | 命名捕获组 |
| `${name}` | 同上，用于名字后面紧跟别的字符时 |
| `$$` | 一个字面量的 `$` |

`replace_all` 返回的也是 `Cow`——**没有匹配时直接返回原串**，不会白分配。

顺带一提，`replace_all` 的第二个参数可以是闭包，用来自定义替换逻辑（练习第 2 题的手机号打码就是这么做的）：

```rust
re.replace_all(text, |caps: &regex::Captures| format!("..."))
```

## 18.6 切分、转义与「把用户输入当模式用」

`Regex::split` 按模式切分，比 `str::split` 更强（支持任意模式）：

```rust
let separator = Regex::new(r"\s*,\s*").unwrap();
let parts: Vec<&str> = separator.split("a, b ,c").collect();
// ["a", "b", "c"]
```

实测 `split = ["a", "b", "c"]`——注意 `\s*,\s*` 把逗号两边的空格一起吃掉了。

**`regex::escape` 是这一节最重要的函数**。把用户输入直接拼进模式里是很危险的操作：

```rust
// 用户输入 "a.b" 想搜字面量，结果 `.` 变成了「任意字符」
let bad = Regex::new(user_input).unwrap();

// 正确做法：先转义
let pattern = format!("^{}$", regex::escape(user_input));
```

实测 `regex::escape("a.b*c") = a\.b\*c`。这既是正确性问题（匹配到了不该匹配的东西），也是安全问题（用户可以用精心构造的模式让搜索变得极慢）——**任何来自外部的字符串进正则之前都要 `escape`**。

## 18.7 实战：解析日志行

把前面的东西组合起来：从日志行里提取等级、时间和消息。

```rust
static LOG_LINE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(?P<level>INFO|WARN|ERROR)\s+(?P<time>\d{2}:\d{2}:\d{2})\s+(?P<msg>.*)$")
        .unwrap()
});

fn parse_log_line(line: &str) -> Option<(String, String, String)> {
    let caps = LOG_LINE.captures(line)?;                 // 不匹配就返回 None
    Some((
        caps.name("level")?.as_str().to_string(),
        caps.name("time")?.as_str().to_string(),
        caps.name("msg")?.as_str().to_string(),
    ))
}
```

实测对四行的处理结果：

```text
    14:30:00 [INFO] 服务启动
    14:30:05 [WARN] 磁盘使用率 85%
    14:31:12 [ERROR] 连接数据库失败
    跳过无法解析的行：这不是日志行
```

这个函数体现了几个好习惯：

- **用 `Option` 而不是 panic** 表示「这行不符合格式」（第 6 章）；
- 用 `?` 串联多个可能失败的点（第 8 章）；
- 模式里用 `^...$` 锚定整行，避免「只匹配到一部分」的意外；
- 命名捕获组让代码里的 `level` / `time` / `msg` 一眼可读。

## 18.8 性能：编译一次，用很多次

**正则编译是昂贵操作**（要把模式编译成状态机）。所以：

```rust
use std::sync::LazyLock;

static DATE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\d{4}-\d{2}-\d{2}").unwrap());
```

`LazyLock`（标准库，Rust 1.80 起）保证**第一次使用时才编译，之后全局复用**。实测 `DATE.is_match("发布于 2026-09-13") = true`。

其他性能要点：

| 做法 | 为什么 |
| --- | --- |
| 用 `LazyLock` / `OnceLock` 缓存编译结果 | 避免每次调用都编译 |
| 别在循环里 `Regex::new` | 同上，这是最常见的性能事故 |
| 只判断「能不能匹配」时用 `is_match` | 比 `find` 少一步构造 `Match` |
| 多个固定模式用 `RegexSet` | 一次扫描判断命中哪些模式 |
| 大文件用 `find_iter` 流式处理 | 不要先 `collect` 成 `Vec` |
| 简单查找优先用 `str::contains` / `starts_with` | 不需要正则引擎的开销 |

关于「灾难性回溯」：Rust 的 `regex` 用的是**有限自动机**，保证匹配时间与输入长度成线性关系，**不会出现 PCRE 那种指数级回溯**。但这不代表正则一定快——用它做大量小字符串的简单匹配，往往还是不如手写 `contains`。

## 18.9 不支持的语法：前瞻与反向引用

从 PCRE / Python / JavaScript 过来的话，有一个必须知道的事实：**Rust 的 `regex` 不支持前瞻、后顾和反向引用**。

```rust
Regex::new(r"\d+(?=px)")     // 前瞻：不支持
Regex::new(r"(\w+)\1")       // 反向引用：不支持
```

实测报错：

```text
    [regex parse error:
        \d+(?=px)
           ^^^
    error: look-around, including look-ahead and look-behind, is not supported]
    [regex parse error:
        (\w+)\1
             ^^
    error: backreferences are not supported]
```

演示代码里这两行各加了 `#[allow(clippy::invalid_regex)]`：clippy 默认把
「正则字面量编译失败」当成 **error**（绝大多数情况下它确实就是 bug），
而这里正是故意犯错给你看，所以显式放行——**这正是「有些 lint 需要按场景放行，
而不是关掉整条规则」的例子**。

**这是设计取舍，不是缺陷**：正是因为排除了这些需要回溯的特性，`regex` 才能保证线性时间——不会因为一条精心构造的输入就把 CPU 打满。

需要这些能力时有三条路：

| 方案 | 说明 |
| --- | --- |
| 改写模式 | 很多前瞻可以用「先匹配、再检查捕获组」两步完成 |
| 分步处理 | 先用 `regex` 粗筛，再用普通代码精确定位 |
| 换引擎 | `fancy-regex`（支持回溯）、`pcre2`（绑定 PCRE2）——但要接受回溯带来的性能和 ReDoS 风险 |

## 18.10 三个真实报错怎么读

正则的语法错误都是**运行时**报出来的（`Regex::new` 返回 `Err`），报错信息里会画出模式中的出错位置。以下三条都是实际运行得到的。

**案例 1：括号没闭合**

```text
regex parse error:
    (\d+
    ^
error: unclosed group
```

模式里 `(` 和 `)` 必须成对；报错的那个 `^` 指向出问题的位置。

**案例 2：用了前瞻（从 PCRE / JS 迁移过来最常见）**

```text
regex parse error:
    \d+(?=px)
       ^^^
error: look-around, including look-ahead and look-behind, is not supported
```

**案例 3：用了反向引用**

```text
regex parse error:
    (\w+)\1
         ^^
error: backreferences are not supported
```

这两个报错**不是你写错了，而是引擎不支持**——按 18.9 的三条路改写即可。

在实际代码里，如果你写的是 `Regex::new(pattern).unwrap()`，这三条错误会以 panic 的形式出现（`called Result::unwrap() on an Err value: ...`）。所以：

```rust
// 模式是写死的常量：可以 unwrap（编译期就该保证它是对的）
static RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\d+").unwrap());

// 模式来自用户输入或配置：必须把 Result 传出去
fn compile(pattern: &str) -> Result<Regex, regex::Error> {
    Regex::new(pattern)
}
```

## 18.11 常见坑速查

| 现象 | 原因 | 解法 |
| --- | --- | --- |
| `len()` 和「字数」对不上 | `len()` 是字节数 | 用 `chars().count()` |
| 截断 emoji 后显示乱码 | 按字节或字符截断 | 用 `graphemes()` 按字素截断 |
| 两个「看起来一样」的字符串不相等 | Unicode 没有规范化（NFC/NFD） | 引入 `unicode-normalization`，两边都 `nfc()` |
| `to_uppercase()` 让字符串变长 | `ß` → `SS` 这类规则 | 别假设长度不变 |
| 正则里的 `\d` 报错 | 用了普通字符串字面量 | 用 `r"\d"` 原始字符串 |
| 循环里跑得很慢 | 每次迭代都 `Regex::new` | 提到循环外用 `LazyLock` |
| 用户搜 `.` 却匹配到所有字符 | 用户输入没转义 | `regex::escape(user_input)` |
| 报「look-around is not supported」 | Rust regex 不支持前瞻/后顾 | 改写模式或用 `fancy-regex` |
| 报「backreferences are not supported」 | 同样不支持反向引用 | 分两步处理，或换引擎 |
| 只匹配到一部分，不是整行 | 模式没加 `^...$` | 用锚点固定整行 |
| 中文按字符切没问题，emoji 出问题 | 字素 vs 字符的区别 | `graphemes()` |
| 大文件处理时内存暴涨 | 把匹配结果全 `collect` 了 | 用 `find_iter` 流式处理 |

## 18.12 练习

1. 写 `fn extract_emails(text: &str) -> Vec<String>`，用正则提取出所有邮箱（模式可以从简：`[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}`）。
2. 写 `fn mask_phone(text: &str) -> String`，把文本里的 11 位手机号（`1[3-9]` 开头）中间四位替换成 `****`（提示：`replace_all` 传闭包）。
3. 用 `graphemes()` 写 `fn truncate_display(text: &str, max: usize) -> String`：按「用户感知的字」截断，超出时加 `…`；用 `👨‍👩‍👧‍👦` 测试，确保不会被切成两半。
4. 写 `fn is_valid_date(text: &str) -> bool`：先用正则检查 `^\d{4}-\d{2}-\d{2}$`，再用 `chrono` 解析一次；解释为什么「正则通过 ≠ 日期合法」。
5. 解释两件事：(a) 为什么 Rust 的 `regex` 不支持前瞻和反向引用？(b) 为什么把用户输入直接拼进正则很危险？

（第 3、5 题是 18.1 和 18.6 / 18.9 讨论过的内容，这里不再单独给答案。）

### 参考答案

::: details 第 1 题

```rust
use regex::Regex;
use std::sync::LazyLock;

static EMAIL: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}").unwrap());

fn extract_emails(text: &str) -> Vec<String> {
    EMAIL.find_iter(text).map(|m| m.as_str().to_string()).collect()
}

fn main() {
    let text = "联系 ada@example.com 或 linus@rust-lang.org，无效的 a@b";
    println!("{:?}", extract_emails(text));
}
```

实测输出 `["ada@example.com", "linus@rust-lang.org"]`——`a@b` 因为顶级域名少于两个字母被正确排除。

提醒：**邮箱的完整语法比这个模式复杂得多**（国际化域名、带引号的本地部分等）。生产代码里要么用更完整的模式，要么直接用 `email_address` 这类专门的校验库。**正则适合「粗筛」，最终校验交给专门的解析器**——第 4 题就是这个思路。

:::

::: details 第 2 题

```rust
use regex::Regex;
use std::sync::LazyLock;

static PHONE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"1[3-9]\d{9}").unwrap());

fn mask_phone(text: &str) -> String {
    PHONE
        .replace_all(text, |caps: &regex::Captures| {
            let matched = caps.get(0).unwrap().as_str();
            format!("{}****{}", &matched[..3], &matched[7..])
        })
        .into_owned()
}

fn main() {
    println!("{}", mask_phone("联系电话 13812345678，备用 15987654321"));
}
```

实测输出 `联系电话 138****5678，备用 159****4321`。

两个细节：闭包参数类型写成 `&regex::Captures`（因为闭包参数类型不会自动推断出这个层次）；切片 `&matched[..3]` 是安全的，因为模式保证了匹配到的是 11 位 ASCII 数字。**如果模式可能匹配到多字节字符，就不能这样按字节切**（第 5 章的教训）。

:::

::: details 第 4 题

```rust
use regex::Regex;
use std::sync::LazyLock;

static DATE_SHAPE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\d{4}-\d{2}-\d{2}$").unwrap());

fn is_valid_date(text: &str) -> bool {
    DATE_SHAPE.is_match(text) && chrono::NaiveDate::parse_from_str(text, "%Y-%m-%d").is_ok()
}

fn main() {
    for candidate in ["2026-09-13", "2026-13-45", "2026/09/13", "2026-02-30"] {
        println!("{candidate} -> {}", is_valid_date(candidate));
    }
}
```

实测：

```text
2026-09-13 -> true
2026-13-45 -> false
2026/09/13 -> false
2026-02-30 -> false
```

**正则只能检查「形状」，不能检查「含义」**：`2026-13-45` 完美符合 `\d{4}-\d{2}-\d{2}`，但 13 月和 45 日并不存在；`2026-02-30` 也是同样的道理。所以正则通过之后，还要用日期库再校验一遍——这正是第 17 章「别自己算日历」的延伸。

:::

## 18.13 小结

- 字符串有三种长度：**字节**（`len()`）、**字符**（`chars()`）、**字素**（`graphemes()`）；面向用户的截断要用字素。

- Unicode 大小写转换可能改变长度（`ß` → `SS`）；看起来一样的字符串可能因为规范化不同而不相等。

- `Cow<'_, str>` 适合「可能修改、也可能原样返回」的函数：不需要修改时零分配。

- `regex` 的核心 API 是 `Regex::new` / `is_match` / `find` / `captures` / `captures_iter` / `replace_all` / `split`；模式习惯用原始字符串 `r"..."`。

- 命名捕获组（`(?P<name>...)`）比编号可读；替换串里用 `$name` 或 `${name}` 引用。

- **用户输入进正则必须先 `regex::escape`**，否则既是正确性问题也是安全问题。

- 正则编译很贵：用 `LazyLock` / `OnceLock` 缓存，别放进循环。

- Rust 的 `regex` **不支持前瞻、后顾和反向引用**，换来的是线性时间的匹配速度；需要这些特性时用 `fancy-regex` / `pcre2`，但要接受回溯风险。

- 正则只能检查「形状」：日期、邮箱这类有语义规则的输入，正则通过后还要用专门的解析器再校验一次。

下一章讲**文件、路径与 IO**：`Path` / `PathBuf` 的跨平台处理、读写文件的几种方式、`BufReader` / `BufWriter` 的缓冲策略、标准输入输出，以及遍历目录。
