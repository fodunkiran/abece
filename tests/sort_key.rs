//! The sort key: fixed vectors for every rule of the order, and the crate
//! against the plain model (`model::sort_key`) on generated input. The key
//! is what an indexed column stores, so its output must not change by a
//! byte.
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

use abece::{CaseRule, sort_key, sort_key_in};
use model::sort_key as plain;
use proptest::prelude::*;

/// The Turkish alphabet with q, w and x in their Latin places.
const ALPHABET: [&str; 32] = [
    "a", "b", "c", "ç", "d", "e", "f", "g", "ğ", "h", "ı", "i", "j", "k", "l", "m", "n", "o", "ö",
    "p", "q", "r", "s", "ş", "t", "u", "ü", "v", "w", "x", "y", "z",
];

fn sorted<'a>(titles: &[&'a str]) -> Vec<&'a str> {
    let mut titles = titles.to_vec();
    titles.sort_by_key(|title| sort_key(title));
    titles
}

/// The key of a title in a known language: the case rule follows the
/// language, as a catalogue of mixed languages would have it.
fn key_in_language(title: &str, language: Option<&str>) -> String {
    sort_key_in(title, CaseRule::for_language(language))
}

#[test]
fn the_letters_follow_the_turkish_alphabet_one_by_one() {
    let keys: Vec<String> = ALPHABET.iter().map(|letter| sort_key(letter)).collect();
    assert!(
        keys.is_sorted_by(|before, after| before < after),
        "{keys:?}"
    );
    assert_eq!(
        sorted(&[
            "Zeki",
            "Çalıkuşu",
            "Cevdet",
            "Şule",
            "Sinan",
            "Ömer",
            "Oya",
            "Ümit"
        ]),
        [
            "Cevdet",
            "Çalıkuşu",
            "Oya",
            "Ömer",
            "Sinan",
            "Şule",
            "Ümit",
            "Zeki"
        ]
    );
    assert_eq!(
        sorted(&["gül", "ğ", "gün", "ha"]),
        ["gül", "gün", "ğ", "ha"]
    );
}

#[test]
fn upper_case_letters_sort_where_their_lower_case_ones_do() {
    for (upper, lower) in [
        ("ABC", "abc"),
        ("ÇAĞRI", "çağrı"),
        ("ŞÖLEN", "şölen"),
        ("ÜZÜM", "üzüm"),
        ("ISPARTA", "ısparta"),
        ("İSTANBUL", "istanbul"),
    ] {
        assert_eq!(sort_key(upper), sort_key(lower), "{upper}");
    }
}

#[test]
fn calikusu_sits_among_the_c_words_between_cevdet_and_dede() {
    assert_eq!(
        sorted(&[
            "Dede Korkut",
            "Çalıkuşu",
            "Zeytindağı",
            "Cevdet Bey",
            "Ayışığı"
        ]),
        [
            "Ayışığı",
            "Cevdet Bey",
            "Çalıkuşu",
            "Dede Korkut",
            "Zeytindağı"
        ]
    );
}

#[test]
fn dotless_words_come_before_dotted_ones() {
    assert_eq!(
        sorted(&[
            "İnce Memed",
            "Ihlamur",
            "Işık",
            "İstanbul",
            "Irmak",
            "Hasan",
            "Jale"
        ]),
        [
            "Hasan",
            "Ihlamur",
            "Irmak",
            "Işık",
            "İnce Memed",
            "İstanbul",
            "Jale"
        ]
    );
    assert!(sort_key("ık") < sort_key("ik"));
    assert!(sort_key("Iğdır") < sort_key("İğne"));
    assert_ne!(sort_key("Isı"), sort_key("İsi"));
    assert_eq!(sort_key("Isı"), sort_key("ısı"));
    assert_eq!(sort_key("İsi"), sort_key("isi"));
}

#[test]
fn the_latin_letters_turkish_lacks_keep_their_latin_places() {
    assert_eq!(
        sorted(&[
            "Xavier", "Wilde", "Quixote", "Yaşar", "Vedat", "Pamuk", "Rıfat"
        ]),
        [
            "Pamuk", "Quixote", "Rıfat", "Vedat", "Wilde", "Xavier", "Yaşar"
        ]
    );
}

