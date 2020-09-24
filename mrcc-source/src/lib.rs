#![warn(rust_2018_idioms)]

//! A library for managing source files, locations and diagnostics.

pub mod diag;
pub mod smap;

mod pos;

pub use diag::{Manager as DiagManager, Reporter as DiagReporter, Result as DResult};
pub use pos::*;
pub use smap::{SourceId, SourceMap};
