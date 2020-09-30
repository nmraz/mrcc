use std::collections::VecDeque;

use rustc_hash::FxHashSet;

use mrcc_lex::{LexCtx, Symbol};
use mrcc_source::{diag::Level, DResult};
use mrcc_source::{smap::ExpansionType, SourceRange};

use crate::PpToken;

use super::data::ReplacementList;

struct PendingReplacement {
    name: Symbol,
    tokens: VecDeque<PpToken>,
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
                    .report(
                        Level::Fatal,
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
                    ppt
                })
                .collect(),
        });

        Ok(true)
    }

    pub fn next_token(&mut self) -> Option<PpToken> {
        while let Some(top) = self.replacements.last_mut() {
            if let Some(ppt) = top.tokens.pop_front() {
                return Some(ppt);
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
