use std::path::PathBuf;
use std::rc::Rc;

use crate::lex::{LexCtx, Token};
use crate::smap::FileContents;
use crate::DResult;
use crate::SourcePos;

use super::state::State;
use super::IncludeKind;

pub use macro_arg_lexer::MacroArgLexer;
use next::NextActionCtx;
use processor::Processor;
use state::FileState;

mod macro_arg_lexer;
mod next;
mod processor;
mod state;

pub enum Action {
    Tok(Token),
    Include(PathBuf, IncludeKind),
}

pub struct File {
    contents: Rc<FileContents>,
    state: FileState,
    start_pos: SourcePos,
    off: u32,
}

impl File {
    pub fn new(contents: Rc<FileContents>, start_pos: SourcePos) -> File {
        File {
            contents,
            state: FileState::default(),
            start_pos,
            off: 0,
        }
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
            &self.contents.src[off as usize..],
            self.start_pos.offset(off),
        );
        let ret = f(&mut processor);
        self.off += processor.off();
        ret
    }
}
