use std::convert::TryInto;
use std::option::Option;
use std::path::{Path, PathBuf};
use std::vec::Vec;

mod source_pos;

pub use source_pos::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceId(usize);

impl SourceId {
    pub(crate) fn from_idx(idx: usize) -> Self {
        SourceId(idx)
    }

    pub(crate) fn to_idx(&self) -> usize {
        self.0
    }
}

pub struct FileSourceInfo {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpansionType {
    Macro,
    MacroArg,
}

pub struct ExpansionSourceInfo {
    spelling_pos: SourcePos,
    expansion_range: SourceRange,
    expansion_type: ExpansionType,
}

impl ExpansionSourceInfo {
    pub fn new(
        spelling_pos: SourcePos,
        expansion_range: SourceRange,
        expansion_type: ExpansionType,
    ) -> Self {
        ExpansionSourceInfo {
            spelling_pos,
            expansion_range,
            expansion_type,
        }
    }

    pub fn spelling_pos(&self) -> SourcePos {
        self.spelling_pos
    }

    pub fn expansion_range(&self) -> SourceRange {
        self.expansion_range
    }

    pub fn expansion_type(&self) -> ExpansionType {
        self.expansion_type
    }
}

pub enum SourceInfo {
    File(FileSourceInfo),
    Expansion(ExpansionSourceInfo),
}

pub struct Source {
    info: SourceInfo,
    range: SourceRange,
}

impl Source {
    pub fn new(info: SourceInfo, range: SourceRange) -> Self {
        Source { info, range }
    }

    pub fn range(&self) -> SourceRange {
        self.range
    }

    pub fn info(&self) -> &SourceInfo {
        &self.info
    }

    pub fn is_file(&self) -> bool {
        match self.info {
            SourceInfo::File(..) => true,
            _ => false,
        }
    }

    pub fn is_expansion(&self) -> bool {
        !self.is_file()
    }

    pub fn unwrap_file(&self) -> &FileSourceInfo {
        match &self.info {
            SourceInfo::File(file) => file,
            _ => panic!("source was not a file"),
        }
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
    ) -> Result<SourceId, SourcesTooLargeError> {
        let offset = self.next_offset;
        self.next_offset = match self.next_offset.checked_add(len) {
            Some(off) => off,
            None => return Err(SourcesTooLargeError),
        };

        self.sources.push(Source::new(
            ctor(),
            SourceRange::new(SourcePos::from_raw(offset), len),
        ));

        Ok(SourceId::from_idx(self.sources.len() - 1))
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
        expansion_type: ExpansionType,
        len: u32,
    ) -> Result<SourceId, SourcesTooLargeError> {
        self.check_range(expansion_range);

        self.add_source(
            || {
                SourceInfo::Expansion(ExpansionSourceInfo::new(
                    spelling_pos,
                    expansion_range,
                    expansion_type,
                ))
            },
            len,
        )
    }

    pub fn get_source_id(&self, pos: SourcePos) -> SourceId {
        let offset = pos.to_raw();
        assert!(offset < self.next_offset);

        SourceId::from_idx(
            self.sources
                .binary_search_by_key(&offset, |source| source.range().begin().to_raw())
                .unwrap_or_else(|i| i - 1),
        )
    }

    pub fn get_source(&self, id: SourceId) -> &Source {
        &self.sources[id.to_idx()]
    }

    fn get_range_source_id(&self, range: SourceRange) -> SourceId {
        let id = self.get_source_id(range.begin());
        assert!(
            range.len() <= self.get_source(id).range().len(),
            "invalid source range"
        );
        id
    }

    fn check_range(&self, range: SourceRange) {
        self.get_range_source_id(range);
    }

    pub fn get_decomposed_pos(&self, pos: SourcePos) -> (SourceId, u32) {
        let id = self.get_source_id(pos);
        let source = self.get_source(id);
        (id, pos.offset_from(source.range().begin()))
    }

    pub fn get_immediate_spelling_pos(&self, pos: SourcePos) -> SourcePos {
        let (id, offset) = self.get_decomposed_pos(pos);
        let source = self.get_source(id);

        match source.info() {
            SourceInfo::File(..) => pos,
            SourceInfo::Expansion(exp) => exp.spelling_pos().with_offset(offset),
        }
    }

    pub fn get_immediate_expansion_range(&self, range: SourceRange) -> SourceRange {
        let source = self.get_source(self.get_range_source_id(range));

        match source.info() {
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
