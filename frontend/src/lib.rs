#![warn(rust_2018_idioms)]

pub mod diag;
pub mod lex;
pub mod smap;

mod intern;
mod pp;
mod spos;

pub use diag::{Manager as DiagManager, Result as DResult};
pub use pp::Preprocessor;
pub use smap::SourceMap;
pub use spos::*;
