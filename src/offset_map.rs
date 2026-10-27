//! The byte map from a folded text back to the text it was folded from.

use alloc::vec::Vec;
use core::ops::Range;

/// A map from the bytes of a folded text to the bytes of the original.
///
/// Folding changes byte offsets: a capital `I` is one byte and the `ı` it
/// lowers to is two, a dropped combining mark is two bytes and nothing. A
/// match found in the folded text (a search hit) therefore cannot be cut
/// from the original with its own offsets. The map built by
/// [`lowercase_with_map`](crate::lowercase_with_map) and
/// [`search_fold_with_map`](crate::search_fold_with_map) answers for each
/// byte of the folded text which character of the original produced it.
///
/// Entry `i` is the byte at which the character that produced folded byte
/// `i` starts in the original; the last entry is the original's length, so
/// the map has one entry more than the folded text has bytes.
///
/// ```
/// use abece::search_fold_with_map;
///
/// let original = "ESAS NO: 2026/12 İSTANBUL";
/// let (folded, map) = search_fold_with_map(original);
/// let start = folded.find("istanbul").unwrap();
/// let end = start + "istanbul".len();
/// assert_eq!(map.original_slice(original, start, end), "İSTANBUL");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct OffsetMap {
    starts: Vec<usize>,
}

impl OffsetMap {
    pub(crate) fn from_starts(starts: Vec<usize>) -> Self {
        Self { starts }
    }

    /// The slice of `original` that produced folded bytes `start..end`.
    ///
    /// Offsets past the end of the folded text are read as its end, and a
    /// reversed range is empty; the function never panics. `original` must
    /// be the text the map was built from.
    ///
    /// ```
    /// let (folded, map) = abece::lowercase_with_map("KARARI");
    /// assert_eq!(folded, "kararı");
    /// // `ı` is two bytes where `I` was one; the map bridges the difference.
    /// assert_eq!(map.original_slice("KARARI", 0, folded.len()), "KARARI");
    /// assert_eq!(map.original_slice("KARARI", 5, 7), "I");
    /// ```
    #[must_use]
    pub fn original_slice<'a>(&self, original: &'a str, start: usize, end: usize) -> &'a str {
        original
            .get(self.original_range(original, start, end))
            .unwrap_or("")
    }

    /// The byte range of `original` that produced folded bytes `start..end`,
    /// clamped to the original's length; see [`original_slice`](Self::original_slice).
    ///
    /// ```
    /// let (folded, map) = abece::lowercase_with_map("İzmir");
    /// assert_eq!(folded, "izmir");
    /// assert_eq!(map.original_range("İzmir", 0, 1), 0..2);
    /// ```
    #[must_use]
    pub fn original_range(&self, original: &str, start: usize, end: usize) -> Range<usize> {
        let first = self.original_start(start).unwrap_or(original.len());
        let last = self
            .original_start(end)
            .unwrap_or(original.len())
            .max(first);
        first..last
    }

    /// Where the character that produced folded byte `folded_byte` starts in
    /// the original, or `None` past the end of the map.
    ///
    /// ```
    /// let (_, map) = abece::lowercase_with_map("IŞIK");
    /// assert_eq!(map.original_start(2), Some(1)); // the `ş` of `ışık` came from byte 1
    /// assert_eq!(map.original_start(99), None);
    /// ```
    #[must_use]
    pub fn original_start(&self, folded_byte: usize) -> Option<usize> {
        self.starts.get(folded_byte).copied()
    }

    /// The entries, one per folded byte plus the final one.
    ///
    /// ```
    /// let (folded, map) = abece::lowercase_with_map("Ali");
    /// assert_eq!(map.as_slice(), &[0, 1, 2, 3]);
    /// assert_eq!(map.len(), folded.len() + 1);
    /// ```
    #[must_use]
    pub fn as_slice(&self) -> &[usize] {
        &self.starts
    }

    /// The number of entries: the folded text's length in bytes plus one.
    ///
    /// ```
    /// let (folded, map) = abece::lowercase_with_map("IŞIK");
    /// assert_eq!(map.len(), folded.len() + 1);
    /// ```
    #[must_use]
    pub fn len(&self) -> usize {
        self.starts.len()
    }

    /// Whether the map has no entries, which only a default map has: even
    /// the map of an empty text has its final entry.
    ///
    /// ```
    /// assert!(abece::OffsetMap::default().is_empty());
    /// assert!(!abece::lowercase_with_map("").1.is_empty());
    /// ```
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.starts.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn out_of_range_offsets_are_clamped_and_never_panic() {
        let map = OffsetMap::from_starts(alloc::vec![0, 1, 2, 3]);
        assert_eq!(map.original_slice("abc", 1, 2), "b");
        assert_eq!(map.original_slice("abc", 2, 99), "c");
        assert_eq!(map.original_slice("abc", 99, 100), "");
        assert_eq!(map.original_slice("abc", 2, 1), "");
        assert_eq!(map.original_range("abc", 2, 1), 2..2);
        assert_eq!(map.as_slice(), &[0, 1, 2, 3]);
        assert_eq!(map.len(), 4);
    }
}
