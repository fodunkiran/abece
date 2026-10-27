//! Case mapping with the Turkish rules for the dotted and dotless i.
//!
//! Rust's `str::to_lowercase` is language-blind: it turns `I` into `i` and
//! `İ` into two characters, `i` + U+0307. Turkish has four letters where
//! English has two: `I` is the capital of `ı`, and `İ` of `i`. Every
//! function here maps them that way and leaves the rest of Unicode to the
//! default mapping.

use alloc::borrow::Cow;
use alloc::string::String;
use core::char::ToLowercase;
use core::cmp::Ordering;

use crate::fold::{
    COMBINING_DOT_ABOVE, FoldOptions, State, fold_char, fold_cow, fold_with_map, is_unchanged,
    recomposed,
};
use crate::nfc::{NfcChars, is_only_below_marks, nfc};
use crate::offset_map::OffsetMap;

/// The Turkish lower case: NFC, then `I` to `ı`, `İ` to `i`, every other
/// character by the Unicode default mapping, and a combining dot above
/// (U+0307) dropped after a plain `i`, where a language-blind lower case
/// leaves it.
///
/// ```
/// use abece::lowercase;
///
/// assert_eq!(lowercase("IŞIK İSTANBUL"), "ışık istanbul");
/// assert_eq!(lowercase("ASLİYE HUKUK MAHKEMESİ"), "asliye hukuk mahkemesi");
/// // Decomposed letters compose first: I + U+0307 is İ, s + U+0327 is ş.
/// assert_eq!(lowercase("I\u{307}STANBUL"), "istanbul");
/// assert_eq!(lowercase("S\u{327}IK"), "şık");
/// // The dot a language-blind lower case leaves after i goes; one after ı stays.
/// assert_eq!(lowercase("i\u{307}stinaf"), "istinaf");
/// assert_eq!(lowercase("ı\u{307}"), "ı\u{307}");
/// ```
///
/// Text that is lower case already is borrowed, not copied:
///
/// ```
/// use std::borrow::Cow;
///
/// assert!(matches!(abece::lowercase("ışık"), Cow::Borrowed(_)));
/// assert!(matches!(abece::lowercase("Işık"), Cow::Owned(_)));
/// ```
#[must_use]
pub fn lowercase(text: &str) -> Cow<'_, str> {
    fold_cow(text, &FoldOptions::LOWERCASE)
}

/// [`lowercase`] with an [`OffsetMap`] from the lowered text back to `text`.
///
/// `text` is not normalized: the map points into it as given, so a caller
/// that wants NFC matching normalizes first ([`nfc`]) and keeps the NFC text
/// as the original. For NFC text the lowered result equals [`lowercase`]'s.
///
/// ```
/// use abece::lowercase_with_map;
///
/// let original = "ESAS NO: 2026/12 İSTANBUL";
/// let (lower, map) = lowercase_with_map(original);
/// assert_eq!(lower, "esas no: 2026/12 istanbul");
/// let start = lower.find("istanbul").unwrap();
/// assert_eq!(map.original_slice(original, start, start + "istanbul".len()), "İSTANBUL");
/// ```
#[must_use]
pub fn lowercase_with_map(text: &str) -> (String, OffsetMap) {
    fold_with_map(text, &FoldOptions::LOWERCASE)
}

/// The Turkish upper case: NFC, then `i` to `İ`, `ı` to `I`, every other
/// character by the Unicode default mapping. An `i` followed by a combining
/// dot above (the language-blind lower case of `İ`) is one `İ`.
///
/// ```
/// use abece::uppercase;
///
/// assert_eq!(uppercase("ışık istanbul"), "IŞIK İSTANBUL");
/// assert_eq!(uppercase("i\u{307}stinaf"), "İSTİNAF");
/// assert_eq!(uppercase("straße"), "STRASSE");
/// ```
///
/// Text that is upper case already is borrowed, not copied:
///
/// ```
/// use std::borrow::Cow;
///
/// assert!(matches!(abece::uppercase("IŞIK"), Cow::Borrowed(_)));
/// ```
#[must_use]
pub fn uppercase(text: &str) -> Cow<'_, str> {
    if text.is_ascii() {
        return ascii_uppercase(text);
    }
    let text = nfc(text);
    match uppercase_if_changed(&text) {
        Some(upper) => Cow::Owned(upper),
        None => text,
    }
}

