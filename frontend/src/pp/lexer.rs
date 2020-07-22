use crate::lex::{LexCtx, PunctKind, Token, TokenKind};
use crate::{DResult, SourceRange};

pub struct PPToken {
    pub tok: Token,
    pub line_start: bool,
    pub leading_trivia: bool,
}

impl PPToken {
    pub fn kind(&self) -> TokenKind {
        self.tok.kind
    }

    pub fn range(&self) -> SourceRange {
        self.tok.range
    }

    pub(super) fn is_directive_start(&self) -> bool {
        self.line_start && self.kind() == TokenKind::Punct(PunctKind::Hash)
    }
}

pub trait PPLexer {
    fn next(&mut self, ctx: &mut LexCtx<'_, '_>) -> DResult<PPToken>;
}
