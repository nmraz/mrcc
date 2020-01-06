use std::convert::TryInto;
use std::option::Option;
use std::path::{Path, PathBuf};
use std::vec::Vec;

mod source_pos;

pub use source_pos::*;

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

impl Source {
    pub fn start_pos(&self) -> SourcePos {
        SourcePos::from_raw(self.offset)
    }
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
    ) -> Result<&Source, SourcesTooLargeError> {
        let offset = self.next_offset;
        self.next_offset = match self.next_offset.checked_add(len) {
            Some(off) => off,
            None => return Err(SourcesTooLargeError),
        };

        self.sources.push(Source {
            offset,
            info: ctor(),
        });

        Ok(self.sources.last().unwrap())
    }

    pub fn create_file(
        &mut self,
        filename: PathBuf,
        src: String,
        include_pos: Option<SourcePos>,
    ) -> Result<(SourcePos, &str), SourcesTooLargeError> {
        let len = match src.len().try_into() {
            Ok(len) => len,
            Err(..) => return Err(SourcesTooLargeError),
        };

        let source = self.add_source(
            || SourceInfo::File(FileSourceInfo::new(filename, src, include_pos)),
            len,
        )?;

        let contents = match &source.info {
            SourceInfo::File(file) => file.src(),
            _ => unreachable!(),
        };

        Ok((source.start_pos(), contents))
    }

    pub fn create_expansion(
        &mut self,
        spelling_pos: SourcePos,
        expansion_range: SourceRange,
        tok_len: u32,
    ) -> Result<SourcePos, SourcesTooLargeError> {
        self.check_range(expansion_range);

        self.add_source(
            || SourceInfo::Expansion(ExpansionSourceInfo::new(spelling_pos, expansion_range)),
            tok_len,
        )
        .map(|source| source.start_pos())
    }

    fn get_source_idx(&self, pos: SourcePos) -> usize {
        let offset = pos.to_raw();
        assert!(offset < self.next_offset);

        self.sources
            .binary_search_by_key(&offset, |source| source.offset)
            .unwrap_or_else(|i| i - 1)
    }

    fn get_range_source_idx(&self, range: SourceRange) -> usize {
        let begin_idx = self.get_source_idx(range.begin());
        let end_idx = self.get_source_idx(range.end());
        assert_eq!(begin_idx, end_idx, "invalid source range");
        begin_idx
    }

    fn check_range(&self, range: SourceRange) {
        self.get_range_source_idx(range);
    }

    fn get_decomposed_pos(&self, pos: SourcePos) -> (&Source, u32) {
        let source = &self.sources[self.get_source_idx(pos)];
        (source, pos.offset_from(source.start_pos()))
    }

    pub fn get_immediate_spelling_pos(&self, pos: SourcePos) -> SourcePos {
        let (source, offset) = self.get_decomposed_pos(pos);

        match &source.info {
            SourceInfo::File(..) => pos,
            SourceInfo::Expansion(exp) => exp.spelling_pos().with_offset(offset),
        }
    }

    pub fn get_immediate_expansion_range(&self, range: SourceRange) -> SourceRange {
        let source = &self.sources[self.get_range_source_idx(range)];

        match &source.info {
            SourceInfo::File(..) => range,
            SourceInfo::Expansion(exp) => exp.expansion_range(),
        }
    }

    pub fn get_spelling_pos(&self, mut pos: SourcePos) -> SourcePos {
        let mut imm_pos = self.get_immediate_spelling_pos(pos);

        while imm_pos != pos {
            pos = imm_pos;
            imm_pos = self.get_immediate_spelling_pos(pos);
        }

        pos
    }

    pub fn get_expansion_range(&self, mut range: SourceRange) -> SourceRange {
        let mut imm_range = self.get_immediate_expansion_range(range);

        while imm_range != range {
            range = imm_range;
            imm_range = self.get_immediate_expansion_range(range);
        }

        range
    }
}
