use std::cmp;
use std::collections::BTreeMap;
use std::fmt;
use std::iter;

use crate::smap::{InterpretedFileRange, LineSnippet};
use crate::{LocalRange, SourceMap, SourcePos};

use super::{
    Level, RenderedDiagnostic, RenderedRanges, RenderedSink, RenderedSubDiagnostic,
    RenderedSuggestion,
};

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

struct AnnotatedLine<'a> {
    line: &'a str,
    line_num: u32,
    primary_range: Option<LocalRange>,
    subranges: Vec<LocalRange>,
    suggestion: Option<(&'a str, u32)>,
}

impl<'a> AnnotatedLine<'a> {
    fn new(line: &'a str, line_num: u32) -> Self {
        Self {
            line,
            line_num,
            primary_range: None,
            subranges: Vec::new(),
            suggestion: None,
        }
    }
}

fn print_subdiag_msg(subdiag: &WrappedSubDiagnostic<'_>) {
    eprintln!("{}: {}", subdiag.level, subdiag.diag.msg);
}

fn print_annotated_subdiag(subdiag: &WrappedSubDiagnostic<'_>, smap: &SourceMap) {
    print_subdiag_msg(subdiag);

    if let Some(ranges) = subdiag.diag.ranges.as_ref() {
        let annotations = build_annotations(ranges, subdiag.diag.suggestion.as_ref(), smap);

        let gutter_width = match annotations.last() {
            Some(last) => count_digits(last.line_num + 1),
            None => return,
        };

        for &include in subdiag.includes {
            print_file_loc(
                &smap.get_interpreted_range(include.into()),
                Some("includer"),
                gutter_width,
            );
        }

        print_file_loc(
            &smap.get_interpreted_range(ranges.primary_range),
            None,
            gutter_width,
        );

        print_annotations(&annotations, gutter_width);
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

fn build_annotations<'a>(
    ranges: &RenderedRanges,
    suggestion: Option<&'a RenderedSuggestion>,
    smap: &'a SourceMap,
) -> Vec<AnnotatedLine<'a>> {
    fn get_line<'a, 'b>(
        line_map: &'a mut BTreeMap<u32, AnnotatedLine<'b>>,
        snippet: &LineSnippet<'b>,
    ) -> &'a mut AnnotatedLine<'b> {
        line_map
            .entry(snippet.line_num)
            .or_insert_with(|| AnnotatedLine::new(snippet.line, snippet.line_num))
    }

    let mut line_map = BTreeMap::new();

    for snippet in smap
        .get_interpreted_range(ranges.primary_range)
        .line_snippets()
    {
        get_line(&mut line_map, &snippet).primary_range = Some(snippet.range);
    }

    for &(subrange, _) in &ranges.subranges {
        for snippet in smap.get_interpreted_range(subrange).line_snippets() {
            get_line(&mut line_map, &snippet)
                .subranges
                .push(snippet.range);
        }
    }

    if let Some(suggestion) = suggestion {
        let linecol = smap
            .get_interpreted_range(suggestion.replacement_range)
            .start_linecol();

        // To avoid confusion, only display the suggestion if it's on a line we're highlighting
        // anyway.
        // TODO: find a better way to surface this.
        if let Some(annotated_line) = line_map.get_mut(&linecol.line) {
            annotated_line.suggestion = Some((&suggestion.insert_text, linecol.col));
        }
    }

    line_map.into_iter().map(|(_, line)| line).collect()
}

fn print_annotations(annotations: &[AnnotatedLine<'_>], gutter_width: usize) {
    let mut last_line_num = None;

    for annotation in annotations {
        if last_line_num
            .filter(|line_num| line_num + 1 < annotation.line_num)
            .is_some()
        {
            // Indicate skipped lines in the snippet.
            eprintln!("...");
        }

        last_line_num = Some(annotation.line_num);
        print_annotation(annotation, gutter_width);
    }
}

fn print_annotation(annotation: &AnnotatedLine<'_>, gutter_width: usize) {
    let highlight_line = build_highlight_line(annotation);

    print_gutter(annotation.line_num + 1, gutter_width);
    eprintln!("{}", annotation.line);

    print_gutter("", gutter_width);
    eprintln!("{}", highlight_line);

    if let Some((text, off)) = annotation.suggestion {
        print_gutter("", gutter_width);
        eprintln!("{pad:off$}{}", text, pad = "", off = off as usize);
    }
}

fn build_highlight_line(annotation: &AnnotatedLine<'_>) -> String {
    let mut highlight_line = " ".repeat(annotation.line.len() + 1);

    for &subrange in &annotation.subranges {
        add_highlight(&mut highlight_line, subrange, "-");
    }

    if let Some(primary_range) = annotation.primary_range {
        add_highlight(&mut highlight_line, primary_range, "^");
    }

    highlight_line
}

fn add_highlight(highlight_line: &mut String, range: LocalRange, marker: &str) {
    let start: usize = range.start().into();
    let len = cmp::max(range.len().into(), 1);

    highlight_line.replace_range(start..start + len, &marker.repeat(len));
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
    fn highlight_line() {
        let annotation = AnnotatedLine {
            line: "int x = 1 + 2;",
            line_num: 0,
            primary_range: Some(LocalRange::at(10.into(), 1.into())),
            subranges: vec![
                LocalRange::at(8.into(), 1.into()),
                LocalRange::at(12.into(), 1.into()),
            ],
            suggestion: None,
        };

        assert_eq!(build_highlight_line(&annotation), "        - ^ -  ");
    }

    #[test]
    fn highlight_line_zero_width() {
        let annotation = AnnotatedLine {
            line: "int x = 1 + 2;",
            line_num: 0,
            primary_range: Some(LocalRange::at(10.into(), 0.into())),
            subranges: Vec::new(),
            suggestion: None,
        };

        assert_eq!(build_highlight_line(&annotation), "          ^    ");
    }

    #[test]
    fn highlight_line_at_end() {
        let annotation = AnnotatedLine {
            line: "#include \"test.h",
            line_num: 0,
            primary_range: Some(LocalRange::at(16.into(), 0.into())),
            subranges: Vec::new(),
            suggestion: None,
        };

        assert_eq!(build_highlight_line(&annotation), "                ^")
    }

    #[test]
    fn highlight_line_no_primary() {
        let annotation = AnnotatedLine {
            line: "int x = 1 + 2;",
            line_num: 0,
            primary_range: None,
            subranges: vec![
                LocalRange::at(8.into(), 1.into()),
                LocalRange::at(12.into(), 1.into()),
            ],
            suggestion: None,
        };

        assert_eq!(build_highlight_line(&annotation), "        -   -  ");
    }

    #[test]
    fn highlight_line_overlap() {
        let annotation = AnnotatedLine {
            line: "int x = A(1, ++);",
            line_num: 0,
            primary_range: Some(LocalRange::at(13.into(), 2.into())),
            subranges: vec![LocalRange::at(8.into(), 8.into())],
            suggestion: None,
        };

        assert_eq!(build_highlight_line(&annotation), "        -----^^-  ");
    }

    #[test]
    fn digit_count() {
        assert_eq!(count_digits(0), 1);
        assert_eq!(count_digits(230), 3);
        assert_eq!(count_digits(10), 2);
    }
}
