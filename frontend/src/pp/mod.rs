use crate::lex::{LexCtx, Lexer, Token, TokenKind};
use crate::smap::SourceId;
use crate::DResult;

use active_file::{Action, ActiveFiles};
use state::State;

pub use lexer::PpToken;

mod active_file;
mod file;
mod lexer;
mod state;

pub struct Preprocessor {
    active_files: ActiveFiles,
    state: State,
}

impl Preprocessor {
    pub fn new(ctx: &mut LexCtx<'_, '_>, main_id: SourceId) -> Self {
        Self {
            active_files: ActiveFiles::new(&ctx.smap, main_id),
            state: State::new(ctx),
        }
    }

    pub fn next_pp(&mut self, ctx: &mut LexCtx<'_, '_>) -> DResult<PpToken> {
        let ppt = loop {
            match self.top_file_action(ctx)? {
                Action::Tok(ppt) => {
                    if ppt.kind() == TokenKind::Eof && self.active_files.have_includes() {
                        self.active_files.pop_include();
                    } else {
                        break ppt;
                    }
                }
                Action::Include {
                    filename,
                    kind,
                    pos,
                } => {}
            }
        };

        Ok(ppt)
    }

    fn top_file_action(&mut self, ctx: &mut LexCtx<'_, '_>) -> DResult<Action> {
        self.active_files.top().next_action(ctx, &mut self.state)
    }
}

impl Lexer for Preprocessor {
    fn next(&mut self, ctx: &mut LexCtx<'_, '_>) -> DResult<Token> {
        self.next_pp(ctx).map(|ppt| ppt.tok)
    }
}
