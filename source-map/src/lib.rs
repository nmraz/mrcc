#![warn(clippy::all)]

use itertools::Itertools;
use rustc_hash::FxHashMap;
use std::cell::{Ref, RefCell};
use std::convert::TryInto;
use std::ops::Range;
use std::option::Option;
use std::rc::Rc;
use std::vec::Vec;

pub mod pos;
mod source;

#[cfg(test)]
mod tests;

use pos::{FragmentedSourceRange, LineCol, SourcePos, SourceRange};
pub use source::{
    ExpansionSourceInfo, ExpansionType, FileContents, FileName, FileSourceInfo, Source, SourceInfo,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SourceId(usize);

pub struct InterpretedFileRange<'f> {
    pub file: Ref<'f, FileSourceInfo>,
    pub off: u32,
    pub len: u32,
}

impl InterpretedFileRange<'_> {
    pub fn local_range(&self) -> Range<u32> {
        self.off..self.off + self.len
    }

    pub fn filename(&self) -> &FileName {
        &self.file.contents.filename
    }

    pub fn include_pos(&self) -> Option<SourcePos> {
        self.file.include_pos
    }

    pub fn start_linecol(&self) -> LineCol {
        self.file.contents.get_linecol(self.off)
    }

    pub fn end_linecol(&self) -> LineCol {
        self.file.contents.get_linecol(self.local_range().end)
    }
}

#[derive(Debug)]
pub struct SourcesTooLargeError;

#[derive(Default)]
pub struct SourceMap {
    sources: RefCell<Vec<Source>>,
}

fn get_location_chain<'sm, T, F>(init: T, f: F) -> impl Iterator<Item = T> + 'sm
where
    T: Copy + 'sm,
    F: Fn(T) -> Option<T> + 'sm,
{
    itertools::iterate(Some(init), move |cur| cur.and_then(&f)).while_some()
}

impl SourceMap {
    pub fn new() -> Self {
        Default::default()
    }

    fn add_source(&self, ctor: impl FnOnce() -> SourceInfo, len: u32) -> SourceId {
        let mut sources = self.sources.borrow_mut();

        let off = sources
            .last()
            .map_or(0, |source| source.range.end().to_raw() + 1);
        let range = SourceRange::new(SourcePos::from_raw(off), len);

        let id = SourceId(sources.len());
        sources.push(Source {
            info: ctor(),
            range,
        });

        id
    }

    pub fn create_file(
        &self,
        contents: Rc<FileContents>,
        include_pos: Option<SourcePos>,
    ) -> Result<SourceId, SourcesTooLargeError> {
        let len = contents
            .src
            .len()
            .try_into()
            .map_err(|_| SourcesTooLargeError)?;

        Ok(self.add_source(
            || SourceInfo::File(FileSourceInfo::new(contents, include_pos)),
            len,
        ))
    }

    pub fn create_expansion(
        &self,
        spelling_range: SourceRange,
        expansion_range: SourceRange,
        expansion_type: ExpansionType,
    ) -> SourceId {
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

    pub fn get_source(&self, id: SourceId) -> Ref<'_, Source> {
        Ref::map(self.sources.borrow(), |sources| &sources[id.0])
    }

    pub fn lookup_source_id(&self, pos: SourcePos) -> SourceId {
        let offset = pos.to_raw();
        let sources = self.sources.borrow();

        let last = sources.last().unwrap();
        assert!(offset <= last.range.end().to_raw());

        SourceId(
            sources
                .binary_search_by_key(&offset, |source| source.range.start().to_raw())
                .unwrap_or_else(|i| i - 1),
        )
    }

    pub fn lookup_source_off(&self, pos: SourcePos) -> (Ref<'_, Source>, u32) {
        let source = self.get_source(self.lookup_source_id(pos));
        let off = pos.offset_from(source.range.start());
        (source, off)
    }

