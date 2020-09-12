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
pub struct Token<D = TokenKind> {
    pub data: D,
    pub range: SourceRange,
}

impl<D> Token<D> {
    pub fn new(kind: D, range: SourceRange) -> Self {
        Self { data: kind, range }
    }

    pub fn map<E>(self, f: impl FnOnce(D) -> E) -> Token<E> {
        Token {
            data: f(self.data),
            range: self.range,
        }
    }

    pub fn maybe_map<E>(self, f: impl FnOnce(D) -> Option<E>) -> Option<Token<E>> {
        let Token { data, range } = self;
        f(data).map(|data| Token { data, range })
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
        match self.tok.data {
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
