//! Lexer traits and definitions.

#![warn(rust_2018_idioms)]

use mrcc_source::{DResult, DiagManager, DiagReporter, SourceMap, SourcePos, SourceRange};

pub use punct::PunctKind;
use raw::{RawToken, RawTokenKind};
pub use token::{ConvertedToken, ConvertedTokenKind, Token, TokenKind};

mod punct;
pub mod raw;
mod token;

/// A string interner type, used to hold identifiers and literals.
pub type Interner = intern::Interner<str>;
/// A symbol for use with `Interner`.
pub type Symbol = intern::Symbol<str>;

/// Trait representing a source of tokens.
pub trait Lexer {
    /// Lexes the next token from the stream.
    ///
    /// This function returns a `DResult` as it may report diagnostics through `ctx`.
    fn next(&mut self, ctx: &mut LexCtx<'_, '_>) -> DResult<Token>;
}

/// A context structure passed to lexers, tying together different pieces of state.
pub struct LexCtx<'a, 'h> {
    /// The interner into which the lexer should place lexed identifiers and literals.
    pub interner: &'a mut Interner,
    /// The diagnostics manager to use when reporting warnings and errors.
    pub diags: &'a mut DiagManager<'h>,
    /// The source map, for use with `diags` and for generating token locations.
    pub smap: &'a mut SourceMap,
}

impl<'a, 'h> LexCtx<'a, 'h> {
    /// Creates a new context with the provided fields.
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

    /// Returns a reporter for emitting diagnostics.
    pub fn reporter(&mut self) -> DiagReporter<'_, 'h> {
        self.diags.reporter(self.smap)
    }

    /// Converts a raw token to a proper token, emitting errors if it is malformed.
    ///
    /// `base_pos` should be the position relative to which `raw.content.off` was specified.
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
            |this: &mut LexCtx<'_, '_>| this.interner.intern_cow(raw.content.cleaned_str());

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
