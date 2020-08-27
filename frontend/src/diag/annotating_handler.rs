use std::cmp;
use std::fmt;
use std::iter;

use crate::smap::InterpretedFileRange;
use crate::SourceMap;

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
    }
}

fn print_subdiag(level: Level, subdiag: &RenderedSubDiagnostic, smap: &SourceMap) {
    match subdiag.ranges() {
        Some(&Ranges { primary_range, .. }) => {
            let interp = smap.get_interpreted_range(primary_range);
            let linecol = interp.start_linecol();
            eprintln!(
                "{}:{}:{}: {}: {}",
                interp.filename(),
                linecol.line + 1,
                linecol.col + 1,
                level,
                subdiag.msg()
            );
            print_annotation(&interp);
        }
        None => print_anon_subdiag(level, subdiag),
    }
}

fn print_annotation(interp: &InterpretedFileRange<'_>) {
    let line_snippets: Vec<_> = interp.line_snippets().collect();
    let line_num_width = match line_snippets.last() {
        Some(last) => count_digits(last.line_num + 1),
        None => return,
    };

    for snippet in &line_snippets {
        print_gutter(snippet.line_num + 1, line_num_width);
        eprintln!("{}", snippet.line);

        print_gutter("", line_num_width);
        println!(
            "{}{}",
            " ".repeat(snippet.off as usize),
            "^".repeat(cmp::max(snippet.len, 1) as usize)
        );
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

fn print_anon_subdiag(level: Level, subdiag: &RenderedSubDiagnostic) {
    eprintln!("{}: {}", level, subdiag.msg())
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
