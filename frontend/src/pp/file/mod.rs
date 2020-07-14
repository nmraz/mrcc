use std::path::PathBuf;
use std::rc::Rc;

use crate::lex::{LexCtx, Token};
use crate::smap::FileContents;
use crate::DResult;
use crate::SourcePos;

use super::state::State;
use super::IncludeKind;

use next::NextActionCtx;
use processor::Processor;
use state::FileState;

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
