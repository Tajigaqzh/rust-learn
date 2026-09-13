# 第 17 章 · 日期与时间

日期时间看起来是小事，实际上是软件里最经典的 bug 来源之一：时区搞错、夏令时跳变、闰年 2 月 29 日、精度丢失、把「时长」当成「时刻」用……

这一章把两件事讲清楚：

1. **两种「时间」是不同的东西**——「时刻」（2026-09-13 14:30:00）和「时长」（90 秒）。
2. **跨时区、跨系统传递时统一用 UTC 或时间戳**，只在展示给用户时转成本地时间。

配套代码在 `src/rust17_datetime/mod.rs`，执行 `cargo run` 可以看到全部输出。本章用到了 `chrono`，它已经加进 `Cargo.toml`：

```toml
[dependencies]
chrono = "0.4"
```

## 17.1 标准库：`SystemTime`、`Instant`、`Duration`

标准库提供了三种基础类型，分工很清楚：

| 类型 | 表示什么 | 用途 |
| --- | --- | --- |
| `SystemTime` | 系统时钟的「时刻」 | 记录时间戳、和外界交换时间 |
| `Instant` | 单调递增的「时刻」 | **测量耗时**（性能测试、超时） |
| `Duration` | 一段时长 | 表示间隔、超时值 |

```rust
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

let now = SystemTime::now();
let since_epoch: Duration = now.duration_since(UNIX_EPOCH).unwrap();

let start = Instant::now();
// ... 干点活 ...
let elapsed = start.elapsed();
```

实测：

```text
    现在晚于 1970-01-01：true
    时间戳已经超过 2020 年：true
    循环求和 total = 4999950000
    耗时大于零：true
    Duration 90 秒 = 1 分 30 秒 = 90000 毫秒
```

**`SystemTime` 和 `Instant` 的区别很重要**：`SystemTime` 会跟着系统时钟走——NTP 校时、用户手动改时间、夏令时切换都会让它「跳」。而 `Instant` 只保证单调递增，所以：

> **测耗时永远用 `Instant`，不要用 `SystemTime`。** 用 `SystemTime` 测出来的耗时可能是负数，或者凭空多出几秒。

## 17.2 UNIX 时间戳

时间戳是「从 1970-01-01 00:00:00 UTC 起经过的秒数」，它是**跨系统交换时间最安全的格式**——没有时区歧义，没有格式歧义：

```rust
let seconds = now.duration_since(UNIX_EPOCH).unwrap().as_secs();   // 秒
let millis = now.duration_since(UNIX_EPOCH).unwrap().as_millis();  // 毫秒
```

注意事项：

- **精度**：秒级时间戳在「同一秒内的多次事件」上会撞车，需要排序或去重时用毫秒（或纳秒）。
- **范围**：`u64` 秒级时间戳能表示到公元 5840 亿年，不用担心；但 32 位系统上的 `time_t` 会有 2038 问题——Rust 的 `Duration` 用 `u64`，不受影响。
- **时区**：时间戳永远是 UTC 的，显示成本地时间是「后面那一步」的事。

## 17.3 chrono：三类日期时间

标准库没有「年月日」的概念，所以要用 `chrono`。它把日期时间分成三类，**理解这三类的区别是本章的核心**：

| 类型 | 含义 | 例子 |
| --- | --- | --- |
| `NaiveDate` | 只有日期 | `2026-09-13` |
| `NaiveTime` | 只有时间 | `14:30:00` |
| `NaiveDateTime` | 日期 + 时间，**不带时区** | `2026-09-13 14:30:00` |
| `DateTime<Tz>` | 带时区的完整时刻 | `2026-09-13 22:30:00 +08:00` |

「Naive」的意思是「天真」——它只知道表盘上的数字，不知道那是哪个时区的时间：

```rust
use chrono::{NaiveDate, NaiveDateTime, NaiveTime};

let date = NaiveDate::from_ymd_opt(2026, 9, 13).unwrap();
let time = NaiveTime::from_hms_opt(14, 30, 0).unwrap();
let datetime = NaiveDateTime::new(date, time);
```

实测：

```text
    日期 = 2026-09-13
    年 = 2026，月 = 9，日 = 13，星期 = Sun
    时间 = 14:30:00
    日期时间 = 2026-09-13 14:30:00
```

