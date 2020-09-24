//! [`SourceMap`](struct.SourceMap.html) is a structure holding all of the source code used in a
//! compilation.
//!
//! It is inspired by the analagous `SourceManager` in clang. In addition to holding the source
//! code, it is responsible for tracking detailed location information within the code, and can
//! resolve a [`SourcePos`](../struct.SourcePos.html) or [`SourceRange`](../struct.SourceRange.html)
//! into file/line/column information with macro traces.
//!
//! # Sources
//!
//! The `SourceMap` contains a number of [`Source`](struct.Source.html) objects, each of which
//! represents an area to which source code can be attributed. There are two kinds of sources: files
//! and expansions.
//!
//! [File sources](struct.FileSourceInfo.html) represent actual source files from which code was
//! read. They point to the content of the file itself and record additional metadata, such as the
//! file name as spelled in the code. Note that a single file on disk may have multiple file source
//! entries in the `SourceMap`, one for every time it is included.
//!
//! [Expansion sources](struct.ExpansionSourceInfo.html) are how the `SourceMap` tracks macro
//! expansions. Instead of containing actual source code, the expansion source merely points to two
//! ranges, the _spelling range_ and the _expansion range_. The spelling range indicates where the
//! expanded code came from, while the expansion range indicates where the code was expanded. Both
//! the spelling and expansion ranges may themselves point into another expansion source, forming a
//! DAG of spellings and expansions.
//!
//! ## Expansion Examples
//!
//! Consider the following c code:
//!
//! ```c
//! #define A (2 + 3)
//! int x = A + 1;
//! ```
//!
//! The expansion of `A` on line 2 has a spelling range corresponding to the `(2 + 3)` on line 1 and
//! an expansion range covering the `A` on line 2.
//!
//! Nesting macros can cause expansion ranges to point into other expansions. In the following
//! example, there is an expansion of `B` on line 3, spelled at line 2. There is then an expansion
//! of `A` into the expansion of `B` spelled at line 1:
//!
//! ```c
//! #define A 4
//! #define B (A * 2)
//! int x = B;
//! ```
//!
//! Spelling ranges can also point into expansions when macros pass arguments to other macros.

use std::convert::TryFrom;
use std::ops::Range;
use std::option::Option;
use std::rc::Rc;
use std::vec::Vec;

use itertools::Itertools;

use crate::{FragmentedSourceRange, LineCol, SourcePos, SourceRange};
pub use source::{
    ExpansionSourceInfo, ExpansionType, FileContents, FileName, FileSourceInfo, Source, SourceInfo,
};

mod source;

#[cfg(test)]
mod tests;

/// An opaque identifier representing a source in a `SourceMap`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SourceId(usize);

/// A structure representing a line of source code with a highlighted range.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct LineSnippet<'f> {
    /// The line of code.
    pub line: &'f str,
    /// The (zero-based) line number.
    pub line_num: u32,
    /// The starting offset of the range within the line.
    pub off: u32,
    /// The length of the range.
    pub len: u32,
}

impl LineSnippet<'_> {
    /// Returns the highlighted range within the line.
    pub fn local_range(&self) -> Range<u32> {
        self.off..self.off + self.len
    }
}

/// Represents an interpreted range within a file, with easy access to filename, line and column
/// numbers.
#[derive(Clone, Copy)]
pub struct InterpretedFileRange<'f> {
    /// The file into which the range points.
    pub file: &'f FileSourceInfo,
    /// The starting offset of the range within the file.
    pub off: u32,
    /// The length of the range.
    pub len: u32,
}

impl<'f> InterpretedFileRange<'f> {
    /// Returns the range within the file that `self` occupies.
    pub fn local_range(&self) -> Range<u32> {
        self.off..self.off + self.len
    }

    /// Returns the name of the interpreted range's file.
    pub fn filename(&self) -> &FileName {
        &self.file.filename
    }

