//! Lower case and case-insensitive equality: abece against the standard
//! library (language-blind), ICU4X's `icu_casemap` with the Turkish locale,
//! and focaccia's Turkic case folding.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::arithmetic_side_effects
)]

mod inputs;

use std::hint::black_box;

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use icu_casemap::CaseMapper;
use icu_locale_core::langid;

fn lowercase(c: &mut Criterion) {
    let mapper = CaseMapper::new();
    let tr = langid!("tr");
    let mut group = c.benchmark_group("lowercase");
    for (name, text) in inputs::all() {
        group.throughput(Throughput::Bytes(text.len() as u64));
        group.bench_with_input(BenchmarkId::new("abece", name), &text, |b, text| {
            b.iter(|| abece::lowercase(black_box(text)));
        });
        group.bench_with_input(BenchmarkId::new("std", name), &text, |b, text| {
            b.iter(|| black_box(text).to_lowercase());
        });
        group.bench_with_input(BenchmarkId::new("icu_casemap", name), &text, |b, text| {
            b.iter(|| mapper.lowercase_to_string(black_box(text), &tr));
        });
    }
    group.finish();
}

fn uppercase(c: &mut Criterion) {
    let mapper = CaseMapper::new();
    let tr = langid!("tr");
    let mut group = c.benchmark_group("uppercase");
    for (name, text) in inputs::all() {
        let text = abece::lowercase(&text).into_owned();
        group.throughput(Throughput::Bytes(text.len() as u64));
        group.bench_with_input(BenchmarkId::new("abece", name), &text, |b, text| {
            b.iter(|| abece::uppercase(black_box(text)));
        });
        group.bench_with_input(BenchmarkId::new("std", name), &text, |b, text| {
            b.iter(|| black_box(text).to_uppercase());
        });
        group.bench_with_input(BenchmarkId::new("icu_casemap", name), &text, |b, text| {
            b.iter(|| mapper.uppercase_to_string(black_box(text), &tr));
        });
    }
    group.finish();
}

fn eq_ignore_case(c: &mut Criterion) {
    let mut group = c.benchmark_group("eq_ignore_case");
    for (name, text) in inputs::all() {
        // The same text in the other case: every character must be compared.
        let other = abece::uppercase(&abece::lowercase(&text)).into_owned();
        assert!(abece::eq_ignore_case(&text, &other));
        group.throughput(Throughput::Bytes(text.len() as u64));
        group.bench_with_input(
            BenchmarkId::new("abece", name),
            &(&text, &other),
            |b, (a, o)| {
                b.iter(|| abece::eq_ignore_case(black_box(a), black_box(o)));
            },
        );
        group.bench_with_input(
            BenchmarkId::new("focaccia", name),
            &(&text, &other),
            |b, (a, o)| {
                b.iter(|| focaccia::CaseFold::Turkic.case_eq(black_box(a), black_box(o)));
            },
        );
        group.bench_with_input(
            BenchmarkId::new("std-lowercase-eq", name),
            &(&text, &other),
            |b, (a, o)| {
                b.iter(|| black_box(a).to_lowercase() == black_box(o).to_lowercase());
            },
        );
    }
    group.finish();
}

criterion_group!(benches, lowercase, uppercase, eq_ignore_case);
criterion_main!(benches);
