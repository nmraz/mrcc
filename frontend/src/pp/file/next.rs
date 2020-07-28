use std::path::PathBuf;

use crate::diag::{RawSuggestion, Reporter};
use crate::lex::{LexCtx, TokenKind};
use crate::DResult;
use crate::SourceRange;

use super::processor::{FileToken, Processor};
use super::{Action, IncludeKind, PpToken};
use crate::pp::State;

pub struct NextActionCtx<'a, 'b, 's, 'h> {
    ctx: &'a mut LexCtx<'b, 'h>,
    state: &'a mut State,
    processor: &'a mut Processor<'s>,
}

impl<'a, 'b, 's, 'h> NextActionCtx<'a, 'b, 's, 'h> {
    pub fn new(
        ctx: &'a mut LexCtx<'b, 'h>,
        state: &'a mut State,
        processor: &'a mut Processor<'s>,
    ) -> Self {
        Self {
            ctx,
            state,
            processor,
        }
    }

    pub fn next_action(&mut self) -> DResult<Action> {
        loop {
            let ppt = loop {
                if let FileToken::Tok(ppt) = self.next_token()? {
                    break ppt;
                }
            };

            if ppt.is_directive_start() {
                if let Some(action) = self.handle_directive()? {
                    break Ok(action);
                }
            } else {
                break Ok(Action::Tok(ppt));
            }
        }
    }

    fn handle_directive(&mut self) -> DResult<Option<Action>> {
        let tok = match self.next_token()? {
            FileToken::Tok(PpToken { tok, .. }) => tok,
            FileToken::Newline => return Ok(None),
        };

        let ident = match tok.kind {
            TokenKind::Ident(ident) => ident,
            _ => {
                self.invalid_directive(tok.range)?;
                return Ok(None);
            }
        };

        let known_idents = &self.state.known_idents;

        if ident == known_idents.dir_include {
            self.handle_include_directive()
        } else {
            self.invalid_directive(tok.range)?;
            Ok(None)
        }
    }

    fn invalid_directive(&mut self, range: SourceRange) -> DResult<()> {
        self.reporter()
            .error(range, "invalid preprocessing directive")
            .emit()?;
        self.advance_to_eod()
    }

    fn handle_include_directive(&mut self) -> DResult<Option<Action>> {
        let reader = self.processor.reader();
        reader.eat_line_ws();

        let (name, kind) = if reader.eat('<') {
            (self.consume_include_name('>')?, IncludeKind::Angle)
        } else if reader.eat('"') {
            (self.consume_include_name('"')?, IncludeKind::Str)
        } else {
            let pos = self.processor.pos();
            self.reporter().error(pos, "expected a file name").emit()?;
            self.advance_to_eod()?;
            return Ok(None);
        };

        Ok(Some(Action::Include(name, kind)))
    }

    fn consume_include_name(&mut self, term: char) -> DResult<PathBuf> {
        let reader = self.processor.reader();

        reader.begin_tok();
        reader.eat_while(|c| c != '\n' && c != term);
        let filename = reader.cur_content().cleaned_str().into_owned().into();

        if !reader.eat(term) {
            let pos = self.processor.pos();
            self.reporter().error_expected_delim(pos, term).emit()?;
        }

        self.finish_directive()?;
        Ok(filename)
    }

    fn finish_directive(&mut self) -> DResult<()> {
        let next = self.next_token()?;

        if next.is_eod() {
            return Ok(());
        }

        if let FileToken::Tok(ppt) = next {
            self.reporter()
                .warn(ppt.range(), "extra tokens after preprocessing directive")
                .add_suggestion(RawSuggestion::new(ppt.range().start(), "// "))
                .emit()?;
            self.advance_to_eod()?;
        }

        Ok(())
    }

    fn advance_to_eod(&mut self) -> DResult<()> {
        self.processor.advance_to_eod(self.ctx)
    }

    fn next_token(&mut self) -> DResult<FileToken> {
        self.processor.next_token(self.ctx)
    }

    fn reporter(&mut self) -> Reporter<'_, 'h> {
        self.ctx.reporter()
    }
}
