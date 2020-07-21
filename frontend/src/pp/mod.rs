use crate::lex::{LexCtx, Lexer, PunctKind, Token, TokenKind};
use crate::smap::SourceId;
use crate::{DResult, SourceRange};

use file::Action;
use files::Files;
use state::State;

mod file;
mod files;
mod state;

pub enum IncludeKind {
    Str,
    Angle,
}

pub struct PPToken {
    tok: Token,
    line_start: bool,
    leading_trivia: bool,
}

impl PPToken {
    pub fn kind(&self) -> TokenKind {
        self.tok.kind
    }

    pub fn range(&self) -> SourceRange {
        self.tok.range
    }

    fn is_directive_start(&self) -> bool {
        self.line_start && self.kind() == TokenKind::Punct(PunctKind::Hash)
    }
}

pub struct Preprocessor {
    files: Files,
    state: State,
}

impl Preprocessor {
    pub fn new(ctx: &mut LexCtx<'_, '_>, main_id: SourceId) -> Self {
        Self {
            files: Files::new(&ctx.smap, main_id),
            state: State::new(ctx),
        }
    }

    pub fn next_pp(&mut self, ctx: &mut LexCtx<'_, '_>) -> DResult<PPToken> {
        let ppt = loop {
            match self.top_file_action(ctx)? {
                Action::Tok(ppt) => {
                    if ppt.kind() == TokenKind::Eof && self.files.have_includes() {
                        self.files.pop_include();
                    } else {
                        break ppt;
                    }
                }
                Action::Include(_, _) => todo!(),
            }
        };

        Ok(ppt)
    }

    fn top_file_action(&mut self, ctx: &mut LexCtx<'_, '_>) -> DResult<Action> {
        self.files.top().next_action(ctx, &mut self.state)
    }
}

impl Lexer for Preprocessor {
    fn next(&mut self, ctx: &mut LexCtx<'_, '_>) -> DResult<Token> {
        self.next_pp(ctx).map(|ppt| ppt.tok)
    }
}
