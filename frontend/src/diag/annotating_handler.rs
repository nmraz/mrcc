use std::cmp;
use std::fmt;
use std::iter;

use crate::smap::InterpretedFileRange;
use crate::{LineCol, SourceMap};

use super::{Level, Ranges, RenderedDiagnostic, RenderedHandler, RenderedSubDiagnostic};

pub struct AnnotatingHandler;

impl RenderedHandler for AnnotatingHandler {
    fn handle(&mut self, diag: &RenderedDiagnostic<'_>) {
        let subdiags =
            iter::once((diag.level, &diag.main)).chain(iter::repeat(Level::Note).zip(&diag.notes));

        match diag.smap {
            Some(smap) => subdiags.for_each(|(level, subdiag)| print_subdiag(level, subdiag, smap)),
            None => subdiags.for_each(|(level, subdiag)| print_anon_subdiag(level, subdiag)),
        }

        println!();
    }
}

fn print_anon_subdiag(level: Level, subdiag: &RenderedSubDiagnostic) {
    eprintln!("{}: {}", level, subdiag.msg());
}

fn print_subdiag(level: Level, subdiag: &RenderedSubDiagnostic, smap: &SourceMap) {
    print_anon_subdiag(level, subdiag);

    if let Some(&Ranges { primary_range, .. }) = subdiag.ranges() {
        let interp = smap.get_interpreted_range(primary_range);
        let suggestion = subdiag.suggestion().map(|sugg| {
            (
                sugg.insert_text.as_str(),
                smap.get_interpreted_range(sugg.replacement_range)
                    .start_linecol(),
            )
        });
        print_annotation(&interp, suggestion);
    }
}

fn print_annotation(interp: &InterpretedFileRange<'_>, suggestion: Option<(&str, LineCol)>) {
    let line_snippets: Vec<_> = interp.line_snippets().collect();
    let line_num_width = match line_snippets.last() {
        Some(last) => count_digits(last.line_num + 1),
        None => return,
    };

    let linecol = interp.start_linecol();
    eprintln!(
        "{pad:width$}--> {}:{}:{}",
        interp.filename(),
        linecol.line + 1,
        linecol.col + 1,
        pad = "",
        width = line_num_width
    );

    for snippet in &line_snippets {
        print_gutter(snippet.line_num + 1, line_num_width);
        eprintln!("{}", snippet.line);

        print_gutter("", line_num_width);
        println!(
            "{pad:off$}{}",
            "^".repeat(cmp::max(snippet.len, 1) as usize),
            pad = "",
            off = snippet.off as usize,
        );

        if let Some((text, linecol)) = suggestion {
            if linecol.line == snippet.line_num {
                print_gutter("", line_num_width);
                println!("{}{}", " ".repeat(linecol.col as usize), text);
            }
        }
    }
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
