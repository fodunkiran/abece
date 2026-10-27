# Design

What the crate does, in what order, and why each rule is the way it is.
The rustdoc on each function states its contract; this document holds the
reasoning that would otherwise be repeated on every one of them.

## The problem the crate exists for

Turkish has four letters where English has two: `I` is the capital of the
dotless `ı`, and `İ` is the capital of `i`. Unicode's default case mapping,
which `str::to_lowercase` implements, is language-blind: it turns `I` into
`i` and `İ` into `i` + U+0307 (a combining dot above). Any program that
lowers Turkish text that way loses the difference between `ısı` (heat) and
`isi` (soot), indexes `İSTANBUL` under `i̇stanbul`, and sorts `Çalıkuşu`
after `Zeki`.

ICU4X (`icu_casemap`) and `focaccia` map and fold with the Turkic rules.
Neither offers what a program that stores Turkish text needs on top of
that: a search fold with a byte map back to the original spelling, a sort
key a database can index, name tokenizing with joined initials, and the
small jobs (suffixes, numbers, dates) that come with writing Turkish. This
crate does those, and holds the two outputs that get stored (the search
fold and the sort key) stable.

## One pipeline for every fold

`lowercase`, `search_fold`, the `FoldOptions` variants and the sort key all
run the same per-character pipeline on the NFC form of the text:

1. **The dot after i.** A combining dot above (U+0307) directly after a
   plain `i` or an `İ` is dropped. It is the residue of a language-blind
   lower case (`"İ".to_lowercase()` is `i` + U+0307), not a letter; text
   that went through such a lower case before reaching the crate folds the
   same as text that did not. A dot after `ı` is kept: nothing produces it
   by accident, and dropping it would change `ı̇` into a different word.
2. **Other combining marks** are dropped when `strip_marks` is on. After
   NFC, a mark that is still separate is one that has no composed form with
   its base (`s` + U+0331), or one that another mark holds apart from it
   (`i` + U+0307 + U+0300); it is noise from a PDF or an OCR pass more often
   than a letter.
3. **The Turkish lower case.** `I` becomes `ı`, `İ` becomes `i`, and every
   other character takes the Unicode default mapping, character by
   character. The per-character mapping is deliberate: it keeps the byte
   map exact and the output of a character independent of its neighbours,
   which is what lets the comparisons skip a common prefix. The one visible
   consequence is Greek: a final sigma is lowered to `σ`, not `ς`, as
   `str::to_lowercase` would do with its context rule.
4. **The i merge.** With `merge_dotless_i`, `ı` becomes `i`.
5. **Accents.** `Accents::Keep` leaves every letter; `Accents::FoldForeign`
   folds letters that are not Turkish letters of their own (`â` to `a`, `é`
   to `e`, `ß` to `ss`, `ﬁ` to `fi`) through a short table and the
   compatibility decomposition; `Accents::FoldAll` also folds `ç ğ ı ö ş ü`
   to `c g i o s u`.

The order matters. The dot rule runs before lowering because it looks at
the *source* character (`i` or `İ`), and lowering before the merge so that
`I` (which lowers to `ı`) merges too. Accent folding runs on lower-case
characters, which keeps the tables small; the case-preserving `accent_fold`
and `ascii_fold` run a separate pass with upper-case tables.

The output is NFC except in one corner, kept on purpose: a combining mark
that did not compose with the letter before it may compose once the fold
has changed that letter or removed what stood between them. `İ` + U+0308
lowers to `i` + U+0308, which NFC composes into `ï`; `H` + U+0331 lowers
to `h` + U+0331, which composes into `ẖ` (there is no capital with that
mark); `ı` + U+0307 folds, with the merge, to `i` + U+0307, which the dot
rule drops; and the dot rule does the same to the mark after the dot:
`i` + U+0307 + U+0300 loses its dot and leaves `i` + U+0300, which
composes into `ì`. So folding such output again changes it. The fold's
output is stored (a full-text index is built from it, and `ı` + U+0307
folding to `i` + U+0307 is one of its fixed test vectors), so recomposing
now would change the key of such text under the stability promise. No
real text has been seen to contain it; a caller that needs NFC output
calls `nfc` on it. `uppercase` and `titlecase`, which nothing stores, do
recompose after a letter that expands (`ß` + U+0327 is `SŞ` in upper
case); the title case of a ligature followed by a mark (`ﬁ` + U+0307)
still lands on an `i` + U+0307 that the next pass drops, the same corner
from the other side.

### Why `ı` merges in search and `ş` stays

The search fold (`FoldOptions::SEARCH`) merges `ı` into `i` and keeps `ç ğ
ö ş ü`. The asymmetry is the point:

- Text capitalised under a non-Turkish locale ("ISTINAF", "IĞDIR" typed in
  caps lock on an English layout) and queries typed without a Turkish
  keyboard have already lost the difference between the two i's. There is
  no way to recover it, so the index and the query meet by merging. The
  price is that pairs like "sık" and "sik" fold alike; in legal and
  bibliographic text that pair is rare and the gain is large.
- `ş`, `ç`, `ğ`, `ö`, `ü` have no such ambiguity in the *index* side: a text
  written with them keeps them. Folding them would make "şahin" match
  "sahin" at the cost of making every search a little noisier. A user who
  types "sahin" without the Turkish letter gets the ASCII fold
  (`FoldOptions::ASCII`), which is what name similarity uses.

`tests/fold.rs` holds the fixed vectors of this rule and a differential
test against the plain model in `tests/model/mod.rs`.

### The byte map

