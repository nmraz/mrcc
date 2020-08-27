use std::string::ToString;

use super::*;

#[test]
fn filename_is_real() {
    let f1 = FileName::real("file.c");
    assert!(f1.is_real());

    let f2 = FileName::synth("paste");
    assert!(!f2.is_real());
}

#[test]
fn filename_to_string() {
    let f1 = FileName::real("file.c");
    assert_eq!(f1.to_string(), "file.c".to_owned());

    let f2 = FileName::synth("paste");
    assert_eq!(f2.to_string(), "<paste>".to_owned());
}

#[test]
fn file_contents_normalized() {
    let src = "line\r\nline\nline";
    let contents = FileContents::new(src);
    assert_eq!(contents.src, "line\nline\nline");
}

#[test]
fn file_contents_linecol() {
    let src = "line 1\nline 2\nline 3";
    let contents = FileContents::new(src);
    assert_eq!(contents.get_linecol(6), LineCol { line: 0, col: 6 });
    assert_eq!(contents.get_linecol(17), LineCol { line: 2, col: 3 });
}

#[test]
fn file_contents_linecol_at_end() {
    let src = "line 1\nline 2\nline 3";
    let contents = FileContents::new(src);
    assert_eq!(contents.get_linecol(20), LineCol { line: 2, col: 6 });
}

#[test]
#[should_panic]
fn file_contents_linecol_past_end() {
    let src = "line\nline\n";
    let contents = FileContents::new(src);
    contents.get_linecol(12);
}

#[test]
fn file_contents_lines() {
    let src = "line 1\nline 2\nline 3";
    let contents = FileContents::new(src);
    assert_eq!(contents.get_lines(0..1), "line 1\nline 2");
    assert_eq!(contents.get_lines(1..2), "line 2\nline 3");
    assert_eq!(contents.get_lines(0..2), "line 1\nline 2\nline 3");
}

#[test]
fn file_contents_line_ranges() {
    let src = "line\r\nline 2\n\nline";
    let contents = FileContents::new(src);
    assert_eq!(contents.get_line_start(0), 0);
    assert_eq!(contents.get_line_end(0), 4);
    assert_eq!(contents.get_line_start(2), 12);
    assert_eq!(contents.get_line_end(2), 12);
    assert_eq!(contents.get_line_end(3), 17);
}

#[test]
fn source_file() {
    let filename = FileName::real("source.c");
    let contents = FileContents::new("source");
    let file = FileSourceInfo::new(filename.clone(), contents, None);
    let source = Source {
        info: SourceInfo::File(file),
        range: SourceRange::new(SourcePos::from_raw(0), 5),
    };

    assert!(source.is_file());
    assert!(!source.is_expansion());

    let unwrapped = source.as_file().unwrap();
    assert_eq!(unwrapped.filename, filename);
    assert_eq!(unwrapped.contents.src, "source");
}

#[test]
fn source_expansion() {
    let spelling_pos = SourcePos::from_raw(5);
    let expansion_range = SourceRange::new(SourcePos::from_raw(27), 6);

    let exp = ExpansionSourceInfo::new(spelling_pos, expansion_range, ExpansionType::Macro);
    let source = Source {
        info: SourceInfo::Expansion(exp),
        range: SourceRange::new(SourcePos::from_raw(40), 5),
    };

    assert!(source.is_expansion());
    assert!(!source.is_file());

    let unwrapped = source.as_expansion().unwrap();
    assert_eq!(unwrapped.spelling_pos, spelling_pos);
    assert_eq!(unwrapped.expansion_range, expansion_range);
    assert_eq!(unwrapped.expansion_type, ExpansionType::Macro);
}
