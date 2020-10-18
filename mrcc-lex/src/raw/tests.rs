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

fn check_first_token(input: &str, tok_str: &str, kind: RawTokenKind) {
    let tok = Tokenizer::new(input).next_token();
    assert_eq!(tok.kind, kind);
    assert_eq!(tok.content.str, tok_str);
}

fn check_single_token_kind(input: &str, kind: RawTokenKind) {
    check_first_token(input, input, kind);
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
    check_first_token(" \x0c \t \x0b\n ", " \x0c \t \x0b", RawTokenKind::Ws);
}

#[test]
fn line_comment() {
    check_single_token_kind("// comment text", RawTokenKind::LineComment);
    check_single_token_kind("// comment\\\n text", RawTokenKind::LineComment);
    check_first_token("// comment\n text", "// comment", RawTokenKind::LineComment);
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

#[test]
fn simple_punct() {
    fn check(punct: char, kind: PunctKind) {
        // Add some trailing characters to make sure we don't consume lookahead when trying to match
        // a compound punctuator.
        let input = format!("{}xxx", punct);
        check_first_token(&input, &input[..1], RawTokenKind::Punct(kind));
    }

    check('#', PunctKind::Hash);
    check(',', PunctKind::Comma);
    check(';', PunctKind::Semi);
    check('[', PunctKind::LSquare);
    check(']', PunctKind::RSquare);
    check('(', PunctKind::LParen);
    check(')', PunctKind::RParen);
    check('{', PunctKind::LCurly);
    check('}', PunctKind::RCurly);
    check('.', PunctKind::Dot);
    check('+', PunctKind::Plus);
    check('-', PunctKind::Minus);
    check('*', PunctKind::Star);
    check('/', PunctKind::Slash);
    check('%', PunctKind::Perc);
    check('&', PunctKind::Amp);
    check('|', PunctKind::Pipe);
    check('^', PunctKind::Caret);
    check('~', PunctKind::Tilde);
    check('!', PunctKind::Bang);
    check('?', PunctKind::Question);
    check('<', PunctKind::Less);
    check('>', PunctKind::Greater);
    check('=', PunctKind::Eq);
}
