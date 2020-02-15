use super::*;

#[test]
fn create_file() {
    let sm = SourceMap::new();

    let filename = FileName::new_real("file");
    let file_start = sm
        .create_file(
            FileContents::new(filename.clone(), "line\nline\nline"),
            None,
        )
        .unwrap()
        .start();

    let file_source = sm.lookup_source(file_start);
    let file = file_source.as_file().unwrap();
    assert_eq!(file.contents.filename, filename);
}

#[test]
fn create_expansion() {
    let sm = SourceMap::new();

    let file_range = sm
        .create_file(
            FileContents::new(FileName::new_real("file.c"), "#define A 5\nA;"),
            None,
        )
        .unwrap();

    let exp_source_start = sm
        .create_expansion(
            file_range.subrange(10, 1),
            file_range.subrange(12, 1),
            ExpansionType::Macro,
        )
        .start();

    let exp_source = sm.lookup_source(exp_source_start);
    let exp = exp_source.as_expansion().unwrap();

    assert_eq!(exp.spelling_pos, file_range.subpos(10));
    assert_eq!(exp.expansion_type, ExpansionType::Macro);
}

#[test]
fn lookup_pos() {
    let sm = SourceMap::new();

    let source_c_range = sm
        .create_file(
            FileContents::new(FileName::new_real("file.c"), "#include <file.h>"),
            None,
        )
        .unwrap();

    let source_empty_range = sm
        .create_file(FileContents::new(FileName::new_real("empty.c"), ""), None)
        .unwrap();

    let source_h_range = sm
        .create_file(
            FileContents::new(FileName::new_real("file.h"), "void f();"),
            Some(source_c_range.start()),
        )
        .unwrap();

    assert_eq!(
        sm.lookup_source(source_c_range.subpos(3))
            .as_file()
            .unwrap()
            .contents
            .filename,
        FileName::new_real("file.c")
    );

    assert_eq!(
        sm.lookup_source(source_empty_range.start())
            .as_file()
            .unwrap()
            .contents
            .filename,
        FileName::new_real("empty.c")
    );

    assert_eq!(
        sm.lookup_source(source_h_range.subpos(3))
            .as_file()
            .unwrap()
            .contents
            .filename,
        FileName::new_real("file.h")
    );
}

#[test]
fn lookup_pos_last() {
    let sm = SourceMap::new();
    let range = sm
        .create_file(FileContents::new(FileName::new_real("file"), ""), None)
        .unwrap();
    sm.lookup_source(range.start()); // Shouldn't panic
}

#[test]
#[should_panic]
fn lookup_pos_past_last() {
    let sm = SourceMap::new();
    let range = sm
        .create_file(FileContents::new(FileName::new_real("file"), ""), None)
        .unwrap();
    sm.lookup_source(range.start().offset(2));
}

fn populate_sm(sm: &SourceMap) -> (SourceRange, SourceRange, SourceRange, SourceRange) {
    let file_range = sm
        .create_file(
            FileContents::new(
                FileName::new_real("file.c"),
                "#define B(x) (x + 3)\n#define A B(5 * 2)\nint x = A;",
            ),
            None,
        )
        .unwrap();

    let exp_a_range = sm.create_expansion(
        file_range.subrange(31, 8),
        file_range.subrange(48, 1),
        ExpansionType::Macro,
    );

    let exp_b_range = sm.create_expansion(
        file_range.subrange(13, 7),
        exp_a_range,
        ExpansionType::Macro,
    );

    let exp_b_x_range = sm.create_expansion(
        exp_a_range.subrange(2, 5),
        exp_b_range.subrange(1, 1),
        ExpansionType::MacroArg,
    );

    (file_range, exp_a_range, exp_b_range, exp_b_x_range)
}

#[test]
fn immediate_spelling_pos() {
    let sm = SourceMap::new();
    let (file_range, exp_a_range, exp_b_range, exp_b_x_range) = populate_sm(&sm);

    let in_file = file_range.subpos(5);
    assert_eq!(sm.get_immediate_spelling_pos(in_file), None);

    let in_a = exp_a_range.subpos(4);
    assert_eq!(
        sm.get_immediate_spelling_pos(in_a),
        Some(file_range.subpos(35))
    );

    let in_b = exp_b_range.subpos(2);
    assert_eq!(
        sm.get_immediate_spelling_pos(in_b),
        Some(file_range.subpos(15))
    );

    let in_b_x = exp_b_x_range.subpos(3);
    assert_eq!(
        sm.get_immediate_spelling_pos(in_b_x),
        Some(exp_a_range.subpos(5))
    );
}

#[test]
fn spelling_pos() {
    let sm = SourceMap::new();
    let (file_range, exp_a_range, exp_b_range, exp_b_x_range) = populate_sm(&sm);

    let in_file = file_range.subpos(5);
    assert_eq!(sm.get_spelling_pos(in_file), in_file);

    let in_a = exp_a_range.subpos(4);
    assert_eq!(sm.get_spelling_pos(in_a), file_range.subpos(35));

    let in_b = exp_b_range.subpos(2);
    assert_eq!(sm.get_spelling_pos(in_b), file_range.subpos(15));

    let in_b_x = exp_b_x_range.subpos(3);
    assert_eq!(sm.get_spelling_pos(in_b_x), file_range.subpos(36));
}

