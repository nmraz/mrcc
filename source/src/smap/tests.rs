use super::*;

#[test]
fn create_file() {
    let mut sm = SourceMap::new();

    let filename = FileName::real("file");
    let id = sm
        .create_file(
            filename.clone(),
            FileContents::new("line\nline\nline"),
            None,
        )
        .unwrap();

    let file_source = sm.get_source(id);
    let file = file_source.as_file().unwrap();
    assert_eq!(file.filename, filename);
}

#[test]
fn create_expansion() {
    let mut sm = SourceMap::new();

    let file_id = sm
        .create_file(
            FileName::real("file.c"),
            FileContents::new("#define A 5\nA;"),
            None,
        )
        .unwrap();

    let file_range = sm.get_source(file_id).range;
    let spelling_range = file_range.subrange(LocalRange::at(10.into(), 1.into()));

    let exp_source_id = sm
        .create_expansion(
            spelling_range,
            file_range.subrange(LocalRange::at(12.into(), 1.into())),
            ExpansionKind::Macro,
        )
        .unwrap();

    let exp_source = sm.get_source(exp_source_id);
    let exp = exp_source.as_expansion().unwrap();

    assert_eq!(exp.spelling_range, spelling_range);
    assert_eq!(exp.kind, ExpansionKind::Macro);
}

#[test]
#[should_panic]
fn include_pos_non_file() {
    let mut sm = SourceMap::new();

    let main_file_id = sm
        .create_file(
            FileName::real("file.c"),
            FileContents::new("#define A 5\nA;"),
            None,
        )
        .unwrap();

    let file_range = sm.get_source(main_file_id).range;

    let exp_source_id = sm
        .create_expansion(
            file_range.subrange(LocalRange::at(10.into(), 1.into())),
            file_range.subrange(LocalRange::at(12.into(), 1.into())),
            ExpansionKind::Macro,
        )
        .unwrap();

    let exp_range = sm.get_source(exp_source_id).range;

    sm.create_file(
        FileName::real("test.h"),
        FileContents::new("#define B 3"),
        Some(exp_range.start()),
    )
    .unwrap();
}

#[test]
fn lookup_pos() {
    let mut sm = SourceMap::new();

    let source_c_id = sm
        .create_file(
            FileName::real("file.c"),
            FileContents::new("#include <file.h>"),
            None,
        )
        .unwrap();

    let source_empty_id = sm
        .create_file(FileName::real("empty.c"), FileContents::new(""), None)
        .unwrap();

    let include_pos = sm.get_source(source_c_id).range.start();
    let source_h_id = sm
        .create_file(
            FileName::real("file.h"),
            FileContents::new("void f();"),
            Some(include_pos),
        )
        .unwrap();

    assert_eq!(
        sm.lookup_source_id(sm.get_source(source_c_id).range.subpos(3.into())),
        source_c_id
    );

    assert_eq!(
        sm.lookup_source_id(sm.get_source(source_empty_id).range.start()),
        source_empty_id
    );

    assert_eq!(
        sm.lookup_source_id(sm.get_source(source_h_id).range.subpos(3.into())),
        source_h_id
    );
}

#[test]
fn lookup_pos_last() {
    let mut sm = SourceMap::new();
    let id = sm
        .create_file(FileName::real("file"), FileContents::new(""), None)
        .unwrap();
    sm.lookup_source_id(sm.get_source(id).range.start()); // Shouldn't panic
}

#[test]
#[should_panic]
fn lookup_pos_past_last() {
    let mut sm = SourceMap::new();
    let id = sm
        .create_file(FileName::real("file"), FileContents::new(""), None)
        .unwrap();
    sm.lookup_source_id(sm.get_source(id).range.start().offset(2.into()));
}

