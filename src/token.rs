//! Splitting text into words for matching, with the dotted initials of
//! names and company types joined: "A.Ş." is one word, "T.C." is one word.
//!
//! A token is a run of alphanumeric characters (combining marks stay with
//! their letter). Everything else separates tokens, and a token remembers
//! whether a full stop followed it, which is what tells an initial ("A.")
//! from a one-letter word.

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::mem;
use unicode_normalization::char::is_combining_mark;

use crate::fold::{FoldOptions, fold_cow};

/// A word of the text, and whether a full stop came right after it.
///
/// ```
/// use abece::tokenize;
///
/// let tokens: Vec<_> = tokenize("Deniz A.Ş.").collect();
/// assert_eq!(tokens.len(), 3);
/// assert_eq!((tokens[0].text(), tokens[0].is_dotted()), ("Deniz", false));
/// assert_eq!((tokens[1].text(), tokens[1].is_dotted()), ("A", true));
/// assert_eq!((tokens[2].text(), tokens[2].is_dotted()), ("Ş", true));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Token<'a> {
    text: &'a str,
    dotted: bool,
}

impl<'a> Token<'a> {
    /// A token with the given text and dot, for feeding [`join_initials`]
    /// from another tokenizer.
    ///
    /// ```
    /// use abece::{Token, join_initials};
    ///
    /// let tokens = [Token::new("t", true), Token::new("c", true), Token::new("ziraat", false)];
    /// assert_eq!(join_initials(tokens), ["tc", "ziraat"]);
    /// ```
    #[must_use]
    pub const fn new(text: &'a str, dotted: bool) -> Self {
        Self { text, dotted }
    }

    /// The word.
    ///
    /// ```
    /// let token = abece::tokenize("Ali").next().unwrap();
    /// assert_eq!(token.text(), "Ali");
    /// ```
    #[must_use]
    pub const fn text(&self) -> &'a str {
        self.text
    }

    /// Whether a full stop came right after the word.
    ///
    /// ```
    /// let token = abece::tokenize("K. Yılmaz").next().unwrap();
    /// assert!(token.is_dotted());
    /// ```
    #[must_use]
    pub const fn is_dotted(&self) -> bool {
        self.dotted
    }
}

/// The tokens of `text`: runs of alphanumeric characters, in order. Case
/// and accents are left as they are; fold first ([`words`] does) to match.
///
/// ```
/// use abece::tokenize;
///
/// let words: Vec<&str> = tokenize("Ahmet K. Yılmaz-Öz (2026)").map(|t| t.text()).collect();
/// assert_eq!(words, ["Ahmet", "K", "Yılmaz", "Öz", "2026"]);
/// ```
#[must_use]
pub fn tokenize(text: &str) -> Tokens<'_> {
    Tokens {
        text,
        pos: 0,
        drop_apostrophe_suffix: false,
    }
}

/// [`tokenize`] with the suffix after an apostrophe dropped: "Mahkemesi'ne"
/// is the token "Mahkemesi". An apostrophe that starts a token is just a
/// separator.
///
/// ```
/// use abece::tokenize_stems;
///
/// let words: Vec<&str> = tokenize_stems("Ankara 3. Asliye Hukuk Mahkemesi'ne").map(|t| t.text()).collect();
/// assert_eq!(words, ["Ankara", "3", "Asliye", "Hukuk", "Mahkemesi"]);
/// ```
#[must_use]
pub fn tokenize_stems(text: &str) -> Tokens<'_> {
    Tokens {
        text,
        pos: 0,
        drop_apostrophe_suffix: true,
    }
}

/// The iterator [`tokenize`] returns.
#[derive(Debug, Clone)]
pub struct Tokens<'a> {
    text: &'a str,
    pos: usize,
    drop_apostrophe_suffix: bool,
}

/// Whether `c` continues a token.
fn is_token_char(c: char) -> bool {
    c.is_alphanumeric() || is_combining_mark(c)
}

/// Whether `c` is an apostrophe, in any of the forms keyboards and word
/// processors produce.
const fn is_apostrophe(c: char) -> bool {
    matches!(c, '\'' | '’' | '‘' | '`' | '´')
}

impl<'a> Iterator for Tokens<'a> {
    type Item = Token<'a>;

