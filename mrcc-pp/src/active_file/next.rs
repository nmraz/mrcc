use std::fmt::Write;
use std::path::PathBuf;

use mrcc_lex::{LexCtx, PunctKind, Symbol, TokenKind};
use mrcc_source::SourceRange;
use mrcc_source::{
    diag::{RawSubDiagnostic, RawSuggestion, Reporter},
    DResult,
};

use crate::expand::{MacroDef, MacroDefKind, ReplacementList};
use crate::state::State;

use super::lexer::FileLexer;
use super::processor::{FileToken, Processor};
use super::{Action, IncludeKind, PpToken};

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
            if let Some(ppt) = self.next_expanded_token()? {
                break Ok(Action::Tok(ppt));
            }

            let ppt = self.next_real_token()?;

            if ppt.is_directive_start() {
                if let Some(action) = self.handle_directive()? {
                    break Ok(action);
                }
            } else if !self.begin_expansion(ppt)? {
                break Ok(Action::Tok(ppt));
            }
        }
    }

    fn next_expanded_token(&mut self) -> DResult<Option<PpToken>> {
        self.state
            .macro_state
            .next_expanded_token(self.ctx, &mut FileLexer::new(self.processor))
    }

    fn begin_expansion(&mut self, ppt: PpToken) -> DResult<bool> {
        self.state
            .macro_state
            .begin_expansion(self.ctx, ppt, &mut FileLexer::new(self.processor))
    }

    fn handle_directive(&mut self) -> DResult<Option<Action>> {
        let ppt = self.next_directive_token()?;

        let ident = match ppt.data() {
            TokenKind::Ident(ident) => ident,
            TokenKind::Eof => return Ok(None), // Null directive
            _ => {
                self.invalid_directive(ppt)?;
                return Ok(None);
            }
        };
        self.processor.reader().eat_line_ws();

        let known_idents = &self.state.known_idents;

        if ident == known_idents.dir_define {
            self.handle_define_directive()?;
            Ok(None)
        } else if ident == known_idents.dir_undef {
            self.handle_undef_directive()?;
            Ok(None)
        } else if ident == known_idents.dir_include {
            self.handle_include_directive()
        } else if ident == known_idents.dir_error {
            self.handle_error_directive(ppt.range())?;
            Ok(None)
        } else {
            self.invalid_directive(ppt)?;
            Ok(None)
        }
    }

    fn invalid_directive(&mut self, ppt: PpToken) -> DResult<()> {
        self.report_and_advance(ppt, "invalid preprocessing directive")
    }

    fn handle_define_directive(&mut self) -> DResult<()> {
        let name_tok = match self.expect_directive_macro_name()? {
            Some(name) => name,
            None => return Ok(()),
        };

        let def = match self.consume_macro_def(name_tok)? {
            Some(def) => def,
            _ => return Ok(()),
        };

        if let Some(prev) = self.state.macro_state.define(def) {
            let prev_range = prev.name_tok.range();
            let msg = format!(
                "redefinition of macro '{}'",
                &self.ctx.interner[name_tok.data()]
            );

            self.reporter()
                .error(name_tok.range(), msg)
                .add_note(RawSubDiagnostic::new(
                    "previous definition here",
                    prev_range.into(),
                ))
                .emit()?;
        }

        Ok(())
    }

    fn consume_macro_def(&mut self, name_tok: PpToken<Symbol>) -> DResult<Option<MacroDef>> {
        let mut tokens = Vec::new();

        if let Some(ppt) = self.next_token()?.non_eod() {
            if !ppt.leading_trivia {
                if ppt.data() == TokenKind::Punct(PunctKind::LParen) {
                    self.reporter()
                        .error(ppt.range(), "function-like macros are not yet implemented")
                        .emit()?;
                    self.advance_to_eod()?;
                    return Ok(None);
                } else {
                    self.reporter()
                        .warn(
                            ppt.range(),
                            "object-like macros require whitespace after the macro name",
                        )
                        .set_suggestion(RawSuggestion::new(ppt.range().start(), " "))
                        .emit()?;
                }
            }

            tokens.push(ppt)
        }

        Ok(Some(MacroDef {
            name_tok,
            kind: MacroDefKind::Object(self.consume_macro_body(tokens)?),
        }))
    }

    fn consume_macro_body(&mut self, mut tokens: Vec<PpToken>) -> DResult<ReplacementList> {
        while let Some(ppt) = self.next_token()?.non_eod() {
            tokens.push(ppt);
        }

        Ok(ReplacementList::new(tokens))
    }

    fn handle_undef_directive(&mut self) -> DResult<()> {
        let name = match self.expect_directive_macro_name()? {
            Some(tok) => tok,
            None => return Ok(()),
        }
        .data();

        self.state.macro_state.undef(name);
        self.finish_directive()
    }

    fn expect_directive_macro_name(&mut self) -> DResult<Option<PpToken<Symbol>>> {
        self.expect_directive_token(
            |kind| match kind {
                TokenKind::Ident(name) => Some(name),
                _ => None,
            },
            "expected a macro name",
        )
    }

    fn handle_include_directive(&mut self) -> DResult<Option<Action>> {
        let start = self.processor.pos();
        let reader = self.processor.reader();

        let (filename, kind) = if reader.eat('<') {
            (self.consume_include_name('>')?, IncludeKind::Angle)
        } else if reader.eat('"') {
            (self.consume_include_name('"')?, IncludeKind::Str)
        } else {
            let pos = self.processor.pos();
            self.reporter().error(pos, "expected a file name").emit()?;
            self.advance_to_eod()?;
            return Ok(None);
        };

        let len = self.processor.pos().offset_from(start);

        Ok(Some(Action::Include {
            filename,
            kind,
            range: SourceRange::new(start, len),
        }))
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

    fn handle_error_directive(&mut self, id_range: SourceRange) -> DResult<()> {
        let mut msg = String::new();
        while let Some(ppt) = self.next_token()?.non_eod() {
            write!(msg, "{}", ppt.display(self.ctx)).unwrap();
        }

        self.ctx.reporter().error(id_range, msg).emit()
    }

    fn finish_directive(&mut self) -> DResult<()> {
        if let Some(ppt) = self.next_token()?.non_eod() {
            self.reporter()
                .warn(ppt.range(), "extra tokens after preprocessing directive")
                .set_suggestion(RawSuggestion::new(ppt.range().start(), "// "))
                .emit()?;
            self.advance_to_eod()?;
        }

        Ok(())
    }

    fn expect_directive_token<T>(
        &mut self,
        f: impl FnOnce(TokenKind) -> Option<T>,
        msg: &str,
    ) -> DResult<Option<PpToken<T>>> {
        let ppt = self.next_directive_token()?;

        match ppt.maybe_map(f) {
            Some(tok) => Ok(Some(tok)),
            None => {
                self.report_and_advance(ppt, msg)?;
                Ok(None)
            }
        }
    }

    fn report_and_advance(&mut self, ppt: PpToken, msg: &str) -> DResult<()> {
        self.reporter().error(ppt.range(), msg).emit()?;
        self.advance_if_non_eof(ppt.data())
    }

    fn advance_to_eod(&mut self) -> DResult<()> {
        self.processor.advance_to_eod(self.ctx)
    }

    fn advance_if_non_eof(&mut self, kind: TokenKind) -> DResult<()> {
        if kind != TokenKind::Eof {
            self.advance_to_eod()?;
        }

        Ok(())
    }

    fn next_token(&mut self) -> DResult<FileToken> {
        self.processor.next_token(self.ctx)
    }

    fn next_real_token(&mut self) -> DResult<PpToken> {
        self.processor.next_real_token(self.ctx)
    }

    fn next_directive_token(&mut self) -> DResult<PpToken> {
        self.processor.next_directive_token(self.ctx)
    }

    fn reporter(&mut self) -> Reporter<'_, 'h> {
        self.ctx.reporter()
    }
}
