use super::{PunctKind, Symbol};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    Unknown,
    Eof,

    Punct(PunctKind),

    Ident(Symbol),
    Number(Symbol),
    Str(Symbol),
    Char(Symbol),
}
