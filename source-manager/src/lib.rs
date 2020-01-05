use std::convert::TryInto;
use std::option::Option;
use std::path::{Path, PathBuf};
use std::vec::Vec;

mod source_pos;

pub use source_pos::*;

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct SourceId(usize);

impl SourceId {
    pub fn from_raw(raw: usize) -> Self {
        SourceId(raw)
    }

    pub fn to_raw(&self) -> usize {
        self.0
    }
}

struct FileSourceInfo {
    filename: Box<Path>,
    src: String,
    include_pos: Option<SourcePos>,
}

impl FileSourceInfo {
    pub fn new(filename: PathBuf, src: String, include_pos: Option<SourcePos>) -> Self {
        FileSourceInfo {
            filename: filename.into_boxed_path(),
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

pub struct SourcesTooLargeError;

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

    fn add_source(
        &mut self,
        ctor: impl FnOnce() -> SourceInfo,
        len: u32,
    ) -> Result<SourceId, SourcesTooLargeError> {
        let offset = self.next_offset;
        self.next_offset = match self.next_offset.checked_add(len) {
            Some(off) => off,
            None => return Err(SourcesTooLargeError),
        };

        self.sources.push(Source {
            offset,
            info: ctor(),
        });

        Ok(SourceId::from_raw(self.sources.len()))
    }

    pub fn create_file(
        &mut self,
        filename: PathBuf,
        src: String,
        include_pos: Option<SourcePos>,
    ) -> Result<SourceId, SourcesTooLargeError> {
        let len = match src.len().try_into() {
            Ok(len) => len,
            Err(..) => return Err(SourcesTooLargeError),
        };

        self.add_source(
            || SourceInfo::File(FileSourceInfo::new(filename, src, include_pos)),
            len,
        )
    }

    pub fn create_expansion(
        &mut self,
        spelling_pos: SourcePos,
        expansion_range: SourceRange,
        tok_len: u32,
    ) -> Result<SourceId, SourcesTooLargeError> {
        self.add_source(
            || SourceInfo::Expansion(ExpansionSourceInfo::new(spelling_pos, expansion_range)),
            tok_len,
        )
    }
}
