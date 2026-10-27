//! The order of the Turkish alphabet, as a key and as a comparator. The
//! encoding is described on [`ORDER`], the order itself on [`sort_key_in`].

use crate::fold::fold_foreign_lower;
use crate::nfc::{NfcChars, is_only_below_marks};
use alloc::string::String;
use alloc::vec::Vec;
use core::cmp::Ordering;

/// Every character that has a place in the order, in that order: the word
/// separator, the ten digits, then the Turkish alphabet with the Latin
/// letters it lacks (q, w, x) where Latin puts them. A character's rank is
/// its position here.
///
/// A [`sort_key`] stores rank `r` as the character `r` places above `/`
/// (U+002F), so the stored characters run in one contiguous ascending
/// range (`/`, `0` to `9`, then `:` to `Y`) and a byte comparison of two
/// keys is a comparison of their ranks. The digits keep their own
/// characters, which makes a stored key readable where it matters.
///
/// ```
/// use abece::{ORDER, sort_key};
///
/// assert_eq!(ORDER.chars().count(), 43);
/// // "a" has rank 11; 11 places above `/` is `:`.
/// assert_eq!(ORDER.find('a'), Some(11));
/// assert_eq!(sort_key("a"), ":");
/// ```
pub const ORDER: &str = " 0123456789abcçdefgğhıijklmnoöpqrsştuüvwxyz";

/// The code point rank 0 is stored as, `/`; rank `r` is stored `r` places
/// above it.
const FIRST_KEY_CHAR: u32 = 0x2F;

/// The languages that tell `I` from `ı`, by their primary subtag: Turkish
/// and Azerbaijani. In every other language a capital `I` is the capital of
/// `i`.
const DOTLESS_I_LANGUAGES: [&str; 2] = ["tr", "az"];

/// Which letter a capital `I` is read as.
///
/// Only one thing about the order depends on the text's language. Under the
/// Turkish rules an English "Ice" is a dotless word, sorted among the `ı`
/// words before every `i` word; a text known to be in another language is
/// read with [`CaseRule::Unicode`] instead, so that "BILL" and "Bill" have
/// one key.
///
/// ```
/// use abece::{CaseRule, sort_key_in};
///
/// assert_ne!(sort_key_in("BILL", CaseRule::Turkish), sort_key_in("Bill", CaseRule::Turkish));
/// assert_eq!(sort_key_in("BILL", CaseRule::Unicode), sort_key_in("Bill", CaseRule::Unicode));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum CaseRule {
    /// `I` is the capital of the dotless `ı`, and `İ` of `i`. The default.
    #[default]
    Turkish,
    /// `I` is the capital of `i`, as is `İ`.
    Unicode,
}

impl CaseRule {
    /// The rule a text in `language` is read by: [`Turkish`](Self::Turkish)
    /// where the language tells `I` from `ı` (Turkish and Azerbaijani,
    /// whatever the region subtag and case: "tr", "tr-TR", "AZ"), and also
    /// where no language is known, because the key is made for a mostly
    /// Turkish collection. Any other language reads `I` as the capital of
    /// `i`.
    ///
    /// ```
    /// use abece::CaseRule;
    ///
    /// assert_eq!(CaseRule::for_language(Some("tr-TR")), CaseRule::Turkish);
    /// assert_eq!(CaseRule::for_language(Some("Az-Latn-AZ")), CaseRule::Turkish);
    /// assert_eq!(CaseRule::for_language(None), CaseRule::Turkish);
    /// assert_eq!(CaseRule::for_language(Some("en-US")), CaseRule::Unicode);
    /// ```
    #[must_use]
    pub fn for_language(language: Option<&str>) -> Self {
        let primary = language
            .and_then(|code| code.split(['-', '_']).next())
            .map(str::trim)
            .unwrap_or_default();
        if primary.is_empty()
            || DOTLESS_I_LANGUAGES
                .iter()
                .any(|dotless| primary.eq_ignore_ascii_case(dotless))
        {
            Self::Turkish
        } else {
            Self::Unicode
        }
    }
}

