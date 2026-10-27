//! Folding: the one pass that lowers text by the Turkish rules and, by
//! choice, merges the two i's, folds accents and drops combining marks.
//! The pipeline is described on [`FoldOptions`].

use alloc::borrow::Cow;
use alloc::string::String;
use alloc::vec::Vec;
use unicode_normalization::UnicodeNormalization;
use unicode_normalization::char::is_combining_mark;

use crate::nfc::nfc;
use crate::offset_map::OffsetMap;

/// The combining dot above, which a language-blind lower case leaves after
/// an `i` that was an `İ`.
pub(crate) const COMBINING_DOT_ABOVE: char = '\u{307}';

/// The six letters Turkish has of its own, in lower case. They are letters,
/// not accented forms of `c g i o s u`, so an accent fold leaves them alone
/// unless asked to fold everything.
pub(crate) const OWN_LETTERS: [char; 6] = ['ç', 'ğ', 'ı', 'ö', 'ş', 'ü'];

/// Lower-case letters that fold to more than one letter, or to a base letter
/// their decomposition does not give them. Everything else with an accent
/// folds through its compatibility decomposition (`é` to `e`, `ñ` to `n`).
const FOLDED_LETTERS: [(char, &str); 8] = [
    ('ß', "ss"),
    ('æ', "ae"),
    ('œ', "oe"),
    ('ø', "o"),
    ('đ', "d"),
    ('ð', "d"),
    ('ł', "l"),
    ('þ', "th"),
];

/// The upper-case forms of [`FOLDED_LETTERS`], for the case-preserving folds.
const FOLDED_LETTERS_UPPER: [(char, &str); 8] = [
    ('ẞ', "SS"),
    ('Æ', "AE"),
    ('Œ', "OE"),
    ('Ø', "O"),
    ('Đ', "D"),
    ('Ð', "D"),
    ('Ł', "L"),
    ('Þ', "TH"),
];

/// What an accent fold does with accented letters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Accents {
    /// Nothing: `é` stays `é`, `â` stays `â`.
    #[default]
    Keep,
    /// Letters that are not Turkish letters of their own fold to their base
    /// letter: `â` to `a`, `é` to `e`, `ñ` to `n`, `ß` to `ss`, `ﬁ` to `fi`.
    /// The six letters Turkish has of its own, `ç ğ ı ö ş ü`, stay.
    FoldForeign,
    /// Everything folds, the six Turkish letters too: `ç` to `c`, `ğ` to
    /// `g`, `ı` to `i`, `ö` to `o`, `ş` to `s`, `ü` to `u`. Text in the
    /// Turkish and Latin alphabets comes out ASCII.
    FoldAll,
}

/// The switches of a fold.
///
/// Every fold runs one pipeline, per character of the NFC text, in this
/// order:
///
/// 1. A combining dot above (U+0307) right after a plain `i` or an `İ` is
///    dropped: it is the leftover of a language-blind lower case
///    (`"İ".to_lowercase()` is `i` + U+0307), not a letter.
/// 2. With [`strip_marks`](Self::strip_marks), any other combining mark is
///    dropped.
/// 3. The Turkish lower case: `I` to `ı`, `İ` to `i`, everything else by the
///    Unicode default mapping, character by character.
/// 4. With [`merge_dotless_i`](Self::merge_dotless_i), `ı` becomes `i`.
/// 5. By [`fold_accents`](Self::fold_accents), accented letters fold to
///    their base letters.
///
/// [`lowercase`](crate::lowercase) is this pipeline with nothing switched
/// on; [`SEARCH`](Self::SEARCH) switches on step 4; [`ASCII`](Self::ASCII)
/// switches on everything. Options are built from a preset with the
/// builder methods:
///
/// ```
/// use abece::{Accents, FoldOptions, search_fold_with};
///
/// // The search fold without the i merge: "ISPARTA" no longer matches "isparta".
/// let options = FoldOptions::SEARCH.merge_dotless_i(false);
/// assert_eq!(search_fold_with("ISPARTA", &options), "ısparta");
///
/// // The search fold plus foreign accents folded, Turkish letters kept.
/// let options = FoldOptions::SEARCH.fold_accents(Accents::FoldForeign);
/// assert_eq!(search_fold_with("Kâğıt Café", &options), "kağit cafe");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FoldOptions {
    merge_dotless_i: bool,
    accents: Accents,
    strip_marks: bool,
}

