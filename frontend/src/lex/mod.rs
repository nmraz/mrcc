use crate::diag::DiagnosticBuilder;
use crate::intern;
use crate::SourceMap;
use crate::{DiagManager, DiagResult};
use crate::{FragmentedSourceRange, SourcePos, SourceRange};

use raw::{RawToken, RawTokenKind};
pub use token_kind::{CommentKind, PunctKind, TokenKind};

pub mod raw;
mod token_kind;

pub type Interner = intern::Interner<str>;
pub type Symbol = intern::Symbol<str>;

pub struct LexCtx<'a> {
    pub ident_interner: &'a mut Interner,
    pub lit_interner: &'a mut Interner,
    pub diags: &'a mut DiagManager,
    pub smap: &'a mut SourceMap,
}

impl LexCtx<'_> {
    pub fn warning(
        &mut self,
        primary_range: FragmentedSourceRange,
        msg: impl Into<String>,
    ) -> DiagnosticBuilder<'_> {
        self.diags.warning(self.smap, primary_range, msg)
    }

    pub fn error(
        &mut self,
        primary_range: FragmentedSourceRange,
        msg: impl Into<String>,
    ) -> DiagnosticBuilder<'_> {
        self.diags.error(self.smap, primary_range, msg)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Token {
    pub kind: TokenKind,
    pub range: SourceRange,
}

impl Token {
    pub fn from_raw(
        raw: &RawToken,
        stream_start: SourcePos,
        ctx: &mut LexCtx<'_>,
    ) -> DiagResult<Option<Self>> {
        let pos = stream_start.offset(raw.content.off);

        let check_terminated = |ctx: &mut LexCtx<'_>, kind: &str| {
            if !raw.terminated {
                ctx.error(pos.into(), format!("unterminated {}", kind))
                    .emit()?;
            }
            Ok(())
        };

        let intern_content = |interner: &mut Interner| interner.intern(raw.content.cleaned_str());

        let kind = match raw.kind {
            RawTokenKind::Unknown => TokenKind::Unknown,
            RawTokenKind::Eof => TokenKind::Eof,
            RawTokenKind::Ws => return Ok(None),
            RawTokenKind::Newline => return Ok(None),

            RawTokenKind::Comment(comment) => {
                check_terminated(ctx, "block comment")?;
                TokenKind::Comment(comment)
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
        Ok(Some(Token { kind, range }))
    }
}

pub trait Lexer {
    fn next(&mut self, ctx: &mut LexCtx<'_>) -> DiagResult<Token>;
}
