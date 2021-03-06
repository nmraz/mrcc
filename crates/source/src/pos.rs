pub use text_size::{TextRange as LocalRange, TextSize as LocalOff};

/// An opaque type representing a position in the source code managed by a
/// [`crate::SourceMap`].
///
/// This can be resolved back to file/line/column/expansion information using the appropriate
/// methods on `SourceMap`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SourcePos(u32);

impl SourcePos {
    #[inline]
    pub(crate) fn from_raw(raw: u32) -> Self {
        SourcePos(raw)
    }

    #[inline]
    pub(crate) fn to_raw(self) -> u32 {
        self.0
    }

    /// Returns a new position lying `offset` bytes forward from `self`.
    ///
    /// The position returned can be meaningless if the [source](crate::smap#sources)
    /// containing `self` does not contain at least `offset` more bytes.
    #[inline]
    pub fn offset(self, offset: LocalOff) -> Self {
        SourcePos(self.0 + u32::from(offset))
    }

    /// Returns the distance in bytes between `self` and `rhs`, assuming that `rhs` lies before
    /// `self` in the same source.
    ///
    /// # Panics
    ///
    /// Panics if `rhs` lies after `self`.
    #[inline]
    pub fn offset_from(self, rhs: SourcePos) -> LocalOff {
        assert!(rhs <= self);
        (self.to_raw() - rhs.to_raw()).into()
    }
}

/// Represents a contiguous byte range within a single [source](crate::smap#sources).
///
/// Contrast with [`FragmentedSourceRange`], which can represent ranges whose endpoints lie within
/// different sources (such as macro expansions). Generally, `SourceRange` is preferred when
/// referring to an atomic run of text (i.e. a single token), while `FragmentedSourceRange` is more
/// flexible and can represent composite structures whose tokens come from different sources.
///
/// `SourceRange` is also useful when displaying diagnostics, where one wants to indicate actual
/// ranges in the source code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceRange(SourcePos, LocalOff);

impl SourceRange {
    /// Creates a new range starting at `begin` and covering `len` bytes.
    #[inline]
    pub fn new(begin: SourcePos, len: LocalOff) -> Self {
        SourceRange(begin, len)
    }

    /// Returns the start position of the range.
    #[inline]
    pub fn start(self) -> SourcePos {
        self.0
    }

    /// Returns the length of the range.
    #[inline]
    pub fn len(self) -> LocalOff {
        self.1
    }

    /// Returns `true` if `self` covers 0 bytes.
    #[inline]
    pub fn is_empty(self) -> bool {
        self.1 == 0.into()
    }

    /// Returns a position just past the end of the range.
    ///
    /// The range is empty iff `end() == start()`.
    #[inline]
    pub fn end(self) -> SourcePos {
        self.start().offset(self.len())
    }

    /// Returns a position `off` (zero-based) bytes into the range.
    ///
    /// # Panics
    ///
    /// Panics if the range would not contain the returned position.
    #[inline]
    pub fn subpos(self, off: LocalOff) -> SourcePos {
        assert!(off < self.len());
        self.start().offset(off)
    }

    /// Returns a subrange corresponding to `local_range`.
    ///
    /// # Panics
    ///
    /// Panics if the range would not contain the returned subrange.
    #[inline]
    pub fn subrange(self, local_range: LocalRange) -> SourceRange {
        assert!(local_range.end() <= self.len());
        SourceRange::new(self.start().offset(local_range.start()), local_range.len())
    }

    /// Returns the local offset that `pos` occupies within this range, or `None` if it does not lie
    /// within it.
    #[inline]
    pub fn local_off(self, pos: SourcePos) -> Option<LocalOff> {
        if !(self.start() <= pos && pos < self.end()) {
            return None;
        }

        Some(pos.offset_from(self.0))
    }

    /// Returns the local range that `other` occupies within this range, or `None` if it does not
    /// lie within it.
    #[inline]
    pub fn local_range(self, other: SourceRange) -> Option<LocalRange> {
        if !(self.start() <= other.start() && other.end() <= self.end()) {
            return None;
        }

        Some(LocalRange::at(
            other.start().offset_from(self.start()),
            other.len(),
        ))
    }
}

/// Converts a position to an empty range around it.
impl From<SourcePos> for SourceRange {
    #[inline]
    fn from(pos: SourcePos) -> Self {
        Self::new(pos, 0.into())
    }
}

/// Represents a range whose endpoints may lie in different
/// [sources](crate::smap#sources).
///
/// Reusing the example discussed in the source map documentation, consider the following code:
///
/// ```c
/// #define A (2 + 3)
/// int x = A + 1;
/// ```
///
/// No [`SourceRange`] can accurately cover the expression `A + 1`, as its
/// left endpoint lies within the expansion of `A` while its right endpoint lies within the
/// surrounding file. For this reason, `FragmentedSourceRange` is a better fit for representing
/// multi-token structures which may contain arbitrarily complex macro expansions.
///
/// [`crate::SourceMap::get_unfragmented_range()`]
/// can be used to try to convert a fragmented range to a contiguous range covering both its
/// endpoints (possibly after macro expansion).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FragmentedSourceRange {
    /// The starting position of the range.
    pub start: SourcePos,
    /// A position past the end of the range.
    pub end: SourcePos,
}

impl FragmentedSourceRange {
    /// Creates a new range with the specified endpoints.
    #[inline]
    pub fn new(start: SourcePos, end: SourcePos) -> Self {
        FragmentedSourceRange { start, end }
    }
}

/// Converts a position to a degenerate fragmented range around it.
impl From<SourcePos> for FragmentedSourceRange {
    #[inline]
    fn from(pos: SourcePos) -> Self {
        Self::new(pos, pos)
    }
}

/// Converts a `SourceRange` to a fragmented range with the same endpoints.
impl From<SourceRange> for FragmentedSourceRange {
    #[inline]
    fn from(range: SourceRange) -> Self {
        Self::new(range.start(), range.end())
    }
}

/// Represents a simple line-column number pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct LineCol {
    /// Zero-based line number.
    pub line: u32,
    /// Zero-based column number.
    pub col: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_range_half_open() {
        let start = SourcePos::from_raw(0);
        let range = SourceRange::new(start, 5.into());
        assert!(range.local_off(start) == Some(0.into()));
        assert!(range.local_off(start.offset(4.into())) == Some(4.into()));
        assert!(range.local_off(start.offset(5.into())).is_none());
    }

    #[test]
    fn source_range_contains_range() {
        let start = SourcePos::from_raw(16);
        let range = SourceRange::new(start.offset(1.into()), 20.into());
        assert!(range.local_range(range) == Some(LocalRange::up_to(20.into())));

        let local_range = LocalRange::at(5.into(), 7.into());
        assert!(range.local_range(range.subrange(local_range)) == Some(local_range));
        assert!(range
            .local_range(SourceRange::new(start, 5.into()))
            .is_none());
        assert!(range
            .local_range(SourceRange::new(start.offset(6.into()), 20.into()))
            .is_none());
    }
}
