use std::collections::VecDeque;

use rustc_hash::FxHashSet;

use mrcc_lex::{LexCtx, Symbol, TokenKind};
use mrcc_source::DResult;
use mrcc_source::{smap::ExpansionType, SourceRange};

use crate::PpToken;

use super::data::ReplacementList;
use super::ReplacementLexer;

#[derive(Debug, Copy, Clone)]
pub struct ReplacementToken {
    pub ppt: PpToken,
    pub allow_expansion: bool,
}

impl From<PpToken> for ReplacementToken {
    fn from(ppt: PpToken) -> Self {
        Self {
            ppt,
            allow_expansion: true,
        }
    }
}

pub fn next_or_lex(
    next: impl FnOnce() -> Option<ReplacementToken>,
    lex: impl FnOnce() -> DResult<PpToken>,
) -> DResult<ReplacementToken> {
    next().map_or_else(|| lex().map(|ppt| ppt.into()), Ok)
}

struct PendingReplacement {
    name: Symbol,
    tokens: VecDeque<ReplacementToken>,
}

impl PendingReplacement {
    fn next_token(&mut self) -> Option<ReplacementToken> {
        self.tokens.pop_front()
    }

    fn peek_token(&self) -> Option<ReplacementToken> {
        self.tokens.front().copied()
    }
}

pub struct PendingReplacements {
    replacements: Vec<PendingReplacement>,
    active_names: FxHashSet<Symbol>,
}

impl PendingReplacements {
    pub fn new() -> Self {
        Self {
            replacements: Vec::new(),
            active_names: Default::default(),
        }
    }

    pub fn is_active(&self, name: Symbol) -> bool {
        self.active_names.contains(&name)
    }

    pub fn push(
        &mut self,
        ctx: &mut LexCtx<'_, '_>,
        name_tok: PpToken<Symbol>,
        replacement_list: &ReplacementList,
    ) -> DResult<bool> {
        let spelling_range = match replacement_list.spelling_range() {
            Some(range) => range,
            None => return Ok(false),
        };

        let exp_id = ctx
            .smap
            .create_expansion(spelling_range, name_tok.range(), ExpansionType::Macro)
            .map_err(|_| {
                ctx.reporter()
                    .fatal(
                        name_tok.range(),
                        "translation unit too large for macro expansion",
                    )
                    .emit()
                    .unwrap_err()
            })?;

        let exp_range = ctx.smap.get_source(exp_id).range;

        self.push_replacement(PendingReplacement {
            name: name_tok.data(),
            tokens: replacement_list
                .tokens()
                .iter()
                .copied()
                .enumerate()
                .map(|(idx, mut ppt)| {
                    if idx == 0 {
                        // The first replacement token inherits `line_start` and `leading_trivia`
                        // from the replaced token.
                        ppt.line_start = name_tok.line_start;
                        ppt.leading_trivia = name_tok.leading_trivia;
                    }

                    // Move every token to point into the newly-created expansion source.
                    ppt.tok.range = move_subrange(ppt.tok.range, spelling_range, exp_range);
                    ppt.into()
                })
                .collect(),
        });

        Ok(true)
    }

    pub fn next_token(&mut self) -> Option<ReplacementToken> {
        self.next(PendingReplacement::next_token)
    }

    pub fn peek_token(&mut self) -> Option<ReplacementToken> {
        self.next(|replacement| replacement.peek_token())
    }

    pub fn eat_or_lex(
        &mut self,
        ctx: &mut LexCtx<'_, '_>,
        lexer: &mut dyn ReplacementLexer,
        pred: impl FnOnce(TokenKind) -> bool,
    ) -> DResult<bool> {
        let ppt = next_or_lex(|| self.peek_token(), || lexer.peek(ctx))?.ppt;

        if pred(ppt.data()) {
            next_or_lex(|| self.next_token(), || lexer.next(ctx))?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn next(
        &mut self,
        mut f: impl FnMut(&mut PendingReplacement) -> Option<ReplacementToken>,
    ) -> Option<ReplacementToken> {
        while let Some(top) = self.replacements.last_mut() {
            if let Some(tok) = f(top) {
                return Some(tok);
            }

            self.pop_replacement();
        }

        None
    }

    fn push_replacement(&mut self, replacement: PendingReplacement) {
        self.active_names.insert(replacement.name);
        self.replacements.push(replacement);
    }

    fn pop_replacement(&mut self) {
        if let Some(replacement) = self.replacements.pop() {
            self.active_names.remove(&replacement.name);
        }
    }
}

fn move_subrange(
    subrange: SourceRange,
    old_range: SourceRange,
    new_range: SourceRange,
) -> SourceRange {
    let off = subrange.start().offset_from(old_range.start());
    let len = subrange.len();

    new_range.subrange(off, len)
}