    fn next(&mut self) -> Option<Token<'a>> {
        let rest = self.text.get(self.pos..)?;
        let mut chars = rest.char_indices().peekable();
        // Skip to the start of the next token.
        let start = loop {
            let (offset, c) = chars.next()?;
            if is_token_char(c) {
                break offset;
            }
        };
        // Then to its end.
        let mut end = rest.len();
        let mut after = None;
        for (offset, c) in chars.by_ref() {
            if !is_token_char(c) {
                end = offset;
                after = Some(c);
                break;
            }
        }
        let text = rest.get(start..end).unwrap_or("");
        let mut resume = after.map_or(rest.len(), |c| end.saturating_add(c.len_utf8()));
        if self.drop_apostrophe_suffix && after.is_some_and(is_apostrophe) {
            // The suffix after the apostrophe is skipped; the token is
            // dotted if a full stop follows that suffix.
            after = None;
            resume = rest.len();
            for (offset, c) in chars {
                if !is_token_char(c) {
                    after = Some(c);
                    resume = offset.saturating_add(c.len_utf8());
                    break;
                }
            }
        }
        self.pos = self.pos.saturating_add(resume);
        Some(Token {
            text,
            dotted: after == Some('.'),
        })
    }
}

/// The words of `tokens` with dotted single letters joined into one word:
/// "a. ş." is "as", "t.c" is "tc", "ltd. şti." stays two words. A single
/// letter without a dot starts a new run; a lone initial stays as it is
/// ("ahmet k. yılmaz" keeps its "k").
///
/// ```
/// use abece::{join_initials, tokenize};
///
/// assert_eq!(join_initials(tokenize("deniz a.ş.")), ["deniz", "aş"]);
/// assert_eq!(join_initials(tokenize("t.c. ziraat")), ["tc", "ziraat"]);
/// assert_eq!(join_initials(tokenize("ahmet k. yılmaz")), ["ahmet", "k", "yılmaz"]);
/// assert_eq!(join_initials(tokenize("yargıtay 3. h.d.")), ["yargıtay", "3", "hd"]);
/// ```
#[must_use]
pub fn join_initials<'a>(tokens: impl IntoIterator<Item = Token<'a>>) -> Vec<String> {
    let mut words = Vec::new();
    let mut initials = String::new();
    // A full stop followed the last initial, so the run may continue.
    let mut can_continue = false;
    for token in tokens {
        let mut chars = token.text.chars();
        let is_initial = chars.next().is_some_and(char::is_alphabetic) && chars.next().is_none();
        if is_initial && (initials.is_empty() || can_continue) {
            initials.push_str(token.text);
            can_continue = token.dotted;
            continue;
        }
        if !initials.is_empty() {
            words.push(mem::take(&mut initials));
        }
        if is_initial {
            initials = token.text.to_string();
            can_continue = token.dotted;
        } else {
            words.push(token.text.to_string());
        }
    }
    if !initials.is_empty() {
        words.push(initials);
    }
    words
}

/// How [`words_with`] folds and splits.
///
/// ```
/// use abece::{FoldOptions, WordOptions, words_with};
///
/// let options = WordOptions::new().fold(FoldOptions::SEARCH).join_initials(false);
/// assert_eq!(words_with("Deniz A.Ş.", &options), ["deniz", "a", "ş"]);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct WordOptions {
    fold: FoldOptions,
    join_initials: bool,
    drop_apostrophe_suffix: bool,
}

impl WordOptions {
    /// The defaults of [`words`]: the ASCII fold, initials joined, apostrophe
    /// suffixes kept as words of their own.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            fold: FoldOptions::ASCII,
            join_initials: true,
            drop_apostrophe_suffix: false,
        }
    }

    /// The fold applied before splitting.
    ///
    /// ```
    /// use abece::{FoldOptions, WordOptions, words_with};
    ///
    /// let options = WordOptions::new().fold(FoldOptions::SEARCH);
    /// assert_eq!(words_with("Çağrı IŞIK", &options), ["çağri", "işik"]);
    /// ```
    #[must_use]
    pub const fn fold(mut self, fold: FoldOptions) -> Self {
        self.fold = fold;
        self
    }

    /// Whether dotted single letters are joined ([`join_initials`]).
    ///
    /// ```
    /// use abece::{WordOptions, words_with};
    ///
    /// let options = WordOptions::new().join_initials(false);
    /// assert_eq!(words_with("T.C. Ziraat", &options), ["t", "c", "ziraat"]);
    /// ```
    #[must_use]
    pub const fn join_initials(mut self, join: bool) -> Self {
        self.join_initials = join;
        self
    }

    /// Whether the suffix after an apostrophe is dropped
    /// ([`tokenize_stems`]).
    ///
    /// ```
    /// use abece::{WordOptions, words_with};
    ///
    /// let options = WordOptions::new().drop_apostrophe_suffix(true);
    /// assert_eq!(words_with("Mahkemesi'ne", &options), ["mahkemesi"]);
    /// assert_eq!(words_with("Mahkemesi'ne", &WordOptions::new()), ["mahkemesi", "ne"]);
    /// ```
    #[must_use]
    pub const fn drop_apostrophe_suffix(mut self, drop: bool) -> Self {
        self.drop_apostrophe_suffix = drop;
        self
    }
}

