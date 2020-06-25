use std::path::PathBuf;

use crate::lex::{LexCtx, Lexer, Token, TokenKind};
use crate::smap::SourceId;
use crate::DResult;

use file::Files;
use next::{next_action, Action};

mod file;
mod next;
mod state;

pub enum IncludeKind {
    Str,
    Angle,
}

pub struct Preprocessor {
    files: Files,
}

impl Preprocessor {
    pub fn new(ctx: &mut LexCtx<'_, '_>, main_id: SourceId) -> Self {
        Self {
            files: Files::new(&ctx.smap, main_id),
        }
    }

    fn top_file_action(&mut self, ctx: &mut LexCtx<'_, '_>) -> DResult<Action> {
        self.files
            .top()
            .with_processor(|processor| next_action(ctx, processor))
    }
}

impl Lexer for Preprocessor {
    fn next(&mut self, ctx: &mut LexCtx<'_, '_>) -> DResult<Token> {
        let tok = loop {
            match self.top_file_action(ctx)? {
                Action::Tok(Token {
                    kind: TokenKind::Eof,
                    ..
                }) if self.files.have_includes() => {
                    self.files.pop_include();
                }
                Action::Tok(tok) => break tok,
                Action::Include(_, _) => todo!(),
            }
        };

        Ok(tok)
    }
}
