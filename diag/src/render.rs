use std::cmp;
use std::hash::BuildHasherDefault;

use indexmap::IndexMap;
use itertools::Itertools;
use rustc_hash::FxHasher;

use source_map::pos::{FragmentedSourceRange, SourceRange};
use source_map::{ExpansionType, SourceId, SourceMap};

use crate::{Ranges, RawRanges, RenderedRanges};
use crate::{RawDiagnostic, RenderedDiagnostic};
use crate::{RawSubDiagnostic, RenderedSubDiagnostic, SubDiagnostic};
use crate::{RawSuggestion, RenderedSuggestion};

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

fn render_suggestion(suggestion: &RawSuggestion, smap: &SourceMap) -> Option<RenderedSuggestion> {
    let range = suggestion.replacement_range;
    let start_id = smap.lookup_source_id(range.start);
    let end_id = smap.lookup_source_id(range.end);

    // Suggestions don't play very well with expansions - it is unclear exactly *where* along the
    // expansion stack the suggestion should be applied, and sometimes there is no good way to apply
    // the suggestion without restructuring other code; conservatively bail out for now.
    if !smap.get_source(start_id).is_file() || !smap.get_source(end_id).is_file() {
        return None;
    }

    assert!(start_id == end_id, "Suggestion spans multiple files");

    Some(RenderedSuggestion {
        replacement_range: SourceRange::new(range.start, range.end.offset_from(range.start)),
        insert_text: suggestion.insert_text.clone(),
    })
}

fn render_stripped_subdiag(raw: &RawSubDiagnostic) -> RenderedSubDiagnostic {
    RenderedSubDiagnostic {
        inner: SubDiagnostic {
            msg: raw.msg.clone(),
            ranges: None,
            suggestions: Vec::new(),
        },
        expansions: Vec::new(),
    }
}

fn render_subdiag(raw: &RawSubDiagnostic, smap: &SourceMap) -> RenderedSubDiagnostic {
    match &raw.ranges {
        None => render_stripped_subdiag(raw),
        Some(ranges) => {
            let (primary_ranges, expansion_ranges) = render_ranges(ranges, smap);
            let rendered_suggestions: Vec<_> = raw
                .suggestions
                .iter()
                .filter_map(|sugg| render_suggestion(sugg, smap))
                .collect();

            RenderedSubDiagnostic {
                inner: SubDiagnostic {
                    msg: raw.msg.clone(),
                    ranges: Some(primary_ranges),
                    suggestions: rendered_suggestions,
                },
                expansions: expansion_ranges,
            }
        }
    }
}

fn render_with<'s>(
    raw: &RawDiagnostic<'s>,
    mut f: impl FnMut(&RawSubDiagnostic) -> RenderedSubDiagnostic,
) -> RenderedDiagnostic<'s> {
    RenderedDiagnostic {
        level: raw.level,
        main: f(&raw.main),
        notes: raw.notes.iter().map(f).collect(),
        smap: raw.smap,
    }
}

pub fn render<'s>(raw: &RawDiagnostic<'s>) -> RenderedDiagnostic<'s> {
    raw.smap.map_or_else(
        || render_with(raw, render_stripped_subdiag),
        |smap| render_with(raw, |subdiag| render_subdiag(subdiag, smap)),
    )
}
