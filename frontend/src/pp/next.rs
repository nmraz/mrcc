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
    todo!()
}
