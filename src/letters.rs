//! The Turkish alphabet: its 29 letters, which are vowels and which are
//! consonants, and the one-to-one case mapping of a single letter.
//!
//! Vowel harmony and the voicing of suffixes ([`suffix`](crate::suffix))
//! read these tables. The circumflexed `â î û`, which Turkish writes in a
//! few loanwords, count as the vowels they are written over.

/// The 29 letters of the Turkish alphabet in lower case, in alphabetical
/// order.
///
/// ```
/// use abece::letters::ALPHABET;
///
/// assert_eq!(ALPHABET.len(), 29);
/// assert_eq!(ALPHABET.iter().collect::<String>(), "abcçdefgğhıijklmnoöprsştuüvyz");
/// ```
pub const ALPHABET: [char; 29] = [
    'a', 'b', 'c', 'ç', 'd', 'e', 'f', 'g', 'ğ', 'h', 'ı', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'ö',
    'p', 'r', 's', 'ş', 't', 'u', 'ü', 'v', 'y', 'z',
];

/// The 29 letters in upper case, in the same order as [`ALPHABET`].
///
/// ```
/// use abece::letters::{ALPHABET, ALPHABET_UPPER};
///
/// assert_eq!(ALPHABET_UPPER.iter().collect::<String>(), "ABCÇDEFGĞHIİJKLMNOÖPRSŞTUÜVYZ");
/// assert_eq!(ALPHABET_UPPER[10], 'I');
/// assert_eq!(ALPHABET[10], 'ı');
/// ```
pub const ALPHABET_UPPER: [char; 29] = [
    'A', 'B', 'C', 'Ç', 'D', 'E', 'F', 'G', 'Ğ', 'H', 'I', 'İ', 'J', 'K', 'L', 'M', 'N', 'O', 'Ö',
    'P', 'R', 'S', 'Ş', 'T', 'U', 'Ü', 'V', 'Y', 'Z',
];

/// Whether `c` is one of the 29 letters, in either case. `q`, `w` and `x`
/// are not; neither are accented letters other than `â î û`.
///
/// ```
/// use abece::letters::is_letter;
///
/// assert!(is_letter('ğ') && is_letter('Ğ') && is_letter('I') && is_letter('â'));
/// assert!(!is_letter('w') && !is_letter('é') && !is_letter('3'));
/// ```
#[must_use]
pub const fn is_letter(c: char) -> bool {
    alphabet_index(c).is_some()
}

/// The position of `c` in [`ALPHABET`], in either case, or `None` for any
/// other character. `â î û` take the places of `a i u`.
///
/// ```
/// use abece::letters::alphabet_index;
///
/// assert_eq!(alphabet_index('a'), Some(0));
/// assert_eq!(alphabet_index('Ç'), Some(3));
/// assert_eq!(alphabet_index('ı'), Some(10));
/// assert_eq!(alphabet_index('İ'), Some(11));
/// assert_eq!(alphabet_index('â'), Some(0));
/// assert_eq!(alphabet_index('w'), None);
/// ```
#[must_use]
pub const fn alphabet_index(c: char) -> Option<usize> {
    Some(match c {
        'a' | 'A' | 'â' | 'Â' => 0,
        'b' | 'B' => 1,
        'c' | 'C' => 2,
        'ç' | 'Ç' => 3,
        'd' | 'D' => 4,
        'e' | 'E' => 5,
        'f' | 'F' => 6,
        'g' | 'G' => 7,
        'ğ' | 'Ğ' => 8,
        'h' | 'H' => 9,
        'ı' | 'I' => 10,
        'i' | 'İ' | 'î' | 'Î' => 11,
        'j' | 'J' => 12,
        'k' | 'K' => 13,
        'l' | 'L' => 14,
        'm' | 'M' => 15,
        'n' | 'N' => 16,
        'o' | 'O' => 17,
        'ö' | 'Ö' => 18,
        'p' | 'P' => 19,
        'r' | 'R' => 20,
        's' | 'S' => 21,
        'ş' | 'Ş' => 22,
        't' | 'T' => 23,
        'u' | 'U' | 'û' | 'Û' => 24,
        'ü' | 'Ü' => 25,
        'v' | 'V' => 26,
        'y' | 'Y' => 27,
        'z' | 'Z' => 28,
        _ => return None,
    })
}

/// Whether `c` is one of the eight vowels `a e ı i o u ö ü`, in either
/// case, or one of the circumflexed `â î û`.
///
/// ```
/// use abece::letters::is_vowel;
///
/// assert!("aeıioöuüAEIİOÖUÜâîû".chars().all(is_vowel));
/// assert!(!"bcçdfgğhjklmnprsştvyz".chars().any(is_vowel));
/// ```
#[must_use]
pub const fn is_vowel(c: char) -> bool {
    matches!(
        c,
        'a' | 'e'
            | 'ı'
            | 'i'
            | 'o'
            | 'ö'
            | 'u'
            | 'ü'
            | 'A'
            | 'E'
            | 'I'
            | 'İ'
            | 'O'
            | 'Ö'
            | 'U'
            | 'Ü'
            | 'â'
            | 'î'
            | 'û'
            | 'Â'
            | 'Î'
            | 'Û'
    )
}

/// Whether `c` is a letter of the alphabet that is not a vowel.
///
/// ```
/// use abece::letters::is_consonant;
///
/// assert!(is_consonant('ğ') && is_consonant('Ş'));
/// assert!(!is_consonant('a') && !is_consonant('w') && !is_consonant('-'));
/// ```
#[must_use]
pub const fn is_consonant(c: char) -> bool {
    is_letter(c) && !is_vowel(c)
}