    /// Returns the include position of the interpreted range's file, if any.
    pub fn include_pos(&self) -> Option<SourcePos> {
        self.file.include_pos
    }

    /// Returns a reference to the contents of the interpreted range's file.
    pub fn contents(&self) -> &'f FileContents {
        &self.file.contents
    }

    /// Returns the line-column pair within the file at which the range starts.
    pub fn start_linecol(&self) -> LineCol {
        self.contents().get_linecol(self.off)
    }

    /// Returns the line-column pair within the file at which the range ends.
    pub fn end_linecol(&self) -> LineCol {
        self.contents().get_linecol(self.local_range().end)
    }

    /// Returns an iterator yielding the lines covered by this range, along with the appropriate
    /// pieces of the range.
    pub fn line_snippets(&self) -> impl Iterator<Item = LineSnippet<'f>> {
        let start_linecol = self.start_linecol();
        let end_linecol = self.end_linecol();

        self.contents()
            .get_lines(start_linecol.line, end_linecol.line)
            .lines()
            .zip(0..)
            .map(move |(line, idx)| {
                let last_line = end_linecol.line - start_linecol.line;

                let start = if idx == 0 { start_linecol.col } else { 0 };
                let end = if idx == last_line {
                    end_linecol.col
                } else {
                    line.len() as u32
                };

                LineSnippet {
                    line,
                    line_num: start_linecol.line + idx,
                    off: start,
                    len: end - start,
                }
            })
    }
}

/// Error type indicating that a source could not be added because there were not enough unused
/// positions to cover it.
#[derive(Debug)]
pub struct SourcesTooLargeError;

/// A structure holding the source code used in a compilation.
///
/// See the module-level documentation for a higher-level explanation of the `SourceMap`'s
/// architecture.
///
/// # Panics
///
/// Unless otherwise specified, all methods taking a `SourcePos` or `SourceRange` will panic if
/// provided an invalid value (i.e. one that does not lie in the map, or, in the case of ranges, one
/// that crosses source boundaries).
#[derive(Default)]
pub struct SourceMap {
    /// A flat list of the sources in the map. These are stored in order of increasing starting
    /// position, to enable binary search for position-based lookup.
    sources: Vec<Source>,
    /// The next offset available for use as a starting position.
    next_offset: u32,
}

impl SourceMap {
    /// Creates an empty `SourceMap`.
    pub fn new() -> Self {
        Default::default()
    }

    /// Adds a new source to the map, checking first that there is sufficient room for the specified
    /// length.
    ///
    /// If there is enough room in the map, `ctor` is invoked to create the info and the ID of the
    /// new source is returned. If there is no room for a source of the specified length, a
    /// `SourcesTooLargeError` is returned instead.
    fn add_source(
        &mut self,
        ctor: impl FnOnce() -> SourceInfo,
        len: u32,
    ) -> Result<SourceId, SourcesTooLargeError> {
        let off = self.next_offset;
        self.next_offset = off.checked_add(len).ok_or(SourcesTooLargeError)?;

        let range = SourceRange::new(SourcePos::from_raw(off), len);

        let id = SourceId(self.sources.len());
        self.sources.push(Source {
            info: Box::new(ctor()),
            range,
        });

        Ok(id)
    }

    /// Creates a new file source with the specified parameters.
    ///
    /// If there is enough room in the map for the file, returns the ID of the newly-created file
    /// source. Otherwise, returns a `SourcesTooLargeError`.
    ///
    /// The created file source will have an additional past-the-end sentinel position, useful for
    /// representing EOF positions and disambiguating empty sources from their successors.
    ///
    /// Note that a new file source should be created (potentially referencing the same `contents`)
    /// every time a file is included, as the `include_pos` is different and the filename may be
    /// spelled differently.
    pub fn create_file(
        &mut self,
        filename: FileName,
        contents: Rc<FileContents>,
        include_pos: Option<SourcePos>,
    ) -> Result<SourceId, SourcesTooLargeError> {
        let len = u32::try_from(contents.src.len())
            .map_err(|_| SourcesTooLargeError)?
            .checked_add(1) // Sentinel byte
            .ok_or(SourcesTooLargeError)?;

        self.add_source(
            || SourceInfo::File(FileSourceInfo::new(filename, contents, include_pos)),
            len,
        )
    }

