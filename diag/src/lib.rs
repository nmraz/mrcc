use source_map::pos::{FragmentedSourceRange, SourceRange};

#[derive(Debug, Clone)]
pub struct LabeledRange<R>(R, String);

#[derive(Debug, Clone)]
pub struct Ranges<R> {
    pub primary_range: R,
    pub subranges: Vec<LabeledRange<R>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Note,
    Warning,
    Error,
}

#[derive(Debug, Clone)]
pub struct SubDiagnostic<R> {
    pub msg: String,
    pub ranges: Option<Ranges<R>>,
}

#[derive(Debug, Clone)]
pub struct Diagnostic<D> {
    pub level: Level,
    pub main: D,
    pub notes: Vec<D>,
}

pub type RawDiagnostic = Diagnostic<SubDiagnostic<FragmentedSourceRange>>;

#[derive(Debug, Clone)]
pub struct RenderedSubDiagnostic {
    pub inner: SubDiagnostic<SourceRange>,
    pub expansions: Vec<SubDiagnostic<SourceRange>>,
}

impl RenderedSubDiagnostic {
    pub fn msg(&self) -> &str {
        &self.inner.msg
    }

    pub fn ranges(&self) -> Option<&Ranges<SourceRange>> {
        self.inner.ranges.as_ref()
    }
}

pub type RenderedDiagnostic = Diagnostic<RenderedSubDiagnostic>;
