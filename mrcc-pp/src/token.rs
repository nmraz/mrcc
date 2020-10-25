use std::fmt;

use mrcc_lex::{LexCtx, PunctKind, Token, TokenKind};
use mrcc_source::SourceRange;

/// A token with auxiliary data relevent to the preprocessor.
#[derive(Debug, Copy, Clone)]
pub struct PpToken<D = TokenKind> {
    /// The underlying lexer token.
    pub tok: Token<D>,

    /// Indicates whether this token was the first on its line.
    pub line_start: bool,

    /// Indicates whether this token was separated from the previous token or newline by any
    /// whitespace or comments.
    pub leading_trivia: bool,
}

impl<D: Copy> PpToken<D> {
    /// Returns the underlying token's contained data.
    pub fn data(&self) -> D {
        self.tok.data
    }

    /// Returns the underlying token's range.
    pub fn range(&self) -> SourceRange {
        self.tok.range
    }

    /// Applies `f` to the underlying token's data, preserving all other information.
    pub fn map<E>(self, f: impl FnOnce(D) -> E) -> PpToken<E> {
        PpToken {
            tok: self.tok.map(f),
            line_start: self.line_start,
            leading_trivia: self.leading_trivia,
        }
    }

    /// Applies `f` to the underlying token's data. If it returns `Some`, a new token with the
    /// processed data is returned. Otherwise, `None` is returned.
    pub fn maybe_map<E>(self, f: impl FnOnce(D) -> Option<E>) -> Option<PpToken<E>> {
        f(self.tok.data).map(|kind| self.map(|_| kind))
    }
}

impl PpToken {
    /// Returns an object that implements `fmt::Display` for printing the token. This displays
    /// leading trivia as a single space character, as per translation phase 3.
    pub fn display<'t, 'a, 'h>(&'t self, ctx: &'t LexCtx<'a, 'h>) -> Display<'t, 'a, 'h> {
        Display { ppt: self, ctx }
    }

    pub(super) fn is_directive_start(&self) -> bool {
        self.line_start && self.data() == TokenKind::Punct(PunctKind::Hash)
    }
}

pub struct Display<'t, 'a, 'h> {
    ppt: &'t PpToken,
    ctx: &'t LexCtx<'a, 'h>,
}

impl fmt::Display for Display<'_, '_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let ppt = self.ppt;
        if ppt.leading_trivia {
            write!(f, " ")?;
        }
        write!(f, "{}", ppt.tok.display(self.ctx))
    }
}