/// The key `text` sorts by under the Turkish case rules; [`sort_key_in`]
/// with [`CaseRule::Turkish`].
///
/// ```
/// use abece::sort_key;
///
/// let mut titles = ["Zeki", "Çalıkuşu", "Cevdet", "Şule", "Sinan", "Ömer", "Oya", "Ümit"];
/// titles.sort_by_key(|title| sort_key(title));
/// assert_eq!(titles, ["Cevdet", "Çalıkuşu", "Oya", "Ömer", "Sinan", "Şule", "Ümit", "Zeki"]);
///
/// // The key of Latin and Turkish text is ASCII, one byte per letter.
/// assert_eq!(sort_key("Çalıkuşu"), "=:HDGSQS");
/// ```
#[must_use]
pub fn sort_key(text: &str) -> String {
    sort_key_in(text, CaseRule::Turkish)
}

/// The key `text` sorts by, reading a capital `I` as `rule` says.
///
/// A database compares text by bytes, which puts "Çalıkuşu" after "Zeki"
/// and "ığdır" after "iğne". Teaching it a collation ties the database
/// file to one program: an index on a collation only that program
/// registers makes every write from another connection fail. So the order
/// is computed here, once per write, kept in a column and indexed, and the
/// database only ever compares bytes.
///
/// Keys are compared as plain strings (a database's `BINARY` collation,
/// which is also `String`'s `Ord`) and come out in this order:
///
/// - Whitespace sorts before everything else, digits before letters, and
///   the letters follow the Turkish alphabet: a b c ç d e f g ğ h ı i j k l
///   m n o ö p q r s ş t u ü v w x y z. The Latin letters Turkish lacks take
///   their Latin places.
/// - Case does not count. With [`CaseRule::Turkish`] `I` is `ı` and `İ` is
///   `i`, so "Işık" sorts among the dotless words and "İnce" among the
///   dotted ones.
/// - A letter with accents sorts as its base letter ("é" as e, "â" as a,
///   "ñ" as n, "ß" as ss), and compatibility forms as what they stand for
///   ("ﬁ" as fi). Only the six letters Turkish has of its own stay apart.
/// - Digits are compared one by one, as the characters they are: "10"
///   comes before "2". Reading numbers as numbers is not attempted.
/// - Any run of whitespace is one separator ("a  b" and " a b " are
///   "a b"), so "ab c" sorts before "abc". Other punctuation and every mark
///   is dropped: "St. John's" is "st johns".
/// - A letter or digit of any other script keeps its own code point, which
///   is above every character the rules above give, so such text sorts
///   after the Latin text, in code point order, deterministically.
///
/// The key is meant to be stored, so its output is stable within a major
/// version of this crate.
///
/// ```
/// use abece::{CaseRule, sort_key_in};
///
/// assert_eq!(sort_key_in("Ice", CaseRule::Unicode), sort_key_in("ice", CaseRule::Unicode));
/// assert!(sort_key_in("Işık", CaseRule::Turkish) < sort_key_in("ibrik", CaseRule::Turkish));
/// assert_eq!(sort_key_in("St. John's", CaseRule::Turkish), sort_key_in("st johns", CaseRule::Turkish));
/// ```
#[must_use]
pub fn sort_key_in(text: &str, rule: CaseRule) -> String {
    if text.is_ascii() {
        return ascii_sort_key(text, rule);
    }
    let mut key = String::with_capacity(text.len());
    key.extend(sort_key_chars(text, rule));
    key
}

/// The order of `a` and `b` in the Turkish alphabet, under the Turkish case
/// rules: the order of their [`sort_key`]s, without building them.
///
/// ```
/// use abece::compare;
///
/// let mut names = ["Şule", "Sinan", "Çetin", "Cem", "ışık", "İnci", "Ömer", "Oya"];
/// names.sort_by(|a, b| compare(a, b));
/// assert_eq!(names, ["Cem", "Çetin", "ışık", "İnci", "Oya", "Ömer", "Sinan", "Şule"]);
/// ```
#[must_use]
pub fn compare(a: &str, b: &str) -> Ordering {
    compare_in(a, b, CaseRule::Turkish)
}

