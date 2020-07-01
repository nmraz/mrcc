use std::path::PathBuf;
use std::rc::Rc;

use crate::lex::{LexCtx, Token};
use crate::smap::FileContents;
use crate::DResult;
use crate::SourcePos;

use super::state::State;
use super::IncludeKind;

use next::NextActionCtx;
use state::FileState;

mod next;
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
        let mut next_ctx = NextActionCtx::new(
            ctx,
            state,
            &mut self.state,
            self.start_pos.offset(self.off),
            &self.contents.src[self.off as usize..],
        );
        let ret = next_ctx.next_action();
        self.off += next_ctx.off();
        ret
    }
}
