use crate::lex;
use lex::raw::{RawToken, Tokenizer};
use lex::{LexCtx, Lexer, Token, TokenKind};

pub struct Preprocessor {}

impl Lexer for Preprocessor {
    fn next(&mut self, ctx: &mut LexCtx<'_>) -> Token {
        todo!()
    }
}
