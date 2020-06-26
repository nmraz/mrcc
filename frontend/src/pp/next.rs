use std::path::PathBuf;

use crate::lex::{LexCtx, PunctKind, Token, TokenKind};
use crate::DResult;

use super::file::FileProcessor;
use super::IncludeKind;

pub enum Action {
    Tok(Token),
    Include(PathBuf, IncludeKind),
}

pub fn next_action(ctx: &mut LexCtx<'_, '_>, processor: &mut FileProcessor<'_>) -> DResult<Action> {
    loop {
        let (tok, is_line_start) = loop {
            let is_line_start = processor.is_line_start();
            let raw = processor.next_token_skip_ws();
            if let Some(tok) = Token::from_raw(&raw, processor.base_pos(), ctx)? {
                break (tok, is_line_start);
            }
        };

        if is_line_start && tok.kind == TokenKind::Punct(PunctKind::Hash) {
            if let Some(action) = handle_directive(ctx, processor)? {
                break Ok(action);
            }
        } else {
            break Ok(Action::Tok(tok));
        }
    }
}

fn handle_directive(
    ctx: &mut LexCtx<'_, '_>,
    processor: &mut FileProcessor<'_>,
) -> DResult<Option<Action>> {
    todo!()
}