    /// Creates a new expansion source with the specified parameters.
    ///
    /// If there is enough room in the map, returns the ID of the newly-created expansion source.
    /// Otherwise, returns a `SourcesTooLargeError`.
    ///
    /// # Panics
    ///
    /// This function may panic if one of `spelling_range` or `expansion_range` is invalid, or if
    /// either is empty.
    pub fn create_expansion(
        &mut self,
        spelling_range: SourceRange,
        expansion_range: SourceRange,
        expansion_type: ExpansionType,
    ) -> Result<SourceId, SourcesTooLargeError> {
        assert!(!spelling_range.is_empty());
        assert!(!expansion_range.is_empty());

        if cfg!(debug_assertions) {
            // Verify that the ranges do not cross source boundaries. Each of these checks incurs an
            // extra search through the list of sources, so avoid them in release builds.
            self.lookup_source_range(spelling_range);
            self.lookup_source_range(expansion_range);
        }

        self.add_source(
            || {
                SourceInfo::Expansion(ExpansionSourceInfo::new(
                    spelling_range,
                    expansion_range,
                    expansion_type,
                ))
            },
            spelling_range.len(),
        )
    }

    /// Gets a source by its ID.
    ///
    /// # Panics
    ///
    /// Panics if the map does not contain a source with the specified ID (can happen if `id` came
    /// from a different `SourceMap`).
    #[inline]
    pub fn get_source(&self, id: SourceId) -> &Source {
        &self.sources[id.0]
    }

    /// Looks up the ID of the source containing `pos`.
    pub fn lookup_source_id(&self, pos: SourcePos) -> SourceId {
        let last = self.sources.last().unwrap();
        assert!(pos <= last.range.end());

        SourceId(
            self.sources
                .binary_search_by_key(&pos, |source| source.range.start())
                .unwrap_or_else(|i| i - 1),
        )
    }

    /// Looks up the source containing `pos` and the offset at which `pos` lies within it.
    pub fn lookup_source_off(&self, pos: SourcePos) -> (&Source, u32) {
        let source = self.get_source(self.lookup_source_id(pos));
        let off = source.local_off(pos);
        (source, off)
    }

    /// Looks up the source containing `range` and local range that `range` occupies within it.
    pub fn lookup_source_range(&self, range: SourceRange) -> (&Source, Range<u32>) {
        let source = self.get_source(self.lookup_source_id(range.start()));
        let local_range = source.local_range(range);
        (source, local_range)
    }

