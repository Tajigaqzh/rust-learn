//! 第 17 章配套代码：日期与时间。
//!
//! 运行方式：`cargo run`，输出接在第 16 章后面。
//!
//! 演示代码里所有「具体时刻」都用固定值，这样输出可复现；
//! 只有「现在」相关的比较才用实时时间，而且只打印布尔结果。

use chrono::{Datelike, Duration, FixedOffset, Months, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Utc};
use std::time::{Duration as StdDuration, Instant, SystemTime, UNIX_EPOCH};

/// 演示系统时间的两种类型：`SystemTime`（真实时刻）与 `Instant`（单调时钟）。
fn clocks_demo() {
    println!("\n--- 1. 两种「时间」：时刻与时长 ---");

    let now = SystemTime::now();
    let since_epoch: StdDuration = now.duration_since(UNIX_EPOCH).unwrap();
    println!("    现在晚于 1970-01-01：{}", now > UNIX_EPOCH);
    println!(
        "    时间戳已经超过 2020 年：{}",
        since_epoch.as_secs() > 1_577_836_800
    );

    // 测耗时要用 Instant：它只往前走，不受系统时间调整影响。
    let start = Instant::now();
    let mut total: u64 = 0;
    for i in 0..100_000u64 {
        total += i;
    }
    let elapsed = start.elapsed();
    println!("    循环求和 total = {total}");
    println!("    耗时大于零：{}", elapsed > StdDuration::ZERO);

    let ninety = StdDuration::from_secs(90);
    println!(
        "    Duration 90 秒 = {} 分 {} 秒 = {} 毫秒",
        ninety.as_secs() / 60,
        ninety.as_secs() % 60,
        ninety.as_millis()
    );
}

/// 演示 chrono 的日期、时间、格式化与时区。
fn chrono_demo() {
    println!("\n--- 2. chrono：日期与时间 ---");

    let date = NaiveDate::from_ymd_opt(2026, 9, 13).unwrap();
    let time = NaiveTime::from_hms_opt(14, 30, 0).unwrap();
    let datetime = NaiveDateTime::new(date, time);

    println!("    日期 = {date}");
    println!(
        "    年 = {}，月 = {}，日 = {}，星期 = {:?}",
        date.year(),
        date.month(),
        date.day(),
        date.weekday()
    );
    println!("    时间 = {time}");
    println!("    日期时间 = {datetime}");

    println!("\n--- 3. 格式化与解析 ---");
    println!("    标准格式 = {}", datetime.format("%Y-%m-%d %H:%M:%S"));
    println!("    中文格式 = {}", datetime.format("%Y 年 %m 月 %d 日"));
    println!("    紧凑格式 = {}", datetime.format("%Y%m%dT%H%M%S"));

    let parsed = NaiveDateTime::parse_from_str("2026-09-13 14:30:00", "%Y-%m-%d %H:%M:%S");
    println!("    解析回来 = {parsed:?}");
    let parsed_date = NaiveDate::parse_from_str("13/09/2026", "%d/%m/%Y");
    println!("    解析日期 = {parsed_date:?}");
    let bad = NaiveDate::parse_from_str("2026-13-45", "%Y-%m-%d");
    println!("    解析非法日期 = {bad:?}");

    println!("\n--- 4. 时区 ---");
    let utc = Utc.with_ymd_and_hms(2026, 9, 13, 14, 30, 0).unwrap();
    let beijing = FixedOffset::east_opt(8 * 3600).unwrap();
    let local = utc.with_timezone(&beijing);
    println!("    UTC 时间 = {utc}");
    println!("    东八区时间 = {local}");
    println!("    两者时间戳相同：{}", utc.timestamp() == local.timestamp());
    println!("    东八区偏移 = {}", beijing);

    println!("\n--- 5. 时间的算术 ---");
    let later = date + Duration::days(30);
    println!("    2026-09-13 加 30 天 = {later}");
    println!("    两者的天数差 = {}", (later - date).num_days());
    let next_month = date.checked_add_months(Months::new(1)).unwrap();
    println!("    加 1 个月 = {next_month}");
    let leap = NaiveDate::from_ymd_opt(2024, 2, 29).unwrap();
    println!("    2024-02-29 加 1 年 = {}", leap.checked_add_months(Months::new(12)).unwrap());
    println!("    2023 年 2 月有 29 号吗：{}", NaiveDate::from_ymd_opt(2023, 2, 29).is_some());

    println!("\n--- 6. 时间戳与日期互转 ---");
    println!("    时间戳（秒）= {}", utc.timestamp());
    println!("    时间戳（毫秒）= {}", utc.timestamp_millis());
    let from_seconds = Utc.timestamp_opt(1_787_000_000, 0).single().unwrap();
    println!("    从秒级时间戳还原 = {from_seconds}");
    let from_millis = chrono::DateTime::from_timestamp_millis(1_787_000_000_000).unwrap();
    println!("    从毫秒时间戳还原 = {from_millis}");
}

/// 演示日期与时间相关的常见坑。
fn pitfalls_demo() {
    println!("\n--- 7. 几个容易踩的坑 ---");
    println!("    ① 测耗时用 Instant，不用 SystemTime：后者会被系统时间调整影响");
    println!("    ② NaiveDateTime 不带时区，只能表示「某个地方的表盘时间」");
    println!("    ③ 跨时区传递、存数据库要统一用 DateTime<Utc> 或时间戳");
    println!("    ④ 解析失败返回 Result，不要直接 unwrap 用户输入");
    println!("    ⑤ 加月份用 checked_add_months，自动处理 2 月 29 日这种边界");

    let midnight = NaiveDate::from_ymd_opt(2026, 9, 13)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap();
    println!("    当天零点 = {midnight}");
    println!("    下一天开始 = {}", midnight + Duration::days(1));
}

/// 演示标准库与 chrono 的日期时间用法。
pub fn datetime_demo() {
    println!("\n========== rust17_datetime: 日期与时间 ==========");
    clocks_demo();
    chrono_demo();
    pitfalls_demo();
    println!("\n========== 日期与时间演示结束 ==========");
}
