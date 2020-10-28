use std::fmt::Write;
use std::path::PathBuf;

use mrcc_lex::{LexCtx, PunctKind, Symbol, Token, TokenKind};
use mrcc_source::SourceRange;
use mrcc_source::{
    diag::{RawSubDiagnostic, RawSuggestion, Reporter},
    DResult,
};

use crate::expand::{MacroDef, MacroDefKind, MacroState, ReplacementList};

use super::lexer::{DirectiveLexer, MacroArgLexer};
use super::processor::{FileToken, Processor};
use super::{Event, IncludeKind, PpToken};

pub struct NextEventCtx<'a, 'b, 's, 'h> {
    ctx: &'a mut LexCtx<'b, 'h>,
    macro_state: &'a mut MacroState,
    processor: &'a mut Processor<'s>,
}

impl<'a, 'b, 's, 'h> NextEventCtx<'a, 'b, 's, 'h> {
    pub fn new(
        ctx: &'a mut LexCtx<'b, 'h>,
        macro_state: &'a mut MacroState,
        processor: &'a mut Processor<'s>,
    ) -> Self {
        Self {
            ctx,
            macro_state,
            processor,
        }
    }

    pub fn next_event(&mut self) -> DResult<Event> {
        loop {
            if let Some(ppt) = self.next_expansion_token()? {
                break Ok(Event::Tok(ppt));
            }

            let ppt = self.next_real_token()?;

            if ppt.is_directive_start() {
                if let Some(event) = self.handle_directive()? {
                    break Ok(event);
                }
            } else if !self.begin_expansion(ppt)? {
                break Ok(Event::Tok(ppt));
            }
        }
    }

    fn next_expansion_token(&mut self) -> DResult<Option<PpToken>> {
        self.macro_state
            .next_expansion_token(self.ctx, &mut MacroArgLexer::new(self.processor))
    }

    fn begin_expansion(&mut self, ppt: PpToken) -> DResult<bool> {
        self.macro_state
            .begin_expansion(self.ctx, ppt, &mut MacroArgLexer::new(self.processor))
    }

    fn handle_directive(&mut self) -> DResult<Option<Event>> {
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

        match &self.ctx.interner[ident] {
            "define" => {
                self.handle_define_directive()?;
                Ok(None)
            }
            "undef" => {
                self.handle_undef_directive()?;
                Ok(None)
            }
            "include" => self.handle_include_directive(),
            "error" => {
                self.handle_error_directive(ppt.range())?;
                Ok(None)
            }
            _ => {
                self.invalid_directive(ppt)?;
                Ok(None)
            }
        }
    }

    fn invalid_directive(&mut self, ppt: PpToken) -> DResult<()> {
        self.report_and_advance(ppt, "invalid preprocessing directive")
    }

    fn handle_define_directive(&mut self) -> DResult<()> {
        let name_tok = match self.expect_macro_name()? {
            Some(name) => name,
            None => return Ok(()),
        };

        let def = match self.consume_macro_def(name_tok)? {
            Some(def) => def,
            _ => return Ok(()),
        };

        if let Some(prev) = self.macro_state.define(def) {
            let prev_range = prev.name_tok.range;
            let msg = format!(
                "redefinition of macro '{}'",
                &self.ctx.interner[name_tok.data]
            );

            self.reporter()
                .error(name_tok.range, msg)
                .add_note(RawSubDiagnostic::new(
                    "previous definition here",
                    prev_range.into(),
                ))
                .emit()?;
        }

        Ok(())
    }

    fn consume_macro_def(&mut self, name_tok: Token<Symbol>) -> DResult<Option<MacroDef>> {
        let mut tokens = Vec::new();

        if let Some(ppt) = self.next_token()?.non_eod() {
            if !ppt.leading_trivia {
                if ppt.data() == TokenKind::Punct(PunctKind::LParen) {
                    let params = match self.consume_macro_params()? {
                        Some(params) => params,
                        None => return Ok(None),
                    };

                    return Ok(Some(MacroDef {
                        name_tok,
                        kind: MacroDefKind::Function {
                            params,
                            replacement: self.consume_macro_body(tokens)?,
                        },
                    }));
                }

                self.reporter()
                    .warn(
                        ppt.range(),
                        "object-like macros require whitespace after the macro name",
                    )
                    .set_suggestion(RawSuggestion::new(ppt.range().start(), " "))
                    .emit()?;
            }

            tokens.push(ppt)
        }

        Ok(Some(MacroDef {
            name_tok,
            kind: MacroDefKind::Object(self.consume_macro_body(tokens)?),
        }))
    }

