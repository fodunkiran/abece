//! The sort key and the comparator: abece against ICU4X's collator with
//! the Turkish locale (which compares but exports no key), on single
//! comparisons and on sorting a list of titles.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing
)]

mod inputs;

use std::hint::black_box;

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use icu_collator::options::CollatorOptions;
use icu_collator::{Collator, CollatorPreferences};
use icu_locale_core::locale;

fn collator() -> icu_collator::CollatorBorrowed<'static> {
    let prefs = CollatorPreferences::from(locale!("tr"));
    Collator::try_new(prefs, CollatorOptions::default()).expect("the Turkish collation")
}

fn sort_key(c: &mut Criterion) {
    let mut group = c.benchmark_group("sort_key");
    for (name, text) in inputs::all() {
        group.throughput(Throughput::Bytes(text.len() as u64));
        group.bench_with_input(BenchmarkId::new("abece", name), &text, |b, text| {
            b.iter(|| abece::sort_key(black_box(text)));
        });
    }
    group.finish();
}

fn compare(c: &mut Criterion) {
    let collator = collator();
    let mut group = c.benchmark_group("compare");
    for (name, text) in inputs::all() {
        // Two texts that differ only in their last character, so every
        // character before it is compared.
        let mut other = text.clone();
        other.pop();
        other.push('z');
        group.throughput(Throughput::Bytes(text.len() as u64));
        group.bench_with_input(
            BenchmarkId::new("abece", name),
            &(&text, &other),
            |b, (a, o)| {
                b.iter(|| abece::compare(black_box(a), black_box(o)));
            },
        );
        group.bench_with_input(
            BenchmarkId::new("icu_collator", name),
            &(&text, &other),
            |b, (a, o)| {
                b.iter(|| collator.compare(black_box(a), black_box(o)));
            },
        );
        group.bench_with_input(
            BenchmarkId::new("str-cmp", name),
            &(&text, &other),
            |b, (a, o)| {
                b.iter(|| black_box(a).cmp(black_box(o)));
            },
        );
    }
    group.finish();
}

/// A thousand titles made from the Turkish sample, each a different window
/// of it, so that keys differ early and late.
fn titles() -> Vec<String> {
    let text = inputs::turkish_long();
    let chars: Vec<char> = text.chars().collect();
    (0..1000)
        .map(|i| {
            let start = (i * 37) % (chars.len() - 40);
            chars[start..start + 40].iter().collect()
        })
        .collect()
}

fn sort_titles(c: &mut Criterion) {
    let collator = collator();
    let titles = titles();
    let mut group = c.benchmark_group("sort-1000-titles");
    group.bench_function("abece-compare", |b| {
        b.iter_batched(
            || titles.clone(),
            |mut titles| titles.sort_by(|a, b| abece::compare(a, b)),
            criterion::BatchSize::SmallInput,
        );
    });
    group.bench_function("abece-sort_key-cached", |b| {
        b.iter_batched(
            || titles.clone(),
            |mut titles| titles.sort_by_cached_key(|t| abece::sort_key(t)),
            criterion::BatchSize::SmallInput,
        );
    });
    group.bench_function("icu_collator", |b| {
        b.iter_batched(
            || titles.clone(),
            |mut titles| titles.sort_by(|a, b| collator.compare(a, b)),
            criterion::BatchSize::SmallInput,
        );
    });
    group.finish();
}

criterion_group!(benches, sort_key, compare, sort_titles);
criterion_main!(benches);