impl FoldOptions {
    /// The search fold: Turkish lower case with `ı` merged into `i`, accents
    /// and marks kept. The default.
    ///
    /// The merge is deliberate: text capitalised under a non-Turkish locale
    /// ("ISTINAF") and queries typed without a Turkish keyboard have already
    /// lost the difference between the two i's, and merging them makes both
    /// match. The price is that pairs like "sık" and "sik" fold alike. The
    /// other Turkish letters are kept: "şahin" finds "Şahin" and not "sahin".
    ///
    /// ```
    /// use abece::{FoldOptions, search_fold_with};
    ///
    /// assert_eq!(search_fold_with("IĞDIR", &FoldOptions::SEARCH), "iğdir");
    /// assert_eq!(FoldOptions::default(), FoldOptions::SEARCH);
    /// ```
    pub const SEARCH: Self = Self {
        merge_dotless_i: true,
        accents: Accents::Keep,
        strip_marks: false,
    };

    /// The ASCII fold: everything switched on. Text in the Turkish and
    /// Latin alphabets comes out as lower-case ASCII letters, which is the
    /// fold for matching names that may have been typed without a Turkish
    /// keyboard or pasted from a PDF with decomposed letters.
    ///
    /// ```
    /// use abece::{FoldOptions, search_fold_with};
    ///
    /// assert_eq!(search_fold_with("Çağrı Öztürk-Şahin", &FoldOptions::ASCII), "cagri ozturk-sahin");
    /// assert_eq!(search_fold_with("S\u{327}AHI\u{307}N", &FoldOptions::ASCII), "sahin");
    /// ```
    pub const ASCII: Self = Self {
        merge_dotless_i: true,
        accents: Accents::FoldAll,
        strip_marks: true,
    };

    /// The plain Turkish lower case: nothing switched on. What
    /// [`lowercase`](crate::lowercase) does.
    pub(crate) const LOWERCASE: Self = Self {
        merge_dotless_i: false,
        accents: Accents::Keep,
        strip_marks: false,
    };

    /// Whether `ı` folds to `i` (step 4 of the pipeline).
    ///
    /// ```
    /// use abece::{FoldOptions, search_fold_with};
    ///
    /// let keep = FoldOptions::SEARCH.merge_dotless_i(false);
    /// assert_eq!(search_fold_with("Işık", &keep), "ışık");
    /// assert!(!keep.merges_dotless_i());
    /// ```
    #[must_use]
    pub const fn merge_dotless_i(mut self, merge: bool) -> Self {
        self.merge_dotless_i = merge;
        self
    }

    /// How accented letters fold (step 5 of the pipeline).
    ///
    /// ```
    /// use abece::{Accents, FoldOptions, search_fold_with};
    ///
    /// let options = FoldOptions::SEARCH.fold_accents(Accents::FoldAll);
    /// assert_eq!(search_fold_with("Şükrü", &options), "sukru");
    /// assert_eq!(options.accents(), Accents::FoldAll);
    /// ```
    #[must_use]
    pub const fn fold_accents(mut self, accents: Accents) -> Self {
        self.accents = accents;
        self
    }

    /// Whether combining marks that survived NFC are dropped (step 2 of the
    /// pipeline): a mark on a letter that has no composed form with it, such
    /// as the macron below in `s` + U+0331.
    ///
    /// ```
    /// use abece::{FoldOptions, search_fold_with};
    ///
    /// let options = FoldOptions::SEARCH.strip_marks(true);
    /// assert_eq!(search_fold_with("Ays\u{331}e", &options), "ayse");
    /// assert!(options.strips_marks());
    /// ```
    #[must_use]
    pub const fn strip_marks(mut self, strip: bool) -> Self {
        self.strip_marks = strip;
        self
    }

