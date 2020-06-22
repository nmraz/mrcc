use std::rc::Rc;

use crate::lex::{LexCtx, Lexer, Token};
use crate::smap::FileContents;
use crate::SourcePos;

use file::File;

mod file;

pub struct Preprocessor {
    main_file: File,
    include_stack: Vec<File>,
}

impl Preprocessor {
    pub fn new(contents: Rc<FileContents>, start_pos: SourcePos) -> Self {
        Self {
            main_file: File::new(contents, start_pos),
            include_stack: vec![],
        }
    }
}

impl Lexer for Preprocessor {
    fn next(&mut self, ctx: &mut LexCtx<'_>) -> Token {
        let file = self.include_stack.last_mut().unwrap_or(&mut self.main_file);

        file.with_tokenizer(|pos, tokenizer| loop {
            let raw = tokenizer.next_token();
            if let Some(tok) = Token::from_raw(&raw, pos, ctx) {
                break tok;
            }
        })
    }
}
