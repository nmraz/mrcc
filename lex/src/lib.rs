#![warn(clippy::all)]

use intern::{Interner, Symbol};
use source_map::pos::SourceRange;

pub type IdentInterner = Interner<str>;
pub type IdentSym = Symbol<str>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommentKind {
    Line,
    Block,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    Unknown,
    Comment(CommentKind),

    Ident(IdentSym),
    Number,
    Str,
    Char,

    Hash,
    HashHash,

    Comma,
    Colon,
    Semi,

    LSquare,
    RSquare,
    LParen,
    RParen,
    LCurly,
    RCurly,

    Dot,
    Ellipsis,
    Arrow,

    Plus,
    PlusPlus,
    Minus,
    MinusMinus,
    Star,
    Slash,
    Perc,
    Amp,
    AmpAmp,
    Pipe,
    PipePipe,
    Caret,
    Tilde,
    Excl,
    Question,
    Less,
    LessLess,
    LessEq,
    Greater,
    GreaterGreater,
    GreaterEq,

    Eq,
    EqEq,
    ExclEq,
    PlusEq,
    MinusEq,
    StarEq,
    PercEq,
    AmpEq,
    PipeEq,
    CaretEq,
    LessLessEq,
    GreaterGreaterEq,
}

#[derive(Debug, Clone, Copy)]
pub struct Token {
    pub kind: TokenKind,
    pub range: SourceRange,
}
