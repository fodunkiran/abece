# abece

Turkish text handling for Rust: case mapping with the dotted and dotless i,
folding for search, the order of the Turkish alphabet as a key a database
can index, and the small jobs around them (suffixes, numbers and dates in
words). `no_std` with `alloc`, one required dependency, no `unsafe`.

```rust
use abece::prelude::*;
use abece::suffix::{Suffix, attach};

// Rust's own lower case is language-blind: "I" becomes "i", "İ" becomes "i̇".
assert_eq!(lowercase("IŞIK İSTANBUL"), "ışık istanbul");
assert_eq!(uppercase("ışık istanbul"), "IŞIK İSTANBUL");
assert_eq!(titlecase("ışık istanbul"), "Işık İstanbul");

// The search fold: every spelling of a word meets the same key.
assert_eq!(search_fold("ISTANBUL"), search_fold("İstanbul"));
assert_eq!(search_fold("ŞAHİN Çağrı"), "şahin çağri");

// The Turkish alphabet's order, as a key a plain byte comparison sorts correctly.
let mut names = ["Şule", "Sinan", "Çetin", "Cem", "Işık", "İnci"];
names.sort_by_cached_key(|n| sort_key(n));
assert_eq!(names, ["Cem", "Çetin", "Işık", "İnci", "Sinan", "Şule"]);

// Suffixes with the apostrophe, vowel harmony and the buffer letters.
assert_eq!(attach("Ayşe", Suffix::Genitive), "Ayşe'nin");
assert_eq!(attach("İstanbul", Suffix::Locative), "İstanbul'da");
assert_eq!(attach("2026", Suffix::Dative), "2026'ya");
```

## What is here

| Area | Functions | Notes |
|---|---|---|
| Normalization | `nfc`, `is_nfc` | The first step of every fold; borrowed when already composed, ASCII never scanned |
| Case | `lowercase`, `uppercase`, `titlecase`, `eq_ignore_case`, `cmp_ignore_case` | `I` is the capital of `ı`, `İ` of `i`; `Cow` return, borrowed when nothing changes |
| Search fold | `search_fold`, `search_fold_with`, `FoldOptions`, `Accents` | `ı` merged into `i`, `ç ğ ö ş ü` kept; switches for the merge, accents and marks |
| Accent fold | `accent_fold`, `ascii_fold` | Case-preserving: `Kâğıt` to `Kağıt`; `Çağrı` to `Cagri` |
| Byte map | `lowercase_with_map`, `search_fold_with_map`, `fold_with_map`, `OffsetMap` | A match in the folded text, cut from the original in its own spelling |
| Order | `sort_key`, `sort_key_in`, `compare`, `compare_in`, `CaseRule` | A storable key whose byte order is the alphabet's; a comparator that builds no key |
| Words | `words`, `tokenize`, `tokenize_stems`, `join_initials`, `WordOptions` | Name matching: "DENİZ GIDA LTD.ŞTİ." is `deniz gida ltd sti`, "A.Ş." is `as` |
| Letters | `letters::is_vowel`, `is_back_vowel`, `is_voiceless_consonant`, `ALPHABET`, … | The 29 letters and their classes |
| Suffixes | `suffix::attach`, `attach_spoken`, `Harmony`, `Suffix` | Proper nouns, numbers and abbreviations: "TBMM'ye", "A.Ş.'ye", "1.000'e", "5'te", "Ahmetler" |
| Numbers | `number::to_words`, `to_words_signed`, `ordinal` | "iki bin yirmi altı", "dördüncü" |
| Dates | `date::Date`, `Month`, `Weekday`, `DateStyle`, `format_range` | "25 Temmuz 2026 Cumartesi", "20 Temmuz – 31 Ağustos 2026" |

Features: `std` (default; the crate itself needs only `alloc`), `serde`
(derives on the option and date types), `jiff` (conversions from and to
`jiff::civil::Date` and `Weekday`).

## Why not ICU4X or focaccia

Both do Turkic case mapping well, and the benchmarks below compare against
them. What they do not offer is the rest of the job:

- **A search fold with a byte map.** A full-text index stores folded text
  and a result page shows the original; `search_fold_with_map` returns the
  offsets that connect the two. ICU4X's `fold_turkic_string` folds; it
  cannot say where a match came from.
- **A sort key that stays put.** `icu_collator` writes sort keys too, but
  as bytes that ICU4X says to presume invalidated by any CLDR, Unicode or
  ICU4X update, with no promise to keep the old algorithm. `sort_key` is a
  string (ASCII for Latin and Turkish text) whose plain byte order is the
  Turkish alphabet's, stored once in an ordinary text column and indexed,
  and its output is covered by the stability promise below.
- **The search fold's rule.** Merging `ı` into `i` while keeping `ş` apart
  from `s` is not a Unicode operation; it is a choice about how people type
  Turkish on the wrong keyboard (see [docs/DESIGN.md](docs/DESIGN.md)).
- **Words, suffixes, numbers, dates**, which no Unicode library covers.
- **Size.** ICU4X carries its own data tables for every locale; this crate
  carries `unicode-normalization` and a few match statements.