fn populate_sm(sm: &mut SourceMap) -> (SourceRange, SourceRange, SourceRange, SourceRange) {
    let file_id = sm
        .create_file(
            FileName::real("file.c"),
            FileContents::new("#define B(x) (x + 3)\n#define A B(5 * 2)\nint x = A;"),
            None,
        )
        .unwrap();
    let file_range = sm.get_source(file_id).range;

    let exp_a_id = sm
        .create_expansion(
            file_range.subrange(LocalRange::at(31.into(), 8.into())),
            file_range.subrange(LocalRange::at(48.into(), 1.into())),
            ExpansionKind::Macro,
        )
        .unwrap();
    let exp_a_range = sm.get_source(exp_a_id).range;

    let exp_b_id = sm
        .create_expansion(
            file_range.subrange(LocalRange::at(13.into(), 7.into())),
            exp_a_range.subrange(LocalRange::at(0.into(), 8.into())),
            ExpansionKind::Macro,
        )
        .unwrap();
    let exp_b_range = sm.get_source(exp_b_id).range;

    let exp_b_x_id = sm
        .create_expansion(
            exp_a_range.subrange(LocalRange::at(2.into(), 5.into())),
            exp_b_range.subrange(LocalRange::at(1.into(), 1.into())),
            ExpansionKind::MacroArg,
        )
        .unwrap();
    let exp_b_x_range = sm.get_source(exp_b_x_id).range;

    (file_range, exp_a_range, exp_b_range, exp_b_x_range)
}

#[test]
fn immediate_spelling_pos() {
    let mut sm = SourceMap::new();
    let (file_range, exp_a_range, exp_b_range, exp_b_x_range) = populate_sm(&mut sm);

    let in_file = file_range.subpos(5.into());
    assert_eq!(sm.get_immediate_spelling_pos(in_file), None);

    let in_a = exp_a_range.subpos(4.into());
    assert_eq!(
        sm.get_immediate_spelling_pos(in_a),
        Some(file_range.subpos(35.into()))
    );

    let in_b = exp_b_range.subpos(2.into());
    assert_eq!(
        sm.get_immediate_spelling_pos(in_b),
        Some(file_range.subpos(15.into()))
    );

    let in_b_x = exp_b_x_range.subpos(3.into());
    assert_eq!(
        sm.get_immediate_spelling_pos(in_b_x),
        Some(exp_a_range.subpos(5.into()))
    );
}

#[test]
fn spelling_pos() {
    let mut sm = SourceMap::new();
    let (file_range, exp_a_range, exp_b_range, exp_b_x_range) = populate_sm(&mut sm);

    let in_file = file_range.subpos(5.into());
    assert_eq!(sm.get_spelling_pos(in_file), in_file);

    let in_a = exp_a_range.subpos(4.into());
    assert_eq!(sm.get_spelling_pos(in_a), file_range.subpos(35.into()));

    let in_b = exp_b_range.subpos(2.into());
    assert_eq!(sm.get_spelling_pos(in_b), file_range.subpos(15.into()));

    let in_b_x = exp_b_x_range.subpos(3.into());
    assert_eq!(sm.get_spelling_pos(in_b_x), file_range.subpos(36.into()));
}

#[test]
fn get_spelling() {
    let mut sm = SourceMap::new();
    let (file_range, exp_a_range, exp_b_range, exp_b_x_range) = populate_sm(&mut sm);

    assert_eq!(
        sm.get_spelling(file_range.subrange(LocalRange::at(0.into(), 7.into()))),
        "#define"
    );
    assert_eq!(
        sm.get_spelling(exp_a_range.subrange(LocalRange::at(2.into(), 5.into()))),
        "5 * 2"
    );
    assert_eq!(
        sm.get_spelling(exp_b_range.subrange(LocalRange::at(0.into(), 4.into()))),
        "(x +"
    );
    assert_eq!(
        sm.get_spelling(exp_b_x_range.subrange(LocalRange::at(0.into(), 5.into()))),
        "5 * 2"
    );
}

#[test]
fn immediate_replacement_range() {
    let mut sm = SourceMap::new();
    let (file_range, exp_a_range, exp_b_range, exp_b_x_range) = populate_sm(&mut sm);

    let in_file = file_range.subrange(LocalRange::at(5.into(), 2.into()));
    assert_eq!(sm.get_immediate_replacement_range(in_file), None);

    let in_a = exp_a_range.subrange(LocalRange::at(3.into(), 3.into()));
    assert_eq!(
        sm.get_immediate_replacement_range(in_a),
        Some(file_range.subrange(LocalRange::at(48.into(), 1.into())))
    );

    let in_b = exp_b_range.subrange(LocalRange::at(2.into(), 1.into()));
    assert_eq!(
        sm.get_immediate_replacement_range(in_b),
        Some(exp_a_range.subrange(LocalRange::at(0.into(), 8.into())))
    );

    let in_b_x = exp_b_x_range.subrange(LocalRange::at(2.into(), 2.into()));
    assert_eq!(
        sm.get_immediate_replacement_range(in_b_x),
        Some(exp_b_range.subrange(LocalRange::at(1.into(), 1.into())))
    );
}