    pub fn lookup_source_range(&self, range: SourceRange) -> (Ref<'_, Source>, Range<u32>) {
        let source = self.get_source(self.lookup_source_id(range.start()));

        assert!(source.range.contains_range(range), "invalid source range");

        let off = range.start().offset_from(source.range.start());
        (source, off..off + range.len())
    }

    pub fn get_immediate_spelling_pos(&self, pos: SourcePos) -> Option<SourcePos> {
        let (source, offset) = self.lookup_source_off(pos);

        source
            .as_expansion()
            .map(|exp| exp.spelling_pos.offset(offset))
    }

    pub fn get_spelling_chain<'sm>(
        &'sm self,
        pos: SourcePos,
    ) -> impl Iterator<Item = SourcePos> + 'sm {
        get_location_chain(pos, move |cur| self.get_immediate_spelling_pos(cur))
    }

    pub fn get_spelling_pos(&self, pos: SourcePos) -> SourcePos {
        self.get_spelling_chain(pos).last().unwrap()
    }

    pub fn get_immediate_expansion_range(&self, range: SourceRange) -> Option<SourceRange> {
        let (source, _) = self.lookup_source_range(range);
        source.as_expansion().map(|exp| exp.expansion_range)
    }

    pub fn get_expansion_chain<'sm>(
        &'sm self,
        range: SourceRange,
    ) -> impl Iterator<Item = SourceRange> + 'sm {
        get_location_chain(range, move |cur| self.get_immediate_expansion_range(cur))
    }

    pub fn get_expansion_range(&self, range: SourceRange) -> SourceRange {
        self.get_expansion_chain(range).last().unwrap()
    }

    pub fn get_immediate_caller_range(&self, range: SourceRange) -> Option<SourceRange> {
        let (source, local_range) = self.lookup_source_range(range);

        source
            .as_expansion()
            .map(|exp| exp.caller_range(local_range))
    }

    pub fn get_caller_chain<'sm>(
        &'sm self,
        range: SourceRange,
    ) -> impl Iterator<Item = SourceRange> + 'sm {
        get_location_chain(range, move |cur| self.get_immediate_caller_range(cur))
    }

    pub fn get_caller_range(&self, range: SourceRange) -> SourceRange {
        self.get_caller_chain(range).last().unwrap()
    }

    pub fn get_interpreted_range(&self, range: SourceRange) -> InterpretedFileRange<'_> {
        let caller_range = self.get_caller_range(range);
        let (source, off) = self.lookup_source_off(caller_range.start());

        InterpretedFileRange {
            file: Ref::map(source, |raw_source| raw_source.as_file().unwrap()),
            off,
            len: caller_range.len(),
        }
    }

    fn get_expansion_source_offs<'sm, F>(
        &'sm self,
        pos: SourcePos,
        extract_pos: F,
    ) -> impl Iterator<Item = (SourceId, SourcePos)> + 'sm
    where
        F: Fn(SourceRange) -> SourcePos + 'sm,
    {
        self.get_expansion_chain(SourceRange::new(pos, 0))
            .map(move |range| {
                let pos = extract_pos(range);
                (self.lookup_source_id(pos), pos)
            })
    }

    pub fn get_unfragmented_range(&self, range: FragmentedSourceRange) -> SourceRange {
        let start_sources: FxHashMap<_, _> = self
            .get_expansion_source_offs(range.start, SourceRange::start)
            .collect();

        let (start_pos, end_pos) = self
            .get_expansion_source_offs(range.end, SourceRange::end)
            .find_map(|(id, end_pos)| {
                start_sources
                    .get(&id)
                    .map(|&start_pos| (start_pos, end_pos))
            })
            .expect("fragmented source range spans multiple files");

        assert!(
            start_pos.to_raw() <= end_pos.to_raw(),
            "invalid source range"
        );

        SourceRange::new(start_pos, end_pos.offset_from(start_pos))
    }
}
