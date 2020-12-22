use std::path::PathBuf;
use std::rc::Rc;

use mrcc_lex::LexCtx;
use mrcc_source::smap::{FileName, SourcesTooLargeError};
use mrcc_source::{DResult, SourceId, SourceMap, SourcePos, SourceRange};

use crate::expand::MacroState;
use crate::file::{File, IncludeKind};
use crate::PpToken;

use next::NextEventCtx;
use processor::{Processor, ProcessorState};

mod lexer;
mod next;
mod processor;

/// A point of interest that can be encountered when preprocessing a source file.
///
/// Generally, most directives can be handled internally while processing the file and need not be
/// reported through an event. Includes, however, are special, as they need to modify the list of
/// active files itself. This cannot happen while the file is being processed, so it must be
/// propagated to the caller.
pub enum Event {
    /// Preprocessing has produced another output token.
    Tok(PpToken),
    /// An include directive has been encountered and should be handled.
    Include {
        filename: PathBuf,
        kind: IncludeKind,
        range: SourceRange,
    },
}

/// A file that is currently being processed by the preprocessor.
///
/// In addition to the file itself, this tracks the current offset and conditional state.
pub struct ActiveFile {
    file: Rc<File>,
    start_pos: SourcePos,
    processor_state: ProcessorState,
}

impl ActiveFile {
    /// Creates a new active file with the specified content and base position.
    fn new(file: Rc<File>, start_pos: SourcePos) -> ActiveFile {
        ActiveFile {
            file,
            start_pos,
            processor_state: ProcessorState::new(),
        }
    }

    /// Returns the underlying file.
    pub fn file(&self) -> &Rc<File> {
        &self.file
    }

    /// Resumes processing of the file and returns the next interesting event
    pub fn next_event(
        &mut self,
        ctx: &mut LexCtx<'_, '_>,
        macro_state: &mut MacroState,
    ) -> DResult<Event> {
        NextEventCtx::new(ctx, macro_state, self.processor()).next_event()
    }

    /// Returns a processor for reading tokens and text from the file.
    fn processor(&mut self) -> Processor<'_> {
        Processor::new(
            &mut self.processor_state,
            &self.file.contents.src,
            self.start_pos,
        )
    }
}

/// A stack of files currently being processed.
///
/// The bottom of this stack is always the main source file, and any includes are pushed on top of
/// it.
pub struct ActiveFiles {
    main: ActiveFile,
    includes: Vec<ActiveFile>,
}

impl ActiveFiles {
    /// Creates a new active file stack with the specified main source file.
    ///
    /// # Panics
    ///
    /// Panics if `main_id` does not point to a source file.
    pub fn new(smap: &SourceMap, main_id: SourceId, parent_dir: Option<PathBuf>) -> Self {
        let source = smap.get_source(main_id);
        let file = source
            .as_file()
            .expect("preprocessor requires a file source");

        ActiveFiles {
            main: ActiveFile::new(
                File::new(Rc::clone(&file.contents), parent_dir),
                source.range.start(),
            ),
            includes: vec![],
        }
    }

    /// Returns the topmost file on the stack.
    pub fn top(&mut self) -> &mut ActiveFile {
        self.includes.last_mut().unwrap_or(&mut self.main)
    }

    /// Checks whether there are any includes on the stack beyond the main source file.
    pub fn has_includes(&self) -> bool {
        self.includes.len() > 0
    }

    /// Pushes a new file onto the include stack, creating an entry for it in the source map.
    pub fn push_include(
        &mut self,
        smap: &mut SourceMap,
        filename: PathBuf,
        file: Rc<File>,
        include_pos: SourcePos,
    ) -> Result<(), SourcesTooLargeError> {
        let id = smap.create_file(
            FileName::real(filename),
            Rc::clone(&file.contents),
            Some(include_pos),
        )?;
        self.includes
            .push(ActiveFile::new(file, smap.get_source(id).range.start()));
        Ok(())
    }

    /// Pops the topmost include on the stack.
    ///
    /// This has no effect if there are no includes; the main file will not be popped.
    pub fn pop_include(&mut self) {
        self.includes.pop();
    }
}
