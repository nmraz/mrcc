use std::convert::TryInto;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SourceId(usize);

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct LineSnippet<'f> {
    pub line: &'f str,
    pub line_num: u32,
    pub off: u32,
    pub len: u32,
}

impl LineSnippet<'_> {
    pub fn local_range(&self) -> Range<u32> {
        self.off..self.off + self.len
    }
}

#[derive(Clone, Copy)]
pub struct InterpretedFileRange<'f> {
    pub file: &'f FileSourceInfo,
    pub off: u32,
    pub len: u32,
}

impl<'f> InterpretedFileRange<'f> {
    pub fn local_range(&self) -> Range<u32> {
        self.off..self.off + self.len
    }

    pub fn filename(&self) -> &FileName {
        &self.file.filename
    }

    pub fn include_pos(&self) -> Option<SourcePos> {
        self.file.include_pos
    }

    pub fn contents(&self) -> &'f FileContents {
        &self.file.contents
    }

    pub fn start_linecol(&self) -> LineCol {
        self.contents().get_linecol(self.off)
    }

    pub fn end_linecol(&self) -> LineCol {
        self.contents().get_linecol(self.local_range().end)
    }

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

#[derive(Debug)]
pub struct SourcesTooLargeError;

#[derive(Default)]
pub struct SourceMap {
    sources: Vec<Source>,
    next_offset: u32,
}

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

impl SourceMap {
    pub fn new() -> Self {
        Default::default()
    }

    fn add_source(
        &mut self,
        ctor: impl FnOnce() -> SourceInfo,
        len: u32,
    ) -> Result<SourceId, SourcesTooLargeError> {
        // Make room for an extra past-the-end character, useful for end-of-file positions and
        // disambiguation of empty sources.
        let extended_len = len.checked_add(1).ok_or(SourcesTooLargeError)?;
        let off = self.next_offset;

        self.next_offset = off.checked_add(extended_len).ok_or(SourcesTooLargeError)?;

        let range = SourceRange::new(SourcePos::from_raw(off), extended_len);

        let id = SourceId(self.sources.len());
        self.sources.push(Source {
            info: ctor(),
            range,
        });

        Ok(id)
    }

    pub fn create_file(
        &mut self,
        filename: FileName,
        contents: Rc<FileContents>,
        include_pos: Option<SourcePos>,
    ) -> Result<SourceId, SourcesTooLargeError> {
        let len = contents
            .src
            .len()
            .try_into()
            .map_err(|_| SourcesTooLargeError)?;

        self.add_source(
            || SourceInfo::File(FileSourceInfo::new(filename, contents, include_pos)),
            len,
        )
    }

    pub fn create_expansion(
        &mut self,
        spelling_range: SourceRange,
        expansion_range: SourceRange,
        expansion_type: ExpansionType,
    ) -> Result<SourceId, SourcesTooLargeError> {
        if cfg!(debug_assertions) {
            // Verify that the ranges do not cross source boundaries. Each of these checks incurs an
            // extra search through the list of sources, so avoid them in release builds.
            self.lookup_source_range(spelling_range);
            self.lookup_source_range(expansion_range);
        }

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

    #[inline]
    pub fn get_source(&self, id: SourceId) -> &Source {
        &self.sources[id.0]
    }

    pub fn lookup_source_id(&self, pos: SourcePos) -> SourceId {
        let last = self.sources.last().unwrap();
        assert!(pos <= last.range.end());

        SourceId(
            self.sources
                .binary_search_by_key(&pos, |source| source.range.start())
                .unwrap_or_else(|i| i - 1),
        )
    }

    pub fn lookup_source_off(&self, pos: SourcePos) -> (&Source, u32) {
        let source = self.get_source(self.lookup_source_id(pos));
        let off = source.local_off(pos);
        (source, off)
    }

    pub fn lookup_source_range(&self, range: SourceRange) -> (&Source, Range<u32>) {
        let source = self.get_source(self.lookup_source_id(range.start()));
        let local_range = source.local_range(range);
        (source, local_range)
    }

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

    pub fn get_immediate_spelling_pos(&self, pos: SourcePos) -> Option<SourcePos> {
        let (source, off) = self.lookup_source_off(pos);
        source.as_expansion().map(|exp| exp.spelling_pos(off))
    }

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

    pub fn get_spelling_pos(&self, pos: SourcePos) -> SourcePos {
        self.get_spelling_chain(pos).last().unwrap().1
    }

    pub fn get_spelling(&self, range: SourceRange) -> &str {
        let (id, pos) = self.get_spelling_chain(range.start()).last().unwrap();
        let source = self.get_source(id);
        let file = source.as_file().unwrap();
        let off = source.local_off(pos);
        file.contents.get_snippet(off..off + range.len())
    }

    pub fn get_immediate_expansion_range(&self, range: SourceRange) -> Option<SourceRange> {
        let (source, _) = self.lookup_source_range(range);
        source.as_expansion().map(|exp| exp.expansion_range)
    }

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

    pub fn get_expansion_range(&self, range: SourceRange) -> SourceRange {
        self.get_expansion_chain(range).last().unwrap().1
    }

    pub fn get_immediate_caller_range(&self, range: SourceRange) -> Option<SourceRange> {
        let (source, local_range) = self.lookup_source_range(range);

        source
            .as_expansion()
            .map(|exp| exp.caller_range(local_range))
    }

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

    pub fn get_caller_range(&self, range: SourceRange) -> SourceRange {
        self.get_caller_chain(range).last().unwrap().1
    }

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

    pub fn get_unfragmented_range(&self, range: FragmentedSourceRange) -> SourceRange {
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
            })
            .expect("fragmented source range spans multiple files");

        assert!(start_pos <= end_pos, "invalid source range");
        SourceRange::new(start_pos, end_pos.offset_from(start_pos))
    }
}