#[test]
fn accents_fold_to_the_base_letter() {
    for (accented, plain) in [
        ("café", "cafe"),
        ("Kâğıt", "kağıt"),
        ("hâlâ", "hala"),
        ("Dîvân", "divan"),
        ("Mûsiki", "musiki"),
        ("mañana", "manana"),
        ("Straße", "strasse"),
        ("Œuvre", "oeuvre"),
        ("Łódź", "lodz"),
        ("Ødegård", "odegard"),
        ("ﬁnal", "final"),
        ("Ｒoman", "roman"),
    ] {
        assert_eq!(sort_key(accented), sort_key(plain), "{accented}");
    }
    for (own, base) in [("ç", "c"), ("ğ", "g"), ("ö", "o"), ("ş", "s"), ("ü", "u")] {
        assert_ne!(sort_key(own), sort_key(base), "{own}");
    }
    assert_eq!(sort_key("c\u{327}ay"), sort_key("çay"));
    assert_eq!(sort_key("g\u{306}ul"), sort_key("ğul"));
    assert_eq!(sort_key("I\u{307}nce"), sort_key("İnce"));
}

#[test]
fn digits_come_before_letters_and_compare_one_by_one() {
    assert!(sort_key("1984") < sort_key("a"));
    assert!(sort_key("9") < sort_key("a"));
    assert!(sort_key("10") < sort_key("2"));
    assert!(sort_key("2 Kapı") < sort_key("Ali"));
    assert_eq!(
        sorted(&["Zebra", "1984", "Ankara", "22 Haziran", "100 Soruda"]),
        ["100 Soruda", "1984", "22 Haziran", "Ankara", "Zebra"]
    );
}

#[test]
fn a_run_of_whitespace_is_one_separator_that_sorts_before_every_letter() {
    assert_eq!(sort_key("a  b"), sort_key("a b"));
    assert_eq!(sort_key(" \t a\u{a0}\u{3000}b \n"), sort_key("a b"));
    assert_ne!(sort_key("ab"), sort_key("a b"));
    assert!(sort_key("ab c") < sort_key("abc"));
    assert!(sort_key("a b") < sort_key("aa"));
    assert!(sort_key("a 1") < sort_key("a1a"));
    assert_eq!(
        sorted(&["Kar Tanesi", "Karanlık", "Kar", "Karakol"]),
        ["Kar", "Kar Tanesi", "Karakol", "Karanlık"]
    );
}

#[test]
fn punctuation_is_dropped() {
    assert_eq!(sort_key("St. John's"), sort_key("st johns"));
    assert_eq!(sort_key("Bilim-Kurgu"), sort_key("bilimkurgu"));
    assert_eq!(sort_key("«Kar»…"), sort_key("kar"));
    assert_eq!(sort_key("a - b"), sort_key("a b"));
    assert_eq!(sort_key("???"), "");
}

#[test]
fn other_scripts_sort_after_every_latin_letter_in_code_point_order() {
    assert!(sort_key("zzz") < sort_key("Ωmega"));
    assert!(sort_key("Zeki") < sort_key("Кар"));
    assert!(sort_key("Альфа") < sort_key("Бета"));
    assert!(sort_key("Кар") < sort_key("雪"));
    assert_eq!(sort_key("Кар"), sort_key("кар"));
    assert!(sort_key("1 Кар") < sort_key("a Кар"));
    assert!(sort_key("a Кар") < sort_key("aКар"));
}

#[test]
fn nothing_makes_an_empty_key() {
    assert_eq!(sort_key(""), "");
    assert_eq!(sort_key("   "), "");
}

