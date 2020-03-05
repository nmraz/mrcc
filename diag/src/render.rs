use std::cmp;
use std::hash::BuildHasherDefault;

use indexmap::IndexMap;
use itertools::Itertools;
use rustc_hash::FxHasher;

use source_map::pos::{FragmentedSourceRange, SourcePos, SourceRange};
use source_map::{ExpansionType, SourceId, SourceMap};

use crate::{Ranges, RawRanges, RenderedRanges};
use crate::{RawDiagnostic, RenderedDiagnostic};
use crate::{RawSubDiagnostic, RenderedSubDiagnostic, SubDiagnostic};

fn render_stripped(raw: &RawSubDiagnostic) -> RenderedSubDiagnostic {
    RenderedSubDiagnostic {
        inner: SubDiagnostic {
            msg: raw.msg.clone(),
            ranges: None,
            suggestions: Vec::new(),
        },
        expansions: Vec::new(),
    }
}

fn trace_expansions(
    range: FragmentedSourceRange,
    smap: &SourceMap,
) -> impl Iterator<Item = (SourceId, SourceRange)> + '_ {
    smap.get_caller_chain(smap.get_unfragmented_range(range))
        .map(move |(id, range)| {
            let source = smap.get_source(id);

            // Shift macro arguments to their use in the macro
            if let Some(exp) = source.as_expansion() {
                if exp.expansion_type == ExpansionType::MacroArg {
                    let use_range = exp.expansion_range;
                    return (smap.lookup_source_id(use_range.start()), use_range);
                }
            }

            (id, range)
        })
}

fn dedup_subranges(
    mut subranges: Vec<(SourceRange, String)>,
) -> impl Iterator<Item = (SourceRange, String)> {
    subranges.sort_unstable_by_key(|(range, _)| range.start());
    subranges.into_iter().coalesce(|(ra, la), (rb, lb)| {
        if ra.end() > rb.start() {
            let start = ra.start();
            let end = cmp::max(ra.end(), rb.end());
            Ok((
                SourceRange::new(start, end.offset_from(start)),
                "".to_owned(),
            ))
        } else {
            Err(((ra, la), (rb, lb)))
        }
    })
}

fn get_spelling_range(smap: &SourceMap, range: SourceRange) -> SourceRange {
    SourceRange::new(smap.get_spelling_pos(range.start()), range.len())
}

fn render_ranges(ranges: &RawRanges, smap: &SourceMap) -> (RenderedRanges, Vec<RenderedRanges>) {
    type FxIndexMap<K, V> = IndexMap<K, V, BuildHasherDefault<FxHasher>>;

    let mut expansion_map: FxIndexMap<_, _> = trace_expansions(ranges.primary_range, smap)
        .map(|(id, range)| (id, RenderedRanges::new(range)))
        .collect();

    for (range, label) in ranges.subranges.iter() {
        for (id, trace_range) in trace_expansions(*range, smap) {
            if let Some(RenderedRanges { subranges, .. }) = expansion_map.get_mut(&id) {
                subranges.push((trace_range, label.clone()))
            }
        }
    }

    let mut expansions: Vec<_> = expansion_map
        .into_iter()
        .map(|(_, ranges)| {
            let Ranges {
                primary_range,
                subranges,
            } = ranges;

            // We currently don't attempt to merge the primary range with subranges, even when there
            // may be overlap, as the primary range has special status and may be rendered
            // differently.

            RenderedRanges {
                primary_range: get_spelling_range(smap, primary_range),
                subranges: dedup_subranges(subranges)
                    .map(|(range, label)| (get_spelling_range(smap, range), label))
                    .collect(),
            }
        })
        .collect();

    // Expansions are currently listed from innermost to outermost (as returned by
    // `trace_expansions`), but we want them from outermost to innermost, with the outermost one
    // being the "primary" expansion at which the diagnostic is reported.

    let outermost = expansions.pop().unwrap();
    expansions.reverse();

    (outermost, expansions)
}

fn do_render(raw: &RawSubDiagnostic, smap: &SourceMap) -> RenderedSubDiagnostic {
    match &raw.ranges {
        None => render_stripped(raw),
        Some(ranges) => unimplemented!(),
    }
}
