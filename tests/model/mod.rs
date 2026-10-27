//! Plain models of the folds and the sort key: every rule written the
//! straightforward way, one character at a time, with no fast path and no
//! borrowing. The differential tests in this directory run the crate
//! against them on generated input, so an optimisation that changes an
//! output by a byte fails here. A change to a model is a change to the
//! stable output of the crate.
#![allow(dead_code)]

/// The Turkish lower case and the search fold, with the byte map.
pub mod fold {
    use std::borrow::Cow;
    use unicode_normalization::{IsNormalized, UnicodeNormalization, is_nfc_quick};

    const COMBINING_DOT_ABOVE: char = '\u{307}';

    /// NFC by the quick check, borrowed when composed already.
    pub fn nfc(s: &str) -> Cow<'_, str> {
        match is_nfc_quick(s.chars()) {
            IsNormalized::Yes => Cow::Borrowed(s),
            IsNormalized::No | IsNormalized::Maybe => Cow::Owned(s.nfc().collect()),
        }
    }

    /// NFC, then the Turkish lower case.
    pub fn lowercase(s: &str) -> String {
        fold(&nfc(s), false, None)
    }

    /// NFC, then the search fold: the lower case with `ı` merged into `i`.
    pub fn search_fold(s: &str) -> String {
        fold(&nfc(s), true, None)
    }

    /// The lower case of `s` as given, without NFC, with the byte map.
    pub fn lowercase_with_map(s: &str) -> (String, Vec<usize>) {
        let mut map = Vec::new();
        let lower = fold(s, false, Some(&mut map));
        (lower, map)
    }

    /// The search fold of `s` as given, without NFC, with the byte map.
    pub fn search_fold_with_map(s: &str) -> (String, Vec<usize>) {
        let mut map = Vec::new();
        let folded = fold(s, true, Some(&mut map));
        (folded, map)
    }

    /// One pass: the dot after a plain `i` dropped, `I` to `ı` (or `i`
    /// under the merge), `İ` to `i`, the rest by the default mapping with
    /// `ı` merged where asked; one map entry per output byte.
    fn fold(s: &str, merge: bool, mut map: Option<&mut Vec<usize>>) -> String {
        let mut out = String::new();
        let mut after_plain_i = false;
        for (byte, ch) in s.char_indices() {
            if ch == COMBINING_DOT_ABOVE && after_plain_i {
                after_plain_i = false;
                continue;
            }
            after_plain_i = matches!(ch, 'i' | 'İ');
            match ch {
                'I' => out.push(if merge { 'i' } else { 'ı' }),
                'İ' => out.push('i'),
                _ => {
                    for lower in ch.to_lowercase() {
                        out.push(if merge && lower == 'ı' { 'i' } else { lower });
                    }
                }
            }
            if let Some(map) = map.as_deref_mut() {
                map.resize(out.len(), byte);
            }
        }
        if let Some(map) = map {
            map.push(s.len());
        }
        out
    }

    /// The slice of `original` behind folded bytes `start..end`, clamped.
    pub fn original_slice<'a>(
        original: &'a str,
        map: &[usize],
        start: usize,
        end: usize,
    ) -> &'a str {
        let s = map.get(start).copied().unwrap_or(original.len());
        let e = map.get(end).copied().unwrap_or(original.len()).max(s);
        original.get(s..e).unwrap_or("")
    }

    /// The Turkish upper case, character by character, without NFC.
    pub fn uppercase(s: &str) -> String {
        let mut out = String::new();
        for ch in s.chars() {
            match ch {
                'i' => out.push('İ'),
                'ı' => out.push('I'),
                _ => out.extend(ch.to_uppercase()),
            }
        }
        out
    }
}

/// The name fold: the lower case, the Turkish letters and the circumflexed
/// vowels to ASCII, combining marks dropped, words split on everything
/// else, dotted initials joined.
pub mod names {
    use super::fold::lowercase;

    /// The words joined by one space.
    pub fn fold(text: &str) -> String {
        words(text).join(" ")
    }

    /// The folded words of `text`.
    pub fn words(text: &str) -> Vec<String> {
        let mut tokens: Vec<(String, bool)> = Vec::new();
        let mut current = String::new();
        for c in lowercase(text).chars() {
            if ('\u{300}'..='\u{36f}').contains(&c) {
                continue;
            }
            let c = ascii_letter(c);
            if c.is_alphanumeric() {
                current.push(c);
            } else if !current.is_empty() {
                tokens.push((std::mem::take(&mut current), c == '.'));
            }
        }
        if !current.is_empty() {
            tokens.push((current, false));
        }
        join_initials(tokens)
    }

    fn ascii_letter(c: char) -> char {
        match c {
            'ç' => 'c',
            'ğ' => 'g',
            'ı' | 'î' => 'i',
            'ö' => 'o',
            'ş' => 's',
            'ü' | 'û' => 'u',
            'â' => 'a',
            other => other,
        }
    }

    /// Runs of dotted single letters become one word.
    fn join_initials(tokens: Vec<(String, bool)>) -> Vec<String> {
        let mut words = Vec::new();
        let mut initials = String::new();
        let mut can_continue = false;
        for (text, dotted) in tokens {
            let is_initial = text.chars().count() == 1 && text.chars().all(char::is_alphabetic);
            if is_initial && (initials.is_empty() || can_continue) {
                initials.push_str(&text);
                can_continue = dotted;
                continue;
            }
            if !initials.is_empty() {
                words.push(std::mem::take(&mut initials));
            }
            if is_initial {
                initials = text;
                can_continue = dotted;
            } else {
                words.push(text);
            }
        }
        if !initials.is_empty() {
            words.push(initials);
        }
        words
    }
}

/// The sort key, as docs/DESIGN.md describes it.
pub mod sort_key {
    use unicode_normalization::UnicodeNormalization;
    use unicode_normalization::char::is_combining_mark;

    const ORDER: &str = " 0123456789abcçdefgğhıijklmnoöpqrsştuüvwxyz";
    const FIRST_KEY_CHAR: u32 = 0x2F;
    const OWN_LETTERS: [char; 6] = ['ç', 'ğ', 'ı', 'ö', 'ş', 'ü'];
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

    /// The key under the Turkish case rule.
    pub fn sort_key(text: &str) -> String {
        sort_key_in(text, true)
    }

    /// The key, reading a capital `I` as `ı` when `turkish`.
    pub fn sort_key_in(text: &str, turkish: bool) -> String {
        let mut key = String::new();
        let mut pending_separator = false;
        for c in text.nfc() {
            fold_char(c, turkish, &mut |letter| {
                if letter.is_whitespace() {
                    pending_separator = true;
                    return;
                }
                let Some(stored) = stored_char(letter) else {
                    return;
                };
                if pending_separator && !key.is_empty() {
                    key.push('/');
                }
                pending_separator = false;
                key.push(stored);
            });
        }
        key
    }

    fn stored_char(letter: char) -> Option<char> {
        if let Some(rank) = ORDER.chars().position(|ranked| ranked == letter) {
            return char::from_u32(FIRST_KEY_CHAR + u32::try_from(rank).ok()?);
        }
        letter.is_alphanumeric().then_some(letter)
    }

    fn fold_char(c: char, turkish: bool, emit: &mut impl FnMut(char)) {
        match c {
            'I' if turkish => emit('ı'),
            'İ' => emit('i'),
            _ => c
                .to_lowercase()
                .for_each(|lower| fold_lower_letter(lower, emit)),
        }
    }

    fn fold_lower_letter(lower: char, emit: &mut impl FnMut(char)) {
        if OWN_LETTERS.contains(&lower) {
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
}