取值用 `Datelike` 和 `Timelike` 这两个 trait（第 9 章讲的 trait 在这里又见面了）：`date.year()`、`date.month()`、`date.day()`、`date.weekday()`、`time.hour()`……

**构造日期用 `from_ymd_opt` 而不是 `from_ymd`**：前者返回 `Option`，非法日期（比如 2 月 30 日）得到 `None`；后者已经废弃，会在非法输入时 panic。这一致于第 6 章的原则——**能预期失败的场合用 `Option` / `Result` 表达**。

## 17.4 格式化与解析

格式化用 `format()`，占位符沿用 C 的 `strftime` 风格：

```rust
datetime.format("%Y-%m-%d %H:%M:%S")   // 2026-09-13 14:30:00
datetime.format("%Y 年 %m 月 %d 日")    // 2026 年 09 月 13 日
datetime.format("%Y%m%dT%H%M%S")       // 20260913T143000
```

实测输出与上面三行一致。最常用的占位符：

| 占位符 | 含义 | 例子 |
| --- | --- | --- |
| `%Y` / `%y` | 四位年 / 两位年 | `2026` / `26` |
| `%m` / `%b` / `%B` | 月份数字 / 缩写 / 全称 | `09` / `Sep` / `September` |
| `%d` | 日 | `13` |
| `%H` / `%I` | 24 小时 / 12 小时 | `14` / `02` |
| `%M` / `%S` | 分 / 秒 | `30` / `00` |
| `%p` | AM / PM | `PM` |
| `%a` / `%A` | 星期缩写 / 全称 | `Sun` / `Sunday` |
| `%z` / `%Z` | 时区偏移 / 名称 | `+0800` / `CST` |
| `%s` | 时间戳（秒） | `1789309800` |

解析是反向操作，**返回 `Result`**：

```rust
NaiveDateTime::parse_from_str("2026-09-13 14:30:00", "%Y-%m-%d %H:%M:%S")
NaiveDate::parse_from_str("13/09/2026", "%d/%m/%Y")
NaiveDate::parse_from_str("2026-13-45", "%Y-%m-%d")     // 非法日期
```

实测：

```text
    解析回来 = Ok(2026-09-13T14:30:00)
    解析日期 = Ok(2026-09-13)
    解析非法日期 = Err(ParseError(OutOfRange))
```

注意最后一行：**解析失败是 `Err`，不会 panic**。但 `%d/%m/%Y` 和 `%m/%d/%Y` 这种「日和月的顺序」在真实数据里极易搞混——**解析外部数据时永远不要 `unwrap()`**，把错误交给调用方（第 8 章的规则）。

## 17.5 时区：`Utc`、`FixedOffset`、`Local`

`DateTime<Tz>` 里的 `Tz` 就是时区。三种常见选择：

```rust
use chrono::{FixedOffset, TimeZone, Utc};

let utc = Utc.with_ymd_and_hms(2026, 9, 13, 14, 30, 0).unwrap();   // UTC
let beijing = FixedOffset::east_opt(8 * 3600).unwrap();            // 东八区
let local = utc.with_timezone(&beijing);                           // 换时区显示
```

实测：

```text
    UTC 时间 = 2026-09-13 14:30:00 UTC
    东八区时间 = 2026-09-13 22:30:00 +08:00
    两者时间戳相同：true
    东八区偏移 = +08:00
```

最后两行是理解时区的关键：**换时区不改变时刻，只改变显示方式**。`with_timezone` 之后时间戳没变（都是同一个瞬间），只是表盘数字从 `14:30` 变成 `22:30`。

| 类型 | 用途 |
| --- | --- |
| `DateTime<Utc>` | **存储、传输、比较、日志**——时间系统的「通用货币」 |
| `DateTime<FixedOffset>` | 明确知道偏移量的场景（比如固定时区的业务） |
| `DateTime<Local>` | 只用于**展示给用户**（`Local::now()`） |
| `NaiveDateTime` | 表盘时间本身（比如「每天早上 9 点」这种本地约定） |

关于夏令时（DST），有一个必须知道的限制：**`FixedOffset` 表达不了夏令时**。欧美很多地区一年里偏移量会变（比如 +01:00 ↔ +02:00），用固定偏移会在切换日算错一小时。需要处理真实时区规则时，要引入 tz 数据库：

```toml
chrono-tz = "0.10"
```