    /// Creates an iterator listing the includer chain of the file containing `pos`, from innermost
    /// to outermost.
    ///
    /// The first item of this iterator is always `pos` itself. If `pos` points into an expansion,
    /// it is guaranteed to be the only item.
    pub fn get_includer_chain(
        &self,
        pos: SourcePos,
    ) -> impl Iterator<Item = (SourceId, SourcePos)> + '_ {
        get_location_chain(
            pos,
            move |pos| self.lookup_source_id(pos),
            move |id, _| {
                self.get_source(id)
                    .as_file()
                    .and_then(|file| file.include_pos)
            },
        )
    }

    /// If `pos` points into an expansion, returns the position within the recoreded spelling range
    /// corresponding to it.
    ///
    /// If `pos` points into a file, returns `None`.
    ///
    /// Note that this function will not retrieve the outermost spelling position, but merely the
    /// one into which the expansion source points.
    pub fn get_immediate_spelling_pos(&self, pos: SourcePos) -> Option<SourcePos> {
        let (source, off) = self.lookup_source_off(pos);
        source.as_expansion().map(|exp| exp.spelling_pos(off))
    }

    /// Creates an iterator listing the chain of spelling positions corresponding to `pos`, from
    /// innermost to outermost.
    ///
    /// The first item of this iterator is always `pos` itself, and the last item always points into
    /// a file.
    pub fn get_spelling_chain(
        &self,
        pos: SourcePos,
    ) -> impl Iterator<Item = (SourceId, SourcePos)> + '_ {
        get_location_chain(
            pos,
            move |pos| self.lookup_source_id(pos),
            move |id, pos| {
                let source = self.get_source(id);
                let off = source.local_off(pos);
                source.as_expansion().map(|exp| exp.spelling_pos(off))
            },
        )
    }

    /// Gets the outermost spelling position corresponding to `pos`, the one at which the source
    /// character at `pos` was actually written.
    ///
    /// This is always guaranteed to be a file position. If `pos` already points into a file, it is
    /// the spelling position.
    pub fn get_spelling_pos(&self, pos: SourcePos) -> SourcePos {
        self.get_spelling_chain(pos).last().unwrap().1
    }

    /// Retrieves the source code snippet indicated by `range`.
    pub fn get_spelling(&self, range: SourceRange) -> &str {
        let (id, pos) = self.get_spelling_chain(range.start()).last().unwrap();
        let source = self.get_source(id);
        let file = source.as_file().unwrap();
        let off = source.local_off(pos);
        file.contents.get_snippet(off..off + range.len())
    }

    /// If `range` points into an expansion, returns the recoreded expansion range.
    ///
    /// If `range` points into a file, returns `None`.
    ///
    /// Note that this function will not retrieve the outermost expansion range, but merely the
    /// one indicated by the expansion source.
    pub fn get_immediate_expansion_range(&self, range: SourceRange) -> Option<SourceRange> {
        let (source, _) = self.lookup_source_range(range);
        source.as_expansion().map(|exp| exp.expansion_range)
    }

    /// Creates an iterator listing the chain of expansion ranges corresponding to `range`, from
    /// innermost to outermost.
    ///
    /// The first item of this iterator is always `range` itself, and the last item always points
    /// into a file.
    pub fn get_expansion_chain(
        &self,
        range: SourceRange,
    ) -> impl Iterator<Item = (SourceId, SourceRange)> + '_ {
        get_location_chain(
            range,
            move |range| self.lookup_source_id(range.start()),
            move |id, _| {
                self.get_source(id)
                    .as_expansion()
                    .map(|exp| exp.expansion_range)
            },
        )
    }

    /// Gets the outermost expansion range corresponding to `range`.
    ///
    /// This is always guaranteed to be a file position. If `range` already points into a file, it
    /// is the expansion range.
    pub fn get_expansion_range(&self, range: SourceRange) -> SourceRange {
        self.get_expansion_chain(range).last().unwrap().1
    }

    /// If `range` points into an expansion, returns the matching
    /// [caller range](struct.ExpansionSourceInfo.html#method.caller_range).
    ///
    /// If `range` points into a file, returns `None`.
    ///
    /// Note that this function will not retrieve the outermost caller range, but merely the
    /// one indicated by the expansion source.
    pub fn get_immediate_caller_range(&self, range: SourceRange) -> Option<SourceRange> {
        let (source, local_range) = self.lookup_source_range(range);

        source
            .as_expansion()
            .map(|exp| exp.caller_range(local_range))
    }

    /// Creates an iterator listing the chain of caller ranges corresponding to `range`, from
    /// innermost to outermost.
    ///
    /// The first item of this iterator is always `range` itself, and the last item always points
    /// into a file.
    pub fn get_caller_chain(
        &self,
        range: SourceRange,
    ) -> impl Iterator<Item = (SourceId, SourceRange)> + '_ {
        get_location_chain(
            range,
            move |range| self.lookup_source_id(range.start()),
            move |id, range| {
                let source = self.get_source(id);
                let local_range = source.local_range(range);
                source
                    .as_expansion()
                    .map(|exp| exp.caller_range(local_range))
            },
        )
    }

    /// Gets the outermost caller range corresponding to `range`.
    ///
    /// This is always guaranteed to be a file position. If `range` already points into a file, it
    /// is the caller range.
    pub fn get_caller_range(&self, range: SourceRange) -> SourceRange {
        self.get_caller_chain(range).last().unwrap().1
    }

    /// Interpret the specified file range, returning a structure that makes it easy to access
    /// information such as filename, line/column information and surrounding code snippets.
    ///
    /// # Panics
    ///
    /// Panics if `range` does not point into a file. Consider using
    /// [`get_spelling_pos()`](#method.get_spelling_pos) or
    /// [`get_expansion_range()`](#method.get_expansion_range) first, as appropriate.
    pub fn get_interpreted_range(&self, range: SourceRange) -> InterpretedFileRange<'_> {
        let (source, local_range) = self.lookup_source_range(range);

        InterpretedFileRange {
            file: source.as_file().unwrap(),
            off: local_range.start,
            len: range.len(),
        }
    }

    fn get_expansion_pos_chain<'a, F>(
        &'a self,
        pos: SourcePos,
        extract_pos: F,
    ) -> impl Iterator<Item = (SourceId, SourcePos)> + 'a
    where
        F: Fn(SourceRange) -> SourcePos + 'a,
    {
        self.get_expansion_chain(pos.into())
            .map(move |(id, range)| (id, extract_pos(range)))
    }

    /// Walks up the expansion chain and attempts to find a contiguous range covering both endpoints
    /// of `range`.
    ///
    /// Effectively, this function searches for the lowest common ancestor of `range.start` and
    /// `range.end` in the expansion forest. If the two endpoints lie in different files, `None` is
    /// returned.
    ///
    /// # Panics
    ///
    /// Panics if the returned range would end before it starts.
    ///
    /// # Example
    ///
    /// ```c
    /// #define A (2 + 3)
    /// int x = A + 1;
    /// ```
    ///
    /// Consider the fragmented range starting at the start of the expansion of `A` and ending at
    /// the `1`. Its corresponding unfragmented range is the range covering the `A + 1` as written
    /// on line 2.
    pub fn get_unfragmented_range(&self, range: FragmentedSourceRange) -> Option<SourceRange> {
        let start_sources: Vec<_> = self
            .get_expansion_pos_chain(range.start, SourceRange::start)
            .collect();

        let end_sources: Vec<_> = self
            .get_expansion_pos_chain(range.end, SourceRange::end)
            .collect();

        // Compute the LCA by walking down from the root to the farthest common file.
        let (start_pos, end_pos) = start_sources
            .iter()
            .rev()
            .zip(end_sources.iter().rev())
            .fold(None, |prev, ((start_id, start_pos), (end_id, end_pos))| {
                if start_id == end_id {
                    Some((*start_pos, *end_pos))
                } else {
                    prev
                }
            })?;

        assert!(start_pos <= end_pos, "invalid source range");
        Some(SourceRange::new(start_pos, end_pos.offset_from(start_pos)))
    }
}

/// Creates an iterator that repeatedly invokes `lookup_id` and `next` until `None` is returned.
fn get_location_chain<T, L, N>(
    init: T,
    lookup_id: L,
    next: N,
) -> impl Iterator<Item = (SourceId, T)>
where
    T: Copy,
    L: Fn(T) -> SourceId,
    N: Fn(SourceId, T) -> Option<T>,
{
    itertools::iterate(Some((lookup_id(init), init)), move |cur| {
        cur.and_then(|(id, val)| next(id, val).map(|next_val| (lookup_id(next_val), next_val)))
    })
    .while_some()
}
