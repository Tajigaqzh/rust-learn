//! 第 23 章配套基准：用 criterion 测量几种写法的性能差异。
//!
//! 运行方式：`cargo bench`（想快速跑一遍可以用 `cargo bench -- --quick`）。
//!
//! 基准代码是**自包含**的：被测函数就写在这个文件里，不依赖 crate 内部实现——
//! 因为二进制 crate 的集成测试和基准都只能看到公开接口。

use criterion::{Criterion, criterion_group, criterion_main};
use std::collections::HashSet;
use std::hint::black_box;

/// 手写循环求和。
fn sum_with_loop(values: &[i64]) -> i64 {
    let mut total = 0;
    for value in values {
        total += value;
    }
    total
}

/// 迭代器求和。
fn sum_with_iter(values: &[i64]) -> i64 {
    values.iter().sum()
}

/// 用 `format!` 拼一百次。
fn join_with_format(parts: &[&str]) -> String {
    let mut result = String::new();
    for (index, part) in parts.iter().enumerate() {
        if index > 0 {
            result = format!("{result},{part}");
        } else {
            result = format!("{part}");
        }
    }
    result
}

/// 用 `push_str` 拼一百次（复用同一块缓冲区）。
fn join_with_push(parts: &[&str]) -> String {
    let mut result = String::new();
    for (index, part) in parts.iter().enumerate() {
        if index > 0 {
            result.push(',');
        }
        result.push_str(part);
    }
    result
}

fn bench_sum(c: &mut Criterion) {
    let values: Vec<i64> = (0..1000).collect();
    let mut group = c.benchmark_group("求和 1000 个元素");
    group.bench_function("手写循环", |b| {
        b.iter(|| sum_with_loop(black_box(&values)))
    });
    group.bench_function("迭代器 sum", |b| {
        b.iter(|| sum_with_iter(black_box(&values)))
    });
    group.finish();
}

fn bench_join(c: &mut Criterion) {
    let parts: Vec<&str> = (0..100).map(|_| "segment").collect();
    let mut group = c.benchmark_group("拼接 100 段字符串");
    group.bench_function("format! 反复重建", |b| {
        b.iter(|| join_with_format(black_box(&parts)))
    });
    group.bench_function("push_str 复用缓冲", |b| {
        b.iter(|| join_with_push(black_box(&parts)))
    });
    group.finish();
}

fn bench_lookup(c: &mut Criterion) {
    let values: Vec<i64> = (0..1000).collect();
    let set: HashSet<i64> = values.iter().copied().collect();
    let needle = 999;

    let mut group = c.benchmark_group("查找一个元素");
    group.bench_function("Vec::contains", |b| {
        b.iter(|| black_box(&values).contains(black_box(&needle)))
    });
    group.bench_function("HashSet::contains", |b| {
        b.iter(|| black_box(&set).contains(black_box(&needle)))
    });
    group.finish();
}

criterion_group!(benches, bench_sum, bench_join, bench_lookup);
criterion_main!(benches);