```rust
use chrono::TimeZone;
use chrono_tz::Asia::Shanghai;

let local = Shanghai.with_ymd_and_hms(2026, 9, 13, 14, 30, 0).unwrap();
```

好消息是：**只要内部统一用 UTC 存储和计算，只在最后展示时转换，夏令时的问题就只在「展示」这一步出现**——这也是「统一 UTC」这条规则的价值。

## 17.6 时间的算术

加一个固定时长用 `Duration`（chrono 的类型，和标准库的 `std::time::Duration` 同名但不同）；
加「天 / 月 / 年」这种日历单位用 `Months`，并且优先用 `checked_*` 版本：

```rust
let later = date + Duration::days(30);
println!("{}", (later - date).num_days());

let next_month = date.checked_add_months(Months::new(1)).unwrap();
```

实测：

```text
    2026-09-13 加 30 天 = 2026-10-13
    两者的天数差 = 30
    加 1 个月 = 2026-10-13
    2024-02-29 加 1 年 = 2025-02-28
    2023 年 2 月有 29 号吗：false
```

第 4 行演示了日历运算的边界：**2024-02-29 加 12 个月，结果是 2025-02-28**——因为 2025 年不是闰年，没有 2 月 29 日，`chrono` 按「夹到当月最后一天」的规则处理。

如果用 `+` 而不是 `checked_add_months`，遇到溢出会直接 panic。**处理用户输入或外部数据时用 `checked_*`**：

| 方法 | 行为 |
| --- | --- |
| `date + Duration::days(n)` | 溢出时 panic |
| `date.checked_add_signed(duration)` | 溢出返回 `None` |
| `date.checked_add_months(Months::new(n))` | 溢出返回 `None`，自动处理月末 |

## 17.7 时间戳与日期互转

`DateTime<Utc>` 和整数时间戳之间的转换是日常操作：

```rust
let seconds = utc.timestamp();            // 1789309800
let millis = utc.timestamp_millis();      // 1789309800000

let from_seconds = Utc.timestamp_opt(1_787_000_000, 0).single().unwrap();
let from_millis = chrono::DateTime::from_timestamp_millis(1_787_000_000_000).unwrap();
```

实测：

```text
    时间戳（秒）= 1789309800
    时间戳（毫秒）= 1789309800000
    从秒级时间戳还原 = 2026-08-17 20:53:20 UTC
    从毫秒时间戳还原 = 2026-08-17 20:53:20 UTC
```

三个细节：

- `Utc.timestamp_opt(秒, 纳秒)` 返回 `LocalResult`（因为某些时区在夏令时切换时会出现「不存在」或「重复」的时刻），`.single()` 取唯一解、再 `.unwrap()`——这就是为什么它比看起来麻烦。
- `DateTime::from_timestamp_millis` 直接返回 `Option`，越界返回 `None`。
- 从 `SystemTime` 到 `DateTime<Utc>` 可以这样转：`DateTime::<Utc>::from(system_time)`（标准库与 chrono 之间实现了 `From`）。

## 17.8 五个容易踩的坑

```text
    ① 测耗时用 Instant，不用 SystemTime：后者会被系统时间调整影响
    ② NaiveDateTime 不带时区，只能表示「某个地方的表盘时间」
    ③ 跨时区传递、存数据库要统一用 DateTime<Utc> 或时间戳
    ④ 解析失败返回 Result，不要直接 unwrap 用户输入
    ⑤ 加月份用 checked_add_months，自动处理 2 月 29 日这种边界
```

配套代码里还有一段「当天零点」的推算：

```text
    当天零点 = 2026-09-13 00:00:00
    下一天开始 = 2026-09-14 00:00:00
```

这种「取当天零点、加一天」的写法在统计日报时很常见。**注意它算的是「本地表盘上的零点」**：如果要做跨时区的日统计，先在 UTC 上对齐，再转换展示。

## 17.9 三个真实报错怎么读

**案例 1：忘了 unwrap，直接对 `Option<NaiveDate>` 做加法（`E0369`）**

```rust
let date = NaiveDate::from_ymd_opt(2026, 9, 13);
let next = date + Duration::days(1);
```

```text
error[E0369]: cannot add `TimeDelta` to `Option<NaiveDate>`
   --> src/main.rs:5:21
    |
  5 |     let next = date + Duration::days(1);
    |                ---- ^ ----------------- TimeDelta
    |                |
    |                Option<NaiveDate>
    |
note: `Option<NaiveDate>` does not implement `Add<TimeDelta>`
```

