//! Suffixes on proper nouns, numbers and abbreviations: the apostrophe,
//! vowel harmony and the buffer letters, as the spelling rules of the
//! Turkish Language Association (TDK) write them.
//!
//! "Ayşe'nin", "İstanbul'da", "Furkan'a", "2026'da", "TBMM'ye": the suffix
//! listens to how the word *sounds* at its end, its last vowel (back or
//! front, rounded or not) and its last sound (a vowel, a voiceless
//! consonant, or another consonant). [`Harmony`] is that analysis;
//! [`attach`] applies it with the apostrophe the rules put between a proper
//! noun and its inflectional suffix.
//!
//! What is *not* done here, because it needs a dictionary: consonant
//! softening of common nouns ("kitap" to "kitabı"), which proper nouns do
//! not show in writing anyway ("Ahmet'i", "Zonguldak'a"); the few words
//! whose harmony does not follow their spelling ("saat", "Bilal'e",
//! "Google'a"); and loanwords read differently from how they are written.
//! For those, [`attach_spoken`] takes the spoken form from the caller.

use alloc::string::String;
use alloc::vec::Vec;

use crate::fold::fold_foreign_lower;
use crate::letters::{
    is_back_vowel, is_consonant, is_letter, is_rounded_vowel, is_voiceless_consonant, is_vowel,
    lowercase_char,
};
use crate::number::to_words;

/// A suffix that attaches to a noun.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Suffix {
    /// `-lar` / `-ler`. On a proper noun, written without the apostrophe
    /// ("Ahmetler"); on a number or an abbreviation, with it ("1990'lar").
    Plural,
    /// `-(n)ın` / `-(n)in` / `-(n)un` / `-(n)ün`: "Ayşe'nin", "İstanbul'un".
    Genitive,
    /// `-(y)a` / `-(y)e`: "Ayşe'ye", "Furkan'a".
    Dative,
    /// `-da` / `-de`, `-ta` / `-te` after a voiceless consonant:
    /// "İstanbul'da", "Sinop'ta".
    Locative,
    /// `-dan` / `-den`, `-tan` / `-ten` after a voiceless consonant:
    /// "Ankara'dan", "Zonguldak'tan".
    Ablative,
    /// `-(y)ı` / `-(y)i` / `-(y)u` / `-(y)ü`: "Ayşe'yi", "İstanbul'u".
    Accusative,
    /// `-(y)la` / `-(y)le` (the clitic "ile"): "Ayşe'yle", "Furkan'la".
    Instrumental,
    /// `-daki` / `-deki`, `-taki` / `-teki`: "İstanbul'daki", "Sinop'taki".
    LocativeRelative,
}

/// How a word sounds at its end: what a suffix listens to.
///
/// ```
/// use abece::suffix::{Harmony, Suffix};
///
/// let ayse = Harmony::of("Ayşe");
/// assert!(!ayse.is_back() && !ayse.is_rounded() && ayse.ends_with_vowel());
/// assert_eq!(ayse.suffix(Suffix::Genitive), "nin");
///
/// let sinop = Harmony::of("Sinop");
/// assert!(sinop.is_back() && sinop.is_rounded() && sinop.ends_voiceless());
/// assert_eq!(sinop.suffix(Suffix::Locative), "ta");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Harmony {
    back: bool,
    rounded: bool,
    ends_with_vowel: bool,
    ends_voiceless: bool,
}

impl Harmony {
    /// The analysis of `spoken`, read as it is written: the last vowel sets
    /// back or front and rounded or not, the last letter sets the final
    /// sound. A word without a vowel is read as back and unrounded.
    /// Characters that are not letters at the end ("Ayşe)") are skipped.
    /// Foreign letters are read by their base letter (`é` as `e`), `w` and
    /// `q` as consonants, `x` as the voiceless `ks`.
    ///
    /// ```
    /// use abece::suffix::Harmony;
    ///
    /// assert!(Harmony::of("İstanbul").is_back());
    /// assert!(Harmony::of("Köln").is_rounded());
    /// assert!(Harmony::of("Ahmet").ends_voiceless());
    /// assert!(Harmony::of("Furkan,").is_back());
    /// assert!(Harmony::of("Café").ends_with_vowel() && !Harmony::of("Café").is_back());
    /// ```
    #[must_use]
    pub fn of(spoken: &str) -> Self {
        let mut last_vowel = None;
        let mut last_letter = None;
        for c in spoken.chars() {
            fold_foreign_lower(lowercase_char(c), &mut |c| {
                if is_vowel(c) {
                    last_vowel = Some(c);
                }
                if c.is_alphabetic() {
                    last_letter = Some(c);
                }
            });
        }
        Self {
            back: last_vowel.is_none_or(is_back_vowel),
            rounded: last_vowel.is_some_and(is_rounded_vowel),
            ends_with_vowel: last_letter.is_some_and(is_vowel),
            ends_voiceless: last_letter.is_some_and(|c| is_voiceless_consonant(c) || c == 'x'),
        }
    }

