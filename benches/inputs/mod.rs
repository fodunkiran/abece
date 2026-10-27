//! The texts every benchmark runs on: a short ASCII title, a short Turkish
//! title, and about four kilobytes of each, so that a per-call cost and a
//! per-byte cost can both be read off.

/// A short title with no byte above ASCII.
pub const ASCII_SHORT: &str = "The Lord of the Rings: The Fellowship of the Ring";

/// A short Turkish title with capitals, the two i's and every Turkish letter.
pub const TURKISH_SHORT: &str = "Çalıkuşu: IŞIK İSTANBUL'DA, Reşat Nuri Güntekin (Öğüt)";

/// About four kilobytes of English prose.
pub fn ascii_long() -> String {
    repeat_to(
        "It was the best of times, it was the worst of times, it was the age of wisdom, \
         it was the age of foolishness, it was the epoch of belief, it was the epoch of \
         incredulity, it was the season of Light, it was the season of Darkness. ",
        4096,
    )
}

/// About four kilobytes of Turkish prose with capitals and every Turkish letter.
pub fn turkish_long() -> String {
    repeat_to(
        "Çalıkuşu Feride, IŞIKLI bir İSTANBUL sabahında Kuşadası'na doğru yola çıktı; \
         Öğretmenlik hayatı Zeyniler Köyü'nde, ĞÜNEŞİN altında, ŞÜKRÜ Bey'in gölgesinde \
         başlayacaktı. Iğdır'dan Ünye'ye, Çanakkale'den Şırnak'a her yer onun. ",
        4096,
    )
}

/// `unit` repeated until the text is at least `len` bytes long.
fn repeat_to(unit: &str, len: usize) -> String {
    let mut out = String::with_capacity(len + unit.len());
    while out.len() < len {
        out.push_str(unit);
    }
    out
}

/// Every input with its name, for a benchmark group.
pub fn all() -> Vec<(&'static str, String)> {
    vec![
        ("ascii-short", ASCII_SHORT.to_string()),
        ("turkish-short", TURKISH_SHORT.to_string()),
        ("ascii-4k", ascii_long()),
        ("turkish-4k", turkish_long()),
    ]
}