For plain language-aware case mapping across many languages, ICU4X is the
right tool; for a Turkic-aware `casecmp`, focaccia. This crate is for
programs whose text is Turkish.

## Stability promise

The output of `search_fold` (with any `FoldOptions`) and of `sort_key` is
stable within a major version: an index or a key column built with one
`0.x` release reads correctly with every later `0.x` release. A change to
either is a breaking change and is released as one.

The promise is tested, not just stated: `tests/model/` holds a plain,
character-at-a-time model of each stored output, and `tests/fold.rs`,
`tests/words.rs` and `tests/sort_key.rs` run the crate against the models
on thousands of generated strings, so an optimisation that moves a byte
fails the build.

## Performance

Every fold checks for ASCII first and takes a byte loop; non-ASCII text
goes through the NFC quick check and a pipeline that borrows the text
until the first character that changes. The comparator and the
case-insensitive comparison allocate nothing for ordinary text.

Measured with `cargo bench` (criterion) on the author's machine, an
x86-64 desktop, Rust 1.99; lower is better, throughput in parentheses.
Inputs: a short ASCII title (49 B), a short Turkish title (65 B), and four
kilobytes of English and of Turkish prose.

| `lowercase` | ASCII title (49 B) | Turkish title (65 B) | English 4 KiB | Turkish 4 KiB |
|---|---|---|---|---|
| abece | 37.5 ns | 395 ns | 4.54 µs | 18.1 µs |
| std | 34.2 ns | 299 ns | 173 ns | 14.4 µs |
| icu_casemap | 558 ns | 669 ns | 36.9 µs | 33.8 µs |

| `uppercase` | ASCII title (49 B) | Turkish title (65 B) | English 4 KiB | Turkish 4 KiB |
|---|---|---|---|---|
| abece | 94.7 ns | 557 ns | 30.2 µs | 29.5 µs |
| std | 34.2 ns | 268 ns | 8.31 µs | 13.5 µs |
| icu_casemap | 592 ns | 699 ns | 37.5 µs | 34.5 µs |

| `eq_ignore_case` | ASCII title (49 B) | Turkish title (65 B) | English 4 KiB | Turkish 4 KiB |
|---|---|---|---|---|
| abece | 388 ns | 660 ns | 33.4 µs | 42.5 µs |
| focaccia | 611 ns | 1 µs | 75.2 µs | 66.2 µs |
| std-lowercase-eq | 216 ns | 613 ns | 13.4 µs | 28.7 µs |

| `search_fold` | ASCII title (49 B) | Turkish title (65 B) | English 4 KiB | Turkish 4 KiB |
|---|---|---|---|---|
| abece | 34.6 ns | 359 ns | 173 ns | 18 µs |
| icu_casemap-fold_turkic | 643 ns | 680 ns | 37 µs | 35 µs |

| `folds` | ASCII title (49 B) | Turkish title (65 B) | English 4 KiB | Turkish 4 KiB |
|---|---|---|---|---|
| ascii-options | 34.6 ns | 496 ns | 175 ns | 27.4 µs |
| search_fold_with_map | 61.9 ns | 377 ns | 735 ns | 18.8 µs |
| accent_fold | 8.03 ns | 84.7 ns | 41 ns | 5.14 µs |
| words | 476 ns | 863 ns | 37.9 µs | 53.3 µs |

| `sort_key` | ASCII title (49 B) | Turkish title (65 B) | English 4 KiB | Turkish 4 KiB |
|---|---|---|---|---|
| abece | 147 ns | 741 ns | 13.5 µs | 51.2 µs |

| `compare` | ASCII title (49 B) | Turkish title (65 B) | English 4 KiB | Turkish 4 KiB |
|---|---|---|---|---|
| abece | 65.4 ns | 74.2 ns | 2.72 µs | 2.71 µs |
| icu_collator | 63.2 ns | 67.3 ns | 942 ns | 941 ns |
| str-cmp | 2.85 ns | 2.72 ns | 42.7 ns | 48.8 ns |

| `sort-1000-titles` | time |
|---|---|
| abece-compare | 1.49 ms |
| abece-sort_key-cached | 686 µs |
| icu_collator | 673 µs |

Run `cargo bench` for the numbers on your machine; `benches/` compares
against `str::to_lowercase`, `icu_casemap` (locale `tr`), `focaccia`'s
Turkic fold and `icu_collator` (locale `tr`).

## Design and limits

[docs/DESIGN.md](docs/DESIGN.md) describes the folding pipeline and its
order, why `ı` merges in search while `ş` stays, the sort key's encoding,
and what is deliberately left out: anything that needs a dictionary
(consonant softening of common nouns, stemming, deasciification) and
anything that is a caller's policy (numeric collation, stop words).

## Minimum supported Rust version

1.85 (edition 2024). The development dependencies need 1.88.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT) at your option.

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in this crate by you, as defined in the Apache-2.0
license, shall be dual licensed as above, without any additional terms or
conditions.