    /// Whether `ı` folds to `i`.
    ///
    /// ```
    /// assert!(abece::FoldOptions::SEARCH.merges_dotless_i());
    /// ```
    #[must_use]
    pub const fn merges_dotless_i(&self) -> bool {
        self.merge_dotless_i
    }

    /// How accented letters fold.
    ///
    /// ```
    /// assert_eq!(abece::FoldOptions::SEARCH.accents(), abece::Accents::Keep);
    /// ```
    #[must_use]
    pub const fn accents(&self) -> Accents {
        self.accents
    }

    /// Whether combining marks that survived NFC are dropped.
    ///
    /// ```
    /// assert!(abece::FoldOptions::ASCII.strips_marks());
    /// ```
    #[must_use]
    pub const fn strips_marks(&self) -> bool {
        self.strip_marks
    }
}

impl FoldOptions {
    /// Whether an ASCII capital `I` comes out as `i`: under the merge, or
    /// under the full accent fold, which folds the `ı` it lowers to. Only
    /// then does ASCII text lower byte for byte.
    const fn lowers_capital_i_to_i(&self) -> bool {
        self.merge_dotless_i || matches!(self.accents, Accents::FoldAll)
    }
}

impl Default for FoldOptions {
    fn default() -> Self {
        Self::SEARCH
    }
}

/// The search fold: NFC, the Turkish lower case, and `ı` merged into `i`,
/// with `ç ğ ö ş ü` kept. See [`FoldOptions::SEARCH`] for why the merge.
///
/// Index and query go through the same function, so every spelling of a
/// word meets the same key:
///
/// ```
/// use abece::search_fold;
///
/// for spelling in ["İstanbul", "ISTANBUL", "istanbul", "İSTANBUL", "ıstanbul"] {
///     assert_eq!(search_fold(spelling), "istanbul");
/// }
/// assert_eq!(search_fold("ŞAHİN Çağrı Öztürk Ünal"), "şahin çağri öztürk ünal");
/// // A decomposed İ (I + U+0307) composes, and the dot of i + U+0307 goes.
/// assert_eq!(search_fold("I\u{307}stinaf"), "istinaf");
/// ```
///
/// Text that is already folded is borrowed, not copied:
///
/// ```
/// use std::borrow::Cow;
///
/// assert!(matches!(abece::search_fold("istanbul"), Cow::Borrowed(_)));
/// ```
#[must_use]
pub fn search_fold(text: &str) -> Cow<'_, str> {
    search_fold_with(text, &FoldOptions::SEARCH)
}

/// The fold `options` describe; [`search_fold`] with every switch exposed.
///
/// ```
/// use abece::{Accents, FoldOptions, search_fold_with};
///
/// let ascii = FoldOptions::ASCII;
/// assert_eq!(search_fold_with("DENİZ GIDA LTD.ŞTİ.", &ascii), "deniz gida ltd.sti.");
///
/// let foreign = FoldOptions::SEARCH.fold_accents(Accents::FoldForeign);
/// assert_eq!(search_fold_with("Straße Çankaya", &foreign), "strasse çankaya");
/// ```
#[must_use]
pub fn search_fold_with<'a>(text: &'a str, options: &FoldOptions) -> Cow<'a, str> {
    fold_cow(text, options)
}

/// [`search_fold`] with an [`OffsetMap`] from the folded text back to `text`.
///
/// `text` is not normalized: the map points into it as given, so a caller
/// that wants NFC matching normalizes first and keeps the NFC text as the
/// original. For NFC text the folded result equals [`search_fold`]'s.
///
/// ```
/// use abece::{nfc, search_fold, search_fold_with_map};
///
/// let original = nfc("ESAS: 2026/101 IĞDIR Asliye");
/// let (folded, map) = search_fold_with_map(&original);
/// assert_eq!(folded, search_fold(&original));
/// let start = folded.find("iğdir").unwrap();
/// assert_eq!(map.original_slice(&original, start, start + "iğdir".len()), "IĞDIR");
/// ```
#[must_use]
pub fn search_fold_with_map(text: &str) -> (String, OffsetMap) {
    fold_with_map(text, &FoldOptions::SEARCH)
}

