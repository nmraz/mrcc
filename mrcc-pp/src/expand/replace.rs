use std::collections::VecDeque;
use std::{iter, mem};

use rustc_hash::FxHashSet;

use mrcc_lex::{LexCtx, PunctKind, Symbol, Token, TokenKind};
use mrcc_source::{diag::RawSubDiagnostic, DResult};
use mrcc_source::{smap::ExpansionType, SourceRange};

use crate::PpToken;

use super::def::{MacroDefKind, MacroTable, ReplacementList};

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

pub trait ReplacementLexer {
    fn next(&mut self, ctx: &mut LexCtx<'_, '_>) -> DResult<PpToken>;
    fn peek(&mut self, ctx: &mut LexCtx<'_, '_>) -> DResult<PpToken>;
}

pub struct ReplacementCtx<'a, 'b, 'h> {
    ctx: &'a mut LexCtx<'b, 'h>,
    defs: &'a MacroTable,
    replacements: &'a mut PendingReplacements,
    lexer: &'a mut dyn ReplacementLexer,
}

impl<'a, 'b, 'h> ReplacementCtx<'a, 'b, 'h> {
    pub fn new(
        ctx: &'a mut LexCtx<'b, 'h>,
        defs: &'a MacroTable,
        replacements: &'a mut PendingReplacements,
        lexer: &'a mut dyn ReplacementLexer,
    ) -> Self {
        Self {
            ctx,
            defs,
            replacements,
            lexer,
        }
    }

    pub fn next_expanded_token(&mut self) -> DResult<Option<ReplacementToken>> {
        while let Some(mut tok) = self.replacements.next_token() {
            if !self.begin_expansion(&mut tok)? {
                return Ok(Some(tok));
            }
        }

        Ok(None)
    }

    pub fn begin_expansion(&mut self, tok: &mut ReplacementToken) -> DResult<bool> {
        if !tok.allow_expansion {
            return Ok(false);
        }

        let name_tok = match tok.ppt.maybe_map(|kind| match kind {
            TokenKind::Ident(name) => Some(name),
            _ => None,
        }) {
            Some(tok) => tok,
            None => return Ok(false),
        };

        let name = name_tok.data();

        if self.replacements.is_active(name) {
            // Prevent further expansions of this token in all contexts, as per §6.10.3.4p2.
            tok.allow_expansion = false;
            return Ok(false);
        }

        if let Some(def) = self.defs.lookup(name) {
            match &def.kind {
                MacroDefKind::Object(replacement) => {
                    self.push_object_macro(name_tok, replacement)?;
                    return Ok(true);
                }

                MacroDefKind::Function {
                    params,
                    replacement,
                } => {
                    return self.try_push_function_macro(
                        name_tok.tok,
                        def.name_tok,
                        params,
                        replacement,
                    );
                }
            }
        }

        Ok(false)
    }

