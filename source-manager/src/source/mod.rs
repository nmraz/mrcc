use std::ops::Range;
use std::ptr;
use std::rc::Rc;

mod line_table;

#[cfg(test)]
mod tests;

use crate::pos::{LineCol, SourcePos, SourceRange};
use line_table::LineTable;

pub struct FileSourceInfo {
    filename: String,
    src: String,
    include_pos: Option<SourcePos>,
    line_table: LineTable,
}

impl FileSourceInfo {
    pub fn new(filename: String, src: String, include_pos: Option<SourcePos>) -> Self {
        let normalized_src: String = normalize_line_endings::normalized(src.chars()).collect();
        let line_table = LineTable::new_for_src(&normalized_src);

        FileSourceInfo {
            filename: filename,
            src: normalized_src,
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
        assert!((off as usize) <= self.src.len());
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
            self.line_table.get_line_start(line + 1) - 1
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpansionType {
    Macro,
    MacroArg,
    Synthesized,
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

    pub fn get_spelling_range(&self, off: u32, len: u32) -> SourceRange {
        SourceRange::new(self.spelling_pos.offset(off), len)
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
    pub(crate) fn new(info: SourceInfo, range: SourceRange) -> Rc<Self> {
        Rc::new(Source { info, range })
    }

    pub fn range(&self) -> SourceRange {
        self.range
    }

    pub fn info(&self) -> &SourceInfo {
        &self.info
    }

    pub fn as_file(&self) -> Option<&FileSourceInfo> {
        match self.info {
            SourceInfo::File(ref file) => Some(file),
            _ => None,
        }
    }

    pub fn as_expansion(&self) -> Option<&ExpansionSourceInfo> {
        match self.info {
            SourceInfo::Expansion(ref exp) => Some(exp),
            _ => None,
        }
    }

    pub fn is_file(&self) -> bool {
        self.as_file().is_some()
    }

    pub fn is_expansion(&self) -> bool {
        self.as_expansion().is_some()
    }
}

impl PartialEq<Source> for Source {
    fn eq(&self, rhs: &Source) -> bool {
        ptr::eq(self, rhs)
    }
}

impl Eq for Source {}
