use std::rc::Rc;

use crate::lex::raw::Tokenizer;
use crate::smap::FileContents;
use crate::SourcePos;

pub struct File {
    contents: Rc<FileContents>,
    start_pos: SourcePos,
    off: u32,
}

impl File {
    pub fn new(contents: Rc<FileContents>, start_pos: SourcePos) -> File {
        File {
            contents,
            start_pos,
            off: 0,
        }
    }

    pub fn with_tokenizer<R>(&mut self, f: impl FnOnce(SourcePos, &mut Tokenizer) -> R) -> R {
        let pos = self.start_pos.offset(self.off);
        let mut tokenizer = Tokenizer::new(&self.contents.src[self.off as usize..]);

        let ret = f(pos, &mut tokenizer);
        self.off += tokenizer.reader.pos() as u32;

        ret
    }
}
