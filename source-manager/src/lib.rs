use std::convert::TryInto;
use std::ops::Deref;
use std::ops::Range;
use std::option::Option;
use std::ptr;
use std::vec::Vec;

pub mod pos;
mod source;

#[cfg(test)]
mod tests;

use pos::{LineCol, SourcePos, SourceRange};
pub use source::{ExpansionSourceInfo, ExpansionType, FileSourceInfo, Source, SourceInfo};

#[derive(Clone, Copy)]
pub struct SourceRef<'s> {
    source: &'s Source,
}

impl<'s> Deref for SourceRef<'s> {
    type Target = &'s Source;

    fn deref(&self) -> &&'s Source {
        &self.source
    }
}

impl<'s> PartialEq<SourceRef<'s>> for SourceRef<'s> {
    fn eq(&self, rhs: &SourceRef) -> bool {
        ptr::eq(self.source, rhs.source)
    }
}

impl Eq for SourceRef<'_> {}

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
    ) -> Result<SourceRef, SourcesTooLargeError> {
        let offset = self.next_offset;
        self.next_offset = match self.next_offset.checked_add(len) {
            Some(off) => off,
            None => return Err(SourcesTooLargeError),
        };

        self.sources.push(Source::new(
            ctor(),
            SourceRange::new(SourcePos::from_raw(offset), len),
        ));

        Ok(SourceRef {
            source: self.sources.last().unwrap(),
        })
    }

    pub fn create_file(
        &mut self,
        filename: String,
        src: String,
        include_pos: Option<SourcePos>,
    ) -> Result<SourceRef, SourcesTooLargeError> {
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
    ) -> Result<SourceRef, SourcesTooLargeError> {
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

    pub fn get_source(&self, pos: SourcePos) -> SourceRef {
        let offset = pos.to_raw();
        assert!(offset < self.next_offset);

        let idx = self
            .sources
            .binary_search_by_key(&offset, |source| source.range().start().to_raw())
            .unwrap_or_else(|i| i - 1);

        SourceRef {
            source: &self.sources[idx],
        }
    }

    fn get_range_source(&self, range: SourceRange) -> SourceRef {
        let source = self.get_source(range.start());
        assert!(range.len() <= source.range().len(), "invalid source range");
        source
    }

    fn check_range(&self, range: SourceRange) {
        self.get_range_source(range);
    }

    pub fn get_decomposed_pos(&self, pos: SourcePos) -> (SourceRef, u32) {
        let source = self.get_source(pos);
        (source, pos.offset_from(source.range().start()))
    }

    pub fn get_immediate_spelling_pos(&self, pos: SourcePos) -> SourcePos {
        let (source, offset) = self.get_decomposed_pos(pos);

        match source.info() {
            SourceInfo::File(..) => pos,
            SourceInfo::Expansion(exp) => exp.spelling_pos().offset(offset),
        }
    }

    pub fn get_immediate_expansion_range(&self, range: SourceRange) -> SourceRange {
        let source = self.get_range_source(range);

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

    pub fn get_interpreted_range(&self, range: SourceRange) -> InterpretedFileRange {
        let expansion_range = self.get_expansion_range(range);
        let (source, start_off) = self.get_decomposed_pos(expansion_range.start());

        let file = source.unwrap_file();

        InterpretedFileRange::new(file, start_off, expansion_range.len())
    }
}
