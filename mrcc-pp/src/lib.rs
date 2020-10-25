//! Preprocessor implementation.

#![warn(rust_2018_idioms)]

use std::mem;
use std::path::PathBuf;

use mrcc_lex::{LexCtx, Lexer, Token, TokenKind};
use mrcc_source::{DResult, SourceId, SourceRange};

use active_file::{Action, ActiveFiles};
use expand::MacroState;
use file::{IncludeError, IncludeKind, IncludeLoader};

pub use token::PpToken;

mod active_file;
mod expand;
mod file;
mod token;

/// Helper structure implementing the builder pattern for constructing a new
/// [`Preprocessor`](struct.Preprocessor.html).
pub struct PreprocessorBuilder<'a, 'b, 'h> {
    ctx: &'a mut LexCtx<'b, 'h>,
    main_id: SourceId,
    parent_dir: Option<PathBuf>,
    include_dirs: Vec<PathBuf>,
}

impl<'a, 'b, 'h> PreprocessorBuilder<'a, 'b, 'h> {
    /// Creates a new builder for preprocessing the source file specified by `main_id` in
    /// `ctx.smap`.
    ///
    /// `main_id` should point into a file source, not an expansion.
    pub fn new(ctx: &'a mut LexCtx<'b, 'h>, main_id: SourceId) -> Self {
        Self {
            ctx,
            main_id,
            parent_dir: None,
            include_dirs: Vec::new(),
        }
    }

    /// Sets the presumed parent directory of the main source file, for use in `#include "filename"`
    /// resolution.
    pub fn parent_dir(&mut self, dir: PathBuf) -> &mut Self {
        self.parent_dir = Some(dir);
        self
    }

    /// Sets the include directories for use in `#include <filename>` resolution. These directories
    /// will be scanned from first to last.
    pub fn include_dirs(&mut self, dirs: Vec<PathBuf>) -> &mut Self {
        self.include_dirs = dirs;
        self
    }

    /// Constructs a new preprocessor using the options set on this builder.
    ///
    /// # Panics
    ///
    /// Panics if the provided `main_id` does not point into a file source.
    pub fn build(&mut self) -> Preprocessor {
        Preprocessor {
            active_files: ActiveFiles::new(&self.ctx.smap, self.main_id, self.parent_dir.take()),
            include_loader: IncludeLoader::new(mem::take(&mut self.include_dirs)),
            macro_state: MacroState::new(),
        }
    }
}

/// A lexer that transparently preprocesses its input source code (up through translation phase 4)
/// and exposes the resulting token stream.
///
/// Use [`PreprocessorBuilder`](struct.PreprocessorBuilder.html) to construct a new `Preprocessor`.
pub struct Preprocessor {
    active_files: ActiveFiles,
    include_loader: IncludeLoader,
    macro_state: MacroState,
}

impl Preprocessor {
    /// Lexes the next preprocessing token from the input, interpreting any preprocessing directives
    /// encountered.
    ///
    /// This method returns tokens with leading whitespace/newline information, which may be
    /// relevant to certain clients. If this auxiliary information is not needed, consider using
    /// `next()` instead.
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

    /// Returns the next action to be taken (either a new token or a new include) from the top of
    /// the active include stack.
    fn top_file_action(&mut self, ctx: &mut LexCtx<'_, '_>) -> DResult<Action> {
        self.active_files
            .top()
            .next_action(ctx, &mut self.macro_state)
    }

    /// Handles the loading and activation of an included file, reporting any errors encountered.
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
                ctx.reporter().fatal(range, msg).emit().unwrap_err()
            })?;

        if self
            .active_files
            .push_include(&mut ctx.smap, filename, file, range.start())
            .is_err()
        {
            ctx.reporter()
                .fatal(range, "translation unit too large")
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
