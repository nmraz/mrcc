use std::fmt;
use std::ops::Range;
use std::path::PathBuf;
use std::rc::Rc;

use crate::{LineCol, SourcePos, SourceRange};
use line_table::LineTable;

mod line_table;

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileName {
    Real(PathBuf),
    Synth(String),
}

impl FileName {
    pub fn real(path: impl Into<PathBuf>) -> Self {
        FileName::Real(path.into())
    }

    pub fn synth(name: impl Into<String>) -> Self {
        FileName::Synth(name.into())
    }

    pub fn is_real(&self) -> bool {
        match self {
            FileName::Real(_) => true,
            _ => false,
        }
    }
}

impl fmt::Display for FileName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FileName::Real(path) => write!(f, "{}", path.display()),
            FileName::Synth(name) => write!(f, "<{}>", name),
        }
    }
}

pub struct FileContents {
    pub src: String,
    line_table: LineTable,
}

impl FileContents {
    pub fn new(src: &str) -> Rc<Self> {
        let normalized_src: String = normalize_line_endings::normalized(src.chars()).collect();
        let line_table = LineTable::new_for_src(&normalized_src);

        Rc::new(FileContents {
            src: normalized_src,
            line_table,
        })
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
            self.src.len() as u32
        } else {
            self.line_table.get_line_start(line + 1) - 1
        }
    }
}

#[derive(Clone)]
pub struct FileSourceInfo {
    pub filename: FileName,
    pub contents: Rc<FileContents>,
    pub include_pos: Option<SourcePos>,
}

impl FileSourceInfo {
    pub fn new(
        filename: FileName,
        contents: Rc<FileContents>,
        include_pos: Option<SourcePos>,
    ) -> Self {
        Self {
            filename,
            contents,
            include_pos,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpansionType {
    Macro,
    MacroArg,
    Synthesized,
}

#[derive(Debug, Clone, Copy)]
pub struct ExpansionSourceInfo {
    pub spelling_pos: SourcePos,
    pub expansion_range: SourceRange,
    pub expansion_type: ExpansionType,
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

    pub fn spelling_pos(&self, off: u32) -> SourcePos {
        self.spelling_pos.offset(off)
    }

    pub fn spelling_range(&self, range: Range<u32>) -> SourceRange {
        SourceRange::new(self.spelling_pos(range.start), range.len() as u32)
    }

    pub fn caller_range(&self, range: Range<u32>) -> SourceRange {
        // For macro arguments, the caller is where the argument was spelled, while for
        // everything else the caller recieves the expansion.
        match self.expansion_type {
            ExpansionType::MacroArg => self.spelling_range(range),
            _ => self.expansion_range,
        }
    }
}

#[derive(Clone)]
pub enum SourceInfo {
    File(FileSourceInfo),
    Expansion(ExpansionSourceInfo),
}

#[derive(Clone)]
pub struct Source {
    pub info: SourceInfo,
    pub range: SourceRange,
}

impl Source {
    pub fn local_off(&self, pos: SourcePos) -> u32 {
        assert!(self.range.contains(pos));
        pos.offset_from(self.range.start())
    }

    pub fn local_range(&self, range: SourceRange) -> Range<u32> {
        assert!(self.range.contains_range(range));
        let off = self.local_off(range.start());
        off..off + range.len()
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
