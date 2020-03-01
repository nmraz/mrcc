use source_map::pos::{FragmentedSourceRange, SourcePos, SourceRange};

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
