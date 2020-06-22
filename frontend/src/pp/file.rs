use std::rc::Rc;

use crate::lex::raw::Tokenizer;
use crate::smap::{FileContents, SourcesTooLargeError};
use crate::{SourceMap, SourcePos};

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

pub struct Files {
    main: File,
    includes: Vec<File>,
}

impl Files {
    pub fn new(main_contents: Rc<FileContents>, main_start_pos: SourcePos) -> Self {
        Files {
            main: File::new(main_contents, main_start_pos),
            includes: vec![],
        }
    }

    pub fn top(&mut self) -> &mut File {
        self.includes.last_mut().unwrap_or(&mut self.main)
    }

    pub fn have_includes(&self) -> bool {
        !self.includes.is_empty()
    }

    pub fn push_include(
        &mut self,
        smap: &mut SourceMap,
        contents: Rc<FileContents>,
        include_pos: SourcePos,
    ) -> Result<(), SourcesTooLargeError> {
        let id = smap.create_file(Rc::clone(&contents), Some(include_pos))?;
        self.includes
            .push(File::new(contents, smap.get_source(id).range.start()));
        Ok(())
    }

    pub fn pop_include(&mut self) {
        self.includes.pop();
    }
}
