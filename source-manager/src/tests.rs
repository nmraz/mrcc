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
        SourceRange::new(start.offset(10), 1),
        SourceRange::new(start.offset(12), 1),
        ExpansionType::Macro,
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

    let file_start = file.range().start();

    let exp_a = sm.create_expansion(
        SourceRange::new(file_start.offset(31), 8),
        SourceRange::new(file_start.offset(48), 1),
        ExpansionType::Macro,
    );

    let exp_b = sm.create_expansion(
        SourceRange::new(file_start.offset(13), 7),
        exp_a.range(),
        ExpansionType::Macro,
    );

    let exp_b_x = sm.create_expansion(
        SourceRange::new(exp_a.range().start().offset(2), 5),
        SourceRange::new(exp_b.range().start().offset(1), 1),
        ExpansionType::MacroArg,
    );

    (sm, file, exp_a, exp_b, exp_b_x)
}

#[test]
fn immediate_spelling_pos() {
    let (sm, file, exp_a, exp_b, exp_b_x) = create_sm();

    let in_file = file.range().start().offset(5);
    assert_eq!(sm.get_immediate_spelling_pos(in_file), in_file);

    let in_a = exp_a.range().start().offset(4);
    assert_eq!(
        sm.get_immediate_spelling_pos(in_a),
        file.range().start().offset(35)
    );

    let in_b = exp_b.range().start().offset(2);
    assert_eq!(
        sm.get_immediate_spelling_pos(in_b),
        file.range().start().offset(15)
    );

    let in_b_x = exp_b_x.range().start().offset(3);
    assert_eq!(
        sm.get_immediate_spelling_pos(in_b_x),
        exp_a.range().start().offset(5)
    );
}

#[test]
fn spelling_pos() {
    let (sm, file, exp_a, exp_b, exp_b_x) = create_sm();

    let in_file = file.range().start().offset(5);
    assert_eq!(sm.get_spelling_pos(in_file), in_file);

    let in_a = exp_a.range().start().offset(4);
    assert_eq!(sm.get_spelling_pos(in_a), file.range().start().offset(35));

    let in_b = exp_b.range().start().offset(2);
    assert_eq!(sm.get_spelling_pos(in_b), file.range().start().offset(15));

    let in_b_x = exp_b_x.range().start().offset(3);
    assert_eq!(sm.get_spelling_pos(in_b_x), file.range().start().offset(36));
}

#[test]
fn immediate_expansion_range() {
    let (sm, file, exp_a, exp_b, exp_b_x) = create_sm();

    let in_file = SourceRange::new(file.range().start().offset(5), 2);
    assert_eq!(sm.get_immediate_expansion_range(in_file), in_file);

    let in_a = SourceRange::new(exp_a.range().start().offset(3), 3);
    assert_eq!(
        sm.get_immediate_expansion_range(in_a),
        SourceRange::new(file.range().start().offset(48), 1)
    );

    let in_b = SourceRange::new(exp_b.range().start().offset(2), 1);
    assert_eq!(sm.get_immediate_expansion_range(in_b), exp_a.range());

    let in_b_x = SourceRange::new(exp_b_x.range().start().offset(2), 2);
    assert_eq!(
        sm.get_immediate_expansion_range(in_b_x),
        SourceRange::new(exp_b.range().start().offset(1), 1)
    );
}

#[test]
fn expansion_range() {
    let (sm, file, exp_a, exp_b, exp_b_x) = create_sm();
    let exp_range = SourceRange::new(file.range().start().offset(48), 1);

    let in_file = SourceRange::new(file.range().start().offset(5), 2);
    assert_eq!(sm.get_expansion_range(in_file), in_file);

    let in_a = SourceRange::new(exp_a.range().start().offset(3), 3);
    assert_eq!(sm.get_expansion_range(in_a), exp_range);

    let in_b = SourceRange::new(exp_b.range().start().offset(2), 1);
    assert_eq!(sm.get_expansion_range(in_b), exp_range);

    let in_b_x = SourceRange::new(exp_b_x.range().start().offset(2), 2);
    assert_eq!(sm.get_expansion_range(in_b_x), exp_range);
}
