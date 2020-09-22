#![warn(rust_2018_idioms)]

use mrcc_source::{DResult, DiagManager, DiagReporter, SourceMap, SourcePos, SourceRange};

pub use punct::PunctKind;
use raw::{RawToken, RawTokenKind};
pub use token::{ConvertedToken, ConvertedTokenKind, Token, TokenKind};

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

    pub fn reporter(&mut self) -> DiagReporter<'_, 'h> {
        DiagReporter::new(self.diags, self.smap)
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

        let range = if kind == ConvertedTokenKind::Newline {
            // Newlines are special: we don't want the range to cover the newline character itself,
            // as that would make it end on the next line.
            pos.into()
        } else {
            SourceRange::new(pos, raw.content.str.len() as u32)
        };

        Ok(ConvertedToken { data: kind, range })
    }
}
