use std::cmp;
use std::hash::BuildHasherDefault;

use indexmap::IndexMap;
use itertools::Itertools;
use rustc_hash::FxHasher;

use crate::smap::{ExpansionKind, SourceId};
use crate::SourceMap;
use crate::SourceRange;

use super::{Diagnostic, RawDiagnostic, RenderedDiagnostic};
use super::{Ranges, RawRanges, RenderedRanges};
use super::{RawSubDiagnostic, RenderedSubDiagnostic, SubDiagnostic};
use super::{RawSuggestion, RenderedSuggestion};

/// Returns an iterator tracing through the expansions of `range`.
///
/// This is almost like the caller chain, except that ranges in macro arguments are moved to point
/// into the macros themselves.
fn trace_expansions(
    range: SourceRange,
    smap: &SourceMap,
) -> impl Iterator<Item = (SourceId, SourceRange)> + '_ {
    smap.get_caller_chain(range).map(move |(id, range)| {
        let source = smap.get_source(id);

        // Shift macro arguments to their use in the macro
        if let Some(exp) = source.as_expansion() {
            if exp.kind == ExpansionKind::MacroArg {
                let use_range = exp.replacement_range;
                return (smap.lookup_source_id(use_range.start()), use_range);
            }
        }

        (id, range)
    })
}

/// Deduplicates the provided subranges and coalesces overlapping ones.
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

/// Retrieves the spelling range of `range` in the source map.
fn get_spelling_range(smap: &SourceMap, range: SourceRange) -> SourceRange {
    SourceRange::new(smap.get_spelling_pos(range.start()), range.len())
}

/// Renders the provided ranges, returning the newly-rendered (outermost) ranges and a trace of the
/// expansions leading up to them, ordered from outermost to innermost.
fn render_ranges(ranges: &RawRanges, smap: &SourceMap) -> (RenderedRanges, Vec<RenderedRanges>) {
    type FxIndexMap<K, V> = IndexMap<K, V, BuildHasherDefault<FxHasher>>;

    // We always need a primary range, so arbitrarily fall back to the start if it spans multiple
    // files.
    let primary_range = smap
        .get_unfragmented_range(ranges.primary_range)
        .unwrap_or_else(|| ranges.primary_range.start.into());

    let mut expansion_map: FxIndexMap<_, _> = trace_expansions(primary_range, smap)
        .map(|(id, range)| (id, RenderedRanges::new(range)))
        .collect();

    for (range, label) in ranges.subranges.iter() {
        let expansions = smap
            .get_unfragmented_range(*range)
            .into_iter()
            .flat_map(|range| trace_expansions(range, smap));

        for (id, trace_range) in expansions {
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

/// Attemts to render the specified suggestion, returning `None` if there was no unambiguous or
/// meaningful way to do so.
fn render_suggestion(suggestion: &RawSuggestion, smap: &SourceMap) -> Option<RenderedSuggestion> {
    let range = suggestion.replacement_range;
    let start_id = smap.lookup_source_id(range.start);
    let end_id = smap.lookup_source_id(range.end);

    // Suggestions don't play very well with expansions - it is unclear exactly *where* along the
    // expansion stack the suggestion should be applied, and sometimes there is no good way to apply
    // the suggestion without restructuring other code; conservatively bail out for now.
    if start_id != end_id || !smap.get_source(start_id).is_file() {
        return None;
    }

    Some(RenderedSuggestion {
        replacement_range: SourceRange::new(range.start, range.end.offset_from(range.start)),
        insert_text: suggestion.insert_text.clone(),
    })
}

/// Renders a subdiagnostic with no location information.
fn render_anon_subdiag(raw: &RawSubDiagnostic) -> RenderedSubDiagnostic {
    RenderedSubDiagnostic {
        inner: SubDiagnostic {
            msg: raw.msg.clone(),
            ranges: None,
            suggestion: None,
        },
        expansions: Vec::new(),
    }
}

/// Renders the provided subdiagnostic using the source map.
fn render_subdiag(raw: &RawSubDiagnostic, smap: &SourceMap) -> RenderedSubDiagnostic {
    match &raw.ranges {
        None => render_anon_subdiag(raw),
        Some(ranges) => {
            let (primary_ranges, expansion_ranges) = render_ranges(ranges, smap);
            let rendered_suggestion = raw
                .suggestion
                .as_ref()
                .and_then(|sugg| render_suggestion(sugg, smap));

            RenderedSubDiagnostic {
                inner: SubDiagnostic {
                    msg: raw.msg.clone(),
                    ranges: Some(primary_ranges),
                    suggestion: rendered_suggestion,
                },
                expansions: expansion_ranges,
            }
        }
    }
}

/// Renders `raw` by invoking `f` on each of its subdiagnostics to obtain a rendered subdiagnostic.
fn render_with<'s>(
    raw: &RawDiagnostic<'s>,
    mut f: impl FnMut(&RawSubDiagnostic) -> RenderedSubDiagnostic,
) -> Diagnostic<'s, RenderedSubDiagnostic> {
    Diagnostic {
        level: raw.level,
        main: f(&raw.main),
        notes: raw.notes.iter().map(f).collect(),
        smap: raw.smap,
    }
}

/// Renders the provided raw diagnostic, using the contained location information and source map to
/// resolve expansions and include traces.
///
/// If the diagnostic has no source map, the rendered diagnostic will have no location information
/// attached, even if the original did.
///
/// # Panics
///
/// This function may panic if any of the ranges in `raw` is invalid or malformed.
pub fn render<'s>(raw: &RawDiagnostic<'s>) -> RenderedDiagnostic<'s> {
    match raw.smap {
        Some(smap) => {
            let inner = render_with(raw, |subdiag| render_subdiag(subdiag, smap));
            let mut includes: Vec<_> = inner
                .main
                .ranges()
                .map(|ranges| {
                    smap.get_includer_chain(ranges.primary_range.start())
                        .dropping(1) // First listing is the file itself
                        .map(|(_, pos)| pos)
                        .collect()
                })
                .unwrap_or_default();

            // Once again, we have includes from innermost to outermost.
            includes.reverse();

            RenderedDiagnostic { inner, includes }
        }
        None => RenderedDiagnostic {
            inner: render_with(raw, render_anon_subdiag),
            includes: Vec::new(),
        },
    }
}
