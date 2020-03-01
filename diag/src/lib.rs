use std::cell::RefCell;

use source_map::pos::{FragmentedSourceRange, SourcePos, SourceRange};

mod render;

#[derive(Debug, Clone)]
pub struct Suggestion<R> {
    pub replacement_range: R,
    pub insert_text: String,
}

impl<R> Suggestion<R>
where
    SourcePos: Into<R>,
{
    pub fn new(replacement_range: R, insert_text: impl Into<String>) -> Self {
        Suggestion {
            replacement_range,
            insert_text: insert_text.into(),
        }
    }

    pub fn new_insertion(pos: SourcePos, text: impl Into<String>) -> Self {
        Self::new(pos.into(), text)
    }

    pub fn new_deletion(range: R) -> Self {
        Self::new(range, "")
    }
}

pub type RawSuggestion = Suggestion<FragmentedSourceRange>;
pub type RenderedSuggestion = Suggestion<SourceRange>;

#[derive(Debug, Clone)]
pub struct LabeledRange<R>(R, String);

#[derive(Debug, Clone)]
pub struct Ranges<R> {
    pub primary_range: R,
    pub subranges: Vec<LabeledRange<R>>,
}

impl<R> Ranges<R> {
    pub fn new(primary_range: R) -> Self {
        Self {
            primary_range,
            subranges: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SubDiagnostic<R> {
    pub msg: String,
    pub ranges: Option<Ranges<R>>,
    pub suggestions: Vec<Suggestion<R>>,
}

impl<R> SubDiagnostic<R> {
    pub fn new(msg: impl Into<String>, primary_range: R) -> Self {
        Self {
            msg: msg.into(),
            ranges: Some(Ranges::new(primary_range)),
            suggestions: Vec::new(),
        }
    }

    pub fn new_anon(msg: impl Into<String>) -> Self {
        Self {
            msg: msg.into(),
            ranges: None,
            suggestions: Vec::new(),
        }
    }

    pub fn add_labeled_range(&mut self, range: R, label: impl Into<String>) {
        self.ranges
            .as_mut()
            .expect("cannot attach range to rangeless diagnostic")
            .subranges
            .push(LabeledRange(range, label.into()));
    }

    pub fn add_range(&mut self, range: R) {
        self.add_labeled_range(range, "");
    }

    pub fn add_suggestion(&mut self, suggestion: Suggestion<R>) {
        self.suggestions.push(suggestion)
    }

    pub fn with_labeled_range(mut self, range: R, label: impl Into<String>) -> Self {
        self.add_labeled_range(range, label);
        self
    }

    pub fn with_range(mut self, range: R) -> Self {
        self.add_range(range);
        self
    }

    pub fn with_suggestion(mut self, suggestion: Suggestion<R>) -> Self {
        self.add_suggestion(suggestion);
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Note,
    Warning,
    Error,
}

#[derive(Debug, Clone)]
pub struct Diagnostic<D> {
    pub level: Level,
    pub main: D,
    pub notes: Vec<D>,
}

pub type RawSubDiagnostic = SubDiagnostic<FragmentedSourceRange>;
pub type RawDiagnostic = Diagnostic<RawSubDiagnostic>;

#[derive(Debug, Clone)]
pub struct RenderedSubDiagnostic {
    pub inner: SubDiagnostic<SourceRange>,
    pub expansions: Vec<Ranges<SourceRange>>,
}

impl RenderedSubDiagnostic {
    pub fn msg(&self) -> &str {
        &self.inner.msg
    }

    pub fn ranges(&self) -> Option<&Ranges<SourceRange>> {
        self.inner.ranges.as_ref()
    }

    pub fn suggestions(&self) -> &[RenderedSuggestion] {
        &self.inner.suggestions
    }
}

pub type RenderedDiagnostic = Diagnostic<RenderedSubDiagnostic>;

pub struct DiagnosticBuilder<'a> {
    diag: RawDiagnostic,
    manager: &'a Manager,
}

impl<'a> DiagnosticBuilder<'a> {
    fn new(
        manager: &'a Manager,
        level: Level,
        msg: String,
        primary_range: Option<FragmentedSourceRange>,
    ) -> Self {
        let main_diag = RawSubDiagnostic {
            msg,
            ranges: primary_range.map(|range| Ranges {
                primary_range: range,
                subranges: Vec::new(),
            }),
            suggestions: Vec::new(),
        };

        let diag = RawDiagnostic {
            level,
            main: main_diag,
            notes: Vec::new(),
        };

        DiagnosticBuilder { diag, manager }
    }

    pub fn add_labeled_range(
        &mut self,
        range: FragmentedSourceRange,
        label: impl Into<String>,
    ) -> &mut Self {
        self.diag.main.add_labeled_range(range, label);
        self
    }

    pub fn add_range(&mut self, range: FragmentedSourceRange) -> &mut Self {
        self.add_labeled_range(range, "")
    }

    pub fn add_suggestion(&mut self, suggestion: RawSuggestion) -> &mut Self {
        self.diag.main.add_suggestion(suggestion);
        self
    }

    pub fn add_note(&mut self, note: RawSubDiagnostic) -> &mut Self {
        self.diag.notes.push(note);
        self
    }
}

impl Drop for DiagnosticBuilder<'_> {
    fn drop(&mut self) {
        self.manager.emit(&self.diag);
    }
}

pub trait RawHandler {
    fn handle(&mut self, diag: &RawDiagnostic);
}

struct ManagerInner {
    handler: Box<dyn RawHandler>,
    warning_count: u32,
    error_count: u32,
}

pub struct Manager {
    inner: RefCell<ManagerInner>,
}

impl Manager {
    pub fn new(handler: Box<dyn RawHandler>) -> Self {
        Manager {
            inner: RefCell::new(ManagerInner {
                handler,
                warning_count: 0,
                error_count: 0,
            }),
        }
    }

    pub fn diag(
        &self,
        level: Level,
        msg: impl Into<String>,
        primary_range: FragmentedSourceRange,
    ) -> DiagnosticBuilder<'_> {
        DiagnosticBuilder::new(self, level, msg.into(), Some(primary_range))
    }

    pub fn diag_anon(&self, level: Level, msg: impl Into<String>) -> DiagnosticBuilder<'_> {
        DiagnosticBuilder::new(self, level, msg.into(), None)
    }

    pub fn warning(
        &self,
        msg: impl Into<String>,
        primary_range: FragmentedSourceRange,
    ) -> DiagnosticBuilder<'_> {
        self.diag(Level::Warning, msg, primary_range)
    }

    pub fn error(
        &self,
        msg: impl Into<String>,
        primary_range: FragmentedSourceRange,
    ) -> DiagnosticBuilder<'_> {
        self.diag(Level::Error, msg, primary_range)
    }

    pub fn warning_count(&self) -> u32 {
        self.inner.borrow().warning_count
    }

    pub fn error_count(&self) -> u32 {
        self.inner.borrow().error_count
    }

    fn emit(&self, diag: &RawDiagnostic) {
        let mut inner = self.inner.borrow_mut();

        match diag.level {
            Level::Warning => inner.warning_count += 1,
            Level::Error => inner.error_count += 1,
            _ => {}
        }

        inner.handler.handle(diag);
    }
}