    /// The analysis of `word` as [`attach`] reads it: a word that ends in
    /// digits sounds like the number in words ("2026" ends in "altı",
    /// "1.000" in "bin"); an abbreviation in capitals that cannot be read as
    /// a word, one written as dotted initials, or a single capital letter
    /// sounds like the name of its last letter ("TBMM" ends in "me", "ABD"
    /// in "de", "A.Ş." in "şe", "Plan B" in "be"); anything else is read as
    /// it is written. Of several words, the last one decides: "Deniz Gıda
    /// A.Ş." is read by "A.Ş.", "A. Ş." by "Ş", "Ahmet K." by "K".
    ///
    /// Which capitalised words are spelled out is a heuristic: a word of
    /// two letters, one without a vowel, or one whose letters do not form
    /// Turkish syllables ("ABD", "AKP", "ÖSYM"). "NATO", "YÖK", "ODTÜ" and
    /// "TÜBİTAK" read as words. Where it guesses wrong, give the spoken
    /// form to [`attach_spoken`].
    ///
    /// ```
    /// use abece::suffix::{Harmony, Suffix};
    ///
    /// assert_eq!(Harmony::of_word("2026").suffix(Suffix::Locative), "da");
    /// assert_eq!(Harmony::of_word("5").suffix(Suffix::Locative), "te");
    /// assert_eq!(Harmony::of_word("1.000").suffix(Suffix::Dative), "e");
    /// assert_eq!(Harmony::of_word("TBMM").suffix(Suffix::Dative), "ye");
    /// assert_eq!(Harmony::of_word("NATO").suffix(Suffix::Dative), "ya");
    /// assert_eq!(Harmony::of_word("Deniz Gıda A.Ş.").suffix(Suffix::Dative), "ye");
    /// assert_eq!(Harmony::of_word("Ahmet K.").suffix(Suffix::Genitive), "nin");
    /// ```
    #[must_use]
    pub fn of_word(word: &str) -> Self {
        let trimmed = word.trim_end_matches(|c: char| !c.is_alphanumeric());
        if trimmed.ends_with(|c: char| c.is_ascii_digit()) {
            return Self::of(&number_words(trimmed));
        }
        if let Some(name) = spelled_out_last_letter(last_word(trimmed)) {
            return Self::of(name);
        }
        Self::of(trimmed)
    }

    /// Whether the last vowel is a back vowel (`a ı o u`), so the suffix
    /// takes its back form.
    ///
    /// ```
    /// assert!(abece::suffix::Harmony::of("Ankara").is_back());
    /// assert!(!abece::suffix::Harmony::of("İzmir").is_back());
    /// ```
    #[must_use]
    pub const fn is_back(&self) -> bool {
        self.back
    }

    /// Whether the last vowel is rounded (`o u ö ü`), so a high suffix
    /// vowel is `u` or `ü`.
    ///
    /// ```
    /// assert!(abece::suffix::Harmony::of("Bursa").is_rounded() == false);
    /// assert!(abece::suffix::Harmony::of("Muğla").is_rounded() == false);
    /// assert!(abece::suffix::Harmony::of("Bolu").is_rounded());
    /// ```
    #[must_use]
    pub const fn is_rounded(&self) -> bool {
        self.rounded
    }

    /// Whether the word ends in a vowel, so a suffix that starts with a
    /// vowel takes its buffer letter (`-y-`, `-n-`).
    ///
    /// ```
    /// assert!(abece::suffix::Harmony::of("Ayşe").ends_with_vowel());
    /// assert!(!abece::suffix::Harmony::of("Furkan").ends_with_vowel());
    /// ```
    #[must_use]
    pub const fn ends_with_vowel(&self) -> bool {
        self.ends_with_vowel
    }

    /// Whether the word ends in a voiceless consonant (`f s t k ç ş h p`),
    /// so `-da` is `-ta`.
    ///
    /// ```
    /// assert!(abece::suffix::Harmony::of("Sinop").ends_voiceless());
    /// assert!(!abece::suffix::Harmony::of("İstanbul").ends_voiceless());
    /// ```
    #[must_use]
    pub const fn ends_voiceless(&self) -> bool {
        self.ends_voiceless
    }

    /// The high vowel the harmony calls for: `ı`, `i`, `u` or `ü`.
    ///
    /// ```
    /// use abece::suffix::Harmony;
    ///
    /// assert_eq!(Harmony::of("Ankara").high_vowel(), 'ı');
    /// assert_eq!(Harmony::of("İzmir").high_vowel(), 'i');
    /// assert_eq!(Harmony::of("Bolu").high_vowel(), 'u');
    /// assert_eq!(Harmony::of("Köln").high_vowel(), 'ü');
    /// ```
    #[must_use]
    pub const fn high_vowel(&self) -> char {
        match (self.back, self.rounded) {
            (true, false) => 'ı',
            (false, false) => 'i',
            (true, true) => 'u',
            (false, true) => 'ü',
        }
    }

