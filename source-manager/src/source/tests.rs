use super::*;

#[test]
fn file_source_new() {
    let include_pos = SourcePos::from_raw(5);
    let file = FileSourceInfo::new("file".to_owned(), "Hello".to_owned(), Some(include_pos));

    assert_eq!(file.src(), "Hello");
    assert_eq!(file.filename(), "file");
    assert_eq!(file.include_pos(), Some(include_pos));
}

#[test]
fn file_source_normalized() {
    let src = "line\r\nline\nline".to_owned();
    let file = FileSourceInfo::new("file".to_owned(), src, None);
    assert_eq!(file.src(), "line\nline\nline");
}

#[test]
fn file_source_linecol() {
    let src = "line 1\nline 2\nline 3".to_owned();
    let file = FileSourceInfo::new("file".to_owned(), src, None);
    assert_eq!(file.get_linecol(6), LineCol { line: 0, col: 6 });
    assert_eq!(file.get_linecol(17), LineCol { line: 2, col: 3 });
}

#[test]
#[should_panic]
fn file_source_linecol_past_end() {
    let src = "line\nline\n".to_owned();
    let file = FileSourceInfo::new("file".to_owned(), src, None);
    file.get_linecol(17);
}

#[test]
fn file_source_line_ranges() {
    let src = "line\r\nline 2\n\nline".to_owned();
    let file = FileSourceInfo::new("file".to_owned(), src, None);
    assert_eq!(file.get_line_start(0), 0);
    assert_eq!(file.get_line_end(0), 4);
    assert_eq!(file.get_line_start(2), 12);
    assert_eq!(file.get_line_end(2), 12);
    assert_eq!(file.get_line_end(3), 17);
}

#[test]
fn source_file() {
    let file = FileSourceInfo::new("file".to_owned(), "source".to_owned(), None);
    let source = Source::new(
        SourceInfo::File(file),
        SourceRange::new(SourcePos::from_raw(0), 5),
    );

    assert!(source.is_file());
    assert!(!source.is_expansion());

    let unwrapped = source.unwrap_file();
    assert_eq!(unwrapped.filename(), "file");
    assert_eq!(unwrapped.src(), "source");
}

#[test]
fn source_expansion() {
    let spelling_pos = SourcePos::from_raw(5);
    let expansion_range = SourceRange::new(SourcePos::from_raw(27), 6);

    let exp = ExpansionSourceInfo::new(spelling_pos, expansion_range, ExpansionType::Macro);
    let source = Source::new(
        SourceInfo::Expansion(exp),
        SourceRange::new(SourcePos::from_raw(40), 5),
    );

    assert!(source.is_expansion());
    assert!(!source.is_file());

    let unwrapped = source.unwrap_expansion();
    assert_eq!(unwrapped.spelling_pos(), spelling_pos);
    assert_eq!(unwrapped.expansion_range(), expansion_range);
    assert_eq!(unwrapped.expansion_type(), ExpansionType::Macro);
}
