use std::mem;

use mrcc_lex::raw::{Reader, Tokenizer};
use mrcc_lex::{ConvertedTokenKind, LexCtx, TokenKind};
use mrcc_source::{DResult, SourcePos};

use super::FileState;
use crate::PpToken;

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

    pub fn as_directive_token(&self) -> PpToken {
        self.map(|kind| kind.non_eod().unwrap_or(TokenKind::Eof))
    }

    pub fn is_eod(&self) -> bool {
        self.data().is_eod()
    }
}

pub struct Processor<'a> {
    state: &'a mut FileState,
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
        self.state
            .lookahead
            .take()
            .map_or_else(|| self.lex_next_token(ctx), Ok)
    }

    pub fn peek_token(&mut self, ctx: &mut LexCtx<'_, '_>) -> DResult<FileToken> {
        match self.state.lookahead {
            Some(tok) => Ok(tok),
            None => {
                let tok = self.lex_next_token(ctx)?;
                self.state.lookahead = Some(tok);
                Ok(tok)
            }
        }
    }

    pub fn next_real_token(&mut self, ctx: &mut LexCtx<'_, '_>) -> DResult<PpToken> {
        loop {
            if let Some(ppt) = self.next_token(ctx)?.real() {
                break Ok(ppt);
            }
        }
    }

    pub fn next_directive_token(&mut self, ctx: &mut LexCtx<'_, '_>) -> DResult<PpToken> {
        self.next_token(ctx).map(|tok| tok.as_directive_token())
    }

    pub fn report_and_advance(
        &mut self,
        ctx: &mut LexCtx<'_, '_>,
        ppt: PpToken,
        msg: &str,
    ) -> DResult<()> {
        ctx.reporter().error(ppt.range(), msg).emit()?;

        if ppt.data() != TokenKind::Eof {
            self.advance_to_eod(ctx)?;
        }

        Ok(())
    }

    pub fn advance_to_eod(&mut self, ctx: &mut LexCtx<'_, '_>) -> DResult<()> {
        while !self.next_token(ctx)?.is_eod() {}
        Ok(())
    }

    pub fn reader(&mut self) -> &mut Reader<'a> {
        &mut self.tokenizer_mut().reader
    }

    pub fn off(&self) -> u32 {
        self.tokenizer.reader.off()
    }

    pub fn pos(&self) -> SourcePos {
        self.check_lookahead();
        self.base_pos.offset(self.off())
    }

    fn lex_next_token(&mut self, ctx: &mut LexCtx<'_, '_>) -> DResult<FileToken> {
        let mut leading_trivia = false;

        let (tok, new_line_start) = loop {
            let converted = ctx.convert_raw(&self.tokenizer_mut().next_token(), self.base_pos)?;
            match converted.data {
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

    fn tokenizer_mut(&mut self) -> &mut Tokenizer<'a> {
        self.check_lookahead();
        &mut self.tokenizer
    }

    fn check_lookahead(&self) {
        assert!(
            self.state.lookahead.is_none(),
            "accessing tokenizer with pending lookahead"
        )
    }
}