/// [`search_fold_with`] with an [`OffsetMap`] from the folded text back to
/// `text`; `text` is not normalized, as with [`search_fold_with_map`].
///
/// ```
/// use abece::{FoldOptions, fold_with_map};
///
/// let (folded, map) = fold_with_map("Çağrı A.Ş.", &FoldOptions::ASCII);
/// assert_eq!(folded, "cagri a.s.");
/// assert_eq!(map.original_slice("Çağrı A.Ş.", 0, 5), "Çağrı");
/// ```
#[must_use]
pub fn fold_with_map(text: &str, options: &FoldOptions) -> (String, OffsetMap) {
    let mut starts = Vec::with_capacity(text.len().saturating_add(1));
    let folded = if text.is_ascii() && options.lowers_capital_i_to_i() {
        // Every byte lowers to one byte, so the map is the identity.
        starts.extend(0..=text.len());
        text.to_ascii_lowercase()
    } else {
        let mut out = String::with_capacity(text.len());
        let mut state = State::default();
        for (byte, ch) in text.char_indices() {
            if state.admit(ch, options) {
                fold_char(ch, options, &mut |folded| out.push(folded));
            }
            starts.resize(out.len(), byte);
        }
        starts.push(text.len());
        out
    };
    (folded, OffsetMap::from_starts(starts))
}

/// Accented letters folded to their base letters, case kept, the six Turkish
/// letters kept: `â` to `a`, `é` to `e`, `ß` to `ss`, `ﬁ` to `fi`, while `ç
/// ğ ı ö ş ü` and `İ` stay. Combining marks left after NFC are dropped.
///
/// ```
/// use abece::accent_fold;
///
/// assert_eq!(accent_fold("Kâğıt"), "Kağıt");
/// assert_eq!(accent_fold("Dîvân-ı Lügati't-Türk"), "Divan-ı Lügati't-Türk");
/// assert_eq!(accent_fold("Straße İÇİN"), "Strasse İÇİN");
/// assert_eq!(accent_fold("naïve café"), "naive cafe");
/// ```
#[must_use]
pub fn accent_fold(text: &str) -> Cow<'_, str> {
    accent_fold_cow(text, false)
}

/// Every letter of the Turkish and Latin alphabets folded to ASCII, case
/// kept: `ç` to `c`, `ğ` to `g`, `ı` to `i`, `İ` to `I`, `ö` to `o`, `ş` to
/// `s`, `ü` to `u`, and what [`accent_fold`] folds. Letters of other scripts
/// are left as they are.
///
/// ```
/// use abece::ascii_fold;
///
/// assert_eq!(ascii_fold("Çağrı ÖZTÜRK"), "Cagri OZTURK");
/// assert_eq!(ascii_fold("İSTANBUL ışık"), "ISTANBUL isik");
/// assert_eq!(ascii_fold("Ωmega"), "Ωmega");
/// ```
#[must_use]
pub fn ascii_fold(text: &str) -> Cow<'_, str> {
    accent_fold_cow(text, true)
}

/// The state the pipeline's first two steps keep between characters.
#[derive(Default)]
pub(crate) struct State {
    /// The last character admitted was a plain `i` or an `İ`, so a combining
    /// dot above that follows is dropped.
    after_plain_i: bool,
}

impl State {
    /// Whether `ch` is kept (steps 1 and 2 of the pipeline). Dropping a
    /// character emits nothing, so the caller skips [`fold_char`] for it.
    pub(crate) fn admit(&mut self, ch: char, options: &FoldOptions) -> bool {
        if ch == COMBINING_DOT_ABOVE && self.after_plain_i {
            self.after_plain_i = false;
            return false;
        }
        self.after_plain_i = matches!(ch, 'i' | 'İ');
        !(options.strip_marks && is_combining_mark(ch))
    }
}

