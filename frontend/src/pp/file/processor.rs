use std::mem;

use crate::lex::raw::{Reader, Tokenizer};
use crate::lex::{FromRawResult, LexCtx, Token, TokenKind};
use crate::{DResult, SourcePos};

use super::FileState;
use super::PPToken;

pub enum FileToken {
    Tok(PPToken),
    Newline,
}

impl FileToken {
    pub fn is_eod(&self) -> bool {
        match self {
            FileToken::Newline => true,
            FileToken::Tok(PPToken { tok, .. }) => tok.kind == TokenKind::Eof,
        }
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
                    break FileToken::Tok(PPToken {
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

    pub fn base_pos(&self) -> SourcePos {
        self.base_pos
    }

    pub fn off(&self) -> u32 {
        self.tokenizer.reader.off()
    }

    pub fn pos(&self) -> SourcePos {
        self.base_pos.offset(self.off())
    }
}
