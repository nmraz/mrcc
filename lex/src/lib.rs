use diag::{DiagnosticBuilder, Manager as DiagManager};
use intern::{Interner, Symbol};
use smap::pos::{FragmentedSourceRange, SourcePos, SourceRange};
use smap::SourceMap;

pub mod raw;
mod token_kind;

use raw::{RawToken, RawTokenKind};
pub use token_kind::{CommentKind, PunctKind, TokenKind};

pub type IdentInterner = Interner<str>;
pub type IdentSym = Symbol<str>;

pub type TokInterner = Interner<str>;
pub type TokSym = Symbol<str>;

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

#[derive(Debug, Clone, Copy)]
pub struct Token {
    pub kind: TokenKind,
    pub range: SourceRange,
}

impl Token {
    pub fn from_raw(raw: RawToken, pos: SourcePos, ctx: &mut LexCtx<'_>) -> Option<Self> {
        let kind = match raw.kind {
            RawTokenKind::Unknown => TokenKind::Unknown,
            RawTokenKind::Eof => TokenKind::Eof,
            RawTokenKind::Ws => return None,
            RawTokenKind::Newline => return None,
            RawTokenKind::Comment(comment) => {
                if !raw.terminated {
                    ctx.error("unterminated block comment", pos.into());
                }
                TokenKind::Comment(comment)
            }
            RawTokenKind::Punct(punct) => TokenKind::Punct(punct),
            RawTokenKind::Ident => {
                TokenKind::Ident(ctx.ident_interner.intern(raw.content.cleaned_str()))
            }
            RawTokenKind::Number => {
                TokenKind::Number(ctx.tok_interner.intern(raw.content.cleaned_str()))
            }
            RawTokenKind::Str => {
                if !raw.terminated {
                    ctx.error("unterminated string literal", pos.into());
                }
                TokenKind::Str(ctx.tok_interner.intern(raw.content.cleaned_str()))
            }
            RawTokenKind::Char => {
                if !raw.terminated {
                    ctx.error("unterminated character literal", pos.into());
                }
                TokenKind::Char(ctx.tok_interner.intern(raw.content.cleaned_str()))
            }
        };

        let range = SourceRange::new(pos, raw.content.str.len() as u32);
        Some(Token { kind, range })
    }
}

pub trait Lexer {
    fn next(&mut self, ctx: &mut LexCtx<'_>) -> Token;
}