/// Steps 3 to 5 of the pipeline for one admitted character. Pure: it may be
/// run again for the same character.
pub(crate) fn fold_char(ch: char, options: &FoldOptions, emit: &mut impl FnMut(char)) {
    match ch {
        'I' => fold_lower('ı', options, emit),
        'İ' => fold_lower('i', options, emit),
        _ if ch.is_ascii() => fold_lower(ch.to_ascii_lowercase(), options, emit),
        _ => {
            for lower in ch.to_lowercase() {
                fold_lower(lower, options, emit);
            }
        }
    }
}

/// Steps 4 and 5 for one lower-case character.
fn fold_lower(lower: char, options: &FoldOptions, emit: &mut impl FnMut(char)) {
    let lower = if options.merge_dotless_i && lower == 'ı' {
        'i'
    } else {
        lower
    };
    match options.accents {
        Accents::Keep => emit(lower),
        Accents::FoldForeign => fold_foreign_lower(lower, emit),
        Accents::FoldAll => fold_foreign_lower(own_letter_to_ascii(lower), emit),
    }
}

/// Hands `emit` what one lower-case letter folds to when the six Turkish
/// letters are kept: itself for them and for ASCII, a table entry for the
/// letters whose decomposition does not give their base, and otherwise the
/// compatibility decomposition without its marks. A part of a decomposition
/// can be a capital even though `lower` is lower case (the script `ℌ` is an
/// `H`), so each part is lowered again, by the Unicode default mapping.
pub(crate) fn fold_foreign_lower(lower: char, emit: &mut impl FnMut(char)) {
    if lower.is_ascii() || OWN_LETTERS.contains(&lower) {
        emit(lower);
        return;
    }
    if let Some((_, replacement)) = FOLDED_LETTERS.iter().find(|(letter, _)| *letter == lower) {
        replacement.chars().for_each(emit);
        return;
    }
    for part in lower.nfkd().filter(|part| !is_combining_mark(*part)) {
        part.to_lowercase().for_each(&mut *emit);
    }
}

/// The ASCII letter one of the six Turkish letters stands in for in an
/// ASCII fold; any other character as it is.
pub(crate) const fn own_letter_to_ascii(lower: char) -> char {
    match lower {
        'ç' => 'c',
        'ğ' => 'g',
        'ı' => 'i',
        'ö' => 'o',
        'ş' => 's',
        'ü' => 'u',
        other => other,
    }
}

/// The ASCII letter a Turkish letter of either case stands in for.
const fn own_letter_to_ascii_any_case(ch: char) -> char {
    match ch {
        'Ç' => 'C',
        'Ğ' => 'G',
        'İ' => 'I',
        'Ö' => 'O',
        'Ş' => 'S',
        'Ü' => 'U',
        other => own_letter_to_ascii(other),
    }
}

/// Whether `ch` is one of the six Turkish letters or `İ`, in either case.
/// (`I` is ASCII and never reaches the fold.)
const fn is_own_letter_any_case(ch: char) -> bool {
    matches!(
        ch,
        'ç' | 'ğ' | 'ı' | 'ö' | 'ş' | 'ü' | 'Ç' | 'Ğ' | 'İ' | 'Ö' | 'Ş' | 'Ü'
    )
}

/// NFC, then the fold, borrowing the text when nothing changes.
pub(crate) fn fold_cow<'a>(text: &'a str, options: &FoldOptions) -> Cow<'a, str> {
    if text.is_ascii() {
        return ascii_lowercase(text, options.lowers_capital_i_to_i());
    }
    let text = nfc(text);
    match fold_if_changed(&text, options) {
        Some(folded) => Cow::Owned(folded),
        None => text,
    }
}

