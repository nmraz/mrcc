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
    let range = file_source.range();

    let exp_source = sm.create_expansion(
        SourceRange::new(range.subpos(10), 1),
        SourceRange::new(range.subpos(12), 1),
        ExpansionType::Macro,
    );

    let exp = exp_source.unwrap_expansion();
    assert_eq!(exp.spelling_pos(), range.subpos(10));
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

    assert!(sm.lookup_source(source_c.range().subpos(3)) == source_c);
    assert!(sm.lookup_source(source_empty.range().start()) == source_empty);
    assert!(sm.lookup_source(source_h.range().start()) == source_h);
}

#[test]
fn lookup_pos_last() {
    let sm = SourceManager::new();
    let source = sm
        .create_file("file".to_owned(), "".to_owned(), None)
        .unwrap();
    assert!(sm.lookup_source(source.range().start()) == source);
}

fn create_sm() -> (
    SourceManager,
    Rc<Source>,
    Rc<Source>,
    Rc<Source>,
    Rc<Source>,
) {
    let sm = SourceManager::new();

    let file = sm
        .create_file(
            "file.c".to_owned(),
            "#define B(x) (x + 3)\n#define A B(5 * 2)\nint x = A;".to_owned(),
            None,
        )
        .unwrap();

    let file_range = file.range();

    let exp_a = sm.create_expansion(
        file_range.subrange(31, 8),
        file_range.subrange(48, 1),
        ExpansionType::Macro,
    );

    let exp_b = sm.create_expansion(
        file_range.subrange(13, 7),
        exp_a.range(),
        ExpansionType::Macro,
    );

    let exp_b_x = sm.create_expansion(
        exp_a.range().subrange(2, 5),
        exp_b.range().subrange(1, 1),
        ExpansionType::MacroArg,
    );

    (sm, file, exp_a, exp_b, exp_b_x)
}

#[test]
fn immediate_spelling_pos() {
    let (sm, file, exp_a, exp_b, exp_b_x) = create_sm();

    let in_file = file.range().subpos(5);
    assert_eq!(sm.get_immediate_spelling_pos(in_file), in_file);

    let in_a = exp_a.range().subpos(4);
    assert_eq!(sm.get_immediate_spelling_pos(in_a), file.range().subpos(35));

    let in_b = exp_b.range().subpos(2);
    assert_eq!(sm.get_immediate_spelling_pos(in_b), file.range().subpos(15));

    let in_b_x = exp_b_x.range().subpos(3);
    assert_eq!(
        sm.get_immediate_spelling_pos(in_b_x),
        exp_a.range().subpos(5)
    );
}

#[test]
fn spelling_pos() {
    let (sm, file, exp_a, exp_b, exp_b_x) = create_sm();
    let file_range = file.range();

    let in_file = file_range.subpos(5);
    assert_eq!(sm.get_spelling_pos(in_file), in_file);

    let in_a = exp_a.range().subpos(4);
    assert_eq!(sm.get_spelling_pos(in_a), file_range.subpos(35));

    let in_b = exp_b.range().subpos(2);
    assert_eq!(sm.get_spelling_pos(in_b), file_range.subpos(15));

    let in_b_x = exp_b_x.range().subpos(3);
    assert_eq!(sm.get_spelling_pos(in_b_x), file_range.subpos(36));
}

#[test]
fn immediate_expansion_range() {
    let (sm, file, exp_a, exp_b, exp_b_x) = create_sm();

    let in_file = file.range().subrange(5, 2);
    assert_eq!(sm.get_immediate_expansion_range(in_file), in_file);

    let in_a = exp_a.range().subrange(3, 3);
    assert_eq!(
        sm.get_immediate_expansion_range(in_a),
        file.range().subrange(48, 1)
    );

    let in_b = exp_b.range().subrange(2, 1);
    assert_eq!(sm.get_immediate_expansion_range(in_b), exp_a.range());

    let in_b_x = exp_b_x.range().subrange(2, 2);
    assert_eq!(
        sm.get_immediate_expansion_range(in_b_x),
        exp_b.range().subrange(1, 1)
    );
}

#[test]
fn expansion_range() {
    let (sm, file, exp_a, exp_b, exp_b_x) = create_sm();
    let exp_range = file.range().subrange(48, 1);

    let in_file = file.range().subrange(5, 2);
    assert_eq!(sm.get_expansion_range(in_file), in_file);

    let in_a = exp_a.range().subrange(3, 3);
    assert_eq!(sm.get_expansion_range(in_a), exp_range);

    let in_b = exp_b.range().subrange(2, 1);
    assert_eq!(sm.get_expansion_range(in_b), exp_range);

    let in_b_x = exp_b_x.range().subrange(2, 2);
    assert_eq!(sm.get_expansion_range(in_b_x), exp_range);
}