/// The upper case of NFC `text`, or `None` when nothing would change.
fn uppercase_if_changed(text: &str) -> Option<String> {
    let mut out: Option<String> = None;
    let mut expanded = false;
    let mut chars = text.char_indices().peekable();
    while let Some((byte, ch)) = chars.next() {
        let upper = match ch {
            'i' => {
                // `i` + U+0307 is a decomposed `İ`; one capital takes both.
                chars.next_if(|&(_, next)| next == COMBINING_DOT_ABOVE);
                Some('İ')
            }
            'ı' => Some('I'),
            _ => None,
        };
        if let Some(out) = out.as_mut() {
            expanded |= push_uppercase(ch, upper, out);
            continue;
        }
        if upper.is_none() && is_upper_unchanged(ch) {
            continue;
        }
        let mut started = String::with_capacity(text.len());
        started.push_str(text.get(..byte).unwrap_or(""));
        expanded |= push_uppercase(ch, upper, &mut started);
        out = Some(started);
    }
    out.map(|out| recomposed(out, expanded))
}

/// Appends `upper`, or the Unicode upper case of `ch`; `true` when that is
/// more than one character.
fn push_uppercase(ch: char, upper: Option<char>, out: &mut String) -> bool {
    if let Some(upper) = upper {
        out.push(upper);
        return false;
    }
    let mut count = 0_u8;
    for upper in ch.to_uppercase() {
        count = count.saturating_add(1);
        out.push(upper);
    }
    count > 1
}

/// Whether the Unicode upper case of `ch` is `ch`.
fn is_upper_unchanged(ch: char) -> bool {
    if ch.is_ascii() {
        return !ch.is_ascii_lowercase();
    }
    // Every character with the Uppercase property is its own upper case
    // (`nfc::tests` checks all of them).
    if ch.is_uppercase() {
        return true;
    }
    let mut upper = ch.to_uppercase();
    upper.next() == Some(ch) && upper.next().is_none()
}

/// The upper case of ASCII `text`: borrowed when it has no lower-case
/// letter, and otherwise with `i` raised to the two-byte `İ`.
fn ascii_uppercase(text: &str) -> Cow<'_, str> {
    let bytes = text.as_bytes();
    let Some(first_lower) = bytes.iter().position(u8::is_ascii_lowercase) else {
        return Cow::Borrowed(text);
    };
    let rest = bytes.get(first_lower..).unwrap_or(&[]);
    if !rest.contains(&b'i') {
        return Cow::Owned(text.to_ascii_uppercase());
    }
    let small_is = rest.iter().filter(|&&b| b == b'i').count();
    let mut out = alloc::vec::Vec::with_capacity(text.len().saturating_add(small_is));
    for &byte in bytes {
        if byte == b'i' {
            out.extend_from_slice("İ".as_bytes());
        } else {
            out.push(byte.to_ascii_uppercase());
        }
    }
    // ASCII bytes and the bytes of `İ` are valid UTF-8; the fallback is unreachable.
    Cow::Owned(String::from_utf8(out).unwrap_or_default())
}

/// The Turkish title case: the first letter of every word in upper case,
/// the rest in lower case. A word is a run of non-whitespace; its first
/// letter is its first alphabetic character, so punctuation before it
/// ("(istanbul)") and digits ("3b") are skipped over.
///
/// ```
/// use abece::titlecase;
///
/// assert_eq!(titlecase("ışık istanbul"), "Işık İstanbul");
/// assert_eq!(titlecase("ASLİYE HUKUK MAHKEMESİ"), "Asliye Hukuk Mahkemesi");
/// assert_eq!(titlecase("istanbul'da (ılık) bir gün"), "İstanbul'da (Ilık) Bir Gün");
/// ```
#[must_use]
pub fn titlecase(text: &str) -> Cow<'_, str> {
    let text = nfc(text);
    let options = &FoldOptions::LOWERCASE;
    let mut out: Option<String> = None;
    let mut state = State::default();
    let mut word_start = true;
    let mut expanded = false;
    for (byte, ch) in text.char_indices() {
        let kept = state.admit(ch, options);
        let raise = kept && word_start && ch.is_alphabetic();
        if ch.is_whitespace() {
            word_start = true;
        } else if raise {
            word_start = false;
        }
        if let Some(out) = out.as_mut() {
            if raise {
                expanded |= push_upper(ch, out);
            } else if kept {
                fold_char(ch, options, &mut |lower| out.push(lower));
            }
            continue;
        }
        let unchanged = if raise {
            is_title_unchanged(ch)
        } else {
            kept && is_unchanged(ch, options)
        };
        if unchanged {
            continue;
        }
        let mut started = String::with_capacity(text.len());
        started.push_str(text.get(..byte).unwrap_or(""));
        if raise {
            expanded |= push_upper(ch, &mut started);
        } else if kept {
            fold_char(ch, options, &mut |lower| started.push(lower));
        }
        out = Some(started);
    }
    match out {
        Some(title) => Cow::Owned(recomposed(title, expanded)),
        None => text,
    }
}

