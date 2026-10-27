//! The Turkish lower case and the search fold: fixed vectors, and the
//! crate against the plain model (`model::fold`) on generated input. The
//! search fold is what a full-text index stores, so its output must not
//! change by a byte.
// Test code: the no-panic policy is for the library.
#![allow(
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    clippy::string_slice,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic
)]

mod model;

use std::borrow::Cow;

use abece::{lowercase, lowercase_with_map, nfc, search_fold, search_fold_with_map, uppercase};
use model::fold as plain;
use proptest::prelude::*;

#[test]
fn dotted_and_dotless_i() {
    assert_eq!(lowercase("IŞIK İSTANBUL"), "ışık istanbul");
    assert_eq!(
        lowercase("ASLİYE HUKUK MAHKEMESİ"),
        "asliye hukuk mahkemesi"
    );
    assert_eq!(uppercase("ışık istanbul"), "IŞIK İSTANBUL");
    assert_ne!("İ".to_lowercase(), "i");
    assert_eq!("I".to_lowercase(), "i");
}

#[test]
fn lowercase_composes_first_and_drops_the_dot_after_i() {
    assert_eq!(lowercase("I\u{307}STANBUL"), "istanbul");
    assert_eq!(lowercase("i\u{307}stinaf"), "istinaf");
    assert_eq!(lowercase("İ\u{307}"), "i");
    assert_eq!(lowercase("S\u{327}IK"), "şık");
    assert_eq!(lowercase("ı\u{307}"), "ı\u{307}");
    let normalize_key = |s: &str| {
        lowercase(s)
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    };
    assert_eq!(
        normalize_key("ASLI\u{307}YE  HUKUK"),
        normalize_key("ASLİYE HUKUK")
    );
    let original = nfc("ESAS i\u{307}ki");
    let (lower, map) = lowercase_with_map(&original);
    assert_eq!(lower, lowercase(&original));
    assert_eq!(map.len(), lower.len() + 1);
    let start = lower.find("iki").unwrap();
    assert_eq!(
        map.original_slice(&original, start, lower.len()),
        "i\u{307}ki"
    );
    assert!(matches!(nfc("Iğdır"), Cow::Borrowed(_)));
    assert!(matches!(nfc("I\u{307}"), Cow::Owned(_)));
}

#[test]
fn map_returns_original_spelling() {
    let original = "ESAS NO: 2026/12 İSTANBUL";
    let (lower, map) = lowercase_with_map(original);
    assert_eq!(map.len(), lower.len() + 1);
    let start = lower.find("istanbul").unwrap();
    let end = start + "istanbul".len();
    assert_eq!(map.original_slice(original, start, end), "İSTANBUL");
    let (lower, map) = lowercase_with_map("KARARI");
    assert_eq!(lower, "kararı");
    assert_eq!(map.original_slice("KARARI", 0, lower.len()), "KARARI");
}

#[test]
fn every_spelling_meets_the_same_key() {
    for spelling in ["İstanbul", "ISTANBUL", "istanbul", "İSTANBUL", "ıstanbul"] {
        assert_eq!(search_fold(spelling), "istanbul", "{spelling}");
    }
    assert_eq!(search_fold("IĞDIR"), "iğdir");
    assert_eq!(search_fold("Iğdır"), "iğdir");
    assert_eq!(search_fold("ışık IŞIK"), "işik işik");
    assert_eq!(
        search_fold("ŞAHİN Çağrı Öztürk Ünal"),
        "şahin çağri öztürk ünal"
    );
    assert_eq!(search_fold("I\u{307}stinaf"), "istinaf");
    assert_eq!(search_fold("i\u{307}stinaf"), "istinaf");
    assert_eq!(search_fold("ı\u{307}"), "i\u{307}");
    assert_eq!(search_fold("  "), "  ");
    for (a, b) in [
        ("ISTINAF", "İstinaf"),
        ("IĞDIR", "Iğdır"),
        ("İSTANBUL", "istanbul"),
    ] {
        assert_eq!(search_fold(a), search_fold(b), "{a} {b}");
    }
}

#[test]
fn search_map_returns_original_spelling() {
    let original = nfc("ESAS: 2026/101 IĞDIR Asliye");
    let (folded, map) = search_fold_with_map(&original);
    assert_eq!(folded, search_fold(&original));
    assert_eq!(map.len(), folded.len() + 1);
    let start = folded.find("iğdir").unwrap();
    let end = start + "iğdir".len();
    assert_eq!(map.original_slice(&original, start, end), "IĞDIR");
    let (folded, map) = search_fold_with_map("i\u{307}ki");
    assert_eq!(folded, "iki");
    assert_eq!(
        map.original_slice("i\u{307}ki", 0, folded.len()),
        "i\u{307}ki"
    );
}

/// Turkish and Latin letters in both cases, digits, punctuation, the
/// combining marks a PDF leaves behind, and a few letters of other scripts.
fn text() -> impl Strategy<Value = String> {
    "[a-zA-ZçÇğĞıIiİöÖşŞüÜâÂîÎûÛéÉñßẞﬁ0-9 .,:/'\\-\u{300}\u{302}\u{307}\u{308}\u{327}\u{331}ΩжЖ日]{0,24}"
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(2000))]

    #[test]
    fn lowercase_matches_the_model(s in text()) {
        prop_assert_eq!(lowercase(&s), plain::lowercase(&s));
    }

    #[test]
    fn search_fold_matches_the_model(s in text()) {
        prop_assert_eq!(search_fold(&s), plain::search_fold(&s));
    }

    #[test]
    fn maps_match_the_model(s in text()) {
        let (lower, map) = lowercase_with_map(&s);
        let (expected, expected_map) = plain::lowercase_with_map(&s);
        prop_assert_eq!(&lower, &expected);
        prop_assert_eq!(map.as_slice(), expected_map.as_slice());
        let (folded, map) = search_fold_with_map(&s);
        let (expected, expected_map) = plain::search_fold_with_map(&s);
        prop_assert_eq!(&folded, &expected);
        prop_assert_eq!(map.as_slice(), expected_map.as_slice());
        for start in 0..=folded.len() {
            for end in start..=folded.len() {
                prop_assert_eq!(
                    map.original_slice(&s, start, end),
                    plain::original_slice(&s, &expected_map, start, end)
                );
            }
        }
    }

    #[test]
    fn lowercase_matches_the_model_on_any_string(s in "\\PC{0,16}") {
        // The model lowers character by character too, so the two agree
        // on every input, not just the alphabet above.
        prop_assert_eq!(lowercase(&s), plain::lowercase(&s));
        prop_assert_eq!(search_fold(&s), plain::search_fold(&s));
    }

    #[test]
    fn uppercase_matches_the_model_on_composed_text(s in "[a-zA-ZçÇğĞıIiİöÖşŞüÜ0-9 .ß]{0,24}") {
        // The model does not normalize and does not read i + U+0307 as
        // one İ; on text without that sequence the two agree.
        prop_assert_eq!(uppercase(&s), plain::uppercase(&s));
    }
}
