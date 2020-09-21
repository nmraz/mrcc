#![warn(rust_2018_idioms)]

pub mod diag;
pub mod smap;

mod pos;

pub use diag::{Manager as DiagManager, Reporter as DiagReporter, Result as DResult};
pub use pos::*;
pub use smap::{SourceId, SourceMap};
