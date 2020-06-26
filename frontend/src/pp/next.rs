use std::path::PathBuf;

use crate::lex::{LexCtx, PunctKind, Token, TokenKind};
use crate::DResult;

use super::file::FileProcessor;
use super::state::State;
use super::IncludeKind;

pub enum Action {
    Tok(Token),
    Include(PathBuf, IncludeKind),
}

impl State {
    pub fn next_action(
        &mut self,
        ctx: &mut LexCtx<'_, '_>,
        processor: &mut FileProcessor<'_>,
    ) -> DResult<Action> {
        loop {
            let (tok, is_line_start) = loop {
                let is_line_start = processor.state.is_line_start;
                let raw = processor.next_token_skip_ws();
                if let Some(tok) = Token::from_raw(&raw, processor.base_pos(), ctx)? {
                    break (tok, is_line_start);
                }
            };

            if is_line_start && tok.kind == TokenKind::Punct(PunctKind::Hash) {
                if let Some(action) = self.handle_directive(ctx, processor)? {
                    break Ok(action);
                }
            } else {
                break Ok(Action::Tok(tok));
            }
        }
    }

    fn handle_directive(
        &mut self,
        ctx: &mut LexCtx<'_, '_>,
        processor: &mut FileProcessor<'_>,
    ) -> DResult<Option<Action>> {
        todo!()
    }
}