    /// The low vowel the harmony calls for: `a` or `e`.
    ///
    /// ```
    /// use abece::suffix::Harmony;
    ///
    /// assert_eq!(Harmony::of("Bolu").low_vowel(), 'a');
    /// assert_eq!(Harmony::of("Köln").low_vowel(), 'e');
    /// ```
    #[must_use]
    pub const fn low_vowel(&self) -> char {
        if self.back { 'a' } else { 'e' }
    }

    /// The form `suffix` takes after a word with this harmony, without the
    /// apostrophe.
    ///
    /// ```
    /// use abece::suffix::{Harmony, Suffix};
    ///
    /// let h = Harmony::of("İstanbul");
    /// assert_eq!(h.suffix(Suffix::Genitive), "un");
    /// assert_eq!(h.suffix(Suffix::Dative), "a");
    /// assert_eq!(h.suffix(Suffix::Locative), "da");
    /// assert_eq!(h.suffix(Suffix::Ablative), "dan");
    /// assert_eq!(h.suffix(Suffix::Accusative), "u");
    /// assert_eq!(h.suffix(Suffix::Instrumental), "la");
    /// assert_eq!(h.suffix(Suffix::Plural), "lar");
    /// assert_eq!(h.suffix(Suffix::LocativeRelative), "daki");
    /// ```
    #[must_use]
    pub const fn suffix(&self, suffix: Suffix) -> &'static str {
        let (back, rounded, vowel, voiceless) = (
            self.back,
            self.rounded,
            self.ends_with_vowel,
            self.ends_voiceless,
        );
        match suffix {
            Suffix::Plural => {
                if back {
                    "lar"
                } else {
                    "ler"
                }
            }
            Suffix::Genitive => match (back, rounded, vowel) {
                (true, false, true) => "nın",
                (true, false, false) => "ın",
                (false, false, true) => "nin",
                (false, false, false) => "in",
                (true, true, true) => "nun",
                (true, true, false) => "un",
                (false, true, true) => "nün",
                (false, true, false) => "ün",
            },
            Suffix::Dative => match (back, vowel) {
                (true, true) => "ya",
                (true, false) => "a",
                (false, true) => "ye",
                (false, false) => "e",
            },
            Suffix::Locative => match (back, voiceless) {
                (true, true) => "ta",
                (true, false) => "da",
                (false, true) => "te",
                (false, false) => "de",
            },
            Suffix::Ablative => match (back, voiceless) {
                (true, true) => "tan",
                (true, false) => "dan",
                (false, true) => "ten",
                (false, false) => "den",
            },
            Suffix::Accusative => match (back, rounded, vowel) {
                (true, false, true) => "yı",
                (true, false, false) => "ı",
                (false, false, true) => "yi",
                (false, false, false) => "i",
                (true, true, true) => "yu",
                (true, true, false) => "u",
                (false, true, true) => "yü",
                (false, true, false) => "ü",
            },
            Suffix::Instrumental => match (back, vowel) {
                (true, true) => "yla",
                (true, false) => "la",
                (false, true) => "yle",
                (false, false) => "le",
            },
            Suffix::LocativeRelative => match (back, voiceless) {
                (true, true) => "taki",
                (true, false) => "daki",
                (false, true) => "teki",
                (false, false) => "deki",
            },
        }
    }
}

/// `word` with `suffix` attached, the apostrophe between them where the
/// rules put one: before every inflectional suffix on a proper noun, a
/// number or an abbreviation, and before the plural suffix only on a
/// number or an abbreviation. The word is read by [`Harmony::of_word`].
///
/// ```
/// use abece::suffix::{Suffix, attach};
///
/// assert_eq!(attach("Ayşe", Suffix::Genitive), "Ayşe'nin");
/// assert_eq!(attach("İstanbul", Suffix::Locative), "İstanbul'da");
/// assert_eq!(attach("Furkan", Suffix::Dative), "Furkan'a");
/// assert_eq!(attach("Sinop", Suffix::Ablative), "Sinop'tan");
/// assert_eq!(attach("Ahmet", Suffix::Accusative), "Ahmet'i");
/// assert_eq!(attach("Ayşe", Suffix::Instrumental), "Ayşe'yle");
/// assert_eq!(attach("Ahmet", Suffix::Plural), "Ahmetler");
/// assert_eq!(attach("2026", Suffix::Locative), "2026'da");
/// assert_eq!(attach("1990", Suffix::Plural), "1990'lar");
/// assert_eq!(attach("1.000", Suffix::Dative), "1.000'e");
/// assert_eq!(attach("TBMM", Suffix::Dative), "TBMM'ye");
/// assert_eq!(attach("Deniz Gıda A.Ş.", Suffix::Dative), "Deniz Gıda A.Ş.'ye");
/// assert_eq!(attach("İstanbul", Suffix::LocativeRelative), "İstanbul'daki");
/// ```
#[must_use]
pub fn attach(word: &str, suffix: Suffix) -> String {
    let harmony = Harmony::of_word(word);
    let apostrophe = suffix != Suffix::Plural || is_number_or_abbreviation(word);
    join(word, harmony.suffix(suffix), apostrophe)
}