`from_ymd_opt` 返回的是 `Option`——类型系统直接把「我忘了处理非法日期」这个疏漏变成编译错误。补上 `.unwrap()` 或 `?` 即可。

**案例 2：把带时区和不带时区的时间混着比（`E0308`）**

```text
error[E0308]: mismatched types
  --> src/main.rs:13:26
   |
13 |     println!("{}", utc > naive);
   |                          ^^^^^ expected `DateTime<_>`, found `NaiveDateTime`
   |
   = note: expected struct `DateTime<_>`
              found struct `NaiveDateTime`
```

`DateTime<Utc>` 和 `NaiveDateTime` 是两个不同的类型，**编译器不允许你把「绝对时刻」和「表盘时间」直接比较**。想比较就先把其中一个转换过去（`naive.and_utc()`，或者用 `DateTime<Utc>` 构造）。

**案例 3：解析失败（运行时 `Err`，不是编译错误）**

```text
    解析非法日期 = Err(ParseError(OutOfRange))
```

`chrono` 的解析不会 panic，而是返回 `Result`。真实代码里应该这样处理：

```rust
let date = NaiveDate::parse_from_str(input, "%Y-%m-%d")
    .map_err(|e| format!("日期格式不对：{e}"))?;      // 第 8 章的 `?`
```

## 17.10 常见坑速查

| 现象 | 原因 | 解法 |
| --- | --- | --- |
| 测出的耗时为负、或忽大忽小 | 用 `SystemTime` 测耗时，被系统时钟调整影响 | 改用 `Instant` |
| 同一秒的事件顺序乱了 | 秒级时间戳精度不够 | 用毫秒 / 纳秒，或带序号 |
| 显示时间比预期差 8 小时 | 拿 UTC 当本地时间展示 | 展示时 `with_timezone(&Local)` |
| 跨时区数据对不上 | 存了 `NaiveDateTime` | 存 UTC 或时间戳，展示时再转 |
| 夏令时切换那天差一小时 | 用了 `FixedOffset` | 用 `chrono-tz` 的地区时区 |
| 2 月 29 日加一年出错 | 用了 `+` 而不是 `checked_*` | `checked_add_months`，它会夹到月末 |
| 解析用户输入时程序崩溃 | 对 `parse_from_str` 直接 `unwrap()` | 返回 `Result`，交给调用方处理 |
| `from_ymd` 报已废弃 | 用的是会 panic 的旧 API | 换 `from_ymd_opt` + `unwrap`/`?` |
| 日期和时间分开存，比较很麻烦 | 拆得太细 | 用 `NaiveDateTime` 或 `DateTime<Utc>` |
| 统计「今天」的数据跨时区对不上 | 「今天」是本地概念 | 先确定按哪个时区切天，再统一换算 |

## 17.11 练习

1. 写 `fn days_between(start: &str, end: &str) -> Result<i64, chrono::ParseError>`：解析两个 `%Y-%m-%d` 字符串，返回相差的天数（后面的日期减前面的，可能为负）。
2. 写 `fn local_string_to_timestamp(text: &str, offset_hours: i32) -> Option<i64>`：把「某个时区的本地时间字符串」转成 UTC 时间戳（提示：`FixedOffset` + `from_local_datetime` + `.single()`）。
3. 计算 2024-02-29 与 2025-02-28 相差多少天，并解释为什么「加一年」不是加 365 天。
4. 写 `fn days_in_month(year: i32, month: u32) -> Option<u32>`：用「下个月第一天减一天」的办法算出某个月有多少天，能正确处理闰年、能对非法月份返回 `None`。
5. 解释两件事：(a) 为什么测耗时必须用 `Instant`？(b) 为什么跨系统传递时间要统一用 UTC 或时间戳？

（第 3、5 题是 17.6 和 17.8 讨论过的内容，这里不再单独给答案。）

### 参考答案

::: details 第 1 题

```rust
use chrono::NaiveDate;

fn days_between(start: &str, end: &str) -> Result<i64, chrono::ParseError> {
    let start = NaiveDate::parse_from_str(start, "%Y-%m-%d")?;
    let end = NaiveDate::parse_from_str(end, "%Y-%m-%d")?;
    Ok((end - start).num_days())
}

fn main() {
    println!("{:?}", days_between("2026-01-01", "2026-09-13"));  // Ok(255)
    println!("{:?}", days_between("2026-09-13", "2026-01-01"));  // Ok(-255)
    println!("{:?}", days_between("bad", "2026-01-01"));         // Err(ParseError(Invalid))
}
```

