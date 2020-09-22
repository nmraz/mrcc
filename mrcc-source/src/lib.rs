#![warn(rust_2018_idioms)]

//! A library for managing source files, locations and diagnostics.
//!
//! The primary exports of this crate are [`SourceMap`](smap/struct.SourceMap.html), which can be
//! used to track source files and macro expansions, and [`DiagManager`](diag/struct.Manager.html),
//! for emitting detailed diagnostics with location information.

pub mod diag;
pub mod smap;

mod pos;

pub use diag::{Manager as DiagManager, Reporter as DiagReporter, Result as DResult};
pub use pos::*;
pub use smap::{SourceId, SourceMap};
