//! Turkish text handling: the dotted and dotless i, folding for search,
//! the order of the Turkish alphabet, and the small jobs around them.
//!
//! # What is here
//!
//! - [`nfc`]: canonical composition, the first step of everything else,
//!   with a fast path for text that is composed already.
//! - [`lowercase`], [`uppercase`], [`titlecase`], [`eq_ignore_case`],
//!   [`cmp_ignore_case`]: case mapping where `I` is the capital of `ı` and
//!   `İ` of `i`.
//! - [`search_fold`], [`FoldOptions`], [`accent_fold`], [`ascii_fold`]:
//!   folding text so that every spelling of a word meets the same key, with
//!   the switches a search index needs.
//! - [`lowercase_with_map`], [`search_fold_with_map`], [`OffsetMap`]: the
//!   same folds with a byte map back to the original, for highlighting a
//!   match in the spelling it was written in.
//! - [`sort_key`], [`compare`], [`CaseRule`]: the order of the Turkish
//!   alphabet, as a key a plain byte comparison orders correctly (for a
//!   database column) and as a comparator (for sorting in memory).
//! - [`words`], [`tokenize`], [`join_initials`]: splitting folded text into
//!   words, with dotted initials joined ("A.Ş." is one word).
//! - [`date`]: the names of the months and the days, and dates written out.
//! - [`suffix`]: attaching a suffix to a proper noun with the apostrophe and
//!   vowel harmony ("Ayşe'nin", "İstanbul'da").
//! - [`number`]: numbers in words ("iki bin yirmi altı", "birinci").
//! - [`letters`]: the Turkish alphabet, its vowels and consonants.
//!
//! # Why not `str::to_lowercase`
//!
//! Rust's case mapping is language-blind: `"I".to_lowercase()` is `"i"`,
//! and `"İ".to_lowercase()` is `"i"` followed by a combining dot. In
//! Turkish `I` is the capital of `ı` and `İ` of `i`, so `IŞIK` must become
//! `ışık` and `İSTANBUL` must become `istanbul`:
//!
//! ```
//! assert_eq!(abece::lowercase("IŞIK İSTANBUL"), "ışık istanbul");
//! assert_eq!(abece::uppercase("ışık istanbul"), "IŞIK İSTANBUL");
//! ```
//!
//! # Stability
//!
//! The output of [`search_fold`] (with any [`FoldOptions`]) and of
//! [`sort_key`] is stable within a major version: a key or an index built
//! with one `0.x` release reads correctly with every later `0.x` release.
//! A change to either is a breaking change and is released as one.
//!
//! # `no_std`
//!
//! The crate needs `alloc` and nothing else from the standard library.
//! Disable the default `std` feature for a `no_std` target.
#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(docsrs, feature(doc_cfg))]

extern crate alloc;

mod case;
pub mod date;
mod fold;
pub mod letters;
mod nfc;
pub mod number;
mod offset_map;
mod sort;
pub mod suffix;
mod token;

pub use case::{
    cmp_ignore_case, eq_ignore_case, lowercase, lowercase_with_map, titlecase, uppercase,
};
pub use fold::{
    Accents, FoldOptions, accent_fold, ascii_fold, fold_with_map, search_fold, search_fold_with,
    search_fold_with_map,
};
pub use nfc::{is_nfc, nfc};
pub use offset_map::OffsetMap;
pub use sort::{CaseRule, ORDER, compare, compare_in, sort_key, sort_key_in};
pub use token::{
    Token, Tokens, WordOptions, join_initials, tokenize, tokenize_stems, words, words_with,
};

/// The names most programs need, for a glob import.
///
/// ```
/// use abece::prelude::*;
///
/// assert_eq!(lowercase("IŞIK"), "ışık");
/// assert_eq!(search_fold("IŞIK"), "işik");
/// ```
pub mod prelude {
    pub use crate::case::{eq_ignore_case, lowercase, titlecase, uppercase};
    pub use crate::fold::{Accents, FoldOptions, accent_fold, ascii_fold, search_fold};
    pub use crate::nfc::nfc;
    pub use crate::sort::{CaseRule, compare, sort_key};
    pub use crate::token::{WordOptions, words};
}

// The README's example runs with the doc tests, so that the first code a
// reader sees cannot drift from the crate. The item exists only then.
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
pub struct ReadmeDoctests;
