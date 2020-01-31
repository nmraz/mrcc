use super::*;

#[test]
fn create_file() {
    let sm = SourceManager::new();
    let source = sm
        .create_file("file".to_owned(), "line\nline\nline".to_owned(), None)
        .unwrap();
    let f = source.unwrap_file();
    assert_eq!(f.filename(), "file");
}

#[test]
fn create_expansion() {
    let sm = SourceManager::new();

    let file_source = sm
        .create_file("file.c".to_owned(), "#define A 5\nA;".to_owned(), None)
        .unwrap();
    let start = file_source.range().start();

    let exp_source = sm.create_expansion(
        start.offset(10),
        SourceRange::new(start.offset(12), 1),
        ExpansionType::Macro,
        1,
    );

    let exp = exp_source.unwrap_expansion();
    assert_eq!(exp.spelling_pos(), start.offset(10));
    assert_eq!(exp.expansion_type(), ExpansionType::Macro);
}

#[test]
fn lookup_pos() {
    let sm = SourceManager::new();

    let source_c = sm
        .create_file("file.c".to_owned(), "#include <file.h>".to_owned(), None)
        .unwrap();

    let source_empty = sm
        .create_file("empty.c".to_owned(), "".to_owned(), None)
        .unwrap();

    let source_h = sm
        .create_file(
            "file.h".to_owned(),
            "void f();".to_owned(),
            Some(source_c.range().start()),
        )
        .unwrap();

    assert!(sm.lookup_source(source_c.range().start().offset(3)) == source_c);
    assert!(sm.lookup_source(source_empty.range().start()) == source_empty);
    assert!(sm.lookup_source(source_h.range().start()) == source_h);
}