#[test]
fn replacement_range() {
    let mut sm = SourceMap::new();
    let (file_range, exp_a_range, exp_b_range, exp_b_x_range) = populate_sm(&mut sm);

    let exp_range = file_range.subrange(LocalRange::at(48.into(), 1.into()));

    let in_file = file_range.subrange(LocalRange::at(5.into(), 2.into()));
    assert_eq!(sm.get_replacement_range(in_file), in_file);

    let in_a = exp_a_range.subrange(LocalRange::at(3.into(), 3.into()));
    assert_eq!(sm.get_replacement_range(in_a), exp_range);

    let in_b = exp_b_range.subrange(LocalRange::at(2.into(), 1.into()));
    assert_eq!(sm.get_replacement_range(in_b), exp_range);

    let in_b_x = exp_b_x_range.subrange(LocalRange::at(2.into(), 2.into()));
    assert_eq!(sm.get_replacement_range(in_b_x), exp_range);
}

#[test]
fn immediate_caller_range() {
    let mut sm = SourceMap::new();
    let (file_range, exp_a_range, exp_b_range, exp_b_x_range) = populate_sm(&mut sm);

    let in_file = file_range.subrange(LocalRange::at(5.into(), 2.into()));
    assert_eq!(sm.get_immediate_caller_range(in_file), None);

    let in_a = exp_a_range.subrange(LocalRange::at(3.into(), 3.into()));
    assert_eq!(
        sm.get_immediate_caller_range(in_a),
        Some(file_range.subrange(LocalRange::at(48.into(), 1.into())))
    );

    let in_b = exp_b_range.subrange(LocalRange::at(2.into(), 1.into()));
    assert_eq!(
        sm.get_immediate_caller_range(in_b),
        Some(exp_a_range.subrange(LocalRange::at(0.into(), 8.into())))
    );

    let in_b_x = exp_b_x_range.subrange(LocalRange::at(2.into(), 2.into()));
    assert_eq!(
        sm.get_immediate_caller_range(in_b_x),
        Some(exp_a_range.subrange(LocalRange::at(4.into(), 2.into())))
    );
}

#[test]
fn caller_range() {
    let mut sm = SourceMap::new();
    let (file_range, exp_a_range, exp_b_range, exp_b_x_range) = populate_sm(&mut sm);

    let exp_range = file_range.subrange(LocalRange::at(48.into(), 1.into()));

    let in_file = file_range.subrange(LocalRange::at(5.into(), 2.into()));
    assert_eq!(sm.get_caller_range(in_file), in_file);

    let in_a = exp_a_range.subrange(LocalRange::at(3.into(), 3.into()));
    assert_eq!(sm.get_caller_range(in_a), exp_range);

    let in_b = exp_b_range.subrange(LocalRange::at(2.into(), 1.into()));
    assert_eq!(sm.get_caller_range(in_b), exp_range);

    let in_b_x = exp_b_x_range.subrange(LocalRange::at(2.into(), 2.into()));
    assert_eq!(sm.get_caller_range(in_b_x), exp_range);
}

#[test]
fn interpreted_range() {
    let mut sm = SourceMap::new();
    let (file_range, ..) = populate_sm(&mut sm);

    let filename = FileName::real("file.c");

    let in_file = file_range.subrange(LocalRange::at(15.into(), 16.into()));
    let interp_in_file = sm.get_interpreted_range(in_file);

    assert_eq!(interp_in_file.filename(), &filename);
    assert_eq!(interp_in_file.range, LocalRange::new(15.into(), 31.into()));
    assert_eq!(interp_in_file.start_linecol(), LineCol { line: 0, col: 15 });
    assert_eq!(interp_in_file.end_linecol(), LineCol { line: 1, col: 10 });
}

