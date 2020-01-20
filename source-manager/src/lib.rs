use std::convert::TryInto;
use std::ops::Range;
use std::option::Option;
use std::vec::Vec;

mod line_table;
mod source_pos;

use line_table::LineTable;
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
    filename: String,
    src: String,
    include_pos: Option<SourcePos>,
    line_table: LineTable,
}

impl FileSourceInfo {
    pub fn new(filename: String, src: String, include_pos: Option<SourcePos>) -> Self {
        let line_table = LineTable::new_for_src(&src);

        FileSourceInfo {
            filename: filename,
            src,
            include_pos,
            line_table,
        }
    }

    pub fn src(&self) -> &str {
        &self.src
    }

    pub fn filename(&self) -> &str {
        &self.filename
    }

    pub fn include_pos(&self) -> Option<SourcePos> {
        self.include_pos
    }

    pub fn get_snippet(&self, range: Range<u32>) -> &str {
        &self.src[range.start as usize..range.end as usize]
    }

    pub fn line_count(&self) -> u32 {
        self.line_table.line_count()
    }

    pub fn get_linecol(&self, off: u32) -> LineCol {
        assert!((off as usize) < self.src.len());
        self.line_table.get_linecol(off)
    }

    pub fn get_line_start(&self, line: u32) -> u32 {
        self.line_table.get_line_start(line)
    }

    pub fn get_line_end(&self, line: u32) -> u32 {
        assert!(line < self.line_count());

        if line == self.line_count() - 1 {
            self.src().len() as u32
        } else {
            self.line_table.get_line_start(line + 1)
        }
    }
}

#[derive(Clone, Copy)]
pub struct InterpretedFileRange<'f> {
    file: &'f FileSourceInfo,
    off: u32,
    len: u32,
}

impl<'f> InterpretedFileRange<'f> {
    pub fn new(file: &'f FileSourceInfo, off: u32, len: u32) -> Self {
        InterpretedFileRange { file, off, len }
    }

    pub fn file(&self) -> &'f FileSourceInfo {
        self.file
    }

    pub fn range(&self) -> Range<u32> {
        self.off..self.off + self.len
    }

    pub fn start_linecol(&self) -> LineCol {
        self.file.get_linecol(self.off)
    }

    pub fn end_linecol(&self) -> LineCol {
        self.file.get_linecol(self.off + self.len)
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

    pub fn unwrap_expansion(&self) -> &ExpansionSourceInfo {
        match &self.info {
            SourceInfo::Expansion(exp) => exp,
            _ => panic!("source was not an expansion"),
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
        filename: String,
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
                .binary_search_by_key(&offset, |source| source.range().start().to_raw())
                .unwrap_or_else(|i| i - 1),
        )
    }

    pub fn get_source(&self, id: SourceId) -> &Source {
        &self.sources[id.to_idx()]
    }

    fn get_range_source_id(&self, range: SourceRange) -> SourceId {
        let id = self.get_source_id(range.start());
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
        (id, pos.offset_from(source.range().start()))
    }

    pub fn get_immediate_spelling_pos(&self, pos: SourcePos) -> SourcePos {
        let (id, offset) = self.get_decomposed_pos(pos);
        let source = self.get_source(id);

        match source.info() {
            SourceInfo::File(..) => pos,
            SourceInfo::Expansion(exp) => exp.spelling_pos().offset(offset),
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

    pub fn get_interpreted_pos(&self, pos: SourcePos) -> InterpretedSourcePos {
        let expansion_pos = self.get_expansion_range(SourceRange::new(pos, 0)).start();
        let (id, off) = self.get_decomposed_pos(expansion_pos);

        let file = self.get_source(id).unwrap_file();

        InterpretedSourcePos::new(file.filename(), file.get_line_col(off))
    }

    pub fn get_interpreted_range(&self, range: SourceRange) -> InterpretedSourceRange {
        let expansion_range = self.get_expansion_range(range);
        let (id, start_off) = self.get_decomposed_pos(expansion_range.start());

        let file = self.get_source(id).unwrap_file();

        InterpretedSourceRange::new(
            file.filename(),
            file.get_line_col(start_off),
            file.get_line_col(start_off + expansion_range.len()),
        )
    }
}
