use std::option::Option;
use std::path::{Path, PathBuf};
use std::vec::Vec;

mod source_pos;

pub use source_pos::*;

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct SourceId(u32);

impl SourceId {
  pub fn from_raw(raw: u32) -> Self {
    SourceId(raw)
  }

  pub fn to_raw(&self) -> u32 {
    self.0
  }
}

struct FileSourceInfo {
  filename: Box<Path>,
  src: String,
  include_pos: Option<SourcePos>,
}

impl FileSourceInfo {
  pub fn new(filename: impl AsRef<Path>, src: String, include_pos: Option<SourcePos>) -> Self {
    FileSourceInfo {
      filename: PathBuf::from(filename.as_ref()).into_boxed_path(),
      src,
      include_pos,
    }
  }

  pub fn src(&self) -> &str {
    &self.src
  }

  pub fn filename(&self) -> &Path {
    &self.filename
  }

  pub fn include_pos(&self) -> Option<SourcePos> {
    self.include_pos
  }
}

struct ExpansionSourceInfo {
  spelling_pos: SourcePos,
  expansion_range: SourceRange,
}

impl ExpansionSourceInfo {
  pub fn new(spelling_pos: SourcePos, expansion_range: SourceRange) -> Self {
    ExpansionSourceInfo {
      spelling_pos,
      expansion_range,
    }
  }

  pub fn spelling_pos(&self) -> SourcePos {
    self.spelling_pos
  }

  pub fn expansion_range(&self) -> SourceRange {
    self.expansion_range
  }
}

enum SourceInfo {
  File(FileSourceInfo),
  Expansion(ExpansionSourceInfo),
}

struct Source {
  pub offset: u32,
  pub info: SourceInfo,
}

pub struct SourceManager {
  sources: Vec<Source>,
  next_offset: u32,
}

impl SourceManager {
  pub fn new() -> Self {
    SourceManager {
      sources: vec![],
      next_offset: 0,
    }
  }

  fn add_source(&mut self, source: Source, len: u32) -> SourceId {
    self.sources.push(source);
    self.next_offset += len;
    SourceId::from_raw(self.sources.len() as u32)
  }

  pub fn create_file(
    &mut self,
    filename: impl AsRef<Path>,
    src: String,
    include_pos: Option<SourcePos>,
  ) -> SourceId {
    let len = src.len() as u32;
    self.add_source(
      Source {
        offset: self.next_offset,
        info: SourceInfo::File(FileSourceInfo::new(filename, src, include_pos)),
      },
      len,
    )
  }
}
