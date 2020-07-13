use std::path::PathBuf;

use crate::diag::{RawSuggestion, Reporter};
use crate::lex::raw::{RawToken, RawTokenKind, Reader, Tokenizer};
use crate::lex::{LexCtx, PunctKind, Token, TokenKind};
use crate::DResult;
use crate::{SourcePos, SourceRange};

use super::{Action, FileState, IncludeKind, State};

enum FileToken {
    Tok { tok: Token, is_line_start: bool },
    Newline,
}

pub struct NextActionCtx<'a, 'b, 'h> {
    ctx: &'a mut LexCtx<'b, 'h>,
    state: &'a mut State,
    file_state: &'a mut FileState,
    base_pos: SourcePos,
    tokenizer: Tokenizer<'a>,
}

impl<'a, 'b, 'h> NextActionCtx<'a, 'b, 'h> {
    pub fn new(
        ctx: &'a mut LexCtx<'b, 'h>,
        state: &'a mut State,
        file_state: &'a mut FileState,
        base_pos: SourcePos,
        remaining_source: &'a str,
    ) -> Self {
        Self {
            ctx,
            state,
            file_state,
            base_pos,
            tokenizer: Tokenizer::new(remaining_source),
        }
    }

    pub fn off(&self) -> u32 {
        self.tokenizer.reader.off()
    }

    pub fn next_action(&mut self) -> DResult<Action> {
        loop {
            let (tok, is_line_start) = loop {
                if let FileToken::Tok { tok, is_line_start } = self.next_file_token()? {
                    break (tok, is_line_start);
                }
            };

            if is_line_start && tok.kind == TokenKind::Punct(PunctKind::Hash) {
                if let Some(action) = self.handle_directive()? {
                    break Ok(action);
                }
            } else {
                break Ok(Action::Tok(tok));
            }
        }
    }

    fn handle_directive(&mut self) -> DResult<Option<Action>> {
        let tok = match self.next_file_token()? {
            FileToken::Tok { tok, .. } => tok,
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
            self.handle_include()
        } else {
            self.invalid_directive(tok.range)?;
            Ok(None)
        }
    }

    fn invalid_directive(&mut self, range: SourceRange) -> DResult<()> {
        self.advance_line();
        self.reporter()
            .error(range, "invalid preprocessing directive")
            .emit()
    }

    fn handle_include(&mut self) -> DResult<Option<Action>> {
        let reader = self.reader();
        reader.eat_line_ws();

        let (name, kind) = if reader.eat('<') {
            (self.consume_include_name('>')?, IncludeKind::Angle)
        } else if reader.eat('"') {
            (self.consume_include_name('"')?, IncludeKind::Str)
        } else {
            let pos = self.pos();
            self.reporter().error(pos, "expected a file name").emit()?;
            self.advance_line();
            return Ok(None);
        };

        Ok(Some(Action::Include(name, kind)))
    }

    fn consume_include_name(&mut self, term: char) -> DResult<PathBuf> {
        let reader = self.reader();

        reader.begin_tok();
        reader.eat_while(|c| c != '\n' && c != term);
        let filename = reader.cur_content().cleaned_str().into_owned().into();

        if !reader.eat(term) {
            let pos = self.pos();
            self.reporter().error_expected_delim(pos, term).emit()?;
        }

        self.finish_directive()?;
        Ok(filename)
    }

    fn finish_directive(&mut self) -> DResult<()> {
        let next = self.next_file_token()?;

        if is_eod(&next) {
            return Ok(());
        }

        if let FileToken::Tok { tok, .. } = next {
            self.advance_line();
            self.reporter()
                .warn(tok.range, "extra tokens after preprocessing directive")
                .add_suggestion(RawSuggestion::new(tok.range.start(), "// "))
                .emit()?;
        }

        Ok(())
    }

    fn advance_line(&mut self) {
        self.reader().eat_to_after('\n');
    }

    fn next_file_token(&mut self) -> DResult<FileToken> {
        let is_line_start = self.file_state.is_line_start;
        let raw = self.next_token_skip_ws();
        Token::from_raw(&raw, self.base_pos, self.ctx).map(|res| {
            res.map(|tok| FileToken::Tok { tok, is_line_start })
                .unwrap_or(FileToken::Newline)
        })
    }

    fn next_token_skip_ws(&mut self) -> RawToken<'a> {
        loop {
            let tok = self.next_token();
            if tok.kind != RawTokenKind::Ws {
                break tok;
            }
        }
    }

    fn next_token(&mut self) -> RawToken<'a> {
        let tok = self.tokenizer.next_token();

        if tok.kind == RawTokenKind::Newline {
            self.file_state.is_line_start = true;
        } else if !is_trivia(tok.kind) {
            self.file_state.is_line_start = false;
        }

        tok
    }

    fn reporter(&mut self) -> Reporter<'_, 'h> {
        self.ctx.reporter()
    }

    fn reader(&mut self) -> &mut Reader<'a> {
        &mut self.tokenizer.reader
    }

    fn pos(&self) -> SourcePos {
        self.base_pos.offset(self.off())
    }
}

fn is_trivia(kind: RawTokenKind) -> bool {
    match kind {
        RawTokenKind::Ws | RawTokenKind::Comment(..) => true,
        _ => false,
    }
}

fn is_eod(file_tok: &FileToken) -> bool {
    match file_tok {
        FileToken::Newline => true,
        FileToken::Tok { tok, .. } => tok.kind == TokenKind::Eof,
    }
}
