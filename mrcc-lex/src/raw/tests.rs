use super::*;

#[test]
fn skip_escaped_newlines() {
    let mut simple = SkipEscapedNewlines::new("simple\ntext");
    assert_eq!(simple.by_ref().collect::<String>(), "simple\ntext");
    assert!(!simple.tainted());

    let mut backslashes = SkipEscapedNewlines::new("'\\t'\n");
    assert_eq!(backslashes.by_ref().collect::<String>(), "'\\t'\n");
    assert!(!backslashes.tainted());

    let mut escaped = SkipEscapedNewlines::new("he\\\nllo world");
    assert_eq!(escaped.by_ref().collect::<String>(), "hello world");
    assert!(escaped.tainted());
}

fn first_token(input: &str) -> RawToken<'_> {
    Tokenizer::new(input).next_token()
}

fn check_single_token_kind(input: &str, kind: RawTokenKind) {
    let tok = first_token(input);
    assert_eq!(tok.kind, kind);
    assert_eq!(tok.content.str, input);
}

#[test]
fn eof() {
    check_single_token_kind("", RawTokenKind::Eof);
}

#[test]
fn newline() {
    check_single_token_kind("\n", RawTokenKind::Newline);
}

#[test]
fn whitespace() {
    check_single_token_kind(" \x0c \t \x0b", RawTokenKind::Ws);

    let tok = first_token(" \x0c \t \x0b\n ");
    assert_eq!(tok.kind, RawTokenKind::Ws);
    assert_eq!(tok.content.str, " \x0c \t \x0b");
}

#[test]
fn line_comment() {
    check_single_token_kind("// comment text", RawTokenKind::LineComment);
    check_single_token_kind("// comment\\\n text", RawTokenKind::LineComment);

    let tok = first_token("// comment\n text");
    assert_eq!(tok.kind, RawTokenKind::LineComment);
    assert_eq!(tok.content.str, "// comment");
}

#[test]
fn block_comment() {
    check_single_token_kind(
        "/* comment text */",
        RawTokenKind::BlockComment { terminated: true },
    );
    check_single_token_kind(
        "/* comment /* text */",
        RawTokenKind::BlockComment { terminated: true },
    );
    check_single_token_kind(
        "/* comment\n text */",
        RawTokenKind::BlockComment { terminated: true },
    );
    check_single_token_kind(
        "/* comment text",
        RawTokenKind::BlockComment { terminated: false },
    );
}
