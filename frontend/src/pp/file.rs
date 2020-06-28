use std::rc::Rc;

use crate::lex::raw::{RawToken, RawTokenKind, Reader, Tokenizer};
use crate::smap::FileContents;
use crate::SourcePos;

pub struct PendingIf {
    pub pos: SourcePos,
}

pub struct FileState {
    pub is_line_start: bool,
    pub pending_ifs: Vec<PendingIf>,
}

impl Default for FileState {
    fn default() -> Self {
        Self {
            is_line_start: true,
            pending_ifs: vec![],
        }
    }
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

    pub fn with_processor<R>(&mut self, f: impl FnOnce(&mut FileProcessor<'_>) -> R) -> R {
        let pos = self.start_pos.offset(self.off);

        let mut processor = FileProcessor {
            state: &mut self.state,
            base_pos: pos,
            tokenizer: Tokenizer::new(&self.contents.src[self.off as usize..]),
        };
        let ret = f(&mut processor);
        self.off += processor.reader().pos() as u32;

        ret
    }
}

pub struct FileProcessor<'a> {
    pub state: &'a mut FileState,
    base_pos: SourcePos,
    tokenizer: Tokenizer<'a>,
}

impl<'a> FileProcessor<'a> {
    pub fn next_token(&mut self) -> RawToken<'a> {
        let tok = self.tokenizer.next_token();

        if tok.kind == RawTokenKind::Newline {
            self.state.is_line_start = true;
        } else if !is_trivia(tok.kind) {
            self.state.is_line_start = false;
        }

        tok
    }

    pub fn next_token_skip_ws(&mut self) -> RawToken<'a> {
        loop {
            let tok = self.next_token();
            if tok.kind != RawTokenKind::Ws {
                break tok;
            }
        }
    }

    pub fn reader(&mut self) -> &mut Reader<'a> {
        &mut self.tokenizer.reader
    }

    pub fn base_pos(&self) -> SourcePos {
        self.base_pos
    }
}

fn is_trivia(kind: RawTokenKind) -> bool {
    match kind {
        RawTokenKind::Ws | RawTokenKind::Comment(..) => true,
        _ => false,
    }
}
