use std::fmt;

use crate::lex::{LexCtx, PunctKind, Token, TokenKind};
use crate::{DResult, SourceRange};

pub trait PpLexer {
    fn next(&mut self, ctx: &mut LexCtx<'_, '_>) -> DResult<PpToken>;
}

#[derive(Debug, Copy, Clone)]
pub struct PpToken<K = TokenKind> {
    pub tok: Token<K>,
    pub line_start: bool,
    pub leading_trivia: bool,
}

impl<K: Copy> PpToken<K> {
    pub fn kind(&self) -> K {
        self.tok.kind
    }

    pub fn range(&self) -> SourceRange {
        self.tok.range
    }

    pub fn map<L>(self, f: impl FnOnce(K) -> L) -> PpToken<L> {
        PpToken {
            tok: self.tok.map(f),
            line_start: self.line_start,
            leading_trivia: self.leading_trivia,
        }
    }

    pub fn maybe_map<L>(self, f: impl FnOnce(K) -> Option<L>) -> Option<PpToken<L>> {
        f(self.tok.kind).map(|kind| self.map(|_| kind))
    }
}

impl PpToken {
    pub fn display<'t, 'a, 'h>(&'t self, ctx: &'t LexCtx<'a, 'h>) -> DisplayPpToken<'t, 'a, 'h> {
        DisplayPpToken { ppt: self, ctx }
    }

    pub(super) fn is_directive_start(&self) -> bool {
        self.line_start && self.kind() == TokenKind::Punct(PunctKind::Hash)
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
