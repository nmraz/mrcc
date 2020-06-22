use std::rc::Rc;

use crate::lex::{LexCtx, Lexer, Token};
use crate::smap::FileContents;
use crate::SourcePos;

use file::Files;

mod file;

pub struct Preprocessor {
    files: Files,
}

impl Preprocessor {
    pub fn new(contents: Rc<FileContents>, start_pos: SourcePos) -> Self {
        Self {
            files: Files::new(contents, start_pos),
        }
    }
}

impl Lexer for Preprocessor {
    fn next(&mut self, ctx: &mut LexCtx<'_>) -> Token {
        self.files.top().with_tokenizer(|pos, tokenizer| loop {
            let raw = tokenizer.next_token();
            if let Some(tok) = Token::from_raw(&raw, pos, ctx) {
                break tok;
            }
        })
    }
}
