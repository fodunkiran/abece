//! Words for matching names: fixed vectors, and the crate against the plain
//! model of the name fold (`model::names`) on generated input. The model's
//! accent table lists only the Turkish letters and the circumflexed vowels;
//! the crate folds every accent (`é` to `e`, `ß` to `ss`), so the generated
//! input stays within the table.
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

use abece::words;
use model::names as plain;
use proptest::prelude::*;

/// The words joined by one space.
fn fold(text: &str) -> String {
    words(text).join(" ")
}

/// The words sorted and deduplicated: a key that ignores the order of the
/// words, for matching "Yılmaz Ayşe" with "Ayşe Yılmaz".
fn sorted_words(name: &str) -> Vec<String> {
    let mut words = words(name);
    words.sort_unstable();
    words.dedup();
    words
}

#[test]
fn fold_handles_turkish_letters_and_punctuation() {
    assert_eq!(fold("IŞIK İSTANBUL"), "isik istanbul");
    assert_eq!(fold("Çağrı Öztürk-Şahin"), "cagri ozturk sahin");
    assert_eq!(fold("ABC Ltd. Şti."), "abc ltd sti");
    assert_eq!(fold("  "), "");
}

#[test]
fn fold_separates_dotted_words_and_joins_initials() {
    assert_eq!(fold("DENİZ GIDA LTD.ŞTİ."), "deniz gida ltd sti");
    assert_eq!(fold("Deniz A.Ş."), "deniz as");
    assert_eq!(fold("T.C. Ziraat"), "tc ziraat");
    assert_eq!(fold("Ahmet K. Yılmaz"), "ahmet k yilmaz");
    assert_eq!(
        fold("T.C. ANKARA 3. ASLİYE HUKUK MAHKEMESİ"),
        "tc ankara 3 asliye hukuk mahkemesi"
    );
    assert_eq!(fold("Yargıtay 3. H.D."), "yargitay 3 hd");
}

#[test]
fn fold_composes_decomposed_letters() {
    assert_eq!(fold("S\u{327}ahin"), "sahin");
    assert_eq!(fold("S\u{327}AHI\u{307}N"), "sahin");
    assert_eq!(fold("O\u{308}zturk"), "ozturk");
    assert_eq!(fold("Ays\u{331}e"), "ayse");
    assert_eq!(
        fold("ISTANBUL 12. ICRA DAIRESI"),
        fold("İSTANBUL 12. İCRA DAİRESİ")
    );
}

#[test]
fn sorted_words_ignore_order_and_case() {
    assert_eq!(sorted_words("Yılmaz Ayşe"), sorted_words("AYŞE YILMAZ"));
    assert_eq!(
        sorted_words("S\u{327}ahin Kaya"),
        sorted_words("Kaya Şahin")
    );
    assert_ne!(sorted_words("Ayşe Yılmaz"), sorted_words("Ayşe Yıldız"));
    assert_eq!(sorted_words("Ali Ali Veli"), ["ali", "veli"]);
}

/// The letters the model's own table covers, so that the two agree: `é`,
/// `ß`, and a mark that composes into a letter outside the table (`a` +
/// U+0307 is `ȧ`) are the documented difference.
fn name() -> impl Strategy<Value = String> {
    "[a-zA-ZçÇğĞıIiİöÖşŞüÜâÂîÎûÛ0-9 .,'\\-\u{300}\u{302}\u{306}\u{307}\u{308}\u{327}\u{331}]{0,24}"
        .prop_filter("composes into a letter outside the table", |s| {
            abece::nfc(s).chars().all(|c| {
                c.is_ascii()
                    || "çÇğĞıİöÖşŞüÜâÂîÎûÛ".contains(c)
                    || unicode_normalization::char::is_combining_mark(c)
            })
        })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(2000))]

    #[test]
    fn words_match_the_model(s in name()) {
        prop_assert_eq!(words(&s), plain::words(&s));
        prop_assert_eq!(fold(&s), plain::fold(&s));
    }
}
