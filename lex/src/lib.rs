#![warn(clippy::all)]

use diag::Manager as DiagManager;
use intern::{Interner, Symbol};
use source_map::pos::SourceRange;
use source_map::SourceMap;

pub mod raw;

pub type IdentInterner = Interner<str>;
pub type IdentSym = Symbol<str>;

pub type TokInterner = Interner<str>;
pub type TokSym = Symbol<str>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommentKind {
    Line,
    Block,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PunctKind {
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
    SlashEq,
    PercEq,
    AmpEq,
    PipeEq,
    CaretEq,
    LessLessEq,
    GreaterGreaterEq,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    Unknown,
    Eof,
    Comment(CommentKind),

    Punct(PunctKind),

    Ident(IdentSym),
    Number(TokSym),
    Str(TokSym),
    Char(TokSym),
}

#[derive(Debug, Clone, Copy)]
pub struct Token {
    pub kind: TokenKind,
    pub range: SourceRange,
}

pub struct LexCtx<'a> {
    pub ident_interner: &'a mut IdentInterner,
    pub tok_interner: &'a mut TokInterner,
    pub diags: &'a mut DiagManager,
    pub smap: &'a mut SourceMap,
}

pub trait Lexer {
    fn next(&mut self, ctx: &mut LexCtx<'_>) -> Token;
}
