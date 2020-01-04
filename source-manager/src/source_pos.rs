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

  pub fn with_offset(&self, offset: i32) -> Self {
    SourcePos(self.0 + offset as u32)
  }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct SourceRange(SourcePos, SourcePos);

impl SourceRange {
  pub fn new(begin: SourcePos, end: SourcePos) -> Self {
    SourceRange(begin, end)
  }

  pub fn begin(&self) -> SourcePos {
    self.0
  }

  pub fn end(&self) -> SourcePos {
    self.1
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