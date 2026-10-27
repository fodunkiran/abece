//! Numbers in words: "iki bin yirmi altı", "birinci".

use alloc::string::String;

use crate::suffix::Harmony;

/// The ones, `1` to `9`.
const ONES: [&str; 10] = [
    "", "bir", "iki", "üç", "dört", "beş", "altı", "yedi", "sekiz", "dokuz",
];

/// The tens, `10` to `90`.
const TENS: [&str; 10] = [
    "", "on", "yirmi", "otuz", "kırk", "elli", "altmış", "yetmiş", "seksen", "doksan",
];

/// The powers of a thousand, up to the one `u64` reaches.
const SCALES: [&str; 7] = [
    "",
    "bin",
    "milyon",
    "milyar",
    "trilyon",
    "katrilyon",
    "kentilyon",
];

/// `n` written out in words.
///
/// "bir" is dropped before "yüz" and "bin" ("yüz", "bin iki yüz"), as
/// Turkish does, and kept before the larger scales ("bir milyon").
///
/// ```
/// use abece::number::to_words;
///
/// assert_eq!(to_words(0), "sıfır");
/// assert_eq!(to_words(17), "on yedi");
/// assert_eq!(to_words(100), "yüz");
/// assert_eq!(to_words(1_000), "bin");
/// assert_eq!(to_words(2_026), "iki bin yirmi altı");
/// assert_eq!(to_words(1_250_000), "bir milyon iki yüz elli bin");
/// assert_eq!(to_words(u64::MAX), "on sekiz kentilyon dört yüz kırk altı katrilyon yedi yüz kırk dört trilyon yetmiş üç milyar yedi yüz dokuz milyon beş yüz elli bir bin altı yüz on beş");
/// ```
#[must_use]
pub fn to_words(n: u64) -> String {
    if n == 0 {
        return String::from("sıfır");
    }
    // The groups of three digits, lowest first, one per scale word.
    let mut groups = [0_u64; SCALES.len()];
    let mut rest = n;
    for group in &mut groups {
        *group = rest % 1_000;
        rest /= 1_000;
    }
    let mut out = String::new();
    for (scale, group) in SCALES.iter().zip(groups).rev() {
        if group == 0 {
            continue;
        }
        if !out.is_empty() {
            out.push(' ');
        }
        if group == 1 && *scale == "bin" {
            // "bin", never "bir bin".
            out.push_str(scale);
            continue;
        }
        push_group(group, &mut out);
        if !scale.is_empty() {
            out.push(' ');
            out.push_str(scale);
        }
    }
    out
}

/// `n` written out in words, with "eksi" before a negative number.
///
/// ```
/// use abece::number::to_words_signed;
///
/// assert_eq!(to_words_signed(-42), "eksi kırk iki");
/// assert_eq!(to_words_signed(7), "yedi");
/// assert_eq!(to_words_signed(i64::MIN), "eksi dokuz kentilyon iki yüz yirmi üç katrilyon üç yüz yetmiş iki trilyon otuz altı milyar sekiz yüz elli dört milyon yedi yüz yetmiş beş bin sekiz yüz sekiz");
/// ```
#[must_use]
pub fn to_words_signed(n: i64) -> String {
    if n < 0 {
        let mut out = String::from("eksi ");
        out.push_str(&to_words(n.unsigned_abs()));
        out
    } else {
        to_words(n.unsigned_abs())
    }
}

/// Appends the words of a group of three digits, `1` to `999`.
fn push_group(group: u64, out: &mut String) {
    let hundreds = group / 100;
    let tens = group / 10 % 10;
    let ones = group % 10;
    let parts = if hundreds > 1 {
        [
            word(&ONES, hundreds),
            Some("yüz"),
            word(&TENS, tens),
            word(&ONES, ones),
        ]
    } else if hundreds == 1 {
        [Some("yüz"), word(&TENS, tens), word(&ONES, ones), None]
    } else {
        [word(&TENS, tens), word(&ONES, ones), None, None]
    };
    let mut first = true;
    for part in parts.into_iter().flatten() {
        if !first {
            out.push(' ');
        }
        out.push_str(part);
        first = false;
    }
}

