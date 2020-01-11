use std::path::Path;

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct SourcePos(u32);

impl SourcePos {
    pub(crate) fn from_raw(raw: u32) -> Self {
        SourcePos(raw)
    }

    pub(crate) fn to_raw(&self) -> u32 {
        self.0
    }

    pub fn with_offset(&self, offset: u32) -> Self {
        SourcePos(self.0 + offset)
    }

    pub fn offset_from(&self, rhs: SourcePos) -> u32 {
        self.to_raw() - rhs.to_raw()
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct SourceRange(SourcePos, u32);

impl SourceRange {
    pub fn new(begin: SourcePos, len: u32) -> Self {
        SourceRange(begin, len)
    }

    pub fn begin(&self) -> SourcePos {
        self.0
    }

    pub fn len(&self) -> u32 {
        self.1
    }

    pub fn end(&self) -> SourcePos {
        self.begin().with_offset(self.len())
    }

    pub fn contains(&self, pos: SourcePos) -> bool {
        let raw = pos.to_raw();
        self.begin().to_raw() <= raw && raw < self.end().to_raw()
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct LineCol {
    pub line: u32,
    pub col: u32,
}

#[derive(Clone, Copy)]
pub struct InterpretedSourcePos<'f> {
    filename: &'f Path,
    pos: LineCol,
}

impl<'f> InterpretedSourcePos<'f> {
    pub fn filename(&self) -> &'f Path {
        self.filename
    }

    pub fn line(&self) -> u32 {
        self.pos.line
    }

    pub fn col(&self) -> u32 {
        self.pos.col
    }
}

#[derive(Clone, Copy)]
pub struct InterpretedSourceRange<'f> {
    filename: &'f Path,
    begin: LineCol,
    end: LineCol,
}

impl<'f> InterpretedSourceRange<'f> {
    pub fn filename(&self) -> &'f Path {
        self.filename
    }

    pub fn begin(&self) -> LineCol {
        self.begin
    }

    pub fn end(&self) -> LineCol {
        self.end
    }
}
