use itertools::Itertools;
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
pub use source::{
    ExpansionSourceInfo, ExpansionType, FileContents, FileName, FileSourceInfo, Source, SourceInfo,
};

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

    pub fn filename(&self) -> &'f FileName {
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

pub struct SourceManager {
    sources: RefCell<Vec<Box<Source>>>,
}

fn get_location_chain<'sm, T, F>(init: T, f: F) -> impl Iterator<Item = T> + 'sm
where
    T: Copy + 'sm,
    F: Fn(T) -> Option<T> + 'sm,
{
    itertools::iterate(Some(init), move |cur| cur.and_then(&f)).while_some()
}

impl SourceManager {
    pub fn new() -> Self {
        SourceManager {
            sources: RefCell::new(vec![]),
        }
    }

    fn add_source(&self, ctor: impl FnOnce() -> SourceInfo, len: u32) -> &Source {
        let mut sources = self.sources.borrow_mut();

        let offset = sources
            .last()
            .map_or(0, |source| source.range.end().to_raw() + 1);

        let boxed = Source::new(ctor(), SourceRange::new(SourcePos::from_raw(offset), len));

        // Safety: the boxed sources are never dropped or reseated after being added
        let source = unsafe { &*(&*boxed as *const _) };
        sources.push(boxed);
        source
    }

    pub fn create_file(
        &self,
        contents: Rc<FileContents>,
        include_pos: Option<SourcePos>,
    ) -> Result<&Source, SourcesTooLargeError> {
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
    ) -> &Source {
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

    pub fn lookup_source(&self, pos: SourcePos) -> &Source {
        let offset = pos.to_raw();
        let sources = self.sources.borrow();

        let last = sources.last().unwrap();
        assert!(offset <= last.range.end().to_raw());

        let idx = sources
            .binary_search_by_key(&offset, |source| source.range.start().to_raw())
            .unwrap_or_else(|i| i - 1);

        // Safety: the boxed sources are never dropped or reseated after being added
        unsafe { &*(&*sources[idx] as *const _) }
    }

    fn lookup_range_source(&self, range: SourceRange) -> &Source {
        let source = self.lookup_source(range.start());
        assert!(source.range.contains_range(range), "invalid source range");
        source
    }

    fn check_range(&self, range: SourceRange) {
        self.lookup_range_source(range);
    }

    pub fn lookup_source_off(&self, pos: SourcePos) -> (&Source, u32) {
        let source = self.lookup_source(pos);
        let off = pos.offset_from(source.range.start());
        (source, off)
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
        self.lookup_range_source(range)
            .as_expansion()
            .map(|exp| exp.expansion_range)
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
        let source = self.lookup_range_source(range);
        let off = range.start().offset_from(source.range.start());

        source.as_expansion().map(|exp| {
            // For macro arguments, the caller is where the argument was spelled, while for
            // everything else the caller recieves the expansion.
            match exp.expansion_type {
                ExpansionType::MacroArg => exp.get_spelling_range(off, range.len()),
                _ => exp.expansion_range,
            }
        })
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

    pub fn get_interpreted_range(&self, range: SourceRange) -> InterpretedFileRange {
        let caller_range = self.get_caller_range(range);
        let (source, start_off) = self.lookup_source_off(caller_range.start());

        InterpretedFileRange {
            file: source.as_file().unwrap(),
            off: start_off,
            len: caller_range.len(),
        }
    }

    fn get_expansion_source_offs<'sm, F>(
        &'sm self,
        pos: SourcePos,
        extract_pos: F,
    ) -> impl Iterator<Item = (SourcePos, u32)> + 'sm
    where
        F: Fn(&SourceRange) -> SourcePos + 'sm,
    {
        self.get_expansion_chain(SourceRange::new(pos, 0))
            .map(move |range| {
                let (source, off) = self.lookup_source_off(extract_pos(&range));
                (source.range.start(), off)
            })
    }

    pub fn get_unfragmented_range(&self, range: FragmentedSourceRange) -> SourceRange {
        let start_source_offs: FxHashMap<_, _> = self
            .get_expansion_source_offs(range.start, SourceRange::start)
            .collect();

        let (lca_source, start_off, end_off) = self
            .get_expansion_source_offs(range.end, SourceRange::end)
            .find_map(|(source, end_off)| {
                start_source_offs
                    .get(&source)
                    .map(|&start_off| (source, start_off, end_off))
            })
            .expect("fragmented source range spans multiple files");

        assert!(start_off <= end_off, "invalid source range");

        SourceRange::new(lca_source.offset(start_off), end_off - start_off)
    }
}
