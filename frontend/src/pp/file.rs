use std::rc::Rc;

use crate::lex::raw::{RawToken, Reader, Tokenizer};
use crate::smap::{FileContents, SourceId, SourcesTooLargeError};
use crate::{SourceMap, SourcePos};

use super::state::FileState;

pub struct File {
    contents: Rc<FileContents>,
    state: FileState,
    start_pos: SourcePos,
    off: u32,
}

pub struct FileProcessor<'a> {
    pub state: &'a mut FileState,
    pub base_pos: SourcePos,
    tokenizer: Tokenizer<'a>,
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

    pub fn with_processor<R>(&mut self, f: impl FnOnce(&mut FileProcessor) -> R) -> R {
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

impl<'a> FileProcessor<'a> {
    pub fn next_token(&mut self) -> RawToken<'a> {
        self.tokenizer.next_token()
    }

    pub fn reader(&mut self) -> &mut Reader<'a> {
        &mut self.tokenizer.reader
    }
}

pub struct Files {
    main: File,
    includes: Vec<File>,
}

impl Files {
    pub fn new(smap: &SourceMap, main_id: SourceId) -> Self {
        let source = smap.get_source(main_id);
        let file = source
            .as_file()
            .expect("preprocessor requires a file source");

        Files {
            main: File::new(Rc::clone(&file.contents), source.range.start()),
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