/// Appends the Turkish title case of one character: its upper case, and
/// where that is more than one character (`ß` to `SS`), only the first
/// in upper case (`Ss`). `true` when more than one character was appended.
fn push_upper(ch: char, out: &mut String) -> bool {
    match ch {
        'i' => out.push('İ'),
        'ı' => out.push('I'),
        _ => {
            let mut upper = ch.to_uppercase();
            out.extend(upper.next());
            let mut expanded = false;
            for rest in upper {
                out.extend(rest.to_lowercase());
                expanded = true;
            }
            return expanded;
        }
    }
    false
}

/// Whether [`push_upper`] would append exactly `ch`.
fn is_title_unchanged(ch: char) -> bool {
    match ch {
        'i' | 'ı' => false,
        'I' | 'İ' => true,
        _ => is_upper_unchanged(ch),
    }
}

/// Whether `a` and `b` are the same text under the Turkish lower case
/// ([`lowercase`]), without building either. `I` equals `ı` and `İ` equals
/// `i`; `I` and `i` differ.
///
/// ```
/// use abece::eq_ignore_case;
///
/// assert!(eq_ignore_case("İSTANBUL", "istanbul"));
/// assert!(eq_ignore_case("IŞIK", "ışık"));
/// assert!(!eq_ignore_case("ISIK", "isik"));
/// assert!(eq_ignore_case("S\u{327}ahin", "ŞAHİN"));
/// ```
#[must_use]
pub fn eq_ignore_case(a: &str, b: &str) -> bool {
    if a.is_ascii() && b.is_ascii() {
        return a.len() == b.len()
            && a.bytes()
                .zip(b.bytes())
                .all(|(x, y)| ascii_lower_turkish(x) == ascii_lower_turkish(y));
    }
    let (a, b) = after_common_prefix(a, b);
    lowercase_chars(a).eq(lowercase_chars(b))
}

/// `a` and `b` without the prefix they share, when the lower case of each
/// character does not depend on its neighbours: text below the first
/// combining mark has nothing to compose and no dot to drop, so the lower
/// cases of the two texts agree exactly where the texts do. Two texts that
/// differ late are then compared from where they differ.
fn after_common_prefix<'a>(a: &'a str, b: &'a str) -> (&'a str, &'a str) {
    if !is_only_below_marks(a) || !is_only_below_marks(b) {
        return (a, b);
    }
    let common = a
        .as_bytes()
        .iter()
        .zip(b.as_bytes())
        .take_while(|(x, y)| x == y)
        .count();
    let mut cut = common;
    while !a.is_char_boundary(cut) {
        cut = cut.saturating_sub(1);
    }
    (a.get(cut..).unwrap_or(""), b.get(cut..).unwrap_or(""))
}

/// The order of `a` and `b` under the Turkish lower case ([`lowercase`]),
/// by code point, without building either. For the order of the Turkish
/// alphabet see [`compare`](crate::compare).
///
/// ```
/// use std::cmp::Ordering;
/// use abece::cmp_ignore_case;
///
/// assert_eq!(cmp_ignore_case("İSTANBUL", "istanbul"), Ordering::Equal);
/// assert_eq!(cmp_ignore_case("ankara", "BURSA"), Ordering::Less);
/// // By code point, as `str`'s own order: `ı` (U+0131) comes after `z`.
/// assert_eq!(cmp_ignore_case("Işık", "zeki"), Ordering::Greater);
/// ```
#[must_use]
pub fn cmp_ignore_case(a: &str, b: &str) -> Ordering {
    if a.is_ascii() && b.is_ascii() {
        return a
            .bytes()
            .map(ascii_lower_turkish_point)
            .cmp(b.bytes().map(ascii_lower_turkish_point));
    }
    let (a, b) = after_common_prefix(a, b);
    lowercase_chars(a).cmp(lowercase_chars(b))
}

/// The Turkish lower case of an ASCII byte for equality: `I` stays apart
/// from `i`, as `ı` is.
const fn ascii_lower_turkish(byte: u8) -> u8 {
    if byte == b'I' {
        b'I'
    } else {
        byte.to_ascii_lowercase()
    }
}

/// The code point of the Turkish lower case of an ASCII byte, for ordering:
/// `I` is `ı`, U+0131, which sorts after every ASCII character.
const fn ascii_lower_turkish_point(byte: u8) -> u32 {
    if byte == b'I' {
        'ı' as u32
    } else {
        byte.to_ascii_lowercase() as u32
    }
}

/// The characters of [`lowercase`]`(text)`, without building it.
fn lowercase_chars(text: &str) -> LowercaseChars<NfcChars<'_>> {
    LowercaseChars {
        inner: NfcChars::of(text),
        pending: None,
        after_plain_i: false,
    }
}

