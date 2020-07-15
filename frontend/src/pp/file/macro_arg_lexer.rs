use crate::lex::{LexCtx, Lexer, PunctKind, Token, TokenKind};
use crate::DResult;

use super::processor::{FileToken, Processor};

pub struct MacroArgLexer<'a, 's> {
    processor: &'a mut Processor<'s>,
}

impl<'a, 's> MacroArgLexer<'a, 's> {
    pub(super) fn new(processor: &'a mut Processor<'s>) -> Self {
        Self { processor }
    }
}

impl Lexer for MacroArgLexer<'_, '_> {
    fn next(&mut self, ctx: &mut LexCtx<'_, '_>) -> DResult<Token> {
        loop {
            if let FileToken::Tok {
                tok, line_start, ..
            } = self.processor.next_token(ctx)?
            {
                if line_start && tok.kind == TokenKind::Punct(PunctKind::Hash) {
                    ctx.reporter()
                        .error(
                            tok.range,
                            "preprocessing directives in macro arguments are undefined behavior",
                        )
                        .emit()?;
                    self.processor.advance_to_eod(ctx)?;
                    continue;
                }

                break Ok(tok);
            }
        }
    }
}
