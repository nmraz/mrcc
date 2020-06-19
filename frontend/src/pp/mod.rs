use std::rc::Rc;

use crate::lex::{raw::Tokenizer, LexCtx, Lexer, Token};
use crate::smap::FileContents;
use crate::SourcePos;

struct OpenFile {
    pub contents: Rc<FileContents>,
    pub start_pos: SourcePos,
    pub off: u32,
}

impl OpenFile {
    pub fn new(contents: Rc<FileContents>, start_pos: SourcePos) -> Self {
        OpenFile {
            contents,
            start_pos,
            off: 0,
        }
    }
}

pub struct Preprocessor {
    main_file: OpenFile,
    include_stack: Vec<OpenFile>,
}

impl Preprocessor {
    pub fn new(contents: Rc<FileContents>, start_pos: SourcePos) -> Self {
        Self {
            main_file: OpenFile::new(contents, start_pos),
            include_stack: vec![],
        }
    }
}

impl Lexer for Preprocessor {
    fn next(&mut self, ctx: &mut LexCtx<'_>) -> Token {
        let mut file = self.include_stack.last_mut().unwrap_or(&mut self.main_file);
        let mut tokenizer = Tokenizer::new(&file.contents.src[file.off as usize..]);

        let tok = loop {
            let raw = tokenizer.next_token();
            if let Some(tok) = Token::from_raw(&raw, file.start_pos, ctx) {
                break tok;
            }
        };

        file.off = tok.range.end().offset_from(file.start_pos);

        tok
    }
}
