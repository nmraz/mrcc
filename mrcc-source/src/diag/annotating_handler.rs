use std::cmp;
use std::fmt;
use std::iter;

use crate::smap::InterpretedFileRange;
use crate::{LineCol, SourceMap, SourcePos};

use super::{
    Level, Ranges, RenderedDiagnostic, RenderedHandler, RenderedRanges, RenderedSubDiagnostic,
    RenderedSuggestion,
};

/// A rendered diagnostic handler that emits messages and annotated code snippets to `stderr`.
pub struct AnnotatingHandler;

impl RenderedHandler for AnnotatingHandler {
    fn handle(&mut self, diag: &RenderedDiagnostic<'_>) {
        let subdiags = main_diag_chain(diag).chain(diag.notes().iter().flat_map(note_diag_chain));

        match diag.smap() {
            Some(smap) => subdiags.for_each(|subdiag| print_subdiag(&subdiag, smap)),
            None => subdiags.for_each(|subdiag| print_anon_subdiag(&subdiag)),
        }

        eprintln!();
    }
}

fn main_diag_chain<'a>(
    diag: &'a RenderedDiagnostic<'_>,
) -> impl Iterator<Item = DisplayableSubDiagnostic<'a>> {
    chain_expansions(
        DisplayableSubDiagnostic::from_subdiag(diag.level(), diag.main(), &diag.includes),
        diag.main(),
    )
}

fn note_diag_chain(
    note: &RenderedSubDiagnostic,
) -> impl Iterator<Item = DisplayableSubDiagnostic<'_>> {
    chain_expansions(
        DisplayableSubDiagnostic::from_subdiag(Level::Note, note, &[]),
        note,
    )
}

fn chain_expansions<'a>(
    primary: DisplayableSubDiagnostic<'a>,
    expansion_subdiag: &'a RenderedSubDiagnostic,
) -> impl Iterator<Item = DisplayableSubDiagnostic<'a>> {
    iter::once(primary).chain(
        expansion_subdiag
            .expansions
            .iter()
            .map(|ranges| DisplayableSubDiagnostic::from_expansion(ranges)),
    )
}

struct DisplayableSubDiagnostic<'a> {
    level: Level,
    msg: &'a str,
    ranges: Option<&'a RenderedRanges>,
    suggestion: Option<&'a RenderedSuggestion>,
    includes: &'a [SourcePos],
}

impl<'a> DisplayableSubDiagnostic<'a> {
    fn from_subdiag(
        level: Level,
        subdiag: &'a RenderedSubDiagnostic,
        includes: &'a [SourcePos],
    ) -> Self {
        Self {
            level,
            msg: subdiag.msg(),
            ranges: subdiag.ranges(),
            suggestion: subdiag.suggestion(),
            includes,
        }
    }

    fn from_expansion(ranges: &'a RenderedRanges) -> Self {
        Self {
            level: Level::Note,
            msg: "expanded from here",
            ranges: Some(ranges),
            suggestion: None,
            includes: &[],
        }
    }
}

fn print_anon_subdiag(subdiag: &DisplayableSubDiagnostic<'_>) {
    eprintln!("{}: {}", subdiag.level, subdiag.msg);
}

fn print_subdiag(subdiag: &DisplayableSubDiagnostic<'_>, smap: &SourceMap) {
    print_anon_subdiag(subdiag);

    if let Some(&Ranges { primary_range, .. }) = subdiag.ranges {
        let interp = smap.get_interpreted_range(primary_range);
        let suggestion = subdiag.suggestion.map(|sugg| {
            (
                sugg.insert_text.as_str(),
                smap.get_interpreted_range(sugg.replacement_range)
                    .start_linecol(),
            )
        });

        print_annotation(
            &interp,
            suggestion,
            subdiag
                .includes
                .iter()
                .map(|&pos| smap.get_interpreted_range(pos.into())),
        );
    }
}

fn print_annotation<'a>(
    interp: &InterpretedFileRange<'_>,
    suggestion: Option<(&str, LineCol)>,
    includes: impl Iterator<Item = InterpretedFileRange<'a>>,
) {
    let line_snippets: Vec<_> = interp.line_snippets().collect();
    let line_num_width = match line_snippets.last() {
        Some(last) => count_digits(last.line_num + 1),
        None => return,
    };

    for include in includes {
        print_file_loc(&include, Some("includer"), line_num_width);
    }

    print_file_loc(interp, None, line_num_width);

    for snippet in &line_snippets {
        print_gutter(snippet.line_num + 1, line_num_width);
        eprintln!("{}", snippet.line);

        print_gutter("", line_num_width);
        eprintln!(
            "{pad:off$}{}",
            "^".repeat(cmp::max(snippet.len, 1) as usize),
            pad = "",
            off = snippet.off as usize,
        );

        if let Some((text, linecol)) = suggestion {
            if linecol.line == snippet.line_num {
                print_gutter("", line_num_width);
                eprintln!("{pad:off$}{}", text, pad = "", off = linecol.col as usize);
            }
        }
    }
}

fn print_file_loc(interp: &InterpretedFileRange<'_>, note: Option<&str>, gutter_width: usize) {
    let note = note.map(|note| format!(" ({})", note)).unwrap_or_default();
    let linecol = interp.start_linecol();

    eprintln!(
        "{pad:width$}--> {}:{}:{}{}",
        interp.filename(),
        linecol.line + 1,
        linecol.col + 1,
        note,
        pad = "",
        width = gutter_width
    );
}

fn print_gutter(obj: impl fmt::Display, width: usize) {
    eprint!("{:>1$} | ", obj, width);
}

fn count_digits(mut val: u32) -> usize {
    let mut digits = 1;
    val /= 10;
    while val > 0 {
        digits += 1;
        val /= 10;
    }
    digits
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn digit_count() {
        assert_eq!(count_digits(0), 1);
        assert_eq!(count_digits(230), 3);
        assert_eq!(count_digits(10), 2);
    }
}
