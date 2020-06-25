use std::path::PathBuf;

use crate::lex::{LexCtx, Token};
use crate::DResult;

use super::file::FileProcessor;
use super::IncludeKind;

pub enum Action {
    Tok(Token),
    Include(PathBuf, IncludeKind),
}

pub fn next_action(ctx: &mut LexCtx<'_, '_>, processor: &mut FileProcessor<'_>) -> DResult<Action> {
    let (tok, is_line_start) = loop {
        let is_line_start = processor.is_line_start();
        let raw = processor.next_token_skip_ws();
        if let Some(tok) = Token::from_raw(&raw, processor.base_pos(), ctx)? {
            break (tok, is_line_start);
        }
    };

    Ok(Action::Tok(tok))
}