/// The order of `a` and `b` in the Turkish alphabet, reading a capital `I`
/// as `rule` says: the order of their [`sort_key_in`] keys, without
/// building them.
///
/// ```
/// use std::cmp::Ordering;
/// use abece::{CaseRule, compare_in};
///
/// assert_eq!(compare_in("BILL", "Bill", CaseRule::Unicode), Ordering::Equal);
/// assert_eq!(compare_in("BILL", "Bill", CaseRule::Turkish), Ordering::Less);
/// assert_eq!(compare_in("Çalıkuşu", "Dede Korkut", CaseRule::Turkish), Ordering::Less);
/// ```
#[must_use]
pub fn compare_in(a: &str, b: &str, rule: CaseRule) -> Ordering {
    if is_only_below_marks(a) && is_only_below_marks(b) {
        // Nothing composes across characters, so the two keys agree exactly
        // where the texts do: skip the common prefix and resume both
        // iterators in the state the prefix leaves behind.
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
        let state = prefix_state(a.get(..cut).unwrap_or(""));
        return resume(a, cut, rule, state).cmp(resume(b, cut, rule, state));
    }
    sort_key_chars(a, rule).cmp(sort_key_chars(b, rule))
}

/// The key characters of `text` from byte `cut` on, in the state
/// (`started`, `pending_separator`) the text before `cut` leaves behind.
fn resume(text: &str, cut: usize, rule: CaseRule, state: (bool, bool)) -> SortKeyChars<'_> {
    let mut chars = sort_key_chars(text.get(cut..).unwrap_or(""), rule);
    (chars.started, chars.pending_separator) = state;
    chars
}

/// The state [`SortKeyChars`] is in after `prefix`, which has no combining
/// marks: whether a character was stored (every alphanumeric character
/// stores at least one, every other character nothing), and whether
/// whitespace came after the last one.
fn prefix_state(prefix: &str) -> (bool, bool) {
    let mut pending_separator = None;
    for c in prefix.chars().rev() {
        if c.is_alphanumeric() {
            return (true, pending_separator.unwrap_or(false));
        }
        if c.is_whitespace() && pending_separator.is_none() {
            pending_separator = Some(true);
        }
    }
    (false, false)
}

/// The characters of [`sort_key_in`]`(text, rule)`, one at a time, without
/// building the key: what [`compare_in`] walks and [`sort_key_in`] collects.
pub(crate) fn sort_key_chars(text: &str, rule: CaseRule) -> SortKeyChars<'_> {
    let source = if text.is_ascii() {
        Source::Ascii(text.as_bytes().iter())
    } else {
        Source::Unicode(NfcChars::of(text))
    };
    SortKeyChars {
        source,
        rule,
        pending: Pending::default(),
        pending_separator: false,
        started: false,
    }
}

/// The iterator [`sort_key_chars`] returns.
#[derive(Clone)]
pub(crate) struct SortKeyChars<'a> {
    source: Source<'a>,
    rule: CaseRule,
    pending: Pending,
    /// Whitespace was seen since the last stored character.
    pending_separator: bool,
    /// At least one character was stored, so a separator may follow it.
    started: bool,
}

/// Where the characters come from: ASCII text needs no normalization.
#[derive(Clone)]
enum Source<'a> {
    Ascii(core::slice::Iter<'a, u8>),
    Unicode(NfcChars<'a>),
}

impl Iterator for Source<'_> {
    type Item = char;

    fn next(&mut self) -> Option<char> {
        match self {
            Source::Ascii(bytes) => bytes.next().map(|&byte| char::from(byte)),
            Source::Unicode(chars) => chars.next(),
        }
    }
}

/// Key characters produced for one input character and not yet yielded: a
/// small inline queue, since a letter yields one or two, with an allocated
/// spill for the rare long decomposition.
#[derive(Debug, Clone, Default)]
struct Pending {
    inline: [char; 8],
    len: u8,
    pos: u8,
    spill: Vec<char>,
    spill_pos: usize,
}

impl Pending {
    fn push(&mut self, ch: char) {
        if self.spill_pos < self.spill.len() {
            // The spill is in use; keep the order by appending to it.
            self.spill.push(ch);
            return;
        }
        self.spill.clear();
        self.spill_pos = 0;
        match self.inline.get_mut(usize::from(self.len)) {
            Some(slot) => {
                *slot = ch;
                self.len = self.len.saturating_add(1);
            }
            None => self.spill.push(ch),
        }
    }

    fn pop(&mut self) -> Option<char> {
        if self.pos < self.len {
            let ch = self.inline.get(usize::from(self.pos)).copied();
            self.pos = self.pos.saturating_add(1);
            if self.pos == self.len {
                self.pos = 0;
                self.len = 0;
            }
            return ch;
        }
        let ch = self.spill.get(self.spill_pos).copied();
        if ch.is_some() {
            self.spill_pos = self.spill_pos.saturating_add(1);
        }
        ch
    }
}