/// The fold of NFC `text`, or `None` when the fold would change nothing.
///
/// The output is NFC too: a letter that folds to several (`ß` to `ss`)
/// could only compose with a combining mark that follows, and the accent
/// folds that expand letters drop every mark.
fn fold_if_changed(text: &str, options: &FoldOptions) -> Option<String> {
    let mut out: Option<String> = None;
    let mut state = State::default();
    for (byte, ch) in text.char_indices() {
        let kept = state.admit(ch, options);
        if let Some(out) = out.as_mut() {
            if kept {
                fold_char(ch, options, &mut |folded| out.push(folded));
            }
            continue;
        }
        if kept && is_unchanged(ch, options) {
            continue;
        }
        // The first character that changes: copy what came before it.
        let mut started = String::with_capacity(text.len());
        started.push_str(text.get(..byte).unwrap_or(""));
        if kept {
            fold_char(ch, options, &mut |folded| started.push(folded));
        }
        out = Some(started);
    }
    out
}

/// `out` composed again when a character expanded into several (`ß` to
/// `SS`) and a mark that follows may compose with the last of them
/// (`S` + U+0327 is `Ş`); the output is then NFC like the input, and
/// mapping it again changes nothing.
pub(crate) fn recomposed(out: String, expanded: bool) -> String {
    if !expanded {
        return out;
    }
    match nfc(&out) {
        Cow::Borrowed(_) => out,
        Cow::Owned(normalized) => normalized,
    }
}

/// Whether [`fold_char`] emits exactly `ch` for `ch`.
pub(crate) fn is_unchanged(ch: char, options: &FoldOptions) -> bool {
    if ch.is_ascii() {
        return !ch.is_ascii_uppercase();
    }
    // A lower-case letter is its own lower case (every character with the
    // Lowercase property maps to itself; `nfc::tests` checks all of them),
    // so with accents kept only `ı` under the merge can change.
    if options.accents == Accents::Keep && ch.is_lowercase() {
        return !(options.merge_dotless_i && ch == 'ı');
    }
    let mut count = 0_u8;
    let mut same = true;
    fold_char(ch, options, &mut |folded| {
        count = count.saturating_add(1);
        same &= folded == ch;
    });
    same && count == 1
}

/// The lower case of ASCII `text`: borrowed when it has no capital, and
/// otherwise with `I` lowered to `i` (`capital_i_to_i`) or to the two-byte
/// `ı`.
pub(crate) fn ascii_lowercase(text: &str, capital_i_to_i: bool) -> Cow<'_, str> {
    let bytes = text.as_bytes();
    let Some(first_upper) = bytes.iter().position(u8::is_ascii_uppercase) else {
        return Cow::Borrowed(text);
    };
    let rest = bytes.get(first_upper..).unwrap_or(&[]);
    if capital_i_to_i || !rest.contains(&b'I') {
        return Cow::Owned(text.to_ascii_lowercase());
    }
    // Each `I` grows by a byte; one pass over the bytes writes the rest.
    let capital_is = rest.iter().filter(|&&b| b == b'I').count();
    let mut out = Vec::with_capacity(text.len().saturating_add(capital_is));
    for &byte in bytes {
        if byte == b'I' {
            out.extend_from_slice("ı".as_bytes());
        } else {
            out.push(byte.to_ascii_lowercase());
        }
    }
    // ASCII bytes and the bytes of `ı` are valid UTF-8; the fallback is unreachable.
    Cow::Owned(String::from_utf8(out).unwrap_or_default())
}

/// NFC, then the case-preserving accent fold, borrowing the text when
/// nothing changes.
fn accent_fold_cow(text: &str, all: bool) -> Cow<'_, str> {
    if text.is_ascii() {
        return Cow::Borrowed(text);
    }
    let text = nfc(text);
    let mut out: Option<String> = None;
    for (byte, ch) in text.char_indices() {
        if let Some(out) = out.as_mut() {
            fold_accent_char(ch, all, &mut |folded| out.push(folded));
            continue;
        }
        if ch.is_ascii() || is_accent_unchanged(ch, all) {
            continue;
        }
        let mut started = String::with_capacity(text.len());
        started.push_str(text.get(..byte).unwrap_or(""));
        fold_accent_char(ch, all, &mut |folded| started.push(folded));
        out = Some(started);
    }
    match out {
        Some(folded) => Cow::Owned(folded),
        None => text,
    }
}