/// `word` with `suffix` attached as [`attach`] does, but read by `spoken`,
/// the way the word sounds, for the words whose spelling misleads: a
/// foreign name, a thin `l`, an abbreviation the heuristic gets wrong.
///
/// ```
/// use abece::suffix::{Suffix, attach_spoken};
///
/// assert_eq!(attach_spoken("Bilal", "bilel", Suffix::Dative), "Bilal'e");
/// assert_eq!(attach_spoken("Google", "gugıl", Suffix::Dative), "Google'a");
/// assert_eq!(attach_spoken("İİK", "iike", Suffix::Genitive), "İİK'nin");
/// ```
#[must_use]
pub fn attach_spoken(word: &str, spoken: &str, suffix: Suffix) -> String {
    let harmony = Harmony::of(spoken);
    let apostrophe = suffix != Suffix::Plural || is_number_or_abbreviation(word);
    join(word, harmony.suffix(suffix), apostrophe)
}

/// The form `suffix` takes after `word`, without the apostrophe:
/// [`Harmony::of_word`] followed by [`Harmony::suffix`].
///
/// ```
/// use abece::suffix::{Suffix, suffix_for};
///
/// assert_eq!(suffix_for("Ayşe", Suffix::Dative), "ye");
/// assert_eq!(suffix_for("Zonguldak", Suffix::Locative), "ta");
/// assert_eq!(suffix_for("kitap", Suffix::Plural), "lar");
/// assert_eq!(suffix_for("gül", Suffix::Plural), "ler");
/// ```
#[must_use]
pub fn suffix_for(word: &str, suffix: Suffix) -> &'static str {
    Harmony::of_word(word).suffix(suffix)
}

/// `word`, the apostrophe if wanted, `suffix`.
fn join(word: &str, suffix: &str, apostrophe: bool) -> String {
    let mut out = String::with_capacity(word.len().saturating_add(suffix.len()).saturating_add(1));
    out.push_str(word);
    if apostrophe {
        out.push('\'');
    }
    out.push_str(suffix);
    out
}

/// Whether `word` ends in a digit or its last word is a run of capitals,
/// of dotted initials or a single capital letter, which take the apostrophe
/// before every suffix, the plural included.
fn is_number_or_abbreviation(word: &str) -> bool {
    let trimmed = word.trim_end_matches(|c: char| !c.is_alphanumeric());
    let last = last_word(trimmed);
    trimmed.ends_with(|c: char| c.is_ascii_digit())
        || is_all_capitals(last)
        || is_dotted_initials(last)
        || is_single_capital(last)
}

/// The last word of `text`: what follows its last whitespace.
fn last_word(text: &str) -> &str {
    text.rsplit(char::is_whitespace).next().unwrap_or(text)
}

/// Whether `word` is two or more capital initials, each but the last
/// followed by a dot: "A.Ş", "T.C", "A.B.D" (the caller has trimmed the
/// last dot).
fn is_dotted_initials(word: &str) -> bool {
    let mut initials = 0_usize;
    for part in word.split('.') {
        if !is_single_capital(part) {
            return false;
        }
        initials = initials.saturating_add(1);
    }
    initials >= 2
}

/// Whether `word` is one capital letter: an initial ("K" of "Ahmet K.",
/// "Ş" of "A. Ş.") or a letter on its own ("Plan B").
fn is_single_capital(word: &str) -> bool {
    let mut chars = word.chars();
    matches!(
        (chars.next(), chars.next()),
        (Some(c), None) if c.is_alphabetic() && !c.is_lowercase()
    )
}

/// Whether `word` is letters only, all capitals, at least two of them.
fn is_all_capitals(word: &str) -> bool {
    let mut letters = 0_usize;
    for c in word.chars() {
        if !c.is_alphabetic() || c.is_lowercase() {
            return false;
        }
        letters = letters.saturating_add(1);
    }
    letters >= 2
}

/// The words of the number `word` ends in. A number too long for `u64`
/// is read by its last eighteen digits, which carry its last word unless
/// they are all zero.
fn number_words(word: &str) -> String {
    let digits = last_number(word.trim_end_matches(|c: char| !c.is_ascii_digit()));
    let tail = digits.len().saturating_sub(18);
    let number = digits
        .get(tail..)
        .and_then(|d| d.parse::<u64>().ok())
        .unwrap_or(0);
    to_words(number)
}

/// The digits of the number `text` ends in, without the dots of a
/// thousands grouping: "1000" of "1.000", "2026" of "E.2026". Dots that do
/// not group thousands end the number ("2026" of "25.07.2026").
fn last_number(text: &str) -> String {
    let digits_and_dots = text.trim_end_matches(|c: char| c.is_ascii_digit() || c == '.');
    let candidate = text
        .get(digits_and_dots.len()..)
        .unwrap_or("")
        .trim_start_matches('.');
    if let Some(grouped) = thousands_grouped(candidate) {
        return grouped;
    }
    let before_digits = text.trim_end_matches(|c: char| c.is_ascii_digit());
    text.get(before_digits.len()..).unwrap_or("").into()
}

