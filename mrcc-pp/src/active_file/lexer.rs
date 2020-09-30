use mrcc_lex::LexCtx;
use mrcc_source::DResult;

use crate::expand::ReplacementLexer;
use crate::PpToken;

use super::Processor;

pub struct FileLexer<'a, 's> {
    processor: &'a mut Processor<'s>,
}

impl<'a, 's> FileLexer<'a, 's> {
    pub fn new(processor: &'a mut Processor<'s>) -> Self {
        Self { processor }
    }
}

impl ReplacementLexer for FileLexer<'_, '_> {
    fn next(&mut self, ctx: &mut LexCtx<'_, '_>) -> DResult<PpToken> {
        self.processor.next_real_token(ctx)
    }

    fn next_macro_arg(&mut self, ctx: &mut LexCtx<'_, '_>) -> DResult<PpToken> {
        loop {
            let ppt = self.next(ctx)?;

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

            break Ok(ppt);
        }
    }

    fn peek(&mut self, ctx: &mut LexCtx<'_, '_>) -> DResult<PpToken> {
        loop {
            if let Some(ppt) = self.processor.peek_token(ctx)?.real() {
                break Ok(ppt);
            }

            // Consume peeked token.
            self.processor.next_token(ctx)?;
        }
    }
}

pub struct DirectiveLexer<'a, 's> {
    processor: &'a mut Processor<'s>,
}

impl<'a, 's> DirectiveLexer<'a, 's> {
    pub fn new(processor: &'a mut Processor<'s>) -> Self {
        Self { processor }
    }
}

impl ReplacementLexer for DirectiveLexer<'_, '_> {
    fn next(&mut self, ctx: &mut LexCtx<'_, '_>) -> DResult<PpToken> {
        self.processor.next_directive_token(ctx)
    }

    fn next_macro_arg(&mut self, ctx: &mut LexCtx<'_, '_>) -> DResult<PpToken> {
        self.next(ctx)
    }

    fn peek(&mut self, ctx: &mut LexCtx<'_, '_>) -> DResult<PpToken> {
        self.processor
            .peek_token(ctx)
            .map(|tok| tok.as_directive_token())
    }
}
