use std::mem;
use std::path::PathBuf;

use crate::diag::{RawSuggestion, Reporter};
use crate::lex::raw::{Reader, Tokenizer};
use crate::lex::{FromRawResult, LexCtx, PunctKind, Token, TokenKind};
use crate::DResult;
use crate::{SourcePos, SourceRange};

use super::{Action, FileState, IncludeKind, State};

enum FileToken {
    Tok {
        tok: Token,
        line_start: bool,
        leading_trivia: bool,
    },
    Newline,
}

impl FileToken {
    pub fn is_eod(&self) -> bool {
        match self {
            FileToken::Newline => true,
            FileToken::Tok { tok, .. } => tok.kind == TokenKind::Eof,
        }
    }
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
            let (tok, line_start) = loop {
                if let FileToken::Tok {
                    tok, line_start, ..
                } = self.next_token()?
                {
                    break (tok, line_start);
                }
            };

            if line_start && tok.kind == TokenKind::Punct(PunctKind::Hash) {
                if let Some(action) = self.handle_directive()? {
                    break Ok(action);
                }
            } else {
                break Ok(Action::Tok(tok));
            }
        }
    }

    fn handle_directive(&mut self) -> DResult<Option<Action>> {
        let tok = match self.next_token()? {
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
        self.reporter()
            .error(range, "invalid preprocessing directive")
            .emit()?;
        self.advance_to_eod()
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
            self.advance_to_eod()?;
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
        let next = self.next_token()?;

        if next.is_eod() {
            return Ok(());
        }

        if let FileToken::Tok { tok, .. } = next {
            self.reporter()
                .warn(tok.range, "extra tokens after preprocessing directive")
                .add_suggestion(RawSuggestion::new(tok.range.start(), "// "))
                .emit()?;
            self.advance_to_eod()?;
        }

        Ok(())
    }

    fn advance_to_eod(&mut self) -> DResult<()> {
        while !self.next_token()?.is_eod() {}
        Ok(())
    }

    fn next_token(&mut self) -> DResult<FileToken> {
        let mut leading_trivia = false;

        let ret = loop {
            match Token::from_raw(&self.tokenizer.next_token(), self.base_pos, self.ctx)? {
                FromRawResult::Tok(tok) => {
                    break FileToken::Tok {
                        tok,
                        line_start: mem::replace(&mut self.file_state.line_start, false),
                        leading_trivia,
                    };
                }

                FromRawResult::Newline => {
                    self.file_state.line_start = true;
                    break FileToken::Newline;
                }

                FromRawResult::Trivia => {
                    leading_trivia = true;
                }
            };
        };

        Ok(ret)
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