    fn consume_macro_params(&mut self) -> DResult<Option<Vec<Symbol>>> {
        let mut params = Vec::new();

        let ppt = self.next_directive_token()?;
        match ppt.data() {
            TokenKind::Punct(PunctKind::RParen) => return Ok(Some(params)),
            TokenKind::Ident(param) => params.push(param),
            _ => {
                self.report_and_advance(ppt, "expected a parameter name or ')'")?;
                return Ok(None);
            }
        }

        loop {
            let ppt = self.next_directive_token()?;
            match ppt.data() {
                TokenKind::Punct(PunctKind::Comma) => {}
                TokenKind::Punct(PunctKind::RParen) => break Ok(Some(params)),
                _ => {
                    self.report_and_advance(ppt, "expected a ')'")?;
                    break Ok(None);
                }
            }

            let ppt = self.next_directive_token()?;
            match ppt.data() {
                TokenKind::Ident(param) => {
                    if params.contains(&param) {
                        let msg =
                            format!("duplicate macro parameter '{}'", &self.ctx.interner[param]);
                        self.report_and_advance(ppt, &msg)?;
                        return Ok(None);
                    }

                    params.push(param);
                }
                _ => {
                    self.report_and_advance(ppt, "expected a parameter name")?;
                    break Ok(None);
                }
            }
        }
    }

    fn consume_macro_body(&mut self, mut tokens: Vec<PpToken>) -> DResult<ReplacementList> {
        while let Some(ppt) = self.next_token()?.non_eod() {
            tokens.push(ppt);
        }

        Ok(ReplacementList::new(tokens))
    }

    fn handle_undef_directive(&mut self) -> DResult<()> {
        let name = match self.expect_macro_name()? {
            Some(tok) => tok,
            None => return Ok(()),
        }
        .data;

        self.macro_state.undef(name);
        self.finish_directive()
    }

    fn expect_macro_name(&mut self) -> DResult<Option<Token<Symbol>>> {
        let ppt = self.next_directive_token()?;

        match ppt.data() {
            TokenKind::Ident(name) => Ok(Some(ppt.tok.map(|_| name))),
            _ => {
                self.report_and_advance(ppt, "expected a macro name")?;
                Ok(None)
            }
        }
    }

    fn handle_include_directive(&mut self) -> DResult<Option<Event>> {
        let start = self.processor.pos();
        let reader = self.processor.reader();

        let (filename, kind) = if reader.eat('<') {
            (self.consume_include_name('>')?, IncludeKind::Angled)
        } else if reader.eat('"') {
            (self.consume_include_name('"')?, IncludeKind::Quoted)
        } else {
            match self.consume_token_include_name()? {
                Some(filename_kind) => filename_kind,
                None => return Ok(None),
            }
        };

        let len = self.processor.pos().offset_from(start);

        Ok(Some(Event::Include {
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

    fn consume_token_include_name(&mut self) -> DResult<Option<(PathBuf, IncludeKind)>> {
        let Token {
            range,
            data: content,
        } = self.consume_expanded_directive_string()?;

        let (kind, term) = if content.starts_with('"') {
            (IncludeKind::Quoted, '"')
        } else if content.starts_with('<') {
            (IncludeKind::Angled, '>')
        } else {
            self.reporter()
                .error(range, r#"expected "filename" or <filename>"#)
                .emit()?;
            return Ok(None);
        };

        let content = &content[1..];

        let name = match content.find(term) {
            Some(end) => &content[..end],
            None => {
                self.reporter()
                    .error_expected_delim(range.end(), term)
                    .emit()?;
                content
            }
        };

        Ok(Some((name.into(), kind)))
    }

    fn consume_expanded_directive_string(&mut self) -> DResult<Token<String>> {
        let mut contents = String::new();

        let start_pos = self.processor.pos();
        let end_pos = loop {
            let ppt = self.next_expanded_directive_token()?;
            if ppt.data() == TokenKind::Eof {
                break ppt.range().start();
            }

            write!(contents, "{}", ppt.display(self.ctx)).unwrap();
        };

        Ok(Token::new(
            contents,
            SourceRange::new(start_pos, end_pos.offset_from(start_pos)),
        ))
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

    fn next_expanded_directive_token(&mut self) -> DResult<PpToken> {
        loop {
            if let Some(ppt) = self
                .macro_state
                .next_expansion_token(self.ctx, &mut DirectiveLexer::new(self.processor))?
            {
                break Ok(ppt);
            }

            let ppt = self.next_directive_token()?;

            if !self.macro_state.begin_expansion(
                self.ctx,
                ppt,
                &mut DirectiveLexer::new(self.processor),
            )? {
                break Ok(ppt);
            }
        }
    }

    fn report_and_advance(&mut self, ppt: PpToken, msg: impl Into<String>) -> DResult<()> {
        self.processor.report_and_advance(self.ctx, ppt, msg.into())
    }

    fn advance_to_eod(&mut self) -> DResult<()> {
        self.processor.advance_to_eod(self.ctx)
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
