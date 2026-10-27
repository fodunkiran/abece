//! Properties every function keeps on arbitrary input: idempotence of the
//! folds, the round trips between the cases, the agreement of the sort key
//! with the comparator, the invariants of the offset map, and the absence
//! of panics everywhere.
// Test code: the no-panic policy is for the library.
#![allow(
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    clippy::string_slice,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic
)]

use std::cmp::Ordering;

use abece::date::{Date, Month, Weekday};
use abece::number::{ordinal, to_words, to_words_signed};
use abece::suffix::{Harmony, Suffix, attach, attach_spoken, suffix_for};
use abece::{
    Accents, CaseRule, FoldOptions, accent_fold, ascii_fold, cmp_ignore_case, compare, compare_in,
    eq_ignore_case, fold_with_map, is_nfc, join_initials, lowercase, lowercase_with_map, nfc,
    search_fold, search_fold_with, search_fold_with_map, sort_key, sort_key_in, titlecase,
    tokenize, tokenize_stems, uppercase, words,
};
use proptest::prelude::*;

/// Turkish and Latin text with the marks and scripts that matter.
fn text() -> impl Strategy<Value = String> {
    "[a-zA-ZçÇğĞıIiİöÖşŞüÜâÂîÎûÛéÉßẞﬁ0-9 \t.,'\\-\u{300}\u{307}\u{308}\u{327}\u{331}Ωж日]{0,24}"
}

fn options() -> impl Strategy<Value = FoldOptions> {
    (any::<bool>(), any::<bool>(), 0..3_u8).prop_map(|(merge, strip, accents)| {
        let accents = match accents {
            0 => Accents::Keep,
            1 => Accents::FoldForeign,
            _ => Accents::FoldAll,
        };
        FoldOptions::SEARCH
            .merge_dotless_i(merge)
            .strip_marks(strip)
            .fold_accents(accents)
    })
}

/// Whether `s` has a combining mark right after a letter whose case
/// mapping changes it (a capital, `ı`, or a letter that expands such as
/// `ß` and `ﬁ`): the one kind of input the folds are not idempotent on,
/// because the mapped letter may compose with the mark where the original
/// did not (`H` + U+0331 lowers to `h` + U+0331, which NFC composes). Kept
/// that way so that stored keys never change (see docs/DESIGN.md).
fn has_a_mark_after_a_case_change(s: &str) -> bool {
    let chars: Vec<char> = s.chars().collect();
    chars.windows(2).any(|pair| {
        let expands = pair[0].to_uppercase().count() > 1 || pair[0].to_lowercase().count() > 1;
        (pair[0].is_uppercase() || pair[0] == 'ı' || expands)
            && unicode_normalization::char::is_combining_mark(pair[1])
    })
}

