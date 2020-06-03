use crate::raw::{RawToken, Tokenizer};
use crate::{LexCtx, Lexer, Token, TokenKind};

pub struct Preprocessor {}

impl Lexer for Preprocessor {
    fn next(&mut self, ctx: &mut LexCtx<'_>) -> Token {
        todo!()
    }
}
