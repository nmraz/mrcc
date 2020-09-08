use std::fmt;

use crate::lex::{LexCtx, PunctKind, Token, TokenKind};
use crate::{DResult, SourceRange};

#[derive(Debug, Copy, Clone)]
pub struct PpToken {
    pub tok: Token,
    pub line_start: bool,
    pub leading_trivia: bool,
}

pub struct DisplayPpToken<'t, 'a, 'h> {
    ppt: &'t PpToken,
    ctx: &'t LexCtx<'a, 'h>,
}

impl PpToken {
    pub fn kind(&self) -> TokenKind {
        self.tok.kind
    }

    pub fn range(&self) -> SourceRange {
        self.tok.range
    }

    pub fn display<'t, 'a, 'h>(&'t self, ctx: &'t LexCtx<'a, 'h>) -> DisplayPpToken<'t, 'a, 'h> {
        DisplayPpToken { ppt: self, ctx }
    }

    pub(super) fn is_directive_start(&self) -> bool {
        self.line_start && self.kind() == TokenKind::Punct(PunctKind::Hash)
    }
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

pub trait PpLexer {
    fn next(&mut self, ctx: &mut LexCtx<'_, '_>) -> DResult<PpToken>;
}