Folding changes byte offsets: a capital `I` is one byte and the `ı` it
becomes is two; a dropped mark is two bytes and nothing. A match found in
the folded text cannot be cut from the original with its own offsets, so
`lowercase_with_map` and `search_fold_with_map` return an `OffsetMap`: one
entry per byte of the folded text, holding the byte at which the character
that produced it starts in the original, plus a final entry equal to the
original's length. `original_slice(original, start, end)` reads the two
entries and clamps, so it never panics.

A character that produces no output has no entry; it is attributed to the
next character's range only through the end entry. That is why the whole
range maps back to the original less any characters dropped before the
first one that produced output, and why the map does not normalize: the
offsets must point into the text as the caller holds it. A caller that
wants NFC matching normalizes first and keeps the NFC text as the original.

## The sort key

A database compares text by bytes. Teaching it a collation ties the file to
one program (an index on a collation only that program registers makes
every write from another connection fail), so the order is computed in the
program and stored. `sort_key` turns text into a string whose plain byte
order is the Turkish alphabet's order:

- The characters that have a place in the order are listed in `ORDER`:
  the word separator, the ten digits, then the Turkish alphabet with `q w
  x` where Latin puts them (`" 0123456789abcçdefgğhıijklmnoöpqrsştuüvwxyz"`).
- A character's rank is its position in `ORDER`, and rank `r` is stored as
  the character `r` places above `/` (U+002F). The stored characters run in
  one contiguous ascending range, `/`, `0` to `9`, `:` to `Y`, so a byte
  comparison of two keys compares ranks. The digits store as themselves,
  which keeps a key readable in a database browser: `sort_key("Çalıkuşu")`
  is `=:HDGSQS`.
- The text is folded first with the sort key's own rule: lower case by
  `CaseRule` (Turkish: `I` is `ı`; Unicode: `I` is `i`, for a title known to
  be English), and `Accents::FoldForeign`, so that "café" sorts as "cafe"
  while `ç ğ ı ö ş ü` keep their own places.
- Any run of whitespace is one separator, which sorts before every
  character, so "Kar Tanesi" sorts before "Karakol". A separator before the
  first character or after the last would only tell apart text that differs
  by a space, so none is stored there. Other punctuation and all marks are
  dropped.
- A letter or digit of another script keeps its own code point, which is
  above every stored character, so such text sorts after the Latin text in
  code point order, deterministically.

`compare` walks the same key characters for two texts without building
either key (`SortKeyChars`), which is what an in-memory sort wants. ASCII
text takes a byte loop in both.

`tests/sort_key.rs` holds the fixed vectors of every rule above and a
differential test against the plain model in `tests/model/mod.rs`.

## Fast paths

Every public fold checks `str::is_ascii` first (one vectorised pass). ASCII
text is NFC by definition and has no marks or accents, so:

- `lowercase` of ASCII text with no capital is borrowed; with capitals but
  no `I` it is `to_ascii_lowercase`; with an `I` it is a byte loop that
  writes the two bytes of `ı`.
- `search_fold` of ASCII text is exactly `to_ascii_lowercase`, since `I`
  merges to `i`.
- `sort_key` of ASCII text is a byte loop over a rank table.
- `eq_ignore_case` and `cmp_ignore_case` of two ASCII texts compare bytes.

Non-ASCII text goes through the NFC quick check (borrowed when already
composed, which Turkish text almost always is) and then through the
pipeline, which borrows the text when no character changes: the loop runs
in a checking mode until the first change and only then allocates and
copies the prefix.

The comparator and the case-insensitive comparison never allocate for
ordinary text; the sort key's streaming iterator keeps an inline queue of
eight characters and spills to a `Vec` only for a decomposition longer than
that (a handful of Arabic presentation forms).

## What is deliberately not here

- **Consonant softening of common nouns** ("kitap" to "kitabı") and the
  words whose harmony does not follow their spelling ("saat", "Bilal'e"):
  these need a dictionary. `attach` handles proper nouns, numbers and
  abbreviations, where the spelling rules are regular, and `attach_spoken`
  takes the spoken form from the caller for everything else.
- **Stemming, spelling correction, deasciification** ("sahin" to "şahin"):
  all need a lexicon or a model.
- **Collation beyond the alphabet** (numeric ordering of "10" and "2",
  ignoring a leading "The"): a caller's policy, and a change to the stored
  key.
- **Time zones and date arithmetic**: `Date` is a plain year, month and
  day that knows its weekday and its spellings; `jiff` (behind a feature)
  or any other date crate does the rest.
- **Stop words** ("A.Ş.", "Ltd. Şti.") in name matching: a domain list,
  kept by the caller and applied to the crate's words.

## Dependencies

- `unicode-normalization` is the only required dependency: NFC
  composition is the first step of every fold, and the compatibility
  decomposition is what accent folding is built on. `core` has the case
  tables but not the normalization tables. The crate is `no_std` with
  `alloc`, which keeps this crate `no_std` too.
- `serde` (optional) derives `Serialize` and `Deserialize` for the option
  and date types, for programs that keep fold settings or dates in
  configuration. `Date` alone reads its fields by hand, through the same
  check as `Date::from_parts`, so that no deserialized date is invalid.
- `jiff` (optional) converts between this crate's `Date` and `Weekday` and
  jiff's, for programs that keep their dates in jiff.
- `criterion`, `icu_casemap`, `icu_collator`, `icu_locale_core`,
  `focaccia`, `proptest` and `serde_json` are development dependencies:
  the benchmark competitors, the property tests and the serde format
  tests. None reaches a user of the crate.

## Stability

The output of `search_fold` with any `FoldOptions`, and of `sort_key`, is
stable within a major version: both are made to be stored (a full-text
index, an indexed column). A change to either is a breaking change and is
released as one, with the change log naming what moved. The other functions
may gain cases and fix mistakes in minor releases.
