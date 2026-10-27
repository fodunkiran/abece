//! The search fold: abece against ICU4X's Turkic case fold, plus the folds
//! that have no counterpart (the ASCII fold, the fold with a byte map).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::arithmetic_side_effects
)]

mod inputs;

use std::hint::black_box;

use abece::FoldOptions;
use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use icu_casemap::CaseMapper;

fn search_fold(c: &mut Criterion) {
    let mapper = CaseMapper::new();
    let mut group = c.benchmark_group("search_fold");
    for (name, text) in inputs::all() {
        group.throughput(Throughput::Bytes(text.len() as u64));
        group.bench_with_input(BenchmarkId::new("abece", name), &text, |b, text| {
            b.iter(|| abece::search_fold(black_box(text)));
        });
        group.bench_with_input(
            BenchmarkId::new("icu_casemap-fold_turkic", name),
            &text,
            |b, text| {
                b.iter(|| mapper.fold_turkic_string(black_box(text)));
            },
        );
    }
    group.finish();
}

fn other_folds(c: &mut Criterion) {
    let mut group = c.benchmark_group("folds");
    for (name, text) in inputs::all() {
        group.throughput(Throughput::Bytes(text.len() as u64));
        group.bench_with_input(BenchmarkId::new("ascii-options", name), &text, |b, text| {
            b.iter(|| abece::search_fold_with(black_box(text), &FoldOptions::ASCII));
        });
        group.bench_with_input(
            BenchmarkId::new("search_fold_with_map", name),
            &text,
            |b, text| {
                b.iter(|| abece::search_fold_with_map(black_box(text)));
            },
        );
        group.bench_with_input(BenchmarkId::new("accent_fold", name), &text, |b, text| {
            b.iter(|| abece::accent_fold(black_box(text)));
        });
        group.bench_with_input(BenchmarkId::new("words", name), &text, |b, text| {
            b.iter(|| abece::words(black_box(text)));
        });
    }
    group.finish();
}

criterion_group!(benches, search_fold, other_folds);
criterion_main!(benches);
