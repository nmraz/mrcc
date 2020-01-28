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
