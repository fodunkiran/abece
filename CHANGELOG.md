# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).
The output of `search_fold` (with any `FoldOptions`) and of `sort_key` is
part of the public contract: a change to either is a breaking change.

## [Unreleased]

## [0.1.0] - 2026-10-27

### Added

- `nfc` and `is_nfc`: canonical composition with a fast path for ASCII and
  for text that is composed already.
- `lowercase`, `uppercase`, `titlecase`: case mapping where `I` is the
  capital of `ı` and `İ` of `i`, borrowing the text when nothing changes.
- `eq_ignore_case` and `cmp_ignore_case`: comparison under the Turkish
  lower case without allocating.
- `search_fold`, `search_fold_with`, `FoldOptions`, `Accents`: the search
  fold (`ı` merged into `i`, `ç ğ ö ş ü` kept) and its switches.
- `accent_fold` and `ascii_fold`: case-preserving accent folds.
- `lowercase_with_map`, `search_fold_with_map`, `fold_with_map`,
  `OffsetMap`: the folds with a byte map back to the original spelling.
- `sort_key`, `sort_key_in`, `compare`, `compare_in`, `CaseRule`, `ORDER`:
  the Turkish alphabet's order as a storable key and as an allocation-free
  comparator.
- `tokenize`, `tokenize_stems`, `join_initials`, `words`, `words_with`,
  `Token`, `WordOptions`: words for matching, with dotted initials joined.
- `letters`: the alphabet, its vowel classes and consonants.
- `suffix`: `attach`, `attach_spoken`, `suffix_for`, `Harmony`, `Suffix`:
  suffixes on proper nouns, numbers (thousands grouped with dots read as
  one number) and abbreviations (dotted initials and single capital
  letters spelled out, the last word of a name deciding) with the
  apostrophe and vowel harmony.
- `number`: `to_words`, `to_words_signed`, `ordinal`: numbers in words.
- `date`: `Date`, `Month`, `Weekday`, `DateStyle`, `format_range`: month and
  weekday names and dates written out.
- Features: `std` (default), `serde`, `jiff`.

[Unreleased]: https://github.com/fodunkiran/abece/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/fodunkiran/abece/releases/tag/v0.1.0
