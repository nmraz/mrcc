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

fn check_single_token(input: &str, kind: RawTokenKind) {
    check_first_token(input, input, kind);
}

#[test]
fn eof() {
    check_single_token("", RawTokenKind::Eof);
}

#[test]
fn newline() {
    check_single_token("\n", RawTokenKind::Newline);
}

#[test]
fn whitespace() {
    check_single_token(" \x0c \t \x0b", RawTokenKind::Ws);
    check_first_token(" \x0c \t \x0b\n ", " \x0c \t \x0b", RawTokenKind::Ws);
}

#[test]
fn line_comment() {
    check_single_token("// comment text", RawTokenKind::LineComment);
    check_single_token("// comment\\\n text", RawTokenKind::LineComment);
    check_first_token("// comment\n text", "// comment", RawTokenKind::LineComment);
}

#[test]
fn block_comment() {
    check_single_token(
        "/* comment text */",
        RawTokenKind::BlockComment { terminated: true },
    );
    check_single_token(
        "/* comment /* text */",
        RawTokenKind::BlockComment { terminated: true },
    );
    check_single_token(
        "/* comment\n text */",
        RawTokenKind::BlockComment { terminated: true },
    );
    check_single_token(
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

#[test]
fn double_punct() {
    fn check(punct: &str, kind: PunctKind) {
        let input = format!("{}xxx", punct);
        check_first_token(&input, punct, RawTokenKind::Punct(kind));

        let input = punct.repeat(2);
        check_first_token(&input, punct, RawTokenKind::Punct(kind));
    }

    check("##", PunctKind::HashHash);
    check("->", PunctKind::Arrow);
    check("++", PunctKind::PlusPlus);
    check("--", PunctKind::MinusMinus);
    check("&&", PunctKind::AmpAmp);
    check("||", PunctKind::PipePipe);
    check("<<", PunctKind::LessLess);
    check("<=", PunctKind::LessEq);
    check(">>", PunctKind::GreaterGreater);
    check(">=", PunctKind::GreaterEq);
    check("==", PunctKind::EqEq);
    check("!=", PunctKind::BangEq);
    check("+=", PunctKind::PlusEq);
    check("-=", PunctKind::MinusEq);
    check("*=", PunctKind::StarEq);
    check("/=", PunctKind::SlashEq);
    check("%=", PunctKind::PercEq);
    check("&=", PunctKind::AmpEq);
    check("|=", PunctKind::PipeEq);
    check("^=", PunctKind::CaretEq);
}

#[test]
fn triple_punct() {
    fn check(punct: &str, kind: PunctKind) {
        check_single_token(punct, RawTokenKind::Punct(kind));
    }

    check("...", PunctKind::Ellipsis);
    // `...` is special in that its prefix `..` is not a complete punctuator itself.
    check_first_token("..", ".", RawTokenKind::Punct(PunctKind::Dot));

    check("<<=", PunctKind::LessLessEq);
    check(">>=", PunctKind::GreaterGreaterEq);
}

#[test]
fn digraphs() {
    fn check(digraph: &str, kind: PunctKind) {
        for &suffix in &["", "<", ":", "%"] {
            let input = format!("{}{}", digraph, suffix);
            check_first_token(&input, digraph, RawTokenKind::Punct(kind));
        }
    }

    check("<:", PunctKind::LSquare);
    check(":>", PunctKind::RSquare);
    check("<%", PunctKind::LCurly);
    check("%>", PunctKind::RCurly);
    check("%:", PunctKind::Hash);
    check("%:%:", PunctKind::HashHash);
}
