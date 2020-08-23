use std::path::PathBuf;
use std::rc::Rc;

use crate::lex::LexCtx;
use crate::smap::{FileName, SourceId, SourcesTooLargeError};
use crate::DResult;
use crate::{SourceMap, SourcePos};

use super::file::{File, IncludeKind};
use super::state::State;
use super::PpToken;

pub use macro_arg_lexer::MacroArgLexer;
use next::NextActionCtx;
use processor::Processor;
use state::FileState;

mod macro_arg_lexer;
mod next;
mod processor;
mod state;

pub enum Action {
    Tok(PpToken),
    Include {
        filename: PathBuf,
        kind: IncludeKind,
        pos: SourcePos,
    },
}

pub struct ActiveFile {
    file: Rc<File>,
    state: FileState,
    start_pos: SourcePos,
    off: u32,
}

impl ActiveFile {
    pub fn new(file: Rc<File>, start_pos: SourcePos) -> ActiveFile {
        ActiveFile {
            file,
            state: FileState::default(),
            start_pos,
            off: 0,
        }
    }

    pub fn file(&self) -> &Rc<File> {
        &self.file
    }

    pub fn next_action(&mut self, ctx: &mut LexCtx<'_, '_>, state: &mut State) -> DResult<Action> {
        self.with_processor(|processor| NextActionCtx::new(ctx, state, processor).next_action())
    }

    pub fn with_macro_arg_lexer<R, F>(&mut self, f: F) -> R
    where
        F: FnOnce(MacroArgLexer<'_, '_>) -> R,
    {
        self.with_processor(|processor| f(MacroArgLexer::new(processor)))
    }

    fn with_processor<R, F: FnOnce(&mut Processor<'_>) -> R>(&mut self, f: F) -> R {
        let off = self.off;
        let mut processor = Processor::new(
            &mut self.state,
            &self.file.contents.src[off as usize..],
            self.start_pos.offset(off),
        );
        let ret = f(&mut processor);
        self.off += processor.off();
        ret
    }
}

pub struct ActiveFiles {
    main: ActiveFile,
    includes: Vec<ActiveFile>,
}

impl ActiveFiles {
    pub fn new(smap: &SourceMap, main_id: SourceId, parent_dir: Option<PathBuf>) -> Self {
        let source = smap.get_source(main_id);
        let file = source
            .as_file()
            .expect("preprocessor requires a file source");

        ActiveFiles {
            main: ActiveFile::new(
                File::new(Rc::clone(&file.contents), parent_dir),
                source.range.start(),
            ),
            includes: vec![],
        }
    }

    pub fn top(&mut self) -> &mut ActiveFile {
        self.includes.last_mut().unwrap_or(&mut self.main)
    }

    pub fn have_includes(&self) -> bool {
        !self.includes.is_empty()
    }

    pub fn push_include(
        &mut self,
        smap: &mut SourceMap,
        filename: FileName,
        file: Rc<File>,
        include_pos: SourcePos,
    ) -> Result<(), SourcesTooLargeError> {
        let id = smap.create_file(filename, Rc::clone(&file.contents), Some(include_pos))?;
        self.includes
            .push(ActiveFile::new(file, smap.get_source(id).range.start()));
        Ok(())
    }

    pub fn pop_include(&mut self) {
        self.includes.pop();
    }
}
