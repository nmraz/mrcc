//! Diagnostic reporting and emission.
//!
//! There are two kinds of diagnostics: [raw diagnostics](type.RawDiagnostic.html) and
//! [rendered diagnostics](struct.RenderedDiagnostic.html).
//!
//! Raw diagnosics are what users construct through [`Reporter`](struct.Reporter.html) and
//! [`DiagnosticBuilder`](struct.DiagnosticBuilder.html). These diagnostics contain
//! [fragmented source ranges](../struct.FragmentedSourceRange.html) for location information
//! and have no awareness of macro expansions or include stacks. This makes them convenient to use
//! when reporting diagnostics, as the user need not concern themselves with creating contiguous
//! ranges and handling macro expansions themselves.
//!
//! However, raw diagnostics can be problematic to use when displaying the diagnostics later. There,
//! the handler wants contiguous ranges to mark up in the source code, preferably with expansions
//! and include stacks mapped out.
//!
//! Rendered diagnostics are more amenable to display - they contain only contiguous ranges and
//! come with the appropriate expansion and include traces. Rendered diagnostics are passed to
//! handlers registered with [`Manager::new()`](struct.Manager.html#method.new). They can also be
//! created manually from raw diagnostics using [`render()`](fn.render.html).

use std::fmt;

use crate::SourceMap;
use crate::{FragmentedSourceRange, SourcePos, SourceRange};

pub use annotating_handler::AnnotatingHandler;
pub use render::render;

mod annotating_handler;
mod render;

/// Diagnostic severity level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Note,
    Warning,
    Error,
    Fatal,
}

impl Level {
    /// Returns a human-readable string describing this level.
    pub fn as_str(&self) -> &'static str {
        match self {
            Level::Note => "note",
            Level::Warning => "warning",
            Level::Error => "error",
            Level::Fatal => "fatal",
        }
    }
}

impl fmt::Display for Level {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Error type indicating that a fatal diagnostic has been emitted and compilation should be
/// aborted.
#[derive(Debug, Copy, Clone)]
pub struct FatalErrorEmitted;

pub type Result<T> = std::result::Result<T, FatalErrorEmitted>;

/// Generic suggestion type indicating that a range of code should be replaced with new code.
///
/// Insertions can be modeled by using an empty replacement range at the desired position.
/// See [`RawSuggestion`](type.RawSuggestion.html) and
/// [`RenderedSuggestion`](type.RenderedSuggestion.html) for concrete types.
#[derive(Debug, Clone)]
pub struct Suggestion<R> {
    /// The range within the source to replace.
    pub replacement_range: R,
    /// The new text to insert at `replacement_range`.
    pub insert_text: String,
}

impl<R> Suggestion<R> {
    /// Creates a new suggestion with the specified parameters.
    pub fn new(replacement_range: impl Into<R>, insert_text: impl Into<String>) -> Self {
        Suggestion {
            replacement_range: replacement_range.into(),
            insert_text: insert_text.into(),
        }
    }

    /// Creates a new suggestion indicating that `range` should be deleted.
    pub fn new_deletion(range: impl Into<R>) -> Self {
        Self::new(range, "")
    }
}

/// A suggestion for use in raw diagnsotics, containing a fragmented replacement range.
pub type RawSuggestion = Suggestion<FragmentedSourceRange>;
/// A suggestion for use in rendered diagnsotics, containing a contiguous replacement range.
pub type RenderedSuggestion = Suggestion<SourceRange>;

/// Generic structure representing the source ranges attached to a (sub)diagnostic.
///
/// Every subdiagnostic containing location information has a _primary range_, treated specially,
/// and may have zero or more (optionally labeled) _subranges_, indicating related areas near the
/// primary range. Note that cases where the ranges are expected to lie farther away (or potentially
/// in other files) are better represented as an additional note subdiagnostic.
///
/// See [`RawRanges`](type.RawRanges.html) and [`RenderedRanges`](type.RenderedRanges.html) for
/// concrete types.
#[derive(Debug, Clone)]
pub struct Ranges<R> {
    pub primary_range: R,
    pub subranges: Vec<(R, String)>,
}

impl<R> Ranges<R> {
    /// Creates a new object with the specified primary range and no subranges.
    pub fn new(primary_range: R) -> Self {
        Self {
            primary_range,
            subranges: Vec::new(),
        }
    }
}

/// Ranges for use in raw diagnsotics, containing fragmented ranges.
pub type RawRanges = Ranges<FragmentedSourceRange>;
/// Ranges for use in rendered diagnsotics, containing contiguous ranges.
pub type RenderedRanges = Ranges<SourceRange>;

/// Generic subdiagnostic structure.
///
/// Every diagnostic contains a main subdiagnostic and zero or more attached notes.
/// See [`RawSubDiagnostic`](type.RawSubDiagnostic.html) and
/// [`RenderedSubDiagnostic`](struct.RenderedSubDiagnostic.html) for concrete types.
#[derive(Debug, Clone)]
pub struct SubDiagnostic<R> {
    /// The message of this subdiagnostic.
    pub msg: String,
    /// The ranges attached to this subdiagnostic, if any.
    pub ranges: Option<Ranges<R>>,
    /// The suggestion attached to this subdiagnostic, if any.
    pub suggestion: Option<Suggestion<R>>,
}

impl<R> SubDiagnostic<R> {
    /// Creates a new subdiagnostic with the specified message and primary range.
    pub fn new(msg: impl Into<String>, primary_range: R) -> Self {
        Self {
            msg: msg.into(),
            ranges: Some(Ranges::new(primary_range)),
            suggestion: None,
        }
    }

