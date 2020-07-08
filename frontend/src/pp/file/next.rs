use crate::lex::raw::{RawToken, RawTokenKind, Reader, Tokenizer};
use crate::lex::{LexCtx, PunctKind, Token, TokenKind};
use crate::DResult;
use crate::{SourcePos, SourceRange};

use super::{Action, FileState, State};

enum FileToken {
    Tok { tok: Token, is_line_start: bool },
    Newline,
}

pub struct NextActionCtx<'a, 'b, 'h> {
    ctx: &'a mut LexCtx<'b, 'h>,
    state: &'a mut State,
    file_state: &'a mut FileState,
    base_pos: SourcePos,
    tokenizer: Tokenizer<'a>,
}

impl<'a, 'b, 'h> NextActionCtx<'a, 'b, 'h> {
    pub fn new(
        ctx: &'a mut LexCtx<'b, 'h>,
        state: &'a mut State,
        file_state: &'a mut FileState,
        base_pos: SourcePos,
        remaining_source: &'a str,
    ) -> Self {
        Self {
            ctx,
            state,
            file_state,
            base_pos,
            tokenizer: Tokenizer::new(remaining_source),
        }
    }

    pub fn off(&self) -> u32 {
        self.tokenizer.reader.pos() as u32
    }

    pub fn next_action(&mut self) -> DResult<Action> {
        loop {
            let (tok, is_line_start) = loop {
                if let FileToken::Tok { tok, is_line_start } = self.next_file_token()? {
                    break (tok, is_line_start);
                }
            };

            if is_line_start && tok.kind == TokenKind::Punct(PunctKind::Hash) {
                if let Some(action) = self.handle_directive()? {
                    break Ok(action);
                }
            } else {
                break Ok(Action::Tok(tok));
            }
        }
    }

    fn handle_directive(&mut self) -> DResult<Option<Action>> {
        let tok = match self.next_file_token()? {
            FileToken::Tok { tok, .. } => tok,
            FileToken::Newline => return Ok(None),
        };

        let ident = match tok.kind {
            TokenKind::Ident(ident) => ident,
            _ => {
                self.invalid_directive(tok.range)?;
                return Ok(None);
            }
        };

        todo!()
    }

    fn invalid_directive(&mut self, range: SourceRange) -> DResult<()> {
        self.ctx
            .error(range.into(), "invalid preprocessing directive")
            .emit()
    }

    fn next_file_token(&mut self) -> DResult<FileToken> {
        let is_line_start = self.file_state.is_line_start;
        let raw = self.next_token_skip_ws();
        Token::from_raw(&raw, self.base_pos, self.ctx).map(|res| {
            res.map(|tok| FileToken::Tok { tok, is_line_start })
                .unwrap_or(FileToken::Newline)
        })
    }

    fn next_token_skip_ws(&mut self) -> RawToken<'a> {
        loop {
            let tok = self.next_token();
            if tok.kind != RawTokenKind::Ws {
                break tok;
            }
        }
    }

    fn next_token(&mut self) -> RawToken<'a> {
        let tok = self.tokenizer.next_token();

        if tok.kind == RawTokenKind::Newline {
            self.file_state.is_line_start = true;
        } else if !is_trivia(tok.kind) {
            self.file_state.is_line_start = false;
        }

        tok
    }

    fn reader(&mut self) -> &mut Reader<'a> {
        &mut self.tokenizer.reader
    }
}

fn is_trivia(kind: RawTokenKind) -> bool {
    match kind {
        RawTokenKind::Ws | RawTokenKind::Comment(..) => true,
        _ => false,
    }
}