/// Whether `c` is a back vowel, `a ı o u` (and `â û`): the vowels after
/// which a suffix takes its back form (`-lar`, `-da`).
///
/// ```
/// use abece::letters::is_back_vowel;
///
/// assert!("aıouAIOUâû".chars().all(is_back_vowel));
/// assert!(!"eiöüEİÖÜî".chars().any(is_back_vowel));
/// ```
#[must_use]
pub const fn is_back_vowel(c: char) -> bool {
    matches!(
        c,
        'a' | 'ı' | 'o' | 'u' | 'A' | 'I' | 'O' | 'U' | 'â' | 'û' | 'Â' | 'Û'
    )
}

/// Whether `c` is a front vowel, `e i ö ü` (and `î`): the vowels after
/// which a suffix takes its front form (`-ler`, `-de`).
///
/// ```
/// use abece::letters::is_front_vowel;
///
/// assert!("eiöüEİÖÜî".chars().all(is_front_vowel));
/// assert!(!is_front_vowel('a'));
/// ```
#[must_use]
pub const fn is_front_vowel(c: char) -> bool {
    is_vowel(c) && !is_back_vowel(c)
}

/// Whether `c` is a rounded vowel, `o u ö ü` (and `û`): the vowels after
/// which a high suffix vowel is `u` or `ü` rather than `ı` or `i`.
///
/// ```
/// use abece::letters::is_rounded_vowel;
///
/// assert!("ouöüOUÖÜû".chars().all(is_rounded_vowel));
/// assert!(!"aeıiAEIİâî".chars().any(is_rounded_vowel));
/// ```
#[must_use]
pub const fn is_rounded_vowel(c: char) -> bool {
    matches!(c, 'o' | 'u' | 'ö' | 'ü' | 'O' | 'U' | 'Ö' | 'Ü' | 'û' | 'Û')
}

/// Whether `c` is one of the voiceless consonants `f s t k ç ş h p`
/// ("fıstıkçı şahap"), after which `d` at the start of a suffix hardens to
/// `t` (`-ta`, `-ten`).
///
/// ```
/// use abece::letters::is_voiceless_consonant;
///
/// assert!("fstkçşhpFSTKÇŞHP".chars().all(is_voiceless_consonant));
/// assert!(!"bcdgğjlmnrvyz".chars().any(is_voiceless_consonant));
/// ```
#[must_use]
pub const fn is_voiceless_consonant(c: char) -> bool {
    matches!(
        c,
        'f' | 's'
            | 't'
            | 'k'
            | 'ç'
            | 'ş'
            | 'h'
            | 'p'
            | 'F'
            | 'S'
            | 'T'
            | 'K'
            | 'Ç'
            | 'Ş'
            | 'H'
            | 'P'
    )
}

/// The Turkish lower case of one character: `I` is `ı`, `İ` is `i`, and
/// every other character takes the first character of its Unicode lower
/// case (which is the whole of it for every letter of a Latin alphabet).
///
/// ```
/// use abece::letters::lowercase_char;
///
/// assert_eq!(lowercase_char('I'), 'ı');
/// assert_eq!(lowercase_char('İ'), 'i');
/// assert_eq!(lowercase_char('Ş'), 'ş');
/// assert_eq!(lowercase_char('É'), 'é');
/// assert_eq!(lowercase_char('3'), '3');
/// ```
#[must_use]
pub fn lowercase_char(c: char) -> char {
    match c {
        'I' => 'ı',
        'İ' => 'i',
        _ => c.to_lowercase().next().unwrap_or(c),
    }
}

/// The Turkish upper case of one character: `i` is `İ`, `ı` is `I`, and
/// every other character takes the first character of its Unicode upper
/// case (`ß` gives `S`; use [`uppercase`](crate::uppercase) for text).
///
/// ```
/// use abece::letters::uppercase_char;
///
/// assert_eq!(uppercase_char('i'), 'İ');
/// assert_eq!(uppercase_char('ı'), 'I');
/// assert_eq!(uppercase_char('ğ'), 'Ğ');
/// assert_eq!(uppercase_char('!'), '!');
/// ```
#[must_use]
pub fn uppercase_char(c: char) -> char {
    match c {
        'i' => 'İ',
        'ı' => 'I',
        _ => c.to_uppercase().next().unwrap_or(c),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_tables_agree() {
        for (index, (&lower, &upper)) in ALPHABET.iter().zip(ALPHABET_UPPER.iter()).enumerate() {
            assert_eq!(alphabet_index(lower), Some(index), "{lower}");
            assert_eq!(alphabet_index(upper), Some(index), "{upper}");
            assert_eq!(lowercase_char(upper), lower);
            assert_eq!(uppercase_char(lower), upper);
            assert!(is_letter(lower) && is_letter(upper));
            assert_ne!(is_vowel(lower), is_consonant(lower), "{lower}");
            if is_vowel(lower) {
                assert_ne!(is_back_vowel(lower), is_front_vowel(lower), "{lower}");
            }
        }
        assert_eq!(ALPHABET.iter().filter(|&&c| is_vowel(c)).count(), 8);
        assert_eq!(
            ALPHABET
                .iter()
                .filter(|&&c| is_voiceless_consonant(c))
                .count(),
            8
        );
    }
}