/// The Turkish lower case of a character stream, with the combining dot
/// above dropped after a plain `i`.
struct LowercaseChars<I: Iterator<Item = char>> {
    inner: I,
    pending: Option<ToLowercase>,
    after_plain_i: bool,
}

impl<I: Iterator<Item = char>> Iterator for LowercaseChars<I> {
    type Item = char;

    fn next(&mut self) -> Option<char> {
        loop {
            if let Some(pending) = self.pending.as_mut() {
                if let Some(lower) = pending.next() {
                    return Some(lower);
                }
                self.pending = None;
            }
            let ch = self.inner.next()?;
            if ch == COMBINING_DOT_ABOVE && self.after_plain_i {
                self.after_plain_i = false;
                continue;
            }
            self.after_plain_i = matches!(ch, 'i' | 'İ');
            match ch {
                'I' => return Some('ı'),
                'İ' => return Some('i'),
                _ if ch.is_ascii() => return Some(ch.to_ascii_lowercase()),
                _ => self.pending = Some(ch.to_lowercase()),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::ToString;

    #[test]
    fn the_four_is() {
        assert_eq!(lowercase("I İ ı i"), "ı i ı i");
        assert_eq!(uppercase("I İ ı i"), "I İ I İ");
        // The language-blind mapping gets both wrong.
        assert_ne!("İ".to_lowercase(), "i");
        assert_eq!("I".to_lowercase(), "i");
    }

    #[test]
    fn uppercase_fast_path_agrees_with_the_general_path() {
        for text in ["isparta ılık", "NO SMALL", "iii", "i", "", "aibic"] {
            let general: String = text
                .chars()
                .map(|c| match c {
                    'i' => "İ".to_string(),
                    'ı' => "I".to_string(),
                    _ => c.to_uppercase().collect(),
                })
                .collect();
            assert_eq!(uppercase(text), general, "{text}");
        }
        assert!(matches!(uppercase("ISPARTA 2026"), Cow::Borrowed(_)));
        assert!(matches!(uppercase("IŞIK"), Cow::Borrowed(_)));
        assert!(matches!(uppercase("Işık"), Cow::Owned(_)));
        assert_eq!(uppercase("i\u{307}"), "İ");
        assert_eq!(uppercase("ı\u{307}"), "I\u{307}");
        // An expansion before a mark composes: SS + U+0327 is SŞ.
        assert_eq!(uppercase("ß\u{327}"), "SŞ");
        assert_eq!(uppercase("SŞ"), "SŞ");
        assert_eq!(titlecase("ß\u{327}"), "Sş");
    }

    #[test]
    fn titlecase_raises_the_first_letter_of_each_word() {
        assert_eq!(titlecase("ılık"), "Ilık");
        assert_eq!(titlecase("  iki  üç "), "  İki  Üç ");
        assert_eq!(titlecase("3b 'ali' kara-kuş"), "3B 'Ali' Kara-kuş");
        assert_eq!(titlecase("ßen ǆem"), "Ssen Ǆem");
        assert_eq!(titlecase("Ssen"), "Ssen");
        assert_eq!(titlecase(""), "");
        assert!(matches!(titlecase("Ali Veli"), Cow::Borrowed(_)));
        assert!(matches!(titlecase("ali"), Cow::Owned(_)));
    }

    #[test]
    fn comparison_agrees_with_lowercase() {
        let pairs = [
            ("Çalıkuşu Reşat", "ÇALIKUŞU REŞAT"),
            ("Çalıkuşu Reşat", "Çalıkuşu Reşa"),
            ("Çalıkuşu Reşat", "Çalıkuşu Reşaz"),
            ("Çalıkuşu I", "Çalıkuşu ı"),
            ("Çalıkuşu I", "Çalıkuşu i"),
            ("Çalıkuşu İ", "Çalıkuşu i"),
            ("Çalıkuşu ab", "Çalıkuşu aB"),
            ("ÇÇ", "Çç"),
            ("Çi\u{307}", "Çi"),
            ("İSTANBUL", "istanbul"),
            ("ISIK", "isik"),
            ("IŞIK", "ışık"),
            ("abc", "ABD"),
            ("Iz", "ız"),
            ("I", "i"),
            ("i\u{307}", "i"),
            ("ı\u{307}", "ı"),
            ("S\u{327}", "ş"),
            ("", "a"),
            ("a", ""),
            ("Ij", "ij"),
            ("Ij", "ıj"),
        ];
        for (a, b) in pairs {
            let (la, lb) = (lowercase(a), lowercase(b));
            assert_eq!(eq_ignore_case(a, b), la == lb, "{a:?} {b:?}");
            assert_eq!(cmp_ignore_case(a, b), la.cmp(&lb), "{a:?} {b:?}");
            assert_eq!(cmp_ignore_case(b, a), lb.cmp(&la), "{b:?} {a:?}");
        }
    }
}
