#![warn(rust_2018_idioms)]

use std::mem;
use std::path::PathBuf;

use mrcc_lex::{LexCtx, Lexer, Token, TokenKind};
use mrcc_source::{diag::Level, DResult, SourceId, SourceRange};

use active_file::{Action, ActiveFiles};
use file::{IncludeError, IncludeKind, IncludeLoader};
use state::State;

pub use lexer::PpToken;

mod active_file;
mod expand;
mod file;
mod lexer;
mod state;

pub struct PreprocessorBuilder<'a, 'b, 'h> {
    ctx: &'a mut LexCtx<'b, 'h>,
    main_id: SourceId,
    parent_dir: Option<PathBuf>,
    include_dirs: Vec<PathBuf>,
}

impl<'a, 'b, 'h> PreprocessorBuilder<'a, 'b, 'h> {
    pub fn new(ctx: &'a mut LexCtx<'b, 'h>, main_id: SourceId) -> Self {
        Self {
            ctx,
            main_id,
            parent_dir: None,
            include_dirs: Vec::new(),
        }
    }

    pub fn parent_dir(&mut self, dir: PathBuf) -> &mut Self {
        self.parent_dir = Some(dir);
        self
    }

    pub fn include_dirs(&mut self, dirs: Vec<PathBuf>) -> &mut Self {
        self.include_dirs = dirs;
        self
    }

    pub fn build(&mut self) -> Preprocessor {
        Preprocessor {
            active_files: ActiveFiles::new(&self.ctx.smap, self.main_id, self.parent_dir.take()),
            include_loader: IncludeLoader::new(mem::take(&mut self.include_dirs)),
            state: State::new(self.ctx),
        }
    }
}

pub struct Preprocessor {
    active_files: ActiveFiles,
    include_loader: IncludeLoader,
    state: State,
}

impl Preprocessor {
    pub fn next_pp(&mut self, ctx: &mut LexCtx<'_, '_>) -> DResult<PpToken> {
        let ppt = loop {
            match self.top_file_action(ctx)? {
                Action::Tok(ppt) => {
                    if ppt.data() == TokenKind::Eof && self.active_files.have_includes() {
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
        let file = self
            .include_loader
            .load(&filename, kind, self.active_files.top().file())
            .map_err(|err| {
                let msg = match err {
                    IncludeError::NotFound => format!("include '{}' not found", filename.display()),
                    IncludeError::Io { full_path, error } => {
                        format!("failed to read '{}': {}", full_path.display(), error)
                    }
                };
                ctx.reporter()
                    .report(Level::Fatal, range, msg)
                    .emit()
                    .unwrap_err()
            })?;

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