/// The digits of `number` without its dots when it is grouped in
/// thousands, one to three digits and then groups of three ("1.000",
/// "250.000"); `None` otherwise. `number` holds only ASCII digits and dots.
fn thousands_grouped(number: &str) -> Option<String> {
    let mut groups = number.split('.');
    let first = groups
        .next()
        .filter(|group| (1..=3).contains(&group.len()))?;
    let mut digits = String::from(first);
    let mut grouped = false;
    for group in groups {
        if group.len() != 3 {
            return None;
        }
        digits.push_str(group);
        grouped = true;
    }
    grouped.then_some(digits)
}

/// The name of the last letter of `word` when `word` is an abbreviation
/// read letter by letter; `None` when it is read as a word.
fn spelled_out_last_letter(word: &str) -> Option<&'static str> {
    if is_single_capital(word) || is_dotted_initials(word) {
        return word.chars().last().map(|c| letter_name(lowercase_char(c)));
    }
    if !is_all_capitals(word) {
        return None;
    }
    let letters: Vec<char> = word.chars().map(lowercase_char).collect();
    if letters.len() > 2 && reads_as_word(&letters) {
        return None;
    }
    letters.last().map(|&c| letter_name(c))
}

/// The name a letter is read by when spelled out, as TDK gives it.
const fn letter_name(lower: char) -> &'static str {
    match lower {
        'a' | 'â' => "a",
        'b' => "be",
        'c' => "ce",
        'ç' => "çe",
        'd' => "de",
        'e' => "e",
        'f' => "fe",
        'g' => "ge",
        'ğ' => "yumuşak ge",
        'h' => "he",
        'ı' => "ı",
        'i' | 'î' => "i",
        'j' => "je",
        'k' => "ke",
        'l' => "le",
        'm' => "me",
        'n' => "ne",
        'o' => "o",
        'ö' => "ö",
        'p' => "pe",
        'q' => "kü",
        'r' => "re",
        's' => "se",
        'ş' => "şe",
        't' => "te",
        'u' | 'û' => "u",
        'ü' => "ü",
        'v' => "ve",
        'w' => "çift ve",
        'x' => "iks",
        'y' => "ye",
        'z' => "ze",
        _ => "",
    }
}

/// The consonant pairs a Turkish syllable may end in.
const FINAL_CLUSTERS: [&str; 30] = [
    "ft", "ht", "ks", "lç", "lf", "lk", "lm", "lp", "ls", "lt", "mp", "nç", "nk", "ns", "nt", "nz",
    "rç", "rf", "rk", "rm", "rp", "rs", "rş", "rt", "rz", "sk", "st", "şk", "şt", "vk",
];

/// Whether `letters` (lower case) can be read as a Turkish word: every
/// syllable has one vowel, at most one consonant before it and at most two
/// after it, two only as a cluster Turkish allows.
fn reads_as_word(letters: &[char]) -> bool {
    if !letters.iter().any(|&c| is_vowel(c)) || !letters.iter().all(|&c| is_letter(c)) {
        return false;
    }
    let mut start = 0;
    let mut i = 1;
    while i <= letters.len() {
        let boundary = i == letters.len() || {
            let here = letters.get(i).copied();
            let before = letters.get(i.wrapping_sub(1)).copied();
            let next = letters.get(i.saturating_add(1)).copied();
            match (before, here, next) {
                (Some(b), Some(h), _) if is_vowel(b) && is_vowel(h) => true,
                (_, Some(h), Some(n)) if is_consonant(h) && is_vowel(n) => true,
                _ => false,
            }
        };
        if boundary {
            if !is_syllable(letters.get(start..i).unwrap_or(&[])) {
                return false;
            }
            start = i;
        }
        i = i.saturating_add(1);
    }
    true
}