impl Iterator for SortKeyChars<'_> {
    type Item = char;

    fn next(&mut self) -> Option<char> {
        loop {
            if let Some(ch) = self.pending.pop() {
                return Some(ch);
            }
            let ch = self.source.next()?;
            // The two letters Rust's own lower case gets wrong for Turkish
            // come first: it turns every `I` into an `i` and an `İ` into an
            // `i` with a mark, losing the difference this order is about.
            match ch {
                'I' if self.rule == CaseRule::Turkish => self.place('ı'),
                'İ' => self.place('i'),
                _ if ch.is_ascii() => self.place(ch.to_ascii_lowercase()),
                _ => {
                    for lower in ch.to_lowercase() {
                        fold_foreign_lower(lower, &mut |letter| self.place(letter));
                    }
                }
            }
        }
    }
}

impl SortKeyChars<'_> {
    /// Queues what one folded letter contributes to the key: nothing yet for
    /// whitespace, which becomes one separator before the next stored
    /// character; nothing for what the key drops; the stored character
    /// otherwise, after a pending separator. A separator before the first
    /// character, or after the last, would only tell apart text that
    /// differs by a space.
    fn place(&mut self, letter: char) {
        if letter.is_whitespace() {
            self.pending_separator = true;
            return;
        }
        let Some(stored) = stored_char(letter) else {
            return;
        };
        if self.pending_separator && self.started {
            self.pending.push(SEPARATOR_KEY_CHAR);
        }
        self.pending_separator = false;
        self.started = true;
        self.pending.push(stored);
    }
}

/// The stored character of the word separator, rank 0.
const SEPARATOR_KEY_CHAR: char = '/';

/// The character `letter` is stored as: its rank in [`ORDER`] placed in the
/// key range, or the letter itself when it belongs to another script.
/// `None` for what the key drops (punctuation, symbols, marks).
fn stored_char(letter: char) -> Option<char> {
    if let Some(rank) = rank(letter) {
        return char::from_u32(FIRST_KEY_CHAR.saturating_add(u32::from(rank)));
    }
    letter.is_alphanumeric().then_some(letter)
}

/// The position of a lower-case letter, digit or space in [`ORDER`].
const fn rank(letter: char) -> Option<u8> {
    Some(match letter {
        ' ' => 0,
        '0'..='9' => (letter as u8).wrapping_sub(b'0').wrapping_add(1),
        'a' => 11,
        'b' => 12,
        'c' => 13,
        'ç' => 14,
        'd' => 15,
        'e' => 16,
        'f' => 17,
        'g' => 18,
        'ğ' => 19,
        'h' => 20,
        'ı' => 21,
        'i' => 22,
        'j' => 23,
        'k' => 24,
        'l' => 25,
        'm' => 26,
        'n' => 27,
        'o' => 28,
        'ö' => 29,
        'p' => 30,
        'q' => 31,
        'r' => 32,
        's' => 33,
        'ş' => 34,
        't' => 35,
        'u' => 36,
        'ü' => 37,
        'v' => 38,
        'w' => 39,
        'x' => 40,
        'y' => 41,
        'z' => 42,
        _ => return None,
    })
}

