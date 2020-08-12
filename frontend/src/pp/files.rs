use std::rc::Rc;

use crate::smap::{FileContents, FileName, SourceId, SourcesTooLargeError};
use crate::{SourceMap, SourcePos};

use super::file::File;

pub enum IncludeKind {
    Str,
    Angle,
}

pub struct ActiveFiles {
    main: File,
    includes: Vec<File>,
}

impl ActiveFiles {
    pub fn new(smap: &SourceMap, main_id: SourceId) -> Self {
        let source = smap.get_source(main_id);
        let file = source
            .as_file()
            .expect("preprocessor requires a file source");

        ActiveFiles {
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
        filename: FileName,
        contents: Rc<FileContents>,
        include_pos: SourcePos,
    ) -> Result<(), SourcesTooLargeError> {
        let id = smap.create_file(filename, Rc::clone(&contents), Some(include_pos))?;
        self.includes
            .push(File::new(contents, smap.get_source(id).range.start()));
        Ok(())
    }

    pub fn pop_include(&mut self) {
        self.includes.pop();
    }
}
