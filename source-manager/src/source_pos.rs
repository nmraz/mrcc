use std::path::Path;

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct SourcePos(u32);

impl SourcePos {
    pub fn from_raw(raw: u32) -> Self {
        SourcePos(raw)
    }

    pub fn to_raw(&self) -> u32 {
        self.0
    }

    pub fn with_offset(&self, offset: u32) -> Self {
        SourcePos(self.0 + offset)
    }

    pub fn offset_from(&self, rhs: SourcePos) -> u32 {
        self.to_raw()
            .checked_sub(rhs.to_raw())
            .expect("other position does not precede self")
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct SourceRange(SourcePos, SourcePos);

impl SourceRange {
    pub fn new(begin: SourcePos, end: SourcePos) -> Self {
        end.offset_from(begin); // Check that begin precedes end
        SourceRange(begin, end)
    }

    pub fn new_with_offset(begin: SourcePos, offset: u32) -> Self {
        SourceRange(begin, begin.with_offset(offset))
    }

    pub fn begin(&self) -> SourcePos {
        self.0
    }

    pub fn end(&self) -> SourcePos {
        self.1
    }

    pub fn len(&self) -> u32 {
        self.end().offset_from(self.begin())
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