/// The key of ASCII `text`: a byte loop, since every ASCII character is a
/// digit, a letter, whitespace or dropped, and only `I` depends on the rule.
fn ascii_sort_key(text: &str, rule: CaseRule) -> String {
    let mut key = String::with_capacity(text.len());
    let mut pending_separator = false;
    for byte in text.bytes() {
        let stored = match byte {
            b'\t'..=b'\r' | b' ' => {
                pending_separator = true;
                continue;
            }
            b'0'..=b'9' => char::from(byte),
            b'I' if rule == CaseRule::Turkish => 'D',
            b'A'..=b'Z' | b'a'..=b'z' => {
                let Some(stored) = stored_char(char::from(byte.to_ascii_lowercase())) else {
                    continue;
                };
                stored
            }
            _ => continue,
        };
        if pending_separator && !key.is_empty() {
            key.push(SEPARATOR_KEY_CHAR);
        }
        pending_separator = false;
        key.push(stored);
    }
    key
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec::Vec;

    fn sorted<'a>(titles: &[&'a str]) -> Vec<&'a str> {
        let mut titles = titles.to_vec();
        titles.sort_by_key(|title| sort_key(title));
        titles
    }

    #[test]
    fn the_stored_characters_are_one_ascending_run_of_one_byte_each() {
        assert_eq!(ORDER.chars().count(), 1 + 10 + 32);
        let mut previous = None;
        for (position, ranked) in ORDER.chars().enumerate() {
            assert_eq!(rank(ranked), u8::try_from(position).ok(), "{ranked}");
            let stored = stored_char(ranked).unwrap();
            assert!(stored.is_ascii(), "{ranked}");
            assert!(previous < Some(stored), "{ranked}");
            previous = Some(stored);
        }
        assert_eq!(stored_char(' '), Some('/'));
        assert_eq!(stored_char('0'), Some('0'));
        assert_eq!(stored_char('9'), Some('9'));
        assert_eq!(stored_char('a'), Some(':'));
        assert_eq!(stored_char('z'), Some('Y'));
        assert_eq!(stored_char('.'), None);
        assert_eq!(stored_char('Ж'), Some('Ж'));
    }

    #[test]
    fn ascii_fast_path_agrees_with_the_general_path() {
        for text in [
            "Dede Korkut",
            "  a  b ",
            "St. John's",
            "10 2",
            "BILL Bill ISIK isik",
            "",
            "???",
            "\t\n a\u{b}b\u{c}c\r",
        ] {
            for rule in [CaseRule::Turkish, CaseRule::Unicode] {
                let general: String = {
                    let mut it = sort_key_chars(text, rule);
                    it.source = Source::Unicode(NfcChars::of(text));
                    it.collect()
                };
                assert_eq!(sort_key_in(text, rule), general, "{text:?} {rule:?}");
            }
        }
    }

    #[test]
    fn compare_agrees_with_the_keys() {
        let texts = [
            "Çalıkuşu Reşat Nuri",
            "Çalıkuşu Reşat Nurı",
            "Çalıkuşu Reşat Nur",
            "Çalıkuşu Reşat  Nuri",
            "Çalıkuşu Reşat - Nuri",
            "Çalıkuşu Reşat -",
            "Çalıkuşu Reşat ",
            "ÇALIKUŞU REŞAT NURİ",
            "  - ",
            "  -  a",
            "  -  b",
            "ab",
            "a b",
            "a  b",
            "a-b",
            "Çalıkuşu",
            "Cevdet",
            "a b",
            "ab",
            "a  b",
            "",
            "Işık",
            "İnce",
            "Кар",
            "雪",
            "ﬁnal",
            "Straße",
            "1984",
            "10",
            "2",
        ];
        for a in texts {
            for b in texts {
                for rule in [CaseRule::Turkish, CaseRule::Unicode] {
                    assert_eq!(
                        compare_in(a, b, rule),
                        sort_key_in(a, rule).cmp(&sort_key_in(b, rule)),
                        "{a:?} {b:?} {rule:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn the_letters_follow_the_turkish_alphabet() {
        assert_eq!(
            sorted(&["gül", "ğ", "gün", "ha"]),
            ["gül", "gün", "ğ", "ha"]
        );
        assert_eq!(
            sorted(&[
                "Xavier", "Wilde", "Quixote", "Yaşar", "Vedat", "Pamuk", "Rıfat"
            ]),
            [
                "Pamuk", "Quixote", "Rıfat", "Vedat", "Wilde", "Xavier", "Yaşar"
            ]
        );
        assert!(sort_key("ık") < sort_key("ik"));
        assert_eq!(sort_key("Isı"), sort_key("ısı"));
        assert_ne!(sort_key("Isı"), sort_key("İsi"));
    }

    #[test]
    fn long_decompositions_keep_their_order() {
        // U+FDFA decomposes to eighteen characters, more than the inline queue holds.
        let key = sort_key("a\u{FDFA}b");
        let expected: String = {
            use unicode_normalization::UnicodeNormalization;
            let mut s = String::from(":");
            for part in '\u{FDFA}'.nfkd() {
                if part.is_whitespace() {
                    s.push('/');
                } else if let Some(stored) = stored_char(part) {
                    s.push(stored);
                }
            }
            s.push(';');
            s
        };
        assert_eq!(key, expected);
        assert_eq!(compare("a\u{FDFA}b", "a\u{FDFA}c"), Ordering::Less);
    }
}
