use std::cell::RefCell;
use std::convert::TryInto;
use std::ops::Range;
use std::option::Option;
use std::rc::Rc;
use std::vec::Vec;

pub mod pos;
mod source;

#[cfg(test)]
mod tests;

use pos::{LineCol, SourcePos, SourceRange};
pub use source::{ExpansionSourceInfo, ExpansionType, FileSourceInfo, Source, SourceInfo};

#[derive(Clone)]
pub struct InterpretedFileRange {
    source: Rc<Source>,
    off: u32,
    len: u32,
}

impl InterpretedFileRange {
    pub fn file(&self) -> &FileSourceInfo {
        self.source.unwrap_file()
    }

    pub fn range(&self) -> Range<u32> {
        self.off..self.off + self.len
    }

    pub fn start_linecol(&self) -> LineCol {
        self.file().get_linecol(self.off)
    }

    pub fn end_linecol(&self) -> LineCol {
        self.file().get_linecol(self.off + self.len)
    }
}

#[derive(Debug)]
pub struct SourcesTooLargeError;

pub struct SourceManager {
    sources: RefCell<Vec<Rc<Source>>>,
}

impl SourceManager {
    pub fn new() -> Self {
        SourceManager {
            sources: RefCell::new(vec![]),
        }
    }

    fn add_source(&self, ctor: impl FnOnce() -> SourceInfo, len: u32) -> Rc<Source> {
        let mut sources = self.sources.borrow_mut();

        let offset = match sources.last() {
            None => 0,
            Some(source) => source.range().end().to_raw() + 1,
        };

        let source = Rc::new(Source::new(
            ctor(),
            SourceRange::new(SourcePos::from_raw(offset), len),
        ));

        sources.push(source.clone());

        source
    }

    pub fn create_file(
        &self,
        filename: String,
        src: String,
        include_pos: Option<SourcePos>,
    ) -> Result<Rc<Source>, SourcesTooLargeError> {
        let len = match src.len().try_into() {
            Ok(len) => len,
            Err(..) => return Err(SourcesTooLargeError),
        };

        Ok(self.add_source(
            || SourceInfo::File(FileSourceInfo::new(filename, src, include_pos)),
            len,
        ))
    }

    pub fn create_expansion(
        &self,
        spelling_range: SourceRange,
        expansion_range: SourceRange,
        expansion_type: ExpansionType,
    ) -> Rc<Source> {
        self.check_range(expansion_range);

        self.add_source(
            || {
                SourceInfo::Expansion(ExpansionSourceInfo::new(
                    spelling_range.start(),
                    expansion_range,
                    expansion_type,
                ))
            },
            spelling_range.len(),
        )
    }

    pub fn lookup_source(&self, pos: SourcePos) -> Rc<Source> {
        let offset = pos.to_raw();
        let sources = self.sources.borrow();

        let last = sources.last().unwrap();
        assert!(offset <= last.range().end().to_raw());

        let idx = sources
            .binary_search_by_key(&offset, |source| source.range().start().to_raw())
            .unwrap_or_else(|i| i - 1);

        sources[idx].clone()
    }

    fn lookup_range_source(&self, range: SourceRange) -> Rc<Source> {
        let source = self.lookup_source(range.start());
        assert!(range.len() <= source.range().len(), "invalid source range");
        source
    }

    fn check_range(&self, range: SourceRange) {
        self.lookup_range_source(range);
    }

    pub fn lookup_source_off(&self, pos: SourcePos) -> (Rc<Source>, u32) {
        let source = self.lookup_source(pos);
        let off = pos.offset_from(source.range().start());
        (source, off)
    }

    pub fn get_immediate_spelling_pos(&self, pos: SourcePos) -> SourcePos {
        let (source, offset) = self.lookup_source_off(pos);

        match source.info() {
            SourceInfo::File(..) => pos,
            SourceInfo::Expansion(exp) => exp.spelling_pos().offset(offset),
        }
    }

    pub fn get_immediate_expansion_range(&self, range: SourceRange) -> SourceRange {
        let source = self.lookup_range_source(range);

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
        let (source, start_off) = self.lookup_source_off(expansion_range.start());

        InterpretedFileRange {
            source,
            off: start_off,
            len: expansion_range.len(),
        }
    }
}