    fn push_object_macro(
        &mut self,
        name_tok: PpToken<Symbol>,
        replacement_list: &ReplacementList,
    ) -> DResult<bool> {
        let spelling_range = match replacement_list.spelling_range() {
            Some(range) => range,
            None => return Ok(false),
        };

        let ctx = &mut self.ctx;

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

        self.replacements.push(
            Some(name_tok.data()),
            replacement_list
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
                    } else {
                        ppt.line_start = false;
                    }

                    // Move every token to point into the newly-created expansion source.
                    ppt.tok.range = move_subrange(ppt.tok.range, spelling_range, exp_range);
                    ppt.into()
                })
                .collect(),
        );

        Ok(true)
    }

    fn try_push_function_macro(
        &mut self,
        name_tok: Token<Symbol>,
        def_tok: Token<Symbol>,
        params: &[Symbol],
        replacement: &ReplacementList,
    ) -> DResult<bool> {
        let peeked = self.peek_token()?;

        if peeked.ppt.data() != TokenKind::Punct(PunctKind::LParen) {
            return Ok(false);
        }

        // Consume the peeked lparen.
        self.next_token()?;

        let args = match self.parse_macro_args(name_tok, def_tok)? {
            Some(args) => args,
            None => return Ok(true),
        };

        if !check_arity(params, &args) {
            let quantifier = if args.len() > params.len() {
                "many"
            } else {
                "few"
            };

            let note = self.macro_def_note(def_tok);

            self.ctx
                .reporter()
                .error(
                    name_tok.range,
                    format!("too {} arguments provided to macro invocation", quantifier),
                )
                .add_note(note)
                .emit()?;
            return Ok(true);
        }

        unimplemented!("function-like macro expansion")
    }

    fn parse_macro_args(
        &mut self,
        name_tok: Token<Symbol>,
        def_tok: Token<Symbol>,
    ) -> DResult<Option<Vec<VecDeque<ReplacementToken>>>> {
        fn finish_arg(
            arg: &mut VecDeque<ReplacementToken>,
            mut tok: ReplacementToken,
        ) -> VecDeque<ReplacementToken> {
            tok.ppt = tok.ppt.map(|_| TokenKind::Eof);
            arg.push_back(tok);
            mem::take(arg)
        }

        let mut args = Vec::new();
        let mut cur_arg = VecDeque::new();
        let mut paren_level = 1; // We've already consumed the opening lparen.

        loop {
            // Make sure that we don't consume the EOF token (if one exists), which could be crucial
            // when using directive lexers and the like.
            if self.peek_token()?.ppt.data() == TokenKind::Eof {
                let note = self.macro_def_note(def_tok);

                self.ctx
                    .reporter()
                    .error(name_tok.range, "unterminated macro invocation")
                    .add_note(note)
                    .emit()?;
                return Ok(None);
            }

            let tok = self.next_token()?;

            match tok.ppt.data() {
                TokenKind::Punct(PunctKind::LParen) => {
                    paren_level += 1;
                    cur_arg.push_back(tok)
                }
                TokenKind::Punct(PunctKind::RParen) => {
                    paren_level -= 1;
                    if paren_level == 0 {
                        args.push(finish_arg(&mut cur_arg, tok));
                        break;
                    }
                    cur_arg.push_back(tok);
                }

                TokenKind::Punct(PunctKind::Comma) if paren_level == 1 => {
                    args.push(finish_arg(&mut cur_arg, tok))
                }

                _ => cur_arg.push_back(tok),
            }
        }

        Ok(Some(args))
    }

    fn pre_expand_macro_arg(
        &mut self,
        arg: VecDeque<ReplacementToken>,
    ) -> DResult<VecDeque<ReplacementToken>> {
        self.replacements.push(None, arg);

        itertools::process_results(
            iter::from_fn(|| self.next_expanded_token().transpose()),
            |iter| {
                iter.take_while(|tok| tok.ppt.data() != TokenKind::Eof)
                    .collect()
            },
        )
    }

    fn macro_def_note(&self, name_tok: Token<Symbol>) -> RawSubDiagnostic {
        RawSubDiagnostic::new(
            format!("macro '{}' defined here", &self.ctx.interner[name_tok.data]),
            name_tok.range.into(),
        )
    }

    fn next_token(&mut self) -> DResult<ReplacementToken> {
        self.next_or_lex(
            |replacements| replacements.next_token(),
            |lexer, ctx| lexer.next(ctx),
        )
    }

    fn peek_token(&mut self) -> DResult<ReplacementToken> {
        self.next_or_lex(
            |replacements| replacements.peek_token(),
            |lexer, ctx| lexer.peek(ctx),
        )
    }

    fn next_or_lex(
        &mut self,
        next: impl FnOnce(&mut PendingReplacements) -> Option<ReplacementToken>,
        lex: impl FnOnce(&mut dyn ReplacementLexer, &mut LexCtx<'_, '_>) -> DResult<PpToken>,
    ) -> DResult<ReplacementToken> {
        next(&mut self.replacements).map_or_else(|| lex(self.lexer, self.ctx).map(Into::into), Ok)
    }
}

fn check_arity(params: &[Symbol], args: &[VecDeque<ReplacementToken>]) -> bool {
    // There is always at least one (empty, EOF-only) argument parsed, so if the macro takes no
    //  parameters just make sure that there is exactly one empty argument.
    args.len() == params.len() || (params.is_empty() && args.len() == 1 && args[0].len() == 1)
}

struct PendingReplacement {
    name: Option<Symbol>,
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

    fn is_active(&self, name: Symbol) -> bool {
        self.active_names.contains(&name)
    }

    fn next_token(&mut self) -> Option<ReplacementToken> {
        self.next(PendingReplacement::next_token)
    }

    fn peek_token(&mut self) -> Option<ReplacementToken> {
        self.next(|replacement| replacement.peek_token())
    }

    fn push(&mut self, name: Option<Symbol>, tokens: VecDeque<ReplacementToken>) {
        if let Some(name) = name {
            self.active_names.insert(name);
        }
        self.replacements.push(PendingReplacement { name, tokens });
    }

    fn pop(&mut self) {
        if let Some(replacement) = self.replacements.pop() {
            if let Some(name) = replacement.name {
                self.active_names.remove(&name);
            }
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

            self.pop();
        }

        None
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