/// Whether `syllable` is (C)V(C)(C) with an allowed final cluster.
fn is_syllable(syllable: &[char]) -> bool {
    let onset = syllable.iter().take_while(|&&c| is_consonant(c)).count();
    let coda = syllable
        .iter()
        .rev()
        .take_while(|&&c| is_consonant(c))
        .count();
    let nucleus = syllable.len().saturating_sub(onset).saturating_sub(coda);
    if onset > 1 || nucleus != 1 || coda > 2 {
        return false;
    }
    if coda == 2 {
        let cluster: String = syllable
            .iter()
            .skip(syllable.len().saturating_sub(2))
            .collect();
        return FINAL_CLUSTERS.contains(&cluster.as_str());
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proper_nouns_take_every_suffix() {
        let expected = [
            (
                "Ayşe",
                [
                    "Ayşeler",
                    "Ayşe'nin",
                    "Ayşe'ye",
                    "Ayşe'de",
                    "Ayşe'den",
                    "Ayşe'yi",
                    "Ayşe'yle",
                    "Ayşe'deki",
                ],
            ),
            (
                "Furkan",
                [
                    "Furkanlar",
                    "Furkan'ın",
                    "Furkan'a",
                    "Furkan'da",
                    "Furkan'dan",
                    "Furkan'ı",
                    "Furkan'la",
                    "Furkan'daki",
                ],
            ),
            (
                "İstanbul",
                [
                    "İstanbullar",
                    "İstanbul'un",
                    "İstanbul'a",
                    "İstanbul'da",
                    "İstanbul'dan",
                    "İstanbul'u",
                    "İstanbul'la",
                    "İstanbul'daki",
                ],
            ),
            (
                "Köln",
                [
                    "Kölnler",
                    "Köln'ün",
                    "Köln'e",
                    "Köln'de",
                    "Köln'den",
                    "Köln'ü",
                    "Köln'le",
                    "Köln'deki",
                ],
            ),
            (
                "Sinop",
                [
                    "Sinoplar",
                    "Sinop'un",
                    "Sinop'a",
                    "Sinop'ta",
                    "Sinop'tan",
                    "Sinop'u",
                    "Sinop'la",
                    "Sinop'taki",
                ],
            ),
            (
                "Zonguldak",
                [
                    "Zonguldaklar",
                    "Zonguldak'ın",
                    "Zonguldak'a",
                    "Zonguldak'ta",
                    "Zonguldak'tan",
                    "Zonguldak'ı",
                    "Zonguldak'la",
                    "Zonguldak'taki",
                ],
            ),
            (
                "Ülkü",
                [
                    "Ülküler",
                    "Ülkü'nün",
                    "Ülkü'ye",
                    "Ülkü'de",
                    "Ülkü'den",
                    "Ülkü'yü",
                    "Ülkü'yle",
                    "Ülkü'deki",
                ],
            ),
            (
                "Bolu",
                [
                    "Bolular",
                    "Bolu'nun",
                    "Bolu'ya",
                    "Bolu'da",
                    "Bolu'dan",
                    "Bolu'yu",
                    "Bolu'yla",
                    "Bolu'daki",
                ],
            ),
        ];
        let suffixes = [
            Suffix::Plural,
            Suffix::Genitive,
            Suffix::Dative,
            Suffix::Locative,
            Suffix::Ablative,
            Suffix::Accusative,
            Suffix::Instrumental,
            Suffix::LocativeRelative,
        ];
        for (word, forms) in expected {
            for (suffix, form) in suffixes.iter().zip(forms) {
                assert_eq!(attach(word, *suffix), form, "{word} {suffix:?}");
            }
        }
    }

    #[test]
    fn numbers_sound_like_their_words() {
        let expected = [
            ("1", "1'e", "1'de"),
            ("2", "2'ye", "2'de"),
            ("3", "3'e", "3'te"),
            ("4", "4'e", "4'te"),
            ("5", "5'e", "5'te"),
            ("6", "6'ya", "6'da"),
            ("7", "7'ye", "7'de"),
            ("8", "8'e", "8'de"),
            ("9", "9'a", "9'da"),
            ("10", "10'a", "10'da"),
            ("20", "20'ye", "20'de"),
            ("40", "40'a", "40'ta"),
            ("60", "60'a", "60'ta"),
            ("100", "100'e", "100'de"),
            ("1000", "1000'e", "1000'de"),
            ("2026", "2026'ya", "2026'da"),
            ("1000000", "1000000'a", "1000000'da"),
            ("%50", "%50'ye", "%50'de"),
            ("2026/101", "2026/101'e", "2026/101'de"),
            ("0", "0'a", "0'da"),
        ];
        for (word, dative, locative) in expected {
            assert_eq!(attach(word, Suffix::Dative), dative, "{word}");
            assert_eq!(attach(word, Suffix::Locative), locative, "{word}");
        }
        assert_eq!(attach("2000", Suffix::Plural), "2000'ler");
        // Beyond u64, the last eighteen digits carry the last word.
        assert_eq!(
            attach("100000000000000000000005", Suffix::Dative),
            "100000000000000000000005'e"
        );
    }

    #[test]
    fn a_number_grouped_in_thousands_is_read_as_one_number() {
        let expected = [
            ("1.000", "1.000'e", "1.000'de"),
            ("10.000", "10.000'e", "10.000'de"),
            ("250.000", "250.000'e", "250.000'de"),
            ("1.500", "1.500'e", "1.500'de"),
            ("1.000.000", "1.000.000'a", "1.000.000'da"),
            ("3.000.000.000", "3.000.000.000'a", "3.000.000.000'da"),
            // A decimal comma ends the number: "bin virgül beş".
            ("1.000,5", "1.000,5'e", "1.000,5'te"),
            ("2,50", "2,50'ye", "2,50'de"),
            // Dots that do not group thousands are not a grouping.
            ("25.07.2026", "25.07.2026'ya", "25.07.2026'da"),
            ("E.2026", "E.2026'ya", "E.2026'da"),
            ("1234.000", "1234.000'a", "1234.000'da"),
            ("1.00", "1.00'a", "1.00'da"),
            ("Kat 1.000", "Kat 1.000'e", "Kat 1.000'de"),
        ];
        for (word, dative, locative) in expected {
            assert_eq!(attach(word, Suffix::Dative), dative, "{word}");
            assert_eq!(attach(word, Suffix::Locative), locative, "{word}");
        }
        assert_eq!(attach("1.000", Suffix::Plural), "1.000'ler");
        assert_eq!(last_number("1.000"), "1000");
        assert_eq!(last_number("x.1.000"), "1000");
        assert_eq!(last_number("25.07.2026"), "2026");
        assert_eq!(last_number(""), "");
        assert_eq!(thousands_grouped("1000"), None);
        assert_eq!(thousands_grouped("1.0000"), None);
        assert_eq!(thousands_grouped(""), None);
    }

    #[test]
    fn a_number_after_a_character_of_several_bytes_is_read_whole() {
        let expected = [
            ("2026–2027", "2026–2027'ye", "2026–2027'de"),
            ("1990–2000", "1990–2000'e", "1990–2000'de"),
            ("№5", "№5'e", "№5'te"),
            ("Kat\u{2011}3", "Kat\u{2011}3'e", "Kat\u{2011}3'te"),
            ("Çağrı 6", "Çağrı 6'ya", "Çağrı 6'da"),
        ];
        for (word, dative, locative) in expected {
            assert_eq!(attach(word, Suffix::Dative), dative, "{word}");
            assert_eq!(attach(word, Suffix::Locative), locative, "{word}");
        }
    }

    #[test]
    fn abbreviations_are_spelled_out_or_read_as_words() {
        let spelled = [
            ("TBMM", "TBMM'ye", "TBMM'nin"),
            ("ABD", "ABD'ye", "ABD'nin"),
            ("AB", "AB'ye", "AB'nin"),
            ("CHP", "CHP'ye", "CHP'nin"),
            ("TDK", "TDK'ye", "TDK'nin"),
            ("KDV", "KDV'ye", "KDV'nin"),
            ("TL", "TL'ye", "TL'nin"),
            ("PTT", "PTT'ye", "PTT'nin"),
            ("SGK", "SGK'ye", "SGK'nin"),
            ("AKP", "AKP'ye", "AKP'nin"),
            ("ÖSYM", "ÖSYM'ye", "ÖSYM'nin"),
            ("AYM", "AYM'ye", "AYM'nin"),
            ("AVM", "AVM'ye", "AVM'nin"),
            ("USB", "USB'ye", "USB'nin"),
            ("TC", "TC'ye", "TC'nin"),
            ("AŞ", "AŞ'ye", "AŞ'nin"),
            ("ÖTV", "ÖTV'ye", "ÖTV'nin"),
        ];
        for (word, dative, genitive) in spelled {
            assert_eq!(attach(word, Suffix::Dative), dative, "{word}");
            assert_eq!(attach(word, Suffix::Genitive), genitive, "{word}");
        }
        let read_as_words = [
            ("NATO", "NATO'ya", "NATO'nun"),
            ("YÖK", "YÖK'e", "YÖK'ün"),
            ("MEB", "MEB'e", "MEB'in"),
            ("ODTÜ", "ODTÜ'ye", "ODTÜ'nün"),
            ("TÜBİTAK", "TÜBİTAK'a", "TÜBİTAK'ın"),
            ("ASELSAN", "ASELSAN'a", "ASELSAN'ın"),
            ("TOKİ", "TOKİ'ye", "TOKİ'nin"),
            ("KARS", "KARS'a", "KARS'ın"),
            ("VAN", "VAN'a", "VAN'ın"),
            ("TÜİK", "TÜİK'e", "TÜİK'in"),
            ("ANKARA", "ANKARA'ya", "ANKARA'nın"),
            ("KEP", "KEP'e", "KEP'in"),
        ];
        for (word, dative, genitive) in read_as_words {
            assert_eq!(attach(word, Suffix::Dative), dative, "{word}");
            assert_eq!(attach(word, Suffix::Genitive), genitive, "{word}");
        }
        assert_eq!(attach("TBMM", Suffix::Plural), "TBMM'ler");
        assert_eq!(attach("TBMM", Suffix::Locative), "TBMM'de");
        assert_eq!(attach("YÖK", Suffix::Locative), "YÖK'te");
    }

    #[test]
    fn dotted_initials_are_spelled_out() {
        let expected = [
            ("A.Ş.", "A.Ş.'ye", "A.Ş.'nin", "A.Ş.'de"),
            ("A.Ş", "A.Ş'ye", "A.Ş'nin", "A.Ş'de"),
            ("T.C.", "T.C.'ye", "T.C.'nin", "T.C.'de"),
            ("A.B.D.", "A.B.D.'ye", "A.B.D.'nin", "A.B.D.'de"),
            ("K.K.T.C.", "K.K.T.C.'ye", "K.K.T.C.'nin", "K.K.T.C.'de"),
            ("T.İ.", "T.İ.'ye", "T.İ.'nin", "T.İ.'de"),
            ("H.D.", "H.D.'ye", "H.D.'nin", "H.D.'de"),
        ];
        for (word, dative, genitive, locative) in expected {
            assert_eq!(attach(word, Suffix::Dative), dative, "{word}");
            assert_eq!(attach(word, Suffix::Genitive), genitive, "{word}");
            assert_eq!(attach(word, Suffix::Locative), locative, "{word}");
        }
        assert_eq!(attach("A.Ş.", Suffix::Plural), "A.Ş.'ler");
        // Not initials: a dotted word, a single initial, lower case.
        assert!(!is_dotted_initials("Dr"));
        assert!(!is_dotted_initials("K"));
        assert!(!is_dotted_initials("a.ş"));
        assert!(!is_dotted_initials("A..Ş"));
        assert!(!is_dotted_initials("AB.C"));
        assert!(!is_dotted_initials(""));
    }

    #[test]
    fn a_single_capital_letter_is_read_by_its_name() {
        let expected = [
            ("A. Ş.", "A. Ş.'ye", "A. Ş.'nin", "A. Ş.'de"),
            (
                "Deniz Gıda A. Ş.",
                "Deniz Gıda A. Ş.'ye",
                "Deniz Gıda A. Ş.'nin",
                "Deniz Gıda A. Ş.'de",
            ),
            ("T. C.", "T. C.'ye", "T. C.'nin", "T. C.'de"),
            ("Ahmet K.", "Ahmet K.'ye", "Ahmet K.'nin", "Ahmet K.'de"),
            ("Plan B", "Plan B'ye", "Plan B'nin", "Plan B'de"),
            ("B", "B'ye", "B'nin", "B'de"),
            // A vowel's name is the vowel: the same as reading it.
            ("Ayşe A.", "Ayşe A.'ya", "Ayşe A.'nın", "Ayşe A.'da"),
            ("Ali I.", "Ali I.'ya", "Ali I.'nın", "Ali I.'da"),
            ("Ali İ.", "Ali İ.'ye", "Ali İ.'nin", "Ali İ.'de"),
        ];
        for (word, dative, genitive, locative) in expected {
            assert_eq!(attach(word, Suffix::Dative), dative, "{word}");
            assert_eq!(attach(word, Suffix::Genitive), genitive, "{word}");
            assert_eq!(attach(word, Suffix::Locative), locative, "{word}");
        }
        assert_eq!(attach("A. Ş.", Suffix::Plural), "A. Ş.'ler");
        assert_eq!(attach("Plan B", Suffix::Plural), "Plan B'ler");
        assert!(is_single_capital("Ş"));
        assert!(!is_single_capital("ş"));
        assert!(!is_single_capital("AB"));
        assert!(!is_single_capital("5"));
        assert!(!is_single_capital(""));
    }

    #[test]
    fn the_last_word_of_a_name_decides() {
        let expected = [
            (
                "Deniz Gıda A.Ş.",
                "Deniz Gıda A.Ş.'ye",
                "Deniz Gıda A.Ş.'nin",
            ),
            ("Deniz Gıda AŞ", "Deniz Gıda AŞ'ye", "Deniz Gıda AŞ'nin"),
            ("Ali Rıza", "Ali Rıza'ya", "Ali Rıza'nın"),
            ("Ankara Batı", "Ankara Batı'ya", "Ankara Batı'nın"),
        ];
        for (word, dative, genitive) in expected {
            assert_eq!(attach(word, Suffix::Dative), dative, "{word}");
            assert_eq!(attach(word, Suffix::Genitive), genitive, "{word}");
        }
        assert_eq!(
            attach("Deniz Gıda A.Ş.", Suffix::Plural),
            "Deniz Gıda A.Ş.'ler"
        );
        assert_eq!(attach("Ali Rıza", Suffix::Plural), "Ali Rızalar");
        assert_eq!(last_word("Deniz Gıda A.Ş"), "A.Ş");
        assert_eq!(last_word("TBMM"), "TBMM");
        assert_eq!(last_word(""), "");
    }

    #[test]
    fn edge_cases_do_not_panic() {
        assert_eq!(attach("", Suffix::Dative), "'a");
        assert_eq!(attach("...", Suffix::Dative), "...'a");
        assert_eq!(attach("Brr", Suffix::Dative), "Brr'a");
        assert_eq!(attach("Show", Suffix::Dative), "Show'a");
        assert_eq!(attach("Max", Suffix::Locative), "Max'ta");
        assert_eq!(attach("Café", Suffix::Dative), "Café'ye");
        assert_eq!(Harmony::of("").suffix(Suffix::Plural), "lar");
        assert_eq!(suffix_for("TBMM", Suffix::Plural), "ler");
        assert!(!reads_as_word(&[]));
        assert!(!reads_as_word(&['b', 'r', 'r']));
        assert!(reads_as_word(&['t', 'ü', 'r', 'k']));
        assert!(!reads_as_word(&['a', 'b', 'd']));
    }
}
