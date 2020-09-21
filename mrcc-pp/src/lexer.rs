use std::fmt;

use mrcc_lex::{LexCtx, PunctKind, Token, TokenKind};
use mrcc_source::{DResult, SourceRange};

pub trait PpLexer {
    fn next(&mut self, ctx: &mut LexCtx<'_, '_>) -> DResult<PpToken>;
    fn peek(&mut self, ctx: &mut LexCtx<'_, '_>) -> DResult<PpToken>;

    fn next_macro_arg(&mut self, ctx: &mut LexCtx<'_, '_>) -> DResult<PpToken> {
        self.next(ctx)
    }
}

#[derive(Debug, Copy, Clone)]
pub struct PpToken<D = TokenKind> {
    pub tok: Token<D>,
    pub line_start: bool,
    pub leading_trivia: bool,
}

impl<D: Copy> PpToken<D> {
    pub fn data(&self) -> D {
        self.tok.data
    }

    pub fn range(&self) -> SourceRange {
        self.tok.range
    }

    pub fn map<E>(self, f: impl FnOnce(D) -> E) -> PpToken<E> {
        PpToken {
            tok: self.tok.map(f),
            line_start: self.line_start,
            leading_trivia: self.leading_trivia,
        }
    }

    pub fn maybe_map<E>(self, f: impl FnOnce(D) -> Option<E>) -> Option<PpToken<E>> {
        f(self.tok.data).map(|kind| self.map(|_| kind))
    }
}

impl PpToken {
    pub fn display<'t, 'a, 'h>(&'t self, ctx: &'t LexCtx<'a, 'h>) -> DisplayPpToken<'t, 'a, 'h> {
        DisplayPpToken { ppt: self, ctx }
    }

    pub(super) fn is_directive_start(&self) -> bool {
        self.line_start && self.data() == TokenKind::Punct(PunctKind::Hash)
    }
}

pub struct DisplayPpToken<'t, 'a, 'h> {
    ppt: &'t PpToken,
    ctx: &'t LexCtx<'a, 'h>,
}

impl fmt::Display for DisplayPpToken<'_, '_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let ppt = self.ppt;
        if ppt.leading_trivia {
            write!(f, " ")?;
        }
        write!(f, "{}", ppt.tok.display(self.ctx))
    }
}