    /// Creates a new subdiagnostic without any attached location information.
    pub fn new_anon(msg: impl Into<String>) -> Self {
        Self {
            msg: msg.into(),
            ranges: None,
            suggestion: None,
        }
    }

    /// Adds a new subrange with the specified label to the subdiagnostic.
    ///
    /// # Panics
    ///
    /// Panics if this subdiagnostic does not have any attached location information to add to.
    pub fn add_labeled_range(&mut self, range: R, label: impl Into<String>) {
        self.ranges
            .as_mut()
            .expect("cannot attach range to rangeless diagnostic")
            .subranges
            .push((range, label.into()));
    }

    /// Adds a new subrange to the subdiagnostic.
    ///
    /// # Panics
    ///
    /// Panics if this subdiagnostic does not have any attached location information to add to.
    pub fn add_range(&mut self, range: R) {
        self.add_labeled_range(range, "");
    }

    /// Updates the suggestion of this subdiagnostic.
    pub fn set_suggestion(&mut self, suggestion: Suggestion<R>) {
        self.suggestion = Some(suggestion);
    }

    /// Adds a new labeled subrange to this subdiagnostic, returning it for chaining.
    ///
    /// # Panics
    ///
    /// Panics if this subdiagnostic does not have any attached location information to add to.
    pub fn with_labeled_range(mut self, range: R, label: impl Into<String>) -> Self {
        self.add_labeled_range(range, label);
        self
    }

    /// Adds a new subrange to this subdiagnostic, returning it for chaining.
    ///
    /// # Panics
    ///
    /// Panics if this subdiagnostic does not have any attached location information to add to.
    pub fn with_range(mut self, range: R) -> Self {
        self.add_range(range);
        self
    }

    /// Updates the suggestion of this subdiagnostic, returning it for chaining.
    pub fn with_suggestion(mut self, suggestion: Suggestion<R>) -> Self {
        self.set_suggestion(suggestion);
        self
    }
}

/// Generic diagnostic structure.
///
/// This contains a main subdiagnostic and any number of note subdiagnostics.
/// See [`RawDiagnostic`](type.RawDiagnostic.html) and
/// [`RenderedDiagnostic`](struct.RenderedDiagnostic.html) for concrete types.
#[derive(Debug, Clone)]
pub struct Diagnostic<D> {
    /// The severity of this diagnostic.
    pub level: Level,
    /// The main subdiagnostic of this diagnostic.
    pub main: D,
    /// The notes attached to this diagnostic.
    pub notes: Vec<D>,
}

/// Raw subdiagnostic, with fragmented ranges and no expansion traces.
pub type RawSubDiagnostic = SubDiagnostic<FragmentedSourceRange>;
/// Raw diagnostic, with fragmented ranges and no expansion or include traces.
pub type RawDiagnostic = Diagnostic<RawSubDiagnostic>;

/// A rendered subdiagnostic, with contiguous ranges and an expansion trace.
#[derive(Debug, Clone)]
pub struct RenderedSubDiagnostic {
    /// The contained subdiagnostic information.
    pub inner: SubDiagnostic<SourceRange>,
    /// An expansion trace of this subdiagnostic's ranges, from outermost to innermost.
    pub expansions: Vec<RenderedRanges>,
}

impl RenderedSubDiagnostic {
    /// Returns this subdiagnostic's message.
    pub fn msg(&self) -> &str {
        &self.inner.msg
    }

    /// Returns this subdiagnostic's attached ranges, if any.
    pub fn ranges(&self) -> Option<&Ranges<SourceRange>> {
        self.inner.ranges.as_ref()
    }

