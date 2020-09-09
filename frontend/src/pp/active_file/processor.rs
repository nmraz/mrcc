use std::mem;

use crate::lex::raw::{Reader, Tokenizer};
use crate::lex::{ConvertedTokenKind, LexCtx, TokenKind};
use crate::{DResult, SourcePos};

use super::FileState;
use super::PpToken;

#[derive(Debug, Copy, Clone)]
pub enum FileTokenKind {
    Real(TokenKind),
    Newline,
}

impl FileTokenKind {
    pub fn real(self) -> Option<TokenKind> {
        match self {
            FileTokenKind::Real(kind) => Some(kind),
            FileTokenKind::Newline => None,
        }
    }

    pub fn non_eod(self) -> Option<TokenKind> {
        self.real().filter(|&kind| kind != TokenKind::Eof)
    }

    pub fn is_eod(&self) -> bool {
        self.non_eod().is_none()
    }
}

pub type FileToken = PpToken<FileTokenKind>;

impl FileToken {
    pub fn real(&self) -> Option<PpToken> {
        self.maybe_map(|kind| kind.real())
    }

    pub fn non_eod(&self) -> Option<PpToken> {
        self.maybe_map(|kind| kind.non_eod())
    }

    pub fn is_eod(&self) -> bool {
        self.kind().is_eod()
    }
}

pub struct Processor<'a> {
    pub state: &'a mut FileState,
    tokenizer: Tokenizer<'a>,
    base_pos: SourcePos,
}

impl<'a> Processor<'a> {
    pub fn new(state: &'a mut FileState, remaining_src: &'a str, base_pos: SourcePos) -> Self {
        Self {
            state,
            tokenizer: Tokenizer::new(remaining_src),
            base_pos,
        }
    }

    pub fn next_token(&mut self, ctx: &mut LexCtx<'_, '_>) -> DResult<FileToken> {
        let mut leading_trivia = false;

        let (tok, new_line_start) = loop {
            let converted = ctx.convert_raw(&self.tokenizer.next_token(), self.base_pos)?;
            match converted.kind {
                ConvertedTokenKind::Real(kind) => {
                    break (converted.map(|_| FileTokenKind::Real(kind)), false)
                }

                ConvertedTokenKind::Newline => {
                    break (converted.map(|_| FileTokenKind::Newline), true);
                }

                ConvertedTokenKind::Trivia => {
                    leading_trivia = true;
                }
            }
        };

        Ok(FileToken {
            tok,
            line_start: mem::replace(&mut self.state.line_start, new_line_start),
            leading_trivia,
        })
    }

    pub fn next_directive_token(&mut self, ctx: &mut LexCtx<'_, '_>) -> DResult<PpToken> {
        self.next_token(ctx)
            .map(|tok| tok.map(|kind| kind.non_eod().unwrap_or(TokenKind::Eof)))
    }

    pub fn advance_to_eod(&mut self, ctx: &mut LexCtx<'_, '_>) -> DResult<()> {
        while !self.next_token(ctx)?.is_eod() {}
        Ok(())
    }

    pub fn reader(&mut self) -> &mut Reader<'a> {
        &mut self.tokenizer.reader
    }

    pub fn off(&self) -> u32 {
        self.tokenizer.reader.off()
    }

    pub fn pos(&self) -> SourcePos {
        self.base_pos.offset(self.off())
    }
}