/// Sakamoto's weekday algorithm, as an independent reference.
fn sakamoto(year: i32, month: u8, day: u8) -> Weekday {
    let offsets = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
    let y = i64::from(year) - i64::from(month < 3);
    let d = (y + y.div_euclid(4) - y.div_euclid(100)
        + y.div_euclid(400)
        + offsets[usize::from(month) - 1]
        + i64::from(day))
    .rem_euclid(7);
    // 0 is Sunday.
    Weekday::from_number(if d == 0 { 7 } else { d as u8 }).unwrap()
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    #[test]
    fn folds_are_idempotent(s in text().prop_filter("a mark after a letter whose case changes", |s| !has_a_mark_after_a_case_change(s)), o in options()) {
        let once = lowercase(&s);
        prop_assert_eq!(lowercase(&once).into_owned(), once.into_owned());
        let once = search_fold(&s);
        prop_assert_eq!(search_fold(&once).into_owned(), once.into_owned());
        let once = search_fold_with(&s, &o);
        prop_assert_eq!(search_fold_with(&once, &o).into_owned(), once.into_owned());
        let once = accent_fold(&s);
        prop_assert_eq!(accent_fold(&once).into_owned(), once.into_owned());
        let once = ascii_fold(&s);
        prop_assert_eq!(ascii_fold(&once).into_owned(), once.into_owned());
        let once = uppercase(&s);
        prop_assert_eq!(uppercase(&once).into_owned(), once.into_owned());
        let once = titlecase(&s);
        prop_assert_eq!(titlecase(&once).into_owned(), once.into_owned());
        let once = nfc(&s);
        prop_assert!(is_nfc(&once));
        prop_assert_eq!(nfc(&once).into_owned(), once.into_owned());
    }

    #[test]
    fn the_cases_round_trip(s in "[a-zA-ZçÇğĞıIiİöÖşŞüÜ0-9 .]{0,24}") {
        // On the Turkish alphabet every letter has one upper and one lower
        // case, so lowering after raising is lowering.
        prop_assert_eq!(lowercase(&uppercase(&s)).into_owned(), lowercase(&s).into_owned());
        prop_assert_eq!(uppercase(&lowercase(&s)).into_owned(), uppercase(&s).into_owned());
        prop_assert_eq!(lowercase(&titlecase(&s)).into_owned(), lowercase(&s).into_owned());
        prop_assert!(eq_ignore_case(&s, &uppercase(&s)));
        prop_assert!(eq_ignore_case(&s, &lowercase(&s)));
        // Neither does case change the sort key (outside ligatures such as
        // ﬁ, whose upper case is a plain FI).
        prop_assert_eq!(sort_key(&uppercase(&s)), sort_key(&s));
    }

    #[test]
    fn the_search_fold_is_the_lower_case_with_the_is_merged(s in text()) {
        prop_assert_eq!(search_fold(&s).into_owned(), lowercase(&s).replace('ı', "i"));
        prop_assert_eq!(search_fold_with(&s, &FoldOptions::SEARCH.merge_dotless_i(false)).into_owned(), lowercase(&s).into_owned());
        // The ASCII fold leaves only ASCII, except letters of other scripts.
        let ascii = search_fold_with(&s, &FoldOptions::ASCII);
        prop_assert!(ascii.chars().all(|c| c.is_ascii() || "ωж日".contains(c)), "{}", ascii);
    }

    #[test]
    fn comparison_agrees_with_lowercase(a in text(), b in text()) {
        prop_assert_eq!(eq_ignore_case(&a, &b), lowercase(&a) == lowercase(&b));
        prop_assert_eq!(cmp_ignore_case(&a, &b), lowercase(&a).cmp(&lowercase(&b)));
        prop_assert_eq!(cmp_ignore_case(&a, &b), cmp_ignore_case(&b, &a).reverse());
    }

    #[test]
    fn the_sort_key_and_the_comparator_agree(a in text(), b in text()) {
        for rule in [CaseRule::Turkish, CaseRule::Unicode] {
            let (ka, kb) = (sort_key_in(&a, rule), sort_key_in(&b, rule));
            prop_assert_eq!(compare_in(&a, &b, rule), ka.cmp(&kb));
            prop_assert_eq!(compare_in(&a, &b, rule), compare_in(&b, &a, rule).reverse());
            // The key of Latin text is ASCII, and its bytes order as its characters do.
            prop_assert_eq!(ka.as_bytes().cmp(kb.as_bytes()), ka.cmp(&kb));
        }
        prop_assert_eq!(compare(&a, &b), sort_key(&a).cmp(&sort_key(&b)));
        // The key is a function of the fold: composition does not change it.
        prop_assert_eq!(sort_key(&nfc(&a)), sort_key(&a));
    }

    #[test]
    fn the_sort_key_is_a_total_order_on_samples(mut items in prop::collection::vec(text(), 0..8)) {
        items.sort_by(|a, b| compare(a, b));
        for pair in items.windows(2) {
            prop_assert_ne!(compare(&pair[0], &pair[1]), Ordering::Greater);
        }
        let keys: Vec<String> = items.iter().map(|s| sort_key(s)).collect();
        prop_assert!(keys.is_sorted());
    }

    #[test]
    fn offset_maps_cover_the_original(s in text(), o in options()) {
        for (folded, map) in [lowercase_with_map(&s), search_fold_with_map(&s), fold_with_map(&s, &o)] {
            prop_assert_eq!(map.len(), folded.len() + 1);
            prop_assert_eq!(map.as_slice().last().copied(), Some(s.len()));
            prop_assert!(map.as_slice().is_sorted());
            // The whole folded text maps back to the original, less any
            // characters dropped before the first one that produced output.
            let whole = map.original_slice(&s, 0, folded.len());
            prop_assert!(s.ends_with(whole), "{:?} is not a suffix of {:?}", whole, s);
            if map.original_start(0) == Some(0) {
                prop_assert_eq!(whole, s.as_str());
            }
            for (i, start) in map.as_slice().iter().enumerate() {
                prop_assert!(s.is_char_boundary(*start), "entry {} is {}", i, start);
            }
        }
        // Without a map, the fold normalizes first; on NFC text the two agree.
        let (folded, _) = fold_with_map(&nfc(&s), &o);
        prop_assert_eq!(folded, search_fold_with(&s, &o).into_owned());
        let (folded, _) = search_fold_with_map(&nfc(&s));
        prop_assert_eq!(folded, search_fold(&s).into_owned());
        let (lower, _) = lowercase_with_map(&nfc(&s));
        prop_assert_eq!(lower, lowercase(&s).into_owned());
    }

    #[test]
    fn words_are_folded_tokens(s in text()) {
        let ws = words(&s);
        for w in &ws {
            prop_assert!(!w.is_empty());
            prop_assert!(w.chars().all(|c| c.is_alphanumeric()), "{:?}", w);
            prop_assert_eq!(search_fold_with(w, &FoldOptions::ASCII).into_owned(), w.as_str());
        }
        prop_assert_eq!(words(&ws.join(" ")), ws);
        let tokens: Vec<_> = tokenize(&s).collect();
        for t in &tokens {
            prop_assert!(!t.text().is_empty());
            prop_assert!(s.contains(t.text()));
        }
        prop_assert!(tokenize_stems(&s).count() <= tokens.len());
        prop_assert!(join_initials(tokens.iter().copied()).len() <= tokens.len());
    }

    #[test]
    fn nothing_panics_on_any_string(s in "\\PC{0,32}", o in options()) {
        let _ = (lowercase(&s), uppercase(&s), titlecase(&s), search_fold(&s), search_fold_with(&s, &o));
        let _ = (accent_fold(&s), ascii_fold(&s), nfc(&s), is_nfc(&s));
        let _ = (lowercase_with_map(&s), search_fold_with_map(&s), fold_with_map(&s, &o));
        let _ = (sort_key(&s), sort_key_in(&s, CaseRule::Unicode), compare(&s, &s), cmp_ignore_case(&s, &s), eq_ignore_case(&s, &s));
        let _ = (words(&s), tokenize(&s).count(), tokenize_stems(&s).count());
        for suffix in [Suffix::Plural, Suffix::Genitive, Suffix::Dative, Suffix::Locative, Suffix::Ablative, Suffix::Accusative, Suffix::Instrumental, Suffix::LocativeRelative] {
            let attached = attach(&s, suffix);
            prop_assert!(attached.starts_with(&s));
            prop_assert!(attached.ends_with(suffix_for(&s, suffix)));
            let _ = attach_spoken(&s, &s, suffix);
        }
        let _ = (Harmony::of(&s), Harmony::of_word(&s));
        prop_assert_eq!(CaseRule::for_language(Some(&s)) == CaseRule::Turkish, s.split(['-', '_']).next().map(str::trim).is_none_or(|p| p.is_empty() || p.eq_ignore_ascii_case("tr") || p.eq_ignore_ascii_case("az")));
    }

    #[test]
    fn suffixes_follow_the_harmony(s in "[a-zA-ZçÇğĞıIiİöÖşŞüÜ]{1,12}") {
        let h = Harmony::of(&s);
        let last_vowel = s.chars().rev().map(abece::letters::lowercase_char).find(|&c| abece::letters::is_vowel(c));
        if let Some(v) = last_vowel {
            prop_assert_eq!(h.is_back(), abece::letters::is_back_vowel(v));
            prop_assert_eq!(h.is_rounded(), abece::letters::is_rounded_vowel(v));
        }
        let last = abece::letters::lowercase_char(s.chars().last().unwrap());
        prop_assert_eq!(h.ends_with_vowel(), abece::letters::is_vowel(last));
        prop_assert_eq!(
            h.ends_voiceless(),
            abece::letters::is_voiceless_consonant(last) || last == 'x'
        );
        // The suffix vowel agrees with the harmony, and the buffer letter with the ending.
        let dative = h.suffix(Suffix::Dative);
        prop_assert_eq!(dative.starts_with('y'), h.ends_with_vowel());
        prop_assert_eq!(dative.ends_with('a'), h.is_back());
        let locative = h.suffix(Suffix::Locative);
        prop_assert_eq!(locative.starts_with('t'), h.ends_voiceless());
        prop_assert_eq!(h.suffix(Suffix::LocativeRelative), format!("{locative}ki"));
        prop_assert_eq!(h.suffix(Suffix::Ablative), format!("{locative}n"));
    }

    #[test]
    fn numbers_in_words_are_made_of_number_words(n in any::<u64>(), i in any::<i64>()) {
        const WORDS: [&str; 27] = ["sıfır", "bir", "iki", "üç", "dört", "beş", "altı", "yedi", "sekiz", "dokuz", "on", "yirmi", "otuz", "kırk", "elli", "altmış", "yetmiş", "seksen", "doksan", "yüz", "bin", "milyon", "milyar", "trilyon", "katrilyon", "kentilyon", "eksi"];
        let words = to_words(n);
        prop_assert!(!words.is_empty());
        prop_assert!(words.split(' ').all(|w| WORDS.contains(&w)), "{}", words);
        prop_assert!(!words.starts_with("bir bin") && !words.contains("bir yüz"));
        let signed = to_words_signed(i);
        prop_assert_eq!(signed.trim_start_matches("eksi "), to_words(i.unsigned_abs()));
        let ord = ordinal(n);
        prop_assert!(ord.starts_with(words.rsplit_once(' ').map_or("", |(head, _)| head)));
        prop_assert!(ord.ends_with("ncı") || ord.ends_with("nci") || ord.ends_with("ncu") || ord.ends_with("ncü"), "{}", ord);
    }

    #[test]
    fn dates_know_their_weekday(year in -4000..8000_i32, month in 1..=12_u8, day in 1..=31_u8) {
        let Some(date) = Date::new(year, month, day) else {
            prop_assert!(day > Month::from_number(month).unwrap().days(year));
            return Ok(());
        };
        prop_assert_eq!(date.weekday(), sakamoto(year, month, day));
        prop_assert_eq!(date.format(abece::date::DateStyle::Long), date.to_string());
        prop_assert_eq!(
            date.format(abece::date::DateStyle::Numeric),
            format!("{day:02}.{month:02}.{year}")
        );
    }
}