    /// Returns this subdiagnostic's suggestion, if any.
    pub fn suggestion(&self) -> Option<&RenderedSuggestion> {
        self.inner.suggestion.as_ref()
    }
}

/// A rendered diagnostic, with expansion traces for every subdiagnostic and a top-level include
/// trace.
#[derive(Debug, Clone)]
pub struct RenderedDiagnostic {
    /// The contained diagnostic information.
    pub inner: Diagnostic<RenderedSubDiagnostic>,
    /// The include trace leading to this diagnostic's file, from outermost to innermost.
    pub includes: Vec<SourcePos>,
}

impl RenderedDiagnostic {
    /// Returns the severity of this diagnostic.
    pub fn level(&self) -> Level {
        self.inner.level
    }

    /// Returns the main subdiagnostic of this diagnostic.
    pub fn main(&self) -> &RenderedSubDiagnostic {
        &self.inner.main
    }

    /// Returns the notes attached to this diagnostic.
    pub fn notes(&self) -> &[RenderedSubDiagnostic] {
        &self.inner.notes
    }
}

/// A helper structure for constructing and emitting diagnostics.
///
/// This structure is returned by the various diagnostic reporting methods on
/// [`Manager`](struct.Manager.html) and [`Reporter`](struct.Reporter.html).
///
/// Once the diagnostic is built, be sure to call [`emit()`](#method.emit) to actually emit it.
#[must_use = "diagnostics should be emitted with `.emit()`"]
pub struct DiagnosticBuilder<'a, 'h> {
    diag: Box<RawDiagnostic>,
    smap: Option<&'a SourceMap>,
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
            suggestion: None,
        };

        let diag = Box::new(RawDiagnostic {
            level,
            main: main_diag,
            notes: Vec::new(),
        });

        DiagnosticBuilder {
            diag,
            smap: primary_range.map(|(_, smap)| smap),
            manager,
        }
    }

    /// Adds a labeled subrange to the diagnostic being built.
    ///
    /// # Panics
    ///
    /// Panics if the diagnostic has no location information attached.
    pub fn add_labeled_range(
        mut self,
        range: FragmentedSourceRange,
        label: impl Into<String>,
    ) -> Self {
        self.diag.main.add_labeled_range(range, label);
        self
    }

    /// Adds a range to the diagnostic being built.
    ///
    /// # Panics
    ///
    /// Panics if the diagnostic has no location information attached.
    pub fn add_range(self, range: FragmentedSourceRange) -> Self {
        self.add_labeled_range(range, "")
    }

    /// Sets the suggestion on the diagnostic being built.
    pub fn set_suggestion(mut self, suggestion: RawSuggestion) -> Self {
        self.diag.main.set_suggestion(suggestion);
        self
    }

    /// Adds a subdiagnostic to the diagnostic being built.
    pub fn add_note(mut self, note: RawSubDiagnostic) -> Self {
        self.diag.notes.push(note);
        self
    }

    /// Emits the built subdiagnostic back to the manager.
    ///
    /// If this diagnostic caused a fatal error to be emitted, either directly or indirectly (e.g.
    /// through the error limit), returns `Err(FatalErrorEmitted)`. Otherwise, returns `Ok(())`.
    pub fn emit(self) -> Result<()> {
        self.manager.emit(&self.diag, self.smap)
    }
}

/// Handler trait for receiving raw diagnostics.
pub trait RawHandler {
    /// Handles a raw diagnostic.
    ///
    /// If the diagnostic was reported with location information, `smap` will be provided as well.
    fn handle(&mut self, diag: &RawDiagnostic, smap: Option<&SourceMap>);
}

/// Handler trait for receiving rendered diagnostics.
pub trait RenderedHandler {
    /// Handles a rendered diagnostic.
    ///
    /// If the diagnostic was reported with location information, `smap` will be provided as well.
    fn handle(&mut self, diag: &RenderedDiagnostic, smap: Option<&SourceMap>);
}

/// Adaptor that bridges between rendered diagnostic handlers and raw diagnostic handlers.
struct RenderingHandlerAdaptor<H> {
    rendered_handler: H,
}

impl<H: RenderedHandler> RawHandler for RenderingHandlerAdaptor<H> {
    fn handle(&mut self, diag: &RawDiagnostic, smap: Option<&SourceMap>) {
        self.rendered_handler.handle(&render(diag, smap), smap);
    }
}

/// A top-level diagnostics engine.
///
/// This structure is responsible for forwarding diagnostics to a handler, enforcing error limits
/// and tracking statistics about emitted diagnostics.
pub struct Manager<'h> {
    handler: Box<dyn RawHandler + 'h>,
    error_limit: Option<u32>,
    warning_count: u32,
    error_count: u32,
}

