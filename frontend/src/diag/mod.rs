use crate::SourceMap;
use crate::{FragmentedSourceRange, SourcePos, SourceRange};

pub use render::render;

mod render;

#[derive(Debug, Clone)]
pub struct Suggestion<R> {
    pub replacement_range: R,
    pub insert_text: String,
}

impl<R> Suggestion<R> {
    pub fn new(replacement_range: impl Into<R>, insert_text: impl Into<String>) -> Self {
        Suggestion {
            replacement_range: replacement_range.into(),
            insert_text: insert_text.into(),
        }
    }

    pub fn new_deletion(range: impl Into<R>) -> Self {
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
    Fatal,
}

pub struct FatalErrorEmitted;

pub type Result<T> = std::result::Result<T, FatalErrorEmitted>;

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

#[must_use = "diagnostics should be emitted with `.emit()`"]
pub struct DiagnosticBuilder<'a, 'h> {
    diag: Box<RawDiagnostic<'a>>,
    manager: &'a mut Manager<'h>,
}

impl<'a, 'h> DiagnosticBuilder<'a, 'h> {
    fn new(
        manager: &'a mut Manager<'h>,
        level: Level,
        msg: String,
        primary_range: Option<(FragmentedSourceRange, &'a SourceMap)>,
    ) -> Self {
        let main_diag = RawSubDiagnostic {
            msg,
            ranges: primary_range.map(|(range, _)| Ranges::new(range)),
            suggestions: Vec::new(),
        };

        let diag = Box::new(RawDiagnostic {
            level,
            main: main_diag,
            notes: Vec::new(),
            smap: primary_range.map(|(_, smap)| smap),
        });

        DiagnosticBuilder { diag, manager }
    }

    pub fn add_labeled_range(
        mut self,
        range: FragmentedSourceRange,
        label: impl Into<String>,
    ) -> Self {
        self.diag.main.add_labeled_range(range, label);
        self
    }

    pub fn add_range(self, range: FragmentedSourceRange) -> Self {
        self.add_labeled_range(range, "")
    }

    pub fn add_suggestion(mut self, suggestion: RawSuggestion) -> Self {
        self.diag.main.add_suggestion(suggestion);
        self
    }

    pub fn add_note(mut self, note: RawSubDiagnostic) -> Self {
        self.diag.notes.push(note);
        self
    }

    pub fn emit(self) -> Result<()> {
        self.manager.emit(&self.diag)
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

pub struct Manager<'h> {
    handler: Box<dyn RawHandler + 'h>,
    error_limit: Option<u32>,
    warning_count: u32,
    error_count: u32,
}

impl<'h> Manager<'h> {
    pub fn new(handler: Box<dyn RawHandler + 'h>, error_limit: Option<u32>) -> Self {
        Manager {
            handler,
            error_limit,
            warning_count: 0,
            error_count: 0,
        }
    }

    pub fn with_rendered_handler(
        handler: impl RenderedHandler + 'h,
        error_limit: Option<u32>,
    ) -> Self {
        Self::new(
            Box::new(RenderingHandlerAdaptor {
                rendered_handler: handler,
            }),
            error_limit,
        )
    }

    pub fn report<'a>(
        &'a mut self,
        smap: &'a SourceMap,
        level: Level,
        primary_range: FragmentedSourceRange,
        msg: String,
    ) -> DiagnosticBuilder<'a, 'h> {
        DiagnosticBuilder::new(self, level, msg, Some((primary_range, smap)))
    }

    pub fn report_anon(&mut self, level: Level, msg: String) -> DiagnosticBuilder<'_, 'h> {
        DiagnosticBuilder::new(self, level, msg, None)
    }

    pub fn warning_count(&self) -> u32 {
        self.warning_count
    }

    pub fn error_count(&self) -> u32 {
        self.error_count
    }

    fn emit(&mut self, diag: &RawDiagnostic<'_>) -> Result<()> {
        match diag.level {
            Level::Warning => self.warning_count += 1,
            Level::Error => self.error_count += 1,
            Level::Fatal => return Err(FatalErrorEmitted),
            _ => {}
        }

        self.handler.handle(diag);

        if let Some(limit) = self.error_limit {
            if self.error_count >= limit {
                return self
                    .report_anon(Level::Fatal, "too many errors emitted".to_owned())
                    .emit();
            }
        }

        Ok(())
    }
}

pub struct Reporter<'a, 'h> {
    manager: &'a mut Manager<'h>,
    smap: &'a SourceMap,
}

impl<'a, 'h> Reporter<'a, 'h> {
    pub fn new(manager: &'a mut Manager<'h>, smap: &'a SourceMap) -> Self {
        Self { manager, smap }
    }

    pub fn report(
        &mut self,
        level: Level,
        primary_range: impl Into<FragmentedSourceRange>,
        msg: impl Into<String>,
    ) -> DiagnosticBuilder<'_, 'h> {
        self.manager
            .report(self.smap, level, primary_range.into(), msg.into())
    }

    pub fn warn(
        &mut self,
        primary_range: impl Into<FragmentedSourceRange>,
        msg: impl Into<String>,
    ) -> DiagnosticBuilder<'_, 'h> {
        self.report(Level::Warning, primary_range, msg)
    }

    pub fn error(
        &mut self,
        primary_range: impl Into<FragmentedSourceRange>,
        msg: impl Into<String>,
    ) -> DiagnosticBuilder<'_, 'h> {
        self.report(Level::Error, primary_range, msg)
    }

    pub fn error_expected_delim(
        &mut self,
        pos: SourcePos,
        delim: char,
    ) -> DiagnosticBuilder<'_, 'h> {
        self.error(pos, format!("expected a '{}'", delim))
            .add_suggestion(RawSuggestion::new(pos, delim.to_string()))
    }
}