实测输出 `Ok(255)`、`Ok(-255)`、`Err(ParseError(Invalid))`。`?` 直接把解析错误传出去——函数的返回类型已经声明了 `chrono::ParseError`，调用方一眼就知道可能出什么错。

:::

::: details 第 2 题

```rust
use chrono::{FixedOffset, NaiveDateTime, TimeZone};

fn local_string_to_timestamp(text: &str, offset_hours: i32) -> Option<i64> {
    let naive = NaiveDateTime::parse_from_str(text, "%Y-%m-%d %H:%M:%S").ok()?;
    let offset = FixedOffset::east_opt(offset_hours * 3600)?;
    let datetime = offset.from_local_datetime(&naive).single()?;
    Some(datetime.timestamp())
}

fn main() {
    println!("{:?}", local_string_to_timestamp("2026-09-13 14:30:00", 8));  // Some(1789281000)
    println!("{:?}", local_string_to_timestamp("bad", 8));                  // None
}
```

实测 `Some(1789281000)` 和 `None`。

注意和 17.7 里那个 UTC 时间戳的对照：`2026-09-13 14:30:00 UTC` 是 `1789309800`，而**东八区**的同一个表盘时间要早 8 小时，所以时间戳小了 `8 * 3600 = 28800` 秒。这就是「同一串数字，不同时区，完全不同的时刻」。

:::

::: details 第 4 题

```rust
use chrono::{Datelike, Duration, Months, NaiveDate};

fn days_in_month(year: i32, month: u32) -> Option<u32> {
    let first = NaiveDate::from_ymd_opt(year, month, 1)?;
    let next_month = first.checked_add_months(Months::new(1))?;
    Some((next_month - Duration::days(1)).day())
}

fn main() {
    println!("{:?}", days_in_month(2024, 2));   // Some(29)
    println!("{:?}", days_in_month(2023, 2));   // Some(28)
    println!("{:?}", days_in_month(2026, 9));   // Some(30)
    println!("{:?}", days_in_month(2026, 13));  // None
}
```

实测输出 `Some(29)`、`Some(28)`、`Some(30)`、`None`。

这个写法比「手写闰年判断」可靠得多：**「下个月第一天减一天」把闰年、大小月全部交给日期库处理**，你不需要记住「四年一闰、百年不闰、四百年再闰」的规则。这也是使用日期库的基本原则——**别自己算日历**。

:::

## 17.12 小结

- 「时刻」和「时长」是两种东西：`SystemTime` / `DateTime` 表示时刻，`Duration` 表示时长。

- 测耗时**必须用 `Instant`**（单调时钟）；`SystemTime` 会被 NTP 校时和用户改时间影响。

- 时间戳（从 1970-01-01 UTC 起的秒数）是跨系统交换时间最安全的形式；需要精度就用毫秒。

- `chrono` 的类型分三类：`NaiveDate` / `NaiveTime` / `NaiveDateTime`（不带时区）和 `DateTime<Tz>`（带时区）。**只有带时区的类型才表示绝对时刻**。

- 构造日期用 `from_ymd_opt`（返回 `Option`），解析用 `parse_from_str`（返回 `Result`），不要用会 panic 的旧 API。

- 格式化用 `strftime` 风格占位符（`%Y-%m-%d %H:%M:%S`）；解析外部数据时永远不要 `unwrap`。

- **内部统一用 UTC 或时间戳存储、比较、传输**，只在展示时转成本地时间；这样夏令时问题只影响展示层。

- `FixedOffset` 表达不了夏令时，需要真实时区规则时用 `chrono-tz`。

- 日历运算（按月、按年）用 `checked_add_months`，它会正确处理 2 月 29 日这类边界；溢出返回 `None` 而不是 panic。

- 别自己算日历：`days_in_month` 这类需求用「下个月第一天减一天」交给库处理，比手写闰年判断可靠。

下一章讲**文本处理与正则**：Unicode 与 UTF-8 的进阶细节、字符串性能与 `Cow`，以及用 `regex` crate 做匹配、提取和替换。
