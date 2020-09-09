use std::fmt;

use crate::diag::Reporter;
use crate::intern;
use crate::SourceMap;
use crate::{DResult, DiagManager};
use crate::{SourcePos, SourceRange};

pub use punct::PunctKind;
use raw::{RawToken, RawTokenKind};
pub use token::{ConvertedToken, ConvertedTokenKind, RangedToken, Token, TokenKind};

mod punct;
pub mod raw;
mod token;

pub type Interner = intern::Interner<str>;
pub type Symbol = intern::Symbol<str>;

pub trait Lexer {
    fn next(&mut self, ctx: &mut LexCtx<'_, '_>) -> DResult<Token>;
}

pub struct LexCtx<'a, 'h> {
    pub interner: &'a mut Interner,
    pub diags: &'a mut DiagManager<'h>,
    pub smap: &'a mut SourceMap,
}

impl<'a, 'h> LexCtx<'a, 'h> {
    pub fn new(
        interner: &'a mut Interner,
        diags: &'a mut DiagManager<'h>,
        smap: &'a mut SourceMap,
    ) -> Self {
        Self {
            interner,
            diags,
            smap,
        }
    }

    pub fn reporter(&mut self) -> Reporter<'_, 'h> {
        Reporter::new(self.diags, self.smap)
    }

    pub fn convert_raw(
        &mut self,
        raw: &RawToken<'_>,
        base_pos: SourcePos,
    ) -> DResult<ConvertedToken> {
        let pos = base_pos.offset(raw.content.off);

        let check_terminated = |this: &mut LexCtx<'_, '_>, kind: &str| {
            if !raw.terminated {
                this.reporter()
                    .error(pos, format!("unterminated {}", kind))
                    .emit()?;
            }
            Ok(())
        };

        let intern_content =
            |this: &mut LexCtx<'_, '_>| this.interner.intern(&raw.content.cleaned_str());

        let kind = match raw.kind {
            RawTokenKind::Unknown => ConvertedTokenKind::Real(TokenKind::Unknown),

            RawTokenKind::Eof => ConvertedTokenKind::Real(TokenKind::Eof),
            RawTokenKind::Newline => ConvertedTokenKind::Newline,

            RawTokenKind::Ws | RawTokenKind::LineComment => ConvertedTokenKind::Trivia,
            RawTokenKind::BlockComment => {
                check_terminated(self, "block comment")?;
                ConvertedTokenKind::Trivia
            }

            RawTokenKind::Punct(punct) => ConvertedTokenKind::Real(TokenKind::Punct(punct)),
            RawTokenKind::Ident => ConvertedTokenKind::Real(TokenKind::Ident(intern_content(self))),
            RawTokenKind::Number => {
                ConvertedTokenKind::Real(TokenKind::Number(intern_content(self)))
            }

            RawTokenKind::Str => {
                check_terminated(self, "string literal")?;
                ConvertedTokenKind::Real(TokenKind::Str(intern_content(self)))
            }

            RawTokenKind::Char => {
                check_terminated(self, "character literal")?;
                ConvertedTokenKind::Real(TokenKind::Char(intern_content(self)))
            }
        };

        let range = SourceRange::new(pos, raw.content.str.len() as u32);
        Ok(ConvertedToken { kind, range })
    }
}
#[derive(Debug, Clone, Copy)]
pub enum FromRawResult {
    Tok(Token),
    Newline,
    Trivia,
}

impl Token {
    pub fn from_raw(
        raw: &RawToken<'_>,
        base_pos: SourcePos,
        ctx: &mut LexCtx<'_, '_>,
    ) -> DResult<FromRawResult> {
        let pos = base_pos.offset(raw.content.off);

        let check_terminated = |ctx: &mut LexCtx<'_, '_>, kind: &str| {
            if !raw.terminated {
                ctx.reporter()
                    .error(pos, format!("unterminated {}", kind))
                    .emit()?;
            }
            Ok(())
        };

        let intern_content =
            |ctx: &mut LexCtx<'_, '_>| ctx.interner.intern(&raw.content.cleaned_str());

        let kind = match raw.kind {
            RawTokenKind::Unknown => TokenKind::Unknown,

            RawTokenKind::Eof => TokenKind::Eof,
            RawTokenKind::Newline => return Ok(FromRawResult::Newline),

            RawTokenKind::Ws | RawTokenKind::LineComment => return Ok(FromRawResult::Trivia),
            RawTokenKind::BlockComment => {
                check_terminated(ctx, "block comment")?;
                return Ok(FromRawResult::Trivia);
            }

            RawTokenKind::Punct(punct) => TokenKind::Punct(punct),
            RawTokenKind::Ident => TokenKind::Ident(intern_content(ctx)),
            RawTokenKind::Number => TokenKind::Number(intern_content(ctx)),

            RawTokenKind::Str => {
                check_terminated(ctx, "string literal")?;
                TokenKind::Str(intern_content(ctx))
            }

            RawTokenKind::Char => {
                check_terminated(ctx, "character literal")?;
                TokenKind::Char(intern_content(ctx))
            }
        };

        let range = SourceRange::new(pos, raw.content.str.len() as u32);
        Ok(FromRawResult::Tok(Token { kind, range }))
    }
}