/// The word of a digit from a table, or `None` for zero.
fn word(table: &[&'static str; 10], digit: u64) -> Option<&'static str> {
    usize::try_from(digit)
        .ok()
        .and_then(|index| table.get(index))
        .copied()
        .filter(|word| !word.is_empty())
}

/// `n` as an ordinal in words: "birinci", "ikinci", "dördüncü", "yirminci".
///
/// The suffix follows vowel harmony and the final sound of the last word,
/// with the one irregular stem, "dört" to "dördüncü".
///
/// ```
/// use abece::number::ordinal;
///
/// assert_eq!(ordinal(1), "birinci");
/// assert_eq!(ordinal(3), "üçüncü");
/// assert_eq!(ordinal(4), "dördüncü");
/// assert_eq!(ordinal(6), "altıncı");
/// assert_eq!(ordinal(10), "onuncu");
/// assert_eq!(ordinal(100), "yüzüncü");
/// assert_eq!(ordinal(2_026), "iki bin yirmi altıncı");
/// assert_eq!(ordinal(0), "sıfırıncı");
/// ```
#[must_use]
pub fn ordinal(n: u64) -> String {
    let mut words = to_words(n);
    if words.ends_with("dört") {
        words.truncate(words.len().saturating_sub("dört".len()));
        words.push_str("dördüncü");
        return words;
    }
    let harmony = Harmony::of(&words);
    let high = harmony.high_vowel();
    if !harmony.ends_with_vowel() {
        words.push(high);
    }
    words.push('n');
    words.push('c');
    words.push(high);
    words
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_scale_and_the_dropped_bir() {
        assert_eq!(to_words(1), "bir");
        assert_eq!(to_words(10), "on");
        assert_eq!(to_words(11), "on bir");
        assert_eq!(to_words(99), "doksan dokuz");
        assert_eq!(to_words(101), "yüz bir");
        assert_eq!(to_words(110), "yüz on");
        assert_eq!(to_words(200), "iki yüz");
        assert_eq!(to_words(999), "dokuz yüz doksan dokuz");
        assert_eq!(to_words(1_001), "bin bir");
        assert_eq!(to_words(1_100), "bin yüz");
        assert_eq!(to_words(2_000), "iki bin");
        assert_eq!(to_words(21_000), "yirmi bir bin");
        assert_eq!(to_words(100_000), "yüz bin");
        assert_eq!(to_words(1_000_000), "bir milyon");
        assert_eq!(to_words(1_000_001), "bir milyon bir");
        assert_eq!(to_words(1_001_000), "bir milyon bin");
        assert_eq!(to_words(1_000_000_000), "bir milyar");
        assert_eq!(to_words(1_000_000_000_000), "bir trilyon");
        assert_eq!(to_words(1_000_000_000_000_000), "bir katrilyon");
        assert_eq!(to_words(1_000_000_000_000_000_000), "bir kentilyon");
        assert_eq!(to_words_signed(0), "sıfır");
        assert_eq!(to_words_signed(i64::MAX), to_words(i64::MAX.unsigned_abs()));
    }

    #[test]
    fn ordinals_of_the_number_words() {
        let expected = [
            (2, "ikinci"),
            (5, "beşinci"),
            (7, "yedinci"),
            (8, "sekizinci"),
            (9, "dokuzuncu"),
            (20, "yirminci"),
            (30, "otuzuncu"),
            (40, "kırkıncı"),
            (50, "ellinci"),
            (60, "altmışıncı"),
            (70, "yetmişinci"),
            (80, "sekseninci"),
            (90, "doksanıncı"),
            (1_000, "bininci"),
            (1_000_000, "bir milyonuncu"),
            (1_000_000_000, "bir milyarıncı"),
            (1_000_000_000_000, "bir trilyonuncu"),
            (14, "on dördüncü"),
            (24, "yirmi dördüncü"),
        ];
        for (n, words) in expected {
            assert_eq!(ordinal(n), words, "{n}");
        }
    }
}
