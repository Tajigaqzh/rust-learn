//! 第 4 章配套代码：控制流。
//!
//! 运行方式：`cargo run`，输出接在第 3 章后面。

/// 按分数给等级，演示 `if` / `else if` 链：从上到下判断，第一条命中就结束。
fn grade(score: u32) -> &'static str {
    if score >= 90 {
        "优秀"
    } else if score >= 75 {
        "良好"
    } else if score >= 60 {
        "及格"
    } else {
        "不及格"
    }
}

/// 两数取大，`if` 直接当表达式用。
fn max_of(a: i32, b: i32) -> i32 {
    if a >= b { a } else { b }
}

/// 用 `loop` 累加 `1..=limit`，这里的 `break` 不带值，只负责结束循环。
fn sum_with_loop(limit: u32) -> u32 {
    let mut total = 0;
    let mut current = 1;
    loop {
        if current > limit {
            break;
        }
        total += current;
        current += 1;
    }
    total
}

/// 用 `while` 累加 `1..=limit`，和 `sum_with_loop` 是同一件事。
fn sum_with_while(limit: u32) -> u32 {
    let mut total = 0;
    let mut current = 1;
    while current <= limit {
        total += current;
        current += 1;
    }
    total
}

/// 演示 `if` / `loop` / `while` / `for`、`break`、`continue` 与循环标签。
pub fn control_flow_demo() {
    println!("\n========== rust04_control_flow: 控制流 ==========");

    // 1. if 与 else if
    println!("\n--- 1. if 与 else if ---");
    for score in [95, 82, 60, 41] {
        println!("{score} 分 -> {}", grade(score));
    }

    // 2. if 是表达式：两条分支各给一个值
    println!("\n--- 2. if 作为表达式 ---");
    let (a, b) = (7, 3);
    println!("max_of({a}, {b}) = {}", max_of(a, b));
    let signed = -5;
    println!("|{signed}| = {}", if signed < 0 { -signed } else { signed });

    // 语句风格要先声明 mut 再改，表达式风格一次赋值完成
    let mut level = "普通";
    if a > 5 {
        level = "优先";
    }
    let level_expr = if a > 5 { "优先" } else { "普通" };
    println!("语句风格 {level} / 表达式风格 {level_expr}");

    // 3. loop 是唯一能 break 出值的循环
    println!("\n--- 3. loop 与带值的 break ---");
    let mut power = 1;
    let reached = loop {
        power *= 2;
        if power > 100 {
            break power;
        }
    };
    println!("2 的幂里第一个超过 100 的是 {reached}");

    // 4. while 先判断条件，再决定要不要执行循环体
    println!("\n--- 4. while：条件驱动 ---");
    let mut fuel = 3;
    while fuel > 0 {
        println!("倒计时 {fuel}");
        fuel -= 1;
    }
    println!("循环结束，fuel = {fuel}");

    // 5. while + continue 的经典坑：推进语句写在 continue 后面就永远走不到
    println!("\n--- 5. while + continue 的坑 ---");
    // 下面是错误示范，加了一个「保险丝」计数，避免演示时真的卡死
    let mut i = 0;
    let mut spins = 0;
    while i < 3 {
        spins += 1;
        if spins > 5 {
            println!("错误示范：转了 {spins} 圈，i 一直是 {i}（主动退出）");
            break;
        }
        if i % 2 == 0 {
            continue; // 这一跳把下面的 i += 1 跳过了
        }
        i += 1;
    }

    // 正确写法：先把计数器推进，再做 continue 判断
    let mut j = 0;
    let mut evens = 0;
    while j < 6 {
        let current = j;
        j += 1;
        if current % 2 != 0 {
            continue;
        }
        evens += 1;
    }
    println!("正确写法：0..6 里的偶数有 {evens} 个");

    // 6. for 遍历范围：含尾用 ..=，倒序用 rev()，跳步用 step_by()
    println!("\n--- 6. for：范围与步长 ---");
    print!("1..=5              -> ");
    for i in 1..=5 {
        print!("{i} ");
    }
    println!();
    print!("(0..5).rev()       -> ");
    for i in (0..5).rev() {
        print!("{i} ");
    }
    println!();
    print!("(0..10).step_by(3) -> ");
    for i in (0..10).step_by(3) {
        print!("{i} ");
    }
    println!();

    // 7. for 遍历容器：数组、Vec、切片、带下标
    println!("\n--- 7. for 遍历各种容器 ---");
    let scores = [88, 92, 79];
    print!("数组按值迭代: ");
    for score in scores {
        print!("{score} ");
    }
    println!();

    let names = vec!["Ada", "Linus", "Graydon"];
    print!("Vec 借用迭代: ");
    for name in &names {
        print!("{name} ");
    }
    println!("（循环后 names 还能用，长度 {}）", names.len());

    for (index, name) in names.iter().enumerate() {
        println!("enumerate -> {index}: {name}");
    }

    print!("切片迭代: ");
    for score in &scores[1..] {
        print!("{score} ");
    }
    println!();

    // 8. break 结束当前循环，continue 跳到下一轮；标签用来跳出多层
    println!("\n--- 8. break、continue 与循环标签 ---");
    print!("跳过偶数: ");
    for i in 1..=8 {
        if i % 2 == 0 {
            continue;
        }
        print!("{i} ");
    }
    println!();

    print!("1..20 里 3 和 5 的第一个公倍数: ");
    for i in 1..20 {
        if i % 3 == 0 && i % 5 == 0 {
            print!("{i}");
            break;
        }
    }
    println!();

    let mut inner_runs = 0;
    for _ in 0..3 {
        for j in 0..10 {
            inner_runs += 1;
            if j == 2 {
                break; // 只跳出内层，外层继续
            }
        }
    }
    println!("无标签的 break 只跳出内层：内层共进入 {inner_runs} 次");

    let mut outer_runs = 0;
    'outer: for i in 1..=4 {
        for j in 1..=4 {
            outer_runs += 1;
            if i * j > 6 {
                break 'outer; // 一次跳出两层
            }
        }
    }
    println!("带标签的 break 一次跳出两层：内层共进入 {outer_runs} 次");

    print!("continue 'rows 只打印下三角: ");
    'rows: for y in 0..3 {
        for x in 0..3 {
            if x > y {
                continue 'rows; // 丢掉内层剩下的部分，直接进入外层下一轮
            }
            print!("({x},{y}) ");
        }
    }
    println!();

    // 9. while let：每轮都尝试匹配，直到不匹配为止
    println!("\n--- 9. while let：边循环边取走 ---");
    let mut stack = vec![1, 2, 3];
    print!("从栈顶弹出: ");
    while let Some(top) = stack.pop() {
        print!("{top} ");
    }
    println!("（结束后 stack 长度 {}）", stack.len());

    // 10. 带标签的块：break 也能从普通块里把值带出来
    println!("\n--- 10. 带标签的块也能带值 ---");
    let found = 'search: {
        for candidate in 1..=10 {
            if candidate % 7 == 0 {
                break 'search candidate;
            }
        }
        0 // 循环走完都没找到时，块的值是这个
    };
    println!("'search 块的值是 {found}");

    // 11. 同一个任务，三种循环写法都能做
    println!("\n--- 11. 同一个任务的三种写法 ---");
    let for_sum: u32 = (1..=5).sum();
    println!(
        "1..=5 求和 -> loop: {} / while: {} / for: {for_sum}",
        sum_with_loop(5),
        sum_with_while(5)
    );

    println!("\n========== 控制流演示结束 ==========");
}
