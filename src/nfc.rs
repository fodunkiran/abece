//! Canonical composition (NFC) with a fast path for text that is composed
//! already.

use alloc::borrow::Cow;
use core::str::Chars;
use unicode_normalization::{IsNormalized, Recompositions, UnicodeNormalization, is_nfc_quick};

/// The first byte of the UTF-8 encoding of U+0300, the first combining
/// mark. Every character below it (ASCII, Latin-1 Supplement, Latin
/// Extended-A and B, which is where every letter of Turkish and of the
/// western European languages lives) is composed and has no combining
/// class, so text whose bytes all stay below this is NFC by inspection;
/// `is_only_below_marks` tests it in one pass over the bytes.
const FIRST_MARK_LEAD_BYTE: u8 = 0xCC;

/// Whether every character of `text` is below U+0300, so that the text is
/// NFC without the quick check's table lookups.
pub(crate) fn is_only_below_marks(text: &str) -> bool {
    text.bytes().all(|byte| byte < FIRST_MARK_LEAD_BYTE)
}

/// The characters of `text` in NFC: the text's own characters when the
/// byte scan proves it composed, the recomposing iterator otherwise.
pub(crate) enum NfcChars<'a> {
    Plain(Chars<'a>),
    Composed(Recompositions<Chars<'a>>),
}

impl<'a> NfcChars<'a> {
    pub(crate) fn of(text: &'a str) -> Self {
        if is_only_below_marks(text) {
            NfcChars::Plain(text.chars())
        } else {
            NfcChars::Composed(text.nfc())
        }
    }
}

impl Clone for NfcChars<'_> {
    fn clone(&self) -> Self {
        match self {
            NfcChars::Plain(chars) => NfcChars::Plain(chars.clone()),
            NfcChars::Composed(chars) => NfcChars::Composed(chars.clone()),
        }
    }
}

impl Iterator for NfcChars<'_> {
    type Item = char;

    fn next(&mut self) -> Option<char> {
        match self {
            NfcChars::Plain(chars) => chars.next(),
            NfcChars::Composed(chars) => chars.next(),
        }
    }
}

/// The NFC (canonically composed) form of `text`.
///
/// Every fold in this crate starts here, so that a letter typed as a base
/// letter and a combining mark (`s` + U+0327) is the same letter as its
/// composed form (`ş`), whichever way a PDF, a keyboard or another program
/// wrote it.
///
/// ASCII text is NFC by definition and is returned without being scanned.
/// Other text goes through the quick check first and is recomposed only when
/// it may not be normalized, so Turkish text, which is almost always NFC
/// already, is borrowed rather than copied.
///
/// ```
/// use std::borrow::Cow;
///
/// assert_eq!(abece::nfc("S\u{327}ahin"), "Şahin");
/// assert!(matches!(abece::nfc("Şahin"), Cow::Borrowed(_)));
/// assert!(matches!(abece::nfc("I\u{307}"), Cow::Owned(_)));
/// ```
#[must_use]
pub fn nfc(text: &str) -> Cow<'_, str> {
    if is_only_below_marks(text) {
        return Cow::Borrowed(text);
    }
    match is_nfc_quick(text.chars()) {
        IsNormalized::Yes => Cow::Borrowed(text),
        IsNormalized::No | IsNormalized::Maybe => Cow::Owned(text.nfc().collect()),
    }
}

/// Whether `text` is in NFC.
///
/// The quick check answers for most text; where it cannot (a "maybe"), the
/// text is recomposed and compared.
///
/// ```
/// assert!(abece::is_nfc("Çağrı"));
/// assert!(!abece::is_nfc("C\u{327}ağrı"));
/// ```
#[must_use]
pub fn is_nfc(text: &str) -> bool {
    if is_only_below_marks(text) {
        return true;
    }
    match is_nfc_quick(text.chars()) {
        IsNormalized::Yes => true,
        IsNormalized::No => false,
        IsNormalized::Maybe => text.nfc().eq(text.chars()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::{String, ToString};

    #[test]
    fn composed_text_is_borrowed_and_decomposed_text_is_composed() {
        assert!(matches!(nfc("Iğdır"), Cow::Borrowed(_)));
        assert!(matches!(nfc("plain ascii"), Cow::Borrowed(_)));
        assert_eq!(nfc("I\u{307}STANBUL"), "İSTANBUL");
        assert_eq!(nfc("S\u{327}IK"), "ŞIK");
        // A mark that composes with nothing stays as it is.
        assert_eq!(nfc("i\u{307}"), "i\u{307}");
        assert!(is_nfc("i\u{307}"));
        assert!(!is_nfc("o\u{308}"));
    }

    #[test]
    fn a_cased_character_is_its_own_case() {
        // What `is_unchanged` and `is_upper_unchanged` rely on.
        for c in char::MIN..=char::MAX {
            if c.is_lowercase() {
                let mut lower = c.to_lowercase();
                assert!(lower.next() == Some(c) && lower.next().is_none(), "{c:?}");
            }
            if c.is_uppercase() {
                let mut upper = c.to_uppercase();
                assert!(upper.next() == Some(c) && upper.next().is_none(), "{c:?}");
            }
        }
    }

    #[test]
    fn every_character_below_the_first_mark_is_composed() {
        // What the byte scan relies on: the quick check says yes and the
        // combining class is zero for every character below U+0300.
        for code in 0..0x300_u32 {
            let c = char::from_u32(code).unwrap();
            assert!(
                matches!(is_nfc_quick(core::iter::once(c)), IsNormalized::Yes),
                "{code:#x}"
            );
            assert_eq!(
                unicode_normalization::char::canonical_combining_class(c),
                0,
                "{code:#x}"
            );
            assert!(is_only_below_marks(&c.to_string()), "{code:#x}");
        }
        assert!(!is_only_below_marks("\u{300}"));
        assert!(!is_only_below_marks("ȧ日"));
        assert!(is_only_below_marks("Çalıkuşu IŞIK İSTANBUL ß æ"));
        assert!(matches!(NfcChars::of("ışık"), NfcChars::Plain(_)));
        assert!(matches!(NfcChars::of("i\u{307}"), NfcChars::Composed(_)));
        assert_eq!(NfcChars::of("S\u{327}IK").collect::<String>(), "ŞIK");
    }
}
