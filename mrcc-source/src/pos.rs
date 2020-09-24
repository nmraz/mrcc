/// An opaque type representing a position in the source code managed by a
/// [`SourceMap`](smap/struct.SourceMap.html).
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
    /// The position returned can be meaningless if the [source](smap/index.html#sources)
    /// containing `self` does not contain at least `offset` more bytes.
    #[inline]
    pub fn offset(self, offset: u32) -> Self {
        SourcePos(self.0 + offset)
    }

    /// Returns the distance in bytes between `self` and `rhs`, assuming that `rhs` lies before
    /// `self` in the same source.
    ///
    /// # Panics
    ///
    /// Panics if `rhs` lies after `self`.
    #[inline]
    pub fn offset_from(self, rhs: SourcePos) -> u32 {
        assert!(rhs <= self);
        self.to_raw() - rhs.to_raw()
    }
}

/// Represents a contiguous byte range within a single [source](smap/index.html#sources).
///
/// Contrast with [`FragmentedSourceRange`](struct.FragmentedSourceRange.html), which can represent
/// ranges whose endpoints lie within different sources (such as macro expansions). Generally,
/// `SourceRange` is preferred when referring to an atomic run of text (i.e. a single token), while
/// `FragmentedSourceRange` is more flexible and can represent composite structures whose tokens
/// come from different sources.
///
/// `SourceRange` is also useful when displaying diagnostics, where one wants to indicate actual
/// ranges in the source code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceRange(SourcePos, u32);

impl SourceRange {
    /// Creates a new range starting at `begin` and covering `len` bytes.
    #[inline]
    pub fn new(begin: SourcePos, len: u32) -> Self {
        SourceRange(begin, len)
    }

    /// Returns the start position of the range.
    #[inline]
    pub fn start(self) -> SourcePos {
        self.0
    }

    /// Returns the length of the range.
    #[inline]
    pub fn len(self) -> u32 {
        self.1
    }

    /// Returns `true` if `self` covers 0 bytes.
    #[inline]
    pub fn is_empty(self) -> bool {
        self.1 == 0
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
    pub fn subpos(self, off: u32) -> SourcePos {
        assert!(off < self.len());
        self.start().offset(off)
    }

    /// Returns a subrange starting `off` (zero-based) bytes in and having length `len`.
    ///
    /// # Panics
    ///
    /// Panics if the range would not contain the returned subrange.
    #[inline]
    pub fn subrange(self, off: u32, len: u32) -> SourceRange {
        assert!(off + len <= self.len());
        SourceRange::new(self.start().offset(off), len)
    }

    /// Checks whether `self` contains `pos`, that is `pos` lies in `[self.start(), self.end())`.
    #[inline]
    pub fn contains(self, pos: SourcePos) -> bool {
        self.start() <= pos && pos < self.end()
    }

    /// Checks whether `self` contains `other`, that is `[other.start(), other.end())` is a subset
    /// of `[self.start(), self.end())`.
    #[inline]
    pub fn contains_range(self, other: SourceRange) -> bool {
        self.start() <= other.start() && other.end() <= self.end()
    }
}

/// Converts a position to an empty range around it.
impl From<SourcePos> for SourceRange {
    #[inline]
    fn from(pos: SourcePos) -> Self {
        Self::new(pos, 0)
    }
}

/// Represents a range whose endpoints may lie in different
/// [sources](smap/index.html#sources).
///
/// Reusing the example discussed in the source map documentation, consider the following code:
///
/// ```c
/// #define A (2 + 3)
/// int x = A + 1;
/// ```
///
/// No [`SourceRange`](struct.SourceRange.html) can accurately cover the expression `A + 1`, as its
/// left endpoint lies within the expansion of `A` while its right endpoint lies within the
/// surrounding file. For this reason, `FragmentedSourceRange` is a better fit for representing
/// multi-token structures which may contain arbitrarily complex macro expansions.
///
/// [`SourceMap::get_unfragmented_range()`](smap/struct.SourceMap.html#method.get_unfragmented_range)
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
        let range = SourceRange::new(start, 5);
        assert!(range.contains(start));
        assert!(range.contains(start.offset(4)));
        assert!(!range.contains(start.offset(5)));
    }
    #[test]
    fn source_range_contains_range() {
        let start = SourcePos::from_raw(16);
        let range = SourceRange::new(start.offset(1), 20);
        assert!(range.contains_range(range));
        assert!(range.contains_range(range.subrange(5, 7)));
        assert!(!range.contains_range(SourceRange::new(start, 5)));
        assert!(!range.contains_range(SourceRange::new(start.offset(6), 20)));
    }
}
