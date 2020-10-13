use std::fmt;
use std::path::PathBuf;
use std::rc::Rc;

use crate::{LineCol, LocalOff, LocalRange, SourcePos, SourceRange};
use line_table::LineTable;

mod line_table;

#[cfg(test)]
mod tests;

/// Represents a file name, which can either be a real path or a name synthesized by the compiler.
///
/// Synthesized names are used for the source code created by a token paste, for example.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileName {
    Real(PathBuf),
    Synth(String),
}

impl FileName {
    /// Creates a new real file name with the specified path.
    pub fn real(path: impl Into<PathBuf>) -> Self {
        FileName::Real(path.into())
    }

    /// Creates a new synthesized file name.
    pub fn synth(name: impl Into<String>) -> Self {
        FileName::Synth(name.into())
    }

    /// Returns `true` if the file name is real.
    pub fn is_real(&self) -> bool {
        matches!(self, FileName::Real(_))
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

/// Represents the contents of a loaded source file.
pub struct FileContents {
    /// The source code in the file.
    pub src: String,
    /// A table used to look up line numbers by file offset.
    line_table: LineTable,
}

impl FileContents {
    /// Creates a new `FileContents` with the specified source.
    ///
    /// Line endings in the source are normalized.
    pub fn new(src: &str) -> Rc<Self> {
        let normalized_src = src.replace("\r\n", "\n");
        let line_table = LineTable::new_for_src(&normalized_src);

        Rc::new(FileContents {
            src: normalized_src,
            line_table,
        })
    }

    /// Retrieves the specified portion of the source code.
    ///
    /// # Panics
    ///
    /// Panics if the range does not lie within the source.
    pub fn get_snippet(&self, range: LocalRange) -> &str {
        &self.src[range]
    }

    /// Returns the number of lines in the source.
    pub fn line_count(&self) -> u32 {
        self.line_table.line_count()
    }

    /// Computes the line and column numbers for the specified position.
    ///
    /// # Panics
    ///
    /// Panics if the offset is longer than the source.
    pub fn get_linecol(&self, off: LocalOff) -> LineCol {
        assert!(off <= LocalOff::of(&self.src));
        self.line_table.get_linecol(off)
    }

    /// Obtains the starting offset within the source of the specified (zero-based) line number.
    ///
    /// # Panics
    ///
    /// Panics if the line number is out of range.
    pub fn get_line_start(&self, line: u32) -> LocalOff {
        self.line_table.get_line_start(line)
    }

    /// Obtains the ending offset within the source of the specified (zero-based) line number.
    ///
    /// # Panics
    ///
    /// Panics if the line number is out of range.
    pub fn get_line_end(&self, line: u32) -> LocalOff {
        assert!(line < self.line_count());

        if line == self.line_count() - 1 {
            LocalOff::of(&self.src)
        } else {
            self.line_table.get_line_start(line + 1) - LocalOff::from(1)
        }
    }

    /// Returns a reference to lines `first..=last` of the source code, including final newline (if
    /// present).
    ///
    /// # Panics
    ///
    /// Panics if either line number is out of range or if `first > last`.
    pub fn get_lines(&self, first: u32, last: u32) -> &str {
        let start = self.get_line_start(first);
        let end = self.get_line_end(last);
        self.get_snippet(LocalRange::new(start, end))
    }

    /// Returns a reference to the specified line of source code, including newline character (if
    /// present).
    ///
    /// # Panics
    ///
    /// Panics if the line number is out of range.
    pub fn get_line(&self, line: u32) -> &str {
        self.get_lines(line, line)
    }
}

/// Holds information about a file [source](index.html#sources).
#[derive(Clone)]
pub struct FileSourceInfo {
    /// The name of the file.
    pub filename: FileName,
    /// The contents of the file. Multiple file sources may share the same contents (e.g. when the
    /// same file is included multiple times).
    pub contents: Rc<FileContents>,
    /// The position at which this file was included, if any.
    pub include_pos: Option<SourcePos>,
}

impl FileSourceInfo {
    /// Creates a new `FileSourceInfo`.
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

/// The different kinds of expansions that can be tracked by an expansion source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpansionKind {
    /// An ordinary macro expansion.
    Macro,
    /// The expansion of a macro argument into its owning macro.
    MacroArg,
    /// An expansion synthesized by the compiler, such as those for token pastes and stringization.
    Synth,
}

/// Holds information about an expansion [source](index.html#sources).
#[derive(Debug, Clone, Copy)]
pub struct ExpansionSourceInfo {
    /// The expansion's spelling range. The length of this range is always one less than that of the
    /// enclosing source (it does not contain a sentinel position).
    pub spelling_range: SourceRange,
    /// The range into which the expansion was performed.
    pub replacement_range: SourceRange,
    /// The kind of expansion recoreded here.
    pub kind: ExpansionKind,
}

impl ExpansionSourceInfo {
    /// Creates a new `ExpansionSourceInfo`.
    pub fn new(
        spelling_range: SourceRange,
        replacement_range: SourceRange,
        kind: ExpansionKind,
    ) -> Self {
        ExpansionSourceInfo {
            spelling_range,
            replacement_range,
            kind,
        }
    }

    /// Returns the position at which the byte at the specified offset was spelled.
    ///
    /// # Panics
    ///
    /// Panics if `off` lies beyond the bounds of the spelling range.
    pub fn spelling_pos(&self, off: LocalOff) -> SourcePos {
        self.spelling_range.subpos(off)
    }

    /// Returns the source range at which the specified range within the expansion was spelled.
    ///
    /// # Panics
    ///
    /// Panics if `range` does not fit in the spelling range.
    pub fn spelling_range(&self, range: LocalRange) -> SourceRange {
        self.spelling_range.subrange(range)
    }

    /// Returns the source range within the macro caller corresponding to the specified range within
    /// the expansion.
    ///
    /// For macro arguments, the caller is where the argument was spelled, while for other types of
    /// expansions it is the replacement range.
    pub fn caller_range(&self, range: LocalRange) -> SourceRange {
        match self.kind {
            ExpansionKind::MacroArg => self.spelling_range(range),
            _ => self.replacement_range,
        }
    }
}

/// Information held by a source, which can be either a file or an expansion.
#[derive(Clone)]
pub enum SourceInfo {
    File(FileSourceInfo),
    Expansion(ExpansionSourceInfo),
}

/// A single source in the `SourceMap`.
#[derive(Clone)]
pub struct Source {
    /// The attached (file or expansion) information.
    pub info: Box<SourceInfo>,
    /// The range spanned by this source. This is one byte longer than the source's true "range",
    /// for disambiguation purposes. This range should thus almost never be used directly - take
    /// subranges as appropriate.
    pub range: SourceRange,
}

impl Source {
    /// Computes the local offset within the source given a position.
    ///
    /// # Panics
    ///
    /// Panics if `self.range` does not contain `pos`.
    pub fn local_off(&self, pos: SourcePos) -> LocalOff {
        self.range
            .local_off(pos)
            .expect("position does not lie within this source")
    }

    /// Computes the local range within this source, given a `SourceRange`.
    ///
    /// # Panics
    ///
    /// Panics if `self.range` does not contain `range`.
    pub fn local_range(&self, range: SourceRange) -> LocalRange {
        self.range
            .local_range(range)
            .expect("range does not lie within this source")
    }

    /// If this source contains a file, returns a reference to the contained file information.
    /// Otherwise, returns `None`.
    pub fn as_file(&self) -> Option<&FileSourceInfo> {
        match *self.info {
            SourceInfo::File(ref file) => Some(file),
            _ => None,
        }
    }

    /// If this source contains an expansion, returns a reference to the contained expansion
    /// information. Otherwise, returns `None`.
    pub fn as_expansion(&self) -> Option<&ExpansionSourceInfo> {
        match *self.info {
            SourceInfo::Expansion(ref exp) => Some(exp),
            _ => None,
        }
    }

    /// Returns `true` if this source contains a file.
    pub fn is_file(&self) -> bool {
        self.as_file().is_some()
    }

    /// Returns `true` if this source contains an expansion.
    pub fn is_expansion(&self) -> bool {
        self.as_expansion().is_some()
    }
}
