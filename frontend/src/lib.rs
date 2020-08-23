#![warn(rust_2018_idioms)]

pub mod diag;
pub mod lex;
pub mod pp;
pub mod smap;

mod intern;
mod spos;

pub use diag::{Manager as DiagManager, Result as DResult};
pub use smap::SourceMap;
pub use spos::*;
