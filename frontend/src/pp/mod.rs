use std::path::PathBuf;

use crate::diag::Level;
use crate::lex::{LexCtx, Lexer, Token, TokenKind};
use crate::smap::SourceId;
use crate::{DResult, SourceRange};

use active_file::{Action, ActiveFiles};
use file::{IncludeError, IncludeKind, IncludeLoader};
use state::State;

pub use lexer::PpToken;

mod active_file;
mod file;
mod lexer;
mod state;

pub struct Preprocessor {
    active_files: ActiveFiles,
    include_loader: IncludeLoader,
    state: State,
}

impl Preprocessor {
    pub fn new(ctx: &mut LexCtx<'_, '_>, main_id: SourceId, parent_dir: Option<PathBuf>) -> Self {
        Self {
            active_files: ActiveFiles::new(&ctx.smap, main_id, parent_dir),
            include_loader: IncludeLoader::new(Vec::new()),
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
                    range,
                } => self.handle_include(ctx, filename, kind, range)?,
            }
        };

        Ok(ppt)
    }

    fn top_file_action(&mut self, ctx: &mut LexCtx<'_, '_>) -> DResult<Action> {
        self.active_files.top().next_action(ctx, &mut self.state)
    }

    fn handle_include(
        &mut self,
        ctx: &mut LexCtx<'_, '_>,
        filename: PathBuf,
        kind: IncludeKind,
        range: SourceRange,
    ) -> DResult<()> {
        let file = match self
            .include_loader
            .load(&filename, kind, self.active_files.top().file())
        {
            Ok(file) => file,
            Err(err) => {
                let msg = match err {
                    IncludeError::NotFound => format!("include '{}' not found", filename.display()),
                    IncludeError::Io { full_path, error } => format!(
                        "failed to include '{}' ({}): {}",
                        filename.display(),
                        full_path.display(),
                        error
                    ),
                };
                ctx.reporter().report(Level::Fatal, range, msg).emit()?;
                unreachable!();
            }
        };

        if self
            .active_files
            .push_include(&mut ctx.smap, filename, file, range.start())
            .is_err()
        {
            ctx.reporter()
                .report(Level::Fatal, range, "translation unit too large")
                .emit()?;
        }

        Ok(())
    }
}

impl Lexer for Preprocessor {
    fn next(&mut self, ctx: &mut LexCtx<'_, '_>) -> DResult<Token> {
        self.next_pp(ctx).map(|ppt| ppt.tok)
    }
}
