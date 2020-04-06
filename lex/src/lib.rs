#![warn(clippy::all)]

use diag::Manager as DiagManager;
use intern::{Interner, Symbol};
use source_map::pos::SourceRange;
use source_map::SourceMap;

pub mod raw;

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
    Eof,
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

pub trait Lexer {
    fn next(&mut self, interner: &mut IdentInterner, diags: &mut DiagManager) -> Token;
    fn source_map(&self) -> &SourceMap;
}