impl<'h> Manager<'h> {
    /// Creates a new `Manager` with the specified handler and error limit.
    ///
    /// If `error_limit` is provided, the manager will emit a fatal diagnostic once the specified
    /// number of errors has been emitted.
    pub fn new(handler: impl RenderedHandler + 'h, error_limit: Option<u32>) -> Self {
        Self::with_raw_handler(
            Box::new(RenderingHandlerAdaptor {
                rendered_handler: handler,
            }),
            error_limit,
        )
    }

    /// Creates a new `Manager` with an [annotating handler](struct.AnnotatingHandler.html) and
    /// the specified error limit.
    pub fn new_annotating(error_limit: Option<u32>) -> Manager<'static> {
        Manager::new(AnnotatingHandler, error_limit)
    }

    /// Creates a new `Manager` with the specified raw diagnostic handler and error limit.
    pub fn with_raw_handler(handler: Box<dyn RawHandler + 'h>, error_limit: Option<u32>) -> Self {
        Manager {
            handler,
            error_limit,
            warning_count: 0,
            error_count: 0,
        }
    }

    /// Creates a new reporter for reporting diagnostics with location information.
    pub fn reporter<'a>(&'a mut self, smap: &'a SourceMap) -> Reporter<'a, 'h> {
        Reporter {
            manager: self,
            smap,
        }
    }

    /// Reports a diagnostic with no location information, returning a diagnostic builder.
    pub fn report_anon(&mut self, level: Level, msg: String) -> DiagnosticBuilder<'_, 'h> {
        DiagnosticBuilder::new(self, level, msg, None)
    }

    /// Returns the number of warnings emitted by this manager.
    pub fn warning_count(&self) -> u32 {
        self.warning_count
    }

    /// Returns the number of errors emitted by this manager.
    pub fn error_count(&self) -> u32 {
        self.error_count
    }

    /// Emits the specified diagnostic.
    ///
    /// Statistics are updated, and a fatal diagnostic is emitted if the error limit is reached.
    fn emit(&mut self, diag: &RawDiagnostic, smap: Option<&SourceMap>) -> Result<()> {
        self.handler.handle(diag, smap);

        match diag.level {
            Level::Warning => self.warning_count += 1,
            Level::Error => self.error_count += 1,
            Level::Fatal => return Err(FatalErrorEmitted),
            _ => {}
        }

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

/// Helper for reporting diagnostics with location information.
///
/// Use [`Manager::reporter()`](struct.Manager.html#method.reporter) to create a new reporter.
pub struct Reporter<'a, 'h> {
    manager: &'a mut Manager<'h>,
    smap: &'a SourceMap,
}

impl<'a, 'h> Reporter<'a, 'h> {
    /// Reports a diagnostic at the specified location, returning a diagnostic builder to allow the
    /// diagnostic to be finished and emitted.
    pub fn report(
        &mut self,
        level: Level,
        primary_range: impl Into<FragmentedSourceRange>,
        msg: impl Into<String>,
    ) -> DiagnosticBuilder<'_, 'h> {
        DiagnosticBuilder::new(
            self.manager,
            level,
            msg.into(),
            Some((primary_range.into(), self.smap)),
        )
    }

    /// Reports a warning at the specified location, returning a diagnostic builder.
    pub fn warn(
        &mut self,
        primary_range: impl Into<FragmentedSourceRange>,
        msg: impl Into<String>,
    ) -> DiagnosticBuilder<'_, 'h> {
        self.report(Level::Warning, primary_range, msg)
    }

    /// Reports an error at the specified location, returning a diagnostic builder.
    pub fn error(
        &mut self,
        primary_range: impl Into<FragmentedSourceRange>,
        msg: impl Into<String>,
    ) -> DiagnosticBuilder<'_, 'h> {
        self.report(Level::Error, primary_range, msg)
    }

    /// Reports a fatal error at the specified location, returning a diagnostic builder.
    pub fn fatal(
        &mut self,
        primary_range: impl Into<FragmentedSourceRange>,
        msg: impl Into<String>,
    ) -> DiagnosticBuilder<'_, 'h> {
        self.report(Level::Fatal, primary_range, msg)
    }

    /// Reports an error that `delim` was expected at `pos` along with a suggestion to insert it.
    ///
    /// A diagnostic builder is returned to allow additional information to be attached.
    pub fn error_expected_delim(
        &mut self,
        pos: SourcePos,
        delim: char,
    ) -> DiagnosticBuilder<'_, 'h> {
        self.error(pos, format!("expected a '{}'", delim))
            .set_suggestion(RawSuggestion::new(pos, delim.to_string()))
    }
}
