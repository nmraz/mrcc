use std::cmp;
use std::fmt;
use std::iter;

use crate::smap::InterpretedFileRange;
use crate::{LineCol, SourceMap, SourcePos};

use super::{Level, Ranges, RenderedDiagnostic, RenderedSink, RenderedSubDiagnostic};

/// A rendered diagnostic sink that emits messages and annotated code snippets to `stderr`.
pub struct AnnotatingSink;

impl RenderedSink for AnnotatingSink {
    fn report(&mut self, diag: &RenderedDiagnostic, smap: Option<&SourceMap>) {
        let subdiags = iter::once(WrappedSubDiagnostic::from_main(diag))
            .chain(diag.notes().iter().map(WrappedSubDiagnostic::from_note));

        match smap {
            Some(smap) => subdiags.for_each(|subdiag| print_annotated_subdiag(&subdiag, smap)),
            None => subdiags.for_each(|subdiag| print_subdiag_msg(&subdiag)),
        }

        eprintln!();
    }
}

struct WrappedSubDiagnostic<'a> {
    level: Level,
    includes: &'a [SourcePos],
    diag: &'a RenderedSubDiagnostic,
}

impl<'a> WrappedSubDiagnostic<'a> {
    fn from_main(diag: &'a RenderedDiagnostic) -> Self {
        Self {
            level: diag.level(),
            includes: &diag.includes,
            diag: diag.main(),
        }
    }

    fn from_note(note: &'a RenderedSubDiagnostic) -> Self {
        Self {
            level: Level::Note,
            includes: &[],
            diag: note,
        }
    }
}

fn print_subdiag_msg(subdiag: &WrappedSubDiagnostic<'_>) {
    eprintln!("{}: {}", subdiag.level, subdiag.diag.msg);
}

fn print_annotated_subdiag(subdiag: &WrappedSubDiagnostic<'_>, smap: &SourceMap) {
    print_subdiag_msg(subdiag);

    if let Some(&Ranges { primary_range, .. }) = subdiag.diag.ranges.as_ref() {
        let interp = smap.get_interpreted_range(primary_range);
        let suggestion = subdiag.diag.suggestion.as_ref().map(|sugg| {
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
            "^".repeat(cmp::max(snippet.range.len().into(), 1)),
            pad = "",
            off = snippet.range.start().into(),
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
