use std::fmt;

use crate::diag::Reporter;
use crate::intern;
use crate::SourceMap;
use crate::{DResult, DiagManager};
use crate::{SourcePos, SourceRange};

use raw::{RawToken, RawTokenKind};
pub use token_kind::{PunctKind, TokenKind};

pub mod raw;
mod token_kind;

pub type Interner = intern::Interner<str>;
pub type Symbol = intern::Symbol<str>;

pub trait Lexer {
    fn next(&mut self, ctx: &mut LexCtx<'_, '_>) -> DResult<Token>;
}

pub struct LexCtx<'a, 'h> {
    pub ident_interner: &'a mut Interner,
    pub lit_interner: &'a mut Interner,
    pub diags: &'a mut DiagManager<'h>,
    pub smap: &'a mut SourceMap,
}

impl<'a, 'h> LexCtx<'a, 'h> {
    pub fn reporter(&mut self) -> Reporter<'_, 'h> {
        Reporter::new(self.diags, self.smap)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Token {
    pub kind: TokenKind,
    pub range: SourceRange,
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

        let intern_content = |interner: &mut Interner| interner.intern(raw.content.cleaned_str());

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
            RawTokenKind::Ident => TokenKind::Ident(intern_content(&mut ctx.ident_interner)),
            RawTokenKind::Number => TokenKind::Number(intern_content(&mut ctx.lit_interner)),

            RawTokenKind::Str => {
                check_terminated(ctx, "string literal")?;
                TokenKind::Str(intern_content(&mut ctx.lit_interner))
            }

            RawTokenKind::Char => {
                check_terminated(ctx, "character literal")?;
                TokenKind::Char(intern_content(&mut ctx.lit_interner))
            }
        };

        let range = SourceRange::new(pos, raw.content.str.len() as u32);
        Ok(FromRawResult::Tok(Token { kind, range }))
    }
}

pub struct DisplayToken<'t, 'a, 'h> {
    ctx: &'t LexCtx<'a, 'h>,
    tok: &'t Token,
}

impl fmt::Display for DisplayToken<'_, '_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.tok.kind {
            TokenKind::Eof => Ok(()),
            TokenKind::Unknown => write!(
                f,
                "{}",
                raw::clean(self.ctx.smap.get_spelling(self.tok.range))
            ),
            TokenKind::Punct(kind) => write!(f, "{}", kind),
            TokenKind::Ident(sym) => write!(f, "{}", &self.ctx.ident_interner[sym]),
            TokenKind::Number(sym) | TokenKind::Str(sym) | TokenKind::Char(sym) => {
                write!(f, "{}", &self.ctx.lit_interner[sym])
            }
        }
    }
}

impl Token {
    pub fn display<'t, 'a, 'h>(&'t self, ctx: &'t LexCtx<'a, 'h>) -> DisplayToken<'t, 'a, 'h> {
        DisplayToken { ctx, tok: self }
    }
}