#[test]
fn interpreted_range_line_snippets() {
    let mut sm = SourceMap::new();
    let (file_range, ..) = populate_sm(&mut sm);

    let gather_snippets = |range| {
        sm.get_interpreted_range(file_range.subrange(range))
            .line_snippets()
            .collect::<Vec<_>>()
    };

    assert_eq!(
        gather_snippets(LocalRange::at(2.into(), 15.into())),
        vec![LineSnippet {
            line: "#define B(x) (x + 3)",
            line_num: 0,
            range: LocalRange::at(2.into(), 15.into())
        }]
    );

    assert_eq!(
        gather_snippets(LocalRange::at(24.into(), 21.into())),
        vec![
            LineSnippet {
                line: "#define A B(5 * 2)",
                line_num: 1,
                range: LocalRange::at(3.into(), 15.into())
            },
            LineSnippet {
                line: "int x = A;",
                line_num: 2,
                range: LocalRange::at(0.into(), 5.into())
            },
        ]
    );

    assert_eq!(
        gather_snippets(LocalRange::at(5.into(), 37.into())),
        vec![
            LineSnippet {
                line: "#define B(x) (x + 3)",
                line_num: 0,
                range: LocalRange::at(5.into(), 15.into())
            },
            LineSnippet {
                line: "#define A B(5 * 2)",
                line_num: 1,
                range: LocalRange::at(0.into(), 18.into())
            },
            LineSnippet {
                line: "int x = A;",
                line_num: 2,
                range: LocalRange::at(0.into(), 2.into())
            },
        ]
    );
}

#[test]
fn unfragmented_range() {
    let mut sm = SourceMap::new();
    let (file_range, exp_a_range, exp_b_range, exp_b_x_range) = populate_sm(&mut sm);

    let in_file =
        FragmentedSourceRange::new(file_range.subpos(3.into()), file_range.subpos(10.into()));
    assert_eq!(
        sm.get_unfragmented_range(in_file),
        Some(file_range.subrange(LocalRange::at(3.into(), 7.into())))
    );

    let in_b_x = FragmentedSourceRange::new(
        exp_b_x_range.subpos(2.into()),
        exp_b_x_range.subpos(4.into()),
    );
    assert_eq!(
        sm.get_unfragmented_range(in_b_x),
        Some(exp_b_x_range.subrange(LocalRange::at(2.into(), 2.into())))
    );

    let cross_b_x = FragmentedSourceRange::new(exp_b_range.start(), exp_b_x_range.subpos(3.into()));
    assert_eq!(
        sm.get_unfragmented_range(cross_b_x),
        Some(exp_b_range.subrange(LocalRange::at(0.into(), 2.into())))
    );

    let cross_a_b = FragmentedSourceRange::new(exp_a_range.start(), exp_b_range.subpos(3.into()));
    assert_eq!(
        sm.get_unfragmented_range(cross_a_b),
        Some(exp_a_range.subrange(LocalRange::at(0.into(), 8.into())))
    );

    let cross_b_file =
        FragmentedSourceRange::new(file_range.subpos(40.into()), exp_b_range.subpos(4.into()));
    assert_eq!(
        sm.get_unfragmented_range(cross_b_file),
        Some(file_range.subrange(LocalRange::at(40.into(), 9.into())))
    );
}

#[test]
fn unfragmented_range_cross_file() {
    let mut sm = SourceMap::new();

    let source_id = sm
        .create_file(
            FileName::real("file.c"),
            FileContents::new("#include \"file.h\"\nint x = A;"),
            None,
        )
        .unwrap();

    let header_id = sm
        .create_file(
            FileName::real("file.h"),
            FileContents::new("#define B(x) (x + 3)\n#define A B(5 * 2)"),
            Some(sm.get_source(source_id).range.start()),
        )
        .unwrap();

    let fragmented = FragmentedSourceRange::new(
        sm.get_source(header_id).range.subpos(7.into()),
        sm.get_source(source_id).range.subpos(3.into()),
    );
    assert_eq!(sm.get_unfragmented_range(fragmented), None);
}
