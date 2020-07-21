use crate::lex::{LexCtx, Lexer, Token};
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
            if let FileToken::Tok(ppt) = self.processor.next_token(ctx)? {
                if ppt.is_directive_start() {
                    ctx.reporter()
                        .error(
                            ppt.range(),
                            "preprocessing directives in macro arguments are undefined behavior",
                        )
                        .emit()?;
                    self.processor.advance_to_eod(ctx)?;
                    continue;
                }

                break Ok(ppt.tok);
            }
        }
    }
}
