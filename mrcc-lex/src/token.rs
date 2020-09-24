use std::fmt;

use mrcc_source::SourceRange;

use super::{raw, LexCtx, PunctKind, Symbol};

/// Enum representing token types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    Unknown,
    Eof,

    Punct(PunctKind),
    Ident(Symbol),

    /// A preprocessing number. Note that the definition of preprocessing numbers is rather lax and
    /// matches many invalid numeric literals as well. See §6.4.8 for details.
    Number(Symbol),
    Str(Symbol),
    Char(Symbol),
}

/// Represents the possible token types returned by
/// [`LexCtx::convert_raw`](struct.LexCtx.html#method.convert_raw).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConvertedTokenKind {
    /// A real token.
    Real(TokenKind),
    /// A newline.
    Newline,
    /// Trivia, such as whitespace or a comment.
    Trivia,
}

/// Represents a token - data attached to a source range.
///
/// By default, the data is a [`TokenKind`](enum.TokenKind.html).
#[derive(Debug, Clone, Copy)]
pub struct Token<D = TokenKind> {
    pub data: D,
    pub range: SourceRange,
}

impl<D> Token<D> {
    /// Creates a new token with the specified data and range.
    pub fn new(data: D, range: SourceRange) -> Self {
        Self { data, range }
    }

    /// Applies `f` to the token's data, preserving the range.
    pub fn map<E>(self, f: impl FnOnce(D) -> E) -> Token<E> {
        Token {
            data: f(self.data),
            range: self.range,
        }
    }

    /// Applies `f` to the token's data. If it returns `Some`, returns a new token with the original
    /// range and the processed data. Otherwise, returns `None`.
    pub fn maybe_map<E>(self, f: impl FnOnce(D) -> Option<E>) -> Option<Token<E>> {
        let Token { data, range } = self;
        f(data).map(|data| Token { data, range })
    }
}

/// Converted token returned by [`LexCtx::convert_raw`](struct.LexCtx.html#method.convert_raw).
pub type ConvertedToken = Token<ConvertedTokenKind>;

impl Token {
    /// Returns an object that implements `fmt::Display` for printing the token.
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