impl Default for WordOptions {
    fn default() -> Self {
        Self::new()
    }
}

/// The words of `text` for matching names: the ASCII fold
/// ([`FoldOptions::ASCII`]), split into tokens, dotted initials joined. The
/// fold for comparing names that may have been typed without a Turkish
/// keyboard or pasted from a PDF with decomposed letters; "Yılmaz Ayşe" and
/// "AYŞE YILMAZ" give the same words in another order.
///
/// ```
/// use abece::words;
///
/// assert_eq!(words("IŞIK İSTANBUL"), ["isik", "istanbul"]);
/// assert_eq!(words("DENİZ GIDA LTD.ŞTİ."), ["deniz", "gida", "ltd", "sti"]);
/// assert_eq!(words("Deniz A.Ş."), ["deniz", "as"]);
/// assert_eq!(words("S\u{327}AHI\u{307}N Kaya"), ["sahin", "kaya"]);
/// assert_eq!(words("  "), Vec::<String>::new());
/// ```
#[must_use]
pub fn words(text: &str) -> Vec<String> {
    words_with(text, &WordOptions::new())
}

/// The words of `text` as `options` say; see [`words`].
///
/// ```
/// use abece::{FoldOptions, WordOptions, words_with};
///
/// let options = WordOptions::new().fold(FoldOptions::SEARCH).drop_apostrophe_suffix(true);
/// assert_eq!(words_with("İstanbul'da Çağrı'nın", &options), ["istanbul", "çağri"]);
/// ```
#[must_use]
pub fn words_with(text: &str, options: &WordOptions) -> Vec<String> {
    let folded = fold_cow(text, &options.fold);
    let tokens = Tokens {
        text: &folded,
        pos: 0,
        drop_apostrophe_suffix: options.drop_apostrophe_suffix,
    };
    if options.join_initials {
        join_initials(tokens)
    } else {
        tokens.map(|token| token.text.to_string()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn texts<'a>(tokens: Tokens<'a>) -> Vec<(&'a str, bool)> {
        tokens.map(|t| (t.text, t.dotted)).collect()
    }

    #[test]
    fn tokens_are_alphanumeric_runs_with_their_dot() {
        assert_eq!(
            texts(tokenize("a.b. c-d e")),
            [
                ("a", true),
                ("b", true),
                ("c", false),
                ("d", false),
                ("e", false)
            ]
        );
        assert_eq!(texts(tokenize("")), []);
        assert_eq!(texts(tokenize(" . ")), []);
        assert_eq!(texts(tokenize("x.")), [("x", true)]);
        assert_eq!(texts(tokenize("Ays\u{331}e")), [("Ays\u{331}e", false)]);
        assert_eq!(
            texts(tokenize("D'Angelo")),
            [("D", false), ("Angelo", false)]
        );
    }

    #[test]
    fn stems_drop_the_suffix_after_an_apostrophe() {
        assert_eq!(
            texts(tokenize_stems("Mahkemesi'ne. Ali")),
            [("Mahkemesi", true), ("Ali", false)]
        );
        assert_eq!(
            texts(tokenize_stems("'ne ali")),
            [("ne", false), ("ali", false)]
        );
        assert_eq!(texts(tokenize_stems("a’b c")), [("a", false), ("c", false)]);
        assert_eq!(texts(tokenize_stems("a'")), [("a", false)]);
        assert_eq!(texts(tokenize_stems("a'b")), [("a", false)]);
    }

    #[test]
    fn initials_join_in_runs() {
        let join = |text: &str| join_initials(tokenize(text));
        assert_eq!(join("a. s."), ["as"]);
        assert_eq!(join("t.c"), ["tc"]);
        assert_eq!(join("a s"), ["a", "s"]);
        assert_eq!(join("a. s b."), ["as", "b"]);
        assert_eq!(join("3. h.d."), ["3", "hd"]);
        assert_eq!(join("k"), ["k"]);
        assert_eq!(join(""), Vec::<String>::new());
    }

    #[test]
    fn words_fold_then_split() {
        assert_eq!(words("Çağrı Öztürk-Şahin"), ["cagri", "ozturk", "sahin"]);
        assert_eq!(words("ABC Ltd. Şti."), ["abc", "ltd", "sti"]);
        assert_eq!(words("T.C. Ziraat"), ["tc", "ziraat"]);
        assert_eq!(words("Ahmet K. Yılmaz"), ["ahmet", "k", "yilmaz"]);
        assert_eq!(words("O\u{308}zturk"), ["ozturk"]);
        assert_eq!(words("Ays\u{331}e"), ["ayse"]);
    }
}
