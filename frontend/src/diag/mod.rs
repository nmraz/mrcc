use crate::SourceMap;
use crate::{FragmentedSourceRange, SourcePos, SourceRange};

pub use render::render;

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
pub struct Ranges<R> {
    pub primary_range: R,
    pub subranges: Vec<(R, String)>,
}

impl<R> Ranges<R> {
    pub fn new(primary_range: R) -> Self {
        Self {
            primary_range,
            subranges: Vec::new(),
        }
    }
}

pub type RawRanges = Ranges<FragmentedSourceRange>;
pub type RenderedRanges = Ranges<SourceRange>;

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
            .push((range, label.into()));
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

#[derive(Clone)]
pub struct Diagnostic<'s, D> {
    pub level: Level,
    pub main: D,
    pub notes: Vec<D>,
    pub smap: Option<&'s SourceMap>,
}

pub type RawSubDiagnostic = SubDiagnostic<FragmentedSourceRange>;
pub type RawDiagnostic<'s> = Diagnostic<'s, RawSubDiagnostic>;

#[derive(Debug, Clone)]
pub struct RenderedSubDiagnostic {
    pub inner: SubDiagnostic<SourceRange>,
    pub expansions: Vec<RenderedRanges>,
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

pub type RenderedDiagnostic<'s> = Diagnostic<'s, RenderedSubDiagnostic>;

pub struct DiagnosticBuilder<'a> {
    diag: RawDiagnostic<'a>,
    manager: &'a mut Manager,
}

impl<'a> DiagnosticBuilder<'a> {
    fn new(
        manager: &'a mut Manager,
        level: Level,
        msg: String,
        primary_range: Option<(FragmentedSourceRange, &'a SourceMap)>,
    ) -> Self {
        let main_diag = RawSubDiagnostic {
            msg,
            ranges: primary_range.map(|(range, _)| Ranges::new(range)),
            suggestions: Vec::new(),
        };

        let diag = RawDiagnostic {
            level,
            main: main_diag,
            notes: Vec::new(),
            smap: primary_range.map(|(_, smap)| smap),
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
    fn handle(&mut self, diag: &RawDiagnostic<'_>);
}

pub trait RenderedHandler {
    fn handle(&mut self, diag: &RenderedDiagnostic<'_>);
}

struct RenderingHandlerAdaptor<H> {
    rendered_handler: H,
}

impl<H: RenderedHandler> RawHandler for RenderingHandlerAdaptor<H> {
    fn handle(&mut self, diag: &RawDiagnostic<'_>) {
        self.rendered_handler.handle(&render(diag));
    }
}

pub struct Manager {
    handler: Box<dyn RawHandler>,
    warning_count: u32,
    error_count: u32,
}

impl Manager {
    pub fn new(handler: Box<dyn RawHandler>) -> Self {
        Manager {
            handler,
            warning_count: 0,
            error_count: 0,
        }
    }

    pub fn with_rendered_handler(handler: impl RenderedHandler + 'static) -> Self {
        Self::new(Box::new(RenderingHandlerAdaptor {
            rendered_handler: handler,
        }))
    }

    pub fn report<'a>(
        &'a mut self,
        smap: &'a SourceMap,
        level: Level,
        primary_range: FragmentedSourceRange,
        msg: impl Into<String>,
    ) -> DiagnosticBuilder<'a> {
        DiagnosticBuilder::new(self, level, msg.into(), Some((primary_range, smap)))
    }

    pub fn report_anon(&mut self, level: Level, msg: impl Into<String>) -> DiagnosticBuilder<'_> {
        DiagnosticBuilder::new(self, level, msg.into(), None)
    }

    pub fn warning<'a>(
        &'a mut self,
        smap: &'a SourceMap,
        primary_range: FragmentedSourceRange,
        msg: impl Into<String>,
    ) -> DiagnosticBuilder<'a> {
        self.report(smap, Level::Warning, primary_range, msg)
    }

    pub fn error<'a>(
        &'a mut self,
        smap: &'a SourceMap,
        primary_range: FragmentedSourceRange,
        msg: impl Into<String>,
    ) -> DiagnosticBuilder<'a> {
        self.report(smap, Level::Error, primary_range, msg)
    }

    pub fn warning_count(&self) -> u32 {
        self.warning_count
    }

    pub fn error_count(&self) -> u32 {
        self.error_count
    }

    fn emit(&mut self, diag: &RawDiagnostic<'_>) {
        match diag.level {
            Level::Warning => self.warning_count += 1,
            Level::Error => self.error_count += 1,
            _ => {}
        }

        self.handler.handle(diag);
    }
}