#[test]
fn the_stored_characters_are_one_ascending_run_of_one_byte_each() {
    assert_eq!(abece::ORDER.chars().count(), 1 + 10 + ALPHABET.len());
    let mut previous = None;
    for ranked in abece::ORDER.chars() {
        let stored = sort_key(&ranked.to_string());
        let stored = if ranked == ' ' {
            // The separator alone makes an empty key; its stored form is
            // seen between two letters.
            sort_key("a a").chars().nth(1).unwrap()
        } else {
            stored.chars().next().unwrap()
        };
        assert!(stored.is_ascii(), "{ranked}");
        assert!(previous < Some(stored), "{ranked}");
        previous = Some(stored);
    }
    assert_eq!(sort_key("a a"), ":/:");
    assert_eq!(sort_key("0"), "0");
    assert!(sort_key("çay").as_bytes() > sort_key("cay").as_bytes());
    assert_eq!(sort_key("Çalıkuşu"), "=:HDGSQS");
}

#[test]
fn only_a_language_that_tells_i_from_dotless_i_reads_a_capital_i_as_dotless() {
    for turkish in [
        None,
        Some(""),
        Some("  "),
        Some("tr"),
        Some("TR"),
        Some("tr-TR"),
        Some("tr_TR"),
        Some("az"),
        Some("Az-Latn-AZ"),
    ] {
        assert_eq!(
            CaseRule::for_language(turkish),
            CaseRule::Turkish,
            "{turkish:?}"
        );
    }
    for other in [Some("en"), Some("en-US"), Some("de"), Some("zz")] {
        assert_eq!(
            CaseRule::for_language(other),
            CaseRule::Unicode,
            "{other:?}"
        );
    }
}

#[test]
fn in_another_language_a_capital_i_is_the_capital_of_i() {
    assert_eq!(
        sort_key_in("BILL", CaseRule::Unicode),
        sort_key_in("Bill", CaseRule::Unicode)
    );
    assert_eq!(
        sort_key_in("Ice", CaseRule::Unicode),
        sort_key_in("ice", CaseRule::Unicode)
    );
    assert_eq!(
        sort_key_in("İnce", CaseRule::Unicode),
        sort_key_in("ince", CaseRule::Unicode)
    );
    assert_ne!(
        sort_key_in("BILL", CaseRule::Turkish),
        sort_key_in("Bill", CaseRule::Turkish)
    );
    assert_eq!(sort_key("Işık"), sort_key_in("Işık", CaseRule::Turkish));
}

#[test]
fn a_title_in_english_sorts_among_the_dotted_words_and_a_turkish_one_among_the_dotless() {
    let key = key_in_language;
    assert!(key("ibrik", Some("tr")) < key("It", Some("en")));
    assert!(key("İnce Memed", Some("tr")) < key("It", Some("en")));
    assert!(key("It", Some("en")) < key("Ivanhoe", Some("en")));
    assert!(key("Ivanhoe", Some("en")) < key("Jale", Some("tr")));
    assert!(key("Hasan", Some("tr")) < key("Işık", Some("tr")));
    assert!(key("Işık", Some("tr")) < key("ibrik", Some("tr")));
    assert_eq!(key("Işık", None), key("Işık", Some("tr")));
    assert_ne!(key("Işık", Some("en")), key("Işık", Some("tr")));
    assert_eq!(key("BILL", Some("en")), key("Bill", Some("en")));
}

/// What a catalogue holds: titles and names in the Turkish and Latin
/// alphabets, with accents, ligatures, digits, punctuation, every kind of
/// whitespace, decomposed letters and other scripts.
fn title() -> impl Strategy<Value = String> {
    "[a-zA-ZçÇğĞıIiİöÖşŞüÜâÂîÎûÛéÉñÑßẞæÆœŒøØđĐðÐłŁþÞﬁＲℌ0-9 \t\n\u{a0}\u{3000}.,:;'\"«»…\\-\u{300}\u{306}\u{307}\u{308}\u{327}ΩωжЖ日١]{0,24}"
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(2000))]

    #[test]
    fn sort_key_matches_the_model(s in title()) {
        prop_assert_eq!(sort_key(&s), plain::sort_key(&s));
        prop_assert_eq!(sort_key_in(&s, CaseRule::Unicode), plain::sort_key_in(&s, false));
    }

    #[test]
    fn sort_key_matches_the_model_on_any_string(s in "\\PC{0,16}") {
        prop_assert_eq!(sort_key(&s), plain::sort_key(&s));
        prop_assert_eq!(sort_key_in(&s, CaseRule::Unicode), plain::sort_key_in(&s, false));
    }
}
