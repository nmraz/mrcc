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