/// Whether [`fold_accent_char`] emits exactly `ch` for `ch`.
fn is_accent_unchanged(ch: char, all: bool) -> bool {
    let mut count = 0_u8;
    let mut same = true;
    fold_accent_char(ch, all, &mut |folded| {
        count = count.saturating_add(1);
        same &= folded == ch;
    });
    same && count == 1
}

/// What one character folds to, case kept: nothing for a combining mark,
/// itself for ASCII and (unless `all`) for the Turkish letters, a table
/// entry or the decomposition without its marks for the rest.
fn fold_accent_char(ch: char, all: bool, emit: &mut impl FnMut(char)) {
    if ch.is_ascii() {
        emit(ch);
        return;
    }
    if is_combining_mark(ch) {
        return;
    }
    let ch = if all {
        own_letter_to_ascii_any_case(ch)
    } else {
        ch
    };
    if ch.is_ascii() || is_own_letter_any_case(ch) {
        emit(ch);
        return;
    }
    if let Some((_, replacement)) = FOLDED_LETTERS
        .iter()
        .chain(FOLDED_LETTERS_UPPER.iter())
        .find(|(letter, _)| *letter == ch)
    {
        replacement.chars().for_each(emit);
        return;
    }
    ch.nfkd()
        .filter(|part| !is_combining_mark(*part))
        .for_each(emit);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_fold_merges_the_two_is_and_keeps_the_other_letters() {
        assert_eq!(search_fold("ışık IŞIK"), "işik işik");
        assert_eq!(search_fold("Iğdır"), "iğdir");
        assert_eq!(search_fold("ı\u{307}"), "i\u{307}");
        assert_eq!(search_fold("  "), "  ");
        assert_eq!(search_fold(""), "");
    }

    #[test]
    fn folded_text_is_borrowed() {
        assert!(matches!(search_fold("istanbul 2026"), Cow::Borrowed(_)));
        assert!(matches!(search_fold("şahin çağri"), Cow::Borrowed(_)));
        assert!(matches!(search_fold("Şahin"), Cow::Owned(_)));
        assert!(matches!(search_fold("ışık"), Cow::Owned(_)));
        assert!(matches!(accent_fold("Çağrı"), Cow::Borrowed(_)));
        assert!(matches!(ascii_fold("Cagri"), Cow::Borrowed(_)));
        assert!(matches!(ascii_fold("Çağrı"), Cow::Owned(_)));
    }

    /// Every combination of the switches.
    fn every_option() -> impl Iterator<Item = FoldOptions> {
        [true, false].into_iter().flat_map(|merge| {
            [true, false].into_iter().flat_map(move |strip| {
                [Accents::Keep, Accents::FoldForeign, Accents::FoldAll]
                    .into_iter()
                    .map(move |accents| {
                        FoldOptions::SEARCH
                            .merge_dotless_i(merge)
                            .strip_marks(strip)
                            .fold_accents(accents)
                    })
            })
        })
    }

    #[test]
    fn ascii_fast_path_agrees_with_the_general_path() {
        for text in ["ISPARTA Ilik", "no capitals", "III", "I", "", "aIbIc"] {
            for options in every_option() {
                let general: String = {
                    let mut out = String::new();
                    let mut state = State::default();
                    for ch in text.chars() {
                        if state.admit(ch, &options) {
                            fold_char(ch, &options, &mut |c| out.push(c));
                        }
                    }
                    out
                };
                assert_eq!(fold_cow(text, &options), general, "{text} {options:?}");
                assert_eq!(
                    fold_with_map(text, &options).0,
                    general,
                    "{text} {options:?}"
                );
            }
        }
        assert_eq!(ascii_lowercase("ISPARTA I", false), "ısparta ı");
        assert_eq!(ascii_lowercase("ISPARTA I", true), "isparta i");
        // The full accent fold lowers I to i even without the merge.
        let all = FoldOptions::SEARCH
            .merge_dotless_i(false)
            .fold_accents(Accents::FoldAll);
        assert_eq!(search_fold_with("I", &all), "i");
        assert_eq!(search_fold_with("ı", &all), "i");
    }

    #[test]
    fn options_change_the_fold() {
        let keep = FoldOptions::SEARCH.merge_dotless_i(false);
        assert_eq!(search_fold_with("IŞIK", &keep), "ışık");
        let foreign = FoldOptions::SEARCH.fold_accents(Accents::FoldForeign);
        assert_eq!(
            search_fold_with("Kâğıt ﬁnal Œuvre", &foreign),
            "kağit final oeuvre"
        );
        assert_eq!(search_fold_with("Çağrı", &foreign), "çağri");
        assert_eq!(search_fold_with("Çağrı", &FoldOptions::ASCII), "cagri");
        assert_eq!(
            search_fold_with("O\u{308}zturk", &FoldOptions::ASCII),
            "ozturk"
        );
        assert_eq!(
            search_fold_with("Ays\u{331}e", &FoldOptions::SEARCH),
            "ays\u{331}e"
        );
        assert_eq!(search_fold_with("Ays\u{331}e", &FoldOptions::ASCII), "ayse");
    }

    #[test]
    fn maps_follow_every_byte() {
        let (folded, map) = search_fold_with_map("i\u{307}ki");
        assert_eq!(folded, "iki");
        assert_eq!(
            map.original_slice("i\u{307}ki", 0, folded.len()),
            "i\u{307}ki"
        );
        let (folded, map) = fold_with_map("ab IC", &FoldOptions::SEARCH);
        assert_eq!(folded, "ab ic");
        assert_eq!(map.as_slice(), &[0, 1, 2, 3, 4, 5]);
        let (folded, map) = fold_with_map("IC", &FoldOptions::LOWERCASE);
        assert_eq!(folded, "ıc");
        assert_eq!(map.as_slice(), &[0, 0, 1, 2]);
        let (folded, map) = fold_with_map("", &FoldOptions::SEARCH);
        assert_eq!(folded, "");
        assert_eq!(map.as_slice(), &[0]);
    }

    #[test]
    fn an_accent_fold_drops_the_mark_after_an_expansion() {
        // With accents kept, nothing expands and the mark stays.
        assert_eq!(
            search_fold_with("ß\u{327}", &FoldOptions::SEARCH),
            "ß\u{327}"
        );
        // With accents folded, the mark goes with the accents, so the
        // `ss` cannot compose with it and the output stays NFC.
        let foreign = FoldOptions::SEARCH.fold_accents(Accents::FoldForeign);
        assert_eq!(search_fold_with("ß\u{327}", &foreign), "ss");
        assert_eq!(search_fold_with("ss", &foreign), "ss");
        assert_eq!(accent_fold("ß\u{327}"), "ss");
        assert_eq!(ascii_fold("ß\u{327}"), "ss");
        assert_eq!(fold_with_map("ß\u{327}", &foreign).0, "ss");
    }

    #[test]
    fn case_preserving_folds() {
        assert_eq!(accent_fold("ÂÎÛ âîû"), "AIU aiu");
        assert_eq!(accent_fold("ẞ Æ Œ Ø Đ Ð Ł Þ"), "SS AE OE O D D L TH");
        assert_eq!(accent_fold("ß æ œ ø đ ð ł þ"), "ss ae oe o d d l th");
        assert_eq!(accent_fold("Ｒoman ℌ"), "Roman H");
        assert_eq!(
            ascii_fold("Ç Ğ I İ Ö Ş Ü ç ğ ı i ö ş ü"),
            "C G I I O S U c g i i o s u"
        );
        assert_eq!(ascii_fold("S\u{327}ahin"), "Sahin");
    }
}
