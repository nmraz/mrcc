use std::mem;

use crate::lex::raw::{Reader, Tokenizer};
use crate::lex::{FromRawResult, LexCtx, Token, TokenKind};
use crate::{DResult, SourcePos};

use super::FileState;
use super::PpToken;

pub enum FileToken {
    Tok(PpToken),
    Newline,
}

impl FileToken {
    pub fn non_eod(&self) -> Option<&PpToken> {
        match self {
            FileToken::Tok(ppt) if ppt.kind() != TokenKind::Eof => Some(ppt),
            _ => None,
        }
    }

    pub fn is_eod(&self) -> bool {
        self.non_eod().is_none()
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

        let ret = loop {
            match Token::from_raw(&self.tokenizer.next_token(), self.base_pos, ctx)? {
                FromRawResult::Tok(tok) => {
                    break FileToken::Tok(PpToken {
                        tok,
                        line_start: mem::replace(&mut self.state.line_start, false),
                        leading_trivia,
                    });
                }

                FromRawResult::Newline => {
                    self.state.line_start = true;
                    break FileToken::Newline;
                }

                FromRawResult::Trivia => {
                    leading_trivia = true;
                }
            };
        };

        Ok(ret)
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
