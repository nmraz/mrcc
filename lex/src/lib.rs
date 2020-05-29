use std::fmt;

use diag::{DiagnosticBuilder, Manager as DiagManager};
use intern::{Interner, Symbol};
use smap::pos::{FragmentedSourceRange, SourceRange};
use smap::SourceMap;

pub mod raw;
mod token_kind;

pub use token_kind::{CommentKind, PunctKind, TokenKind};

pub type IdentInterner = Interner<str>;
pub type IdentSym = Symbol<str>;

pub type TokInterner = Interner<str>;
pub type TokSym = Symbol<str>;

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

impl LexCtx<'_> {
    pub fn warning(
        &mut self,
        msg: impl Into<String>,
        primary_range: FragmentedSourceRange,
    ) -> DiagnosticBuilder<'_> {
        self.diags.warning(self.smap, msg, primary_range)
    }

    pub fn error(
        &mut self,
        msg: impl Into<String>,
        primary_range: FragmentedSourceRange,
    ) -> DiagnosticBuilder<'_> {
        self.diags.error(self.smap, msg, primary_range)
    }
}

pub trait Lexer {
    fn next(&mut self, ctx: &mut LexCtx<'_>) -> Token;
}
