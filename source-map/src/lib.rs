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
struct SourceMapInner {
    sources: Vec<Source>,
    next_offset: u32,
}

#[derive(Default)]
pub struct SourceMap {
    inner: RefCell<SourceMapInner>,
}

fn get_location_chain<'a, T, F, G>(
    init: T,
    lookup_id: F,
    next: G,
) -> impl Iterator<Item = (SourceId, T)> + 'a
where
    T: Copy + 'a,
    F: Fn(T) -> SourceId + 'a,
    G: Fn(SourceId, T) -> Option<T> + 'a,
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
        &self,
        ctor: impl FnOnce() -> SourceInfo,
        len: u32,
    ) -> Result<SourceId, SourcesTooLargeError> {
        let mut inner = self.inner.borrow_mut();

        let off = inner.next_offset;
        inner.next_offset = off
            .checked_add(len)
            .and_then(|val| val.checked_add(1))
            .ok_or(SourcesTooLargeError)?;

        let range = SourceRange::new(SourcePos::from_raw(off), len);

        let id = SourceId(inner.sources.len());
        inner.sources.push(Source {
            info: ctor(),
            range,
        });

        Ok(id)
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

        self.add_source(
            || SourceInfo::File(FileSourceInfo::new(contents, include_pos)),
            len,
        )
    }

    pub fn create_expansion(
        &self,
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

    pub fn get_source(&self, id: SourceId) -> Ref<'_, Source> {
        Ref::map(self.inner.borrow(), |inner| &inner.sources[id.0])
    }

    pub fn lookup_source_id(&self, pos: SourcePos) -> SourceId {
        let offset = pos.to_raw();
        let sources = Ref::map(self.inner.borrow(), |inner| &inner.sources);

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
        let off = source.local_off(pos);
        (source, off)
    }

    pub fn lookup_source_range(&self, range: SourceRange) -> (Ref<'_, Source>, Range<u32>) {
        let source = self.get_source(self.lookup_source_id(range.start()));
        let local_range = source.local_range(range);
        (source, local_range)
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
                let range = source.local_range(range);
                source.as_expansion().map(|exp| exp.caller_range(range))
            },
        )
    }

    pub fn get_caller_range(&self, range: SourceRange) -> SourceRange {
        self.get_caller_chain(range).last().unwrap().1
    }

    pub fn get_interpreted_range(&self, range: SourceRange) -> InterpretedFileRange<'_> {
        let (source, local_range) = self.lookup_source_range(range);

        InterpretedFileRange {
            file: Ref::map(source, |raw_source| raw_source.as_file().unwrap()),
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
        let start_sources: FxHashMap<_, _> = self
            .get_expansion_pos_chain(range.start, SourceRange::start)
            .collect();

        let (start_pos, end_pos) = self
            .get_expansion_pos_chain(range.end, SourceRange::end)
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
