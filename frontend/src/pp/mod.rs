use crate::lex::{LexCtx, Lexer, Token};
use crate::smap::SourceId;
use crate::DResult;

use files::Files;

mod files;

pub struct Preprocessor {
    files: Files,
}

impl Preprocessor {
    pub fn new(ctx: &mut LexCtx<'_, '_>, main_id: SourceId) -> Self {
        Self {
            files: Files::new(&ctx.smap, main_id),
        }
    }
}

impl Lexer for Preprocessor {
    fn next(&mut self, ctx: &mut LexCtx<'_, '_>) -> DResult<Token> {
        self.files.top().with_tokenizer(|pos, tokenizer| loop {
            let raw = tokenizer.next_token();
            if let Some(tok) = Token::from_raw(&raw, pos, ctx)? {
                break Ok(tok);
            }
        })
    }
}
