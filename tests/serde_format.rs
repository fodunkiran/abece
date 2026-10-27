//! The serde representation of every type that has one. The format is
//! pinned: settings and dates kept in configuration must read back after
//! a field or a variant is renamed in the source. And a `Date` is checked
//! as it is read, in every form serde accepts.
#![cfg(feature = "serde")]
// Test code: the no-panic policy is for the library.
#![allow(
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    clippy::string_slice,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic
)]

use std::fmt::Debug;

use abece::date::{Date, DateStyle, Month, Weekday};
use abece::suffix::{Harmony, Suffix};
use abece::{Accents, CaseRule, FoldOptions, WordOptions};
use serde::Serialize;
use serde::de::DeserializeOwned;

/// `value` is written as `json`, and `json` is read back as `value`.
fn assert_format<T>(value: &T, json: &str)
where
    T: Serialize + DeserializeOwned + PartialEq + Debug,
{
    assert_eq!(serde_json::to_string(value).unwrap(), json);
    assert_eq!(&serde_json::from_str::<T>(json).unwrap(), value);
}

#[test]
fn dates_months_weekdays_and_styles() {
    assert_format(
        &Date::new(2026, 7, 25).unwrap(),
        r#"{"year":2026,"month":"July","day":25}"#,
    );
    assert_format(
        &Date::new(-44, 3, 15).unwrap(),
        r#"{"year":-44,"month":"March","day":15}"#,
    );
    assert_format(
        &Month::ALL,
        r#"["January","February","March","April","May","June","July","August","September","October","November","December"]"#,
    );
    assert_format(
        &Weekday::ALL,
        r#"["Monday","Tuesday","Wednesday","Thursday","Friday","Saturday","Sunday"]"#,
    );
    assert_format(
        &[
            DateStyle::Numeric,
            DateStyle::Long,
            DateStyle::LongWithWeekday,
            DateStyle::LongCommaWeekday,
            DateStyle::DayMonth,
            DateStyle::ShortDayMonth,
            DateStyle::MonthYear,
        ],
        r#"["Numeric","Long","LongWithWeekday","LongCommaWeekday","DayMonth","ShortDayMonth","MonthYear"]"#,
    );
}

#[test]
fn fold_word_and_sort_options() {
    assert_format(
        &[Accents::Keep, Accents::FoldForeign, Accents::FoldAll],
        r#"["Keep","FoldForeign","FoldAll"]"#,
    );
    assert_format(
        &FoldOptions::SEARCH,
        r#"{"merge_dotless_i":true,"accents":"Keep","strip_marks":false}"#,
    );
    assert_format(
        &FoldOptions::ASCII,
        r#"{"merge_dotless_i":true,"accents":"FoldAll","strip_marks":true}"#,
    );
    assert_format(
        &WordOptions::new(),
        r#"{"fold":{"merge_dotless_i":true,"accents":"FoldAll","strip_marks":true},"join_initials":true,"drop_apostrophe_suffix":false}"#,
    );
    assert_format(
        &WordOptions::new()
            .fold(FoldOptions::SEARCH)
            .join_initials(false)
            .drop_apostrophe_suffix(true),
        r#"{"fold":{"merge_dotless_i":true,"accents":"Keep","strip_marks":false},"join_initials":false,"drop_apostrophe_suffix":true}"#,
    );
    assert_format(
        &[CaseRule::Turkish, CaseRule::Unicode],
        r#"["Turkish","Unicode"]"#,
    );
}

#[test]
fn suffixes_and_harmony() {
    assert_format(
        &[
            Suffix::Plural,
            Suffix::Genitive,
            Suffix::Dative,
            Suffix::Locative,
            Suffix::Ablative,
            Suffix::Accusative,
            Suffix::Instrumental,
            Suffix::LocativeRelative,
        ],
        r#"["Plural","Genitive","Dative","Locative","Ablative","Accusative","Instrumental","LocativeRelative"]"#,
    );
    assert_format(
        &Harmony::of("Sinop"),
        r#"{"back":true,"rounded":true,"ends_with_vowel":false,"ends_voiceless":true}"#,
    );
    assert_format(
        &Harmony::of("Ayşe"),
        r#"{"back":false,"rounded":false,"ends_with_vowel":true,"ends_voiceless":false}"#,
    );
}

#[test]
fn a_date_is_checked_as_it_is_read() {
    for (json, error) in [
        (
            r#"{"year":2026,"month":"February","day":29}"#,
            "no such day: 2026-02-29",
        ),
        (
            r#"{"year":2026,"month":"February","day":31}"#,
            "no such day: 2026-02-31",
        ),
        (
            r#"{"year":2026,"month":"April","day":31}"#,
            "no such day: 2026-04-31",
        ),
        (
            r#"{"year":2026,"month":"July","day":0}"#,
            "no such day: 2026-07-00",
        ),
        (
            r#"{"year":2026,"month":"July","day":255}"#,
            "no such day: 2026-07-255",
        ),
        // The sequence form the derive accepts goes through the same check.
        (r#"[2026,"February",31]"#, "no such day: 2026-02-31"),
    ] {
        let message = serde_json::from_str::<Date>(json).unwrap_err().to_string();
        assert!(message.starts_with(error), "{json}: {message}");
    }

    assert_eq!(
        serde_json::from_str::<Date>(r#"{"year":2024,"month":"February","day":29}"#).unwrap(),
        Date::new(2024, 2, 29).unwrap()
    );
    assert_eq!(
        serde_json::from_str::<Date>(r#"[2026,"July",25]"#).unwrap(),
        Date::new(2026, 7, 25).unwrap()
    );
}
