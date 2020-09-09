use std::fmt;

use crate::SourceRange;

use super::{raw, LexCtx, PunctKind, Symbol};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    Unknown,
    Eof,

    Punct(PunctKind),

    Ident(Symbol),
    Number(Symbol),
    Str(Symbol),
    Char(Symbol),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConvertedTokenKind {
    Real(TokenKind),
    Newline,
    Trivia,
}

#[derive(Debug, Clone, Copy)]
pub struct Token<K = TokenKind> {
    pub kind: K,
    pub range: SourceRange,
}

impl<K> Token<K> {
    pub fn new(kind: K, range: SourceRange) -> Self {
        Self { kind, range }
    }

    pub fn map<L>(self, f: impl FnOnce(K) -> L) -> Token<L> {
        Token {
            kind: f(self.kind),
            range: self.range,
        }
    }

    pub fn maybe_map<L>(self, f: impl FnOnce(K) -> Option<L>) -> Option<Token<L>> {
        let Token { kind, range } = self;
        f(kind).map(|kind| Token { kind, range })
    }
}

pub type ConvertedToken = Token<ConvertedTokenKind>;

impl Token {
    pub fn display<'t, 'a, 'h>(&'t self, ctx: &'t LexCtx<'a, 'h>) -> Display<'t, 'a, 'h> {
        Display { tok: self, ctx }
    }
}

pub struct Display<'t, 'a, 'h> {
    tok: &'t Token,
    ctx: &'t LexCtx<'a, 'h>,
}

impl fmt::Display for Display<'_, '_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.tok.kind {
            TokenKind::Eof => Ok(()),
            TokenKind::Unknown => write!(
                f,
                "{}",
                raw::clean(self.ctx.smap.get_spelling(self.tok.range))
            ),
            TokenKind::Punct(kind) => write!(f, "{}", kind),
            TokenKind::Ident(sym)
            | TokenKind::Number(sym)
            | TokenKind::Str(sym)
            | TokenKind::Char(sym) => write!(f, "{}", &self.ctx.interner[sym]),
        }
    }
}