#[test]
fn immediate_expansion_range() {
    let sm = SourceMap::new();
    let (file_range, exp_a_range, exp_b_range, exp_b_x_range) = populate_sm(&sm);

    let in_file = file_range.subrange(5, 2);
    assert_eq!(sm.get_immediate_expansion_range(in_file), None);

    let in_a = exp_a_range.subrange(3, 3);
    assert_eq!(
        sm.get_immediate_expansion_range(in_a),
        Some(file_range.subrange(48, 1))
    );

    let in_b = exp_b_range.subrange(2, 1);
    assert_eq!(sm.get_immediate_expansion_range(in_b), Some(exp_a_range));

    let in_b_x = exp_b_x_range.subrange(2, 2);
    assert_eq!(
        sm.get_immediate_expansion_range(in_b_x),
        Some(exp_b_range.subrange(1, 1))
    );
}

#[test]
fn expansion_range() {
    let sm = SourceMap::new();
    let (file_range, exp_a_range, exp_b_range, exp_b_x_range) = populate_sm(&sm);

    let exp_range = file_range.subrange(48, 1);

    let in_file = file_range.subrange(5, 2);
    assert_eq!(sm.get_expansion_range(in_file), in_file);

    let in_a = exp_a_range.subrange(3, 3);
    assert_eq!(sm.get_expansion_range(in_a), exp_range);

    let in_b = exp_b_range.subrange(2, 1);
    assert_eq!(sm.get_expansion_range(in_b), exp_range);

    let in_b_x = exp_b_x_range.subrange(2, 2);
    assert_eq!(sm.get_expansion_range(in_b_x), exp_range);
}

#[test]
fn immediate_caller_range() {
    let sm = SourceMap::new();
    let (file_range, exp_a_range, exp_b_range, exp_b_x_range) = populate_sm(&sm);

    let in_file = file_range.subrange(5, 2);
    assert_eq!(sm.get_immediate_caller_range(in_file), None);

    let in_a = exp_a_range.subrange(3, 3);
    assert_eq!(
        sm.get_immediate_caller_range(in_a),
        Some(file_range.subrange(48, 1))
    );

    let in_b = exp_b_range.subrange(2, 1);
    assert_eq!(sm.get_immediate_caller_range(in_b), Some(exp_a_range));

    let in_b_x = exp_b_x_range.subrange(2, 2);
    assert_eq!(
        sm.get_immediate_caller_range(in_b_x),
        Some(exp_a_range.subrange(4, 2))
    );
}

#[test]
fn caller_range() {
    let sm = SourceMap::new();
    let (file_range, exp_a_range, exp_b_range, exp_b_x_range) = populate_sm(&sm);

    let exp_range = file_range.subrange(48, 1);

    let in_file = file_range.subrange(5, 2);
    assert_eq!(sm.get_caller_range(in_file), in_file);

    let in_a = exp_a_range.subrange(3, 3);
    assert_eq!(sm.get_caller_range(in_a), exp_range);

    let in_b = exp_b_range.subrange(2, 1);
    assert_eq!(sm.get_caller_range(in_b), exp_range);

    let in_b_x = exp_b_x_range.subrange(2, 2);
    assert_eq!(sm.get_caller_range(in_b_x), exp_range);
}

#[test]
fn interpreted_range() {
    let sm = SourceMap::new();
    let (file_range, exp_a_range, exp_b_range, exp_b_x_range) = populate_sm(&sm);

    let filename = FileName::new_real("file.c");

    let in_file = file_range.subrange(15, 16);
    let interp_in_file = sm.get_interpreted_range(in_file);

    assert_eq!(interp_in_file.filename(), &filename);
    assert_eq!(interp_in_file.local_range(), 15..31);
    assert_eq!(interp_in_file.start_linecol(), LineCol { line: 0, col: 15 });
    assert_eq!(interp_in_file.end_linecol(), LineCol { line: 1, col: 10 });

    let check_exp_interp = |interp: InterpretedFileRange<'_>| {
        assert_eq!(interp.filename(), &filename);
        assert_eq!(interp.local_range(), 48..49);
    };

    let in_a = exp_a_range.subrange(3, 3);
    check_exp_interp(sm.get_interpreted_range(in_a));

    let in_b = exp_b_range.subrange(2, 1);
    check_exp_interp(sm.get_interpreted_range(in_b));

    let in_b_x = exp_b_x_range.subrange(2, 2);
    check_exp_interp(sm.get_interpreted_range(in_b_x));
}

#[test]
fn unfragmented_range() {
    let sm = SourceMap::new();
    let (file_range, exp_a_range, exp_b_range, exp_b_x_range) = populate_sm(&sm);

    let in_file = FragmentedSourceRange::new(file_range.subpos(3), file_range.subpos(10));
    assert_eq!(
        sm.get_unfragmented_range(in_file),
        file_range.subrange(3, 7)
    );

    let in_b_x = FragmentedSourceRange::new(exp_b_x_range.subpos(2), exp_b_x_range.subpos(4));
    assert_eq!(
        sm.get_unfragmented_range(in_b_x),
        exp_b_x_range.subrange(2, 2)
    );

    let cross_b_x = FragmentedSourceRange::new(exp_b_range.start(), exp_b_x_range.subpos(3));
    assert_eq!(
        sm.get_unfragmented_range(cross_b_x),
        exp_b_range.subrange(0, 2)
    );

    let cross_a_b = FragmentedSourceRange::new(exp_a_range.start(), exp_b_range.subpos(3));
    assert_eq!(sm.get_unfragmented_range(cross_a_b), exp_a_range);

    let cross_b_file = FragmentedSourceRange::new(file_range.subpos(40), exp_b_range.subpos(4));
    assert_eq!(
        sm.get_unfragmented_range(cross_b_file),
        file_range.subrange(40, 9)
    );
}
