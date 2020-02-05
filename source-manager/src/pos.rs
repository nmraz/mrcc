#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourcePos(u32);

impl SourcePos {
    pub(crate) fn from_raw(raw: u32) -> Self {
        SourcePos(raw)
    }

    pub(crate) fn to_raw(&self) -> u32 {
        self.0
    }

    pub fn offset(&self, offset: u32) -> Self {
        SourcePos(self.0 + offset)
    }

    pub fn offset_from(&self, rhs: SourcePos) -> u32 {
        self.to_raw() - rhs.to_raw()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceRange(SourcePos, u32);

impl SourceRange {
    pub fn new(begin: SourcePos, len: u32) -> Self {
        SourceRange(begin, len)
    }

    pub fn start(&self) -> SourcePos {
        self.0
    }

    pub fn len(&self) -> u32 {
        self.1
    }

    pub fn end(&self) -> SourcePos {
        self.start().offset(self.len())
    }

    pub fn subpos(&self, off: u32) -> SourcePos {
        assert!(off < self.len());
        self.start().offset(off)
    }

    pub fn subrange(&self, off: u32, len: u32) -> SourceRange {
        assert!(off + len <= self.len());
        SourceRange::new(self.start().offset(off), len)
    }

    pub fn contains(&self, pos: SourcePos) -> bool {
        let raw = pos.to_raw();
        self.start().to_raw() <= raw && raw < self.end().to_raw()
    }

    pub fn contains_range(&self, other: SourceRange) -> bool {
        self.start().to_raw() <= other.start().to_raw()
            && other.end().to_raw() <= self.end().to_raw()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct LineCol {
    pub line: u32,
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
