use std::collections::VecDeque;
use std::{iter, mem};

use itertools::Itertools;
use rustc_hash::FxHashSet;

use mrcc_lex::{LexCtx, PunctKind, Symbol, Token, TokenKind};
use mrcc_source::{diag::RawSubDiagnostic, DResult};
use mrcc_source::{smap::ExpansionKind, FragmentedSourceRange, SourceId, SourceRange};

use crate::PpToken;

use super::def::{MacroDefKind, MacroTable, ReplacementList};

/// An abstraction over a token stream necessary for handling function-like macros during
/// replacement.
pub trait ReplacementLexer {
    /// Retrieves the next token and advances the stream.
    fn next(&mut self, ctx: &mut LexCtx<'_, '_>) -> DResult<PpToken>;

    /// Retrieves the next token, but without advancing the stream.
    ///
    /// In general, the next call to `next()` need not return the same token (for example,
    /// `MacroArgLexer` skips preprocessing directives), but if this function returns an `Eof` then
    /// `next()` should as well.
    fn peek(&mut self, ctx: &mut LexCtx<'_, '_>) -> DResult<PpToken>;
}

/// A token that tracks whether it can be expanded.
///
/// This is necessary in order to properly implement an edge case that arises in §6.10.3.4p2; in
/// general, avoiding repeated replacements of the same name can be done by tracking which
/// expansions are currently in flight and avoiding those replacements. However, this scheme doesn't
/// work in the presence of function-like macro arguments, which are rescanned at two distinct
/// points: first when pre-expanding the argument before substitution, and later when rescanning the
/// expanded function-like macro. The latter rescanning must not replace any names skipped during
/// the former.
#[derive(Debug, Copy, Clone)]
pub struct ReplacementToken {
    /// The underlying preprocessor token.
    pub ppt: PpToken,
    /// A flag indicating whether expansion of this token is allowed.
    pub allow_expansion: bool,
}

impl From<PpToken> for ReplacementToken {
    /// Converts a preprocessor token to a replacement token that allows expansion.
    fn from(ppt: PpToken) -> Self {
        Self {
            ppt,
            allow_expansion: true,
        }
    }
}

/// A structure pointing to the state necessary for macro replacement.
pub struct ReplacementCtx<'a, 'b, 'h> {
    ctx: &'a mut LexCtx<'b, 'h>,
    defs: &'a MacroTable,
    replacements: &'a mut PendingReplacements,
    lexer: &'a mut dyn ReplacementLexer,
}

impl<'a, 'b, 'h> ReplacementCtx<'a, 'b, 'h> {
    /// Creates a new context with the specified state.
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

    /// Returns the next pending replacement token, if any.
    ///
    /// This also handles rescanning of the expanded token stream, as per §6.10.3.4.
    pub fn next_expansion_token(&mut self) -> DResult<Option<ReplacementToken>> {
        while let Some(mut tok) = self.replacements.next_token() {
            if !self.begin_expansion(&mut tok)? {
                return Ok(Some(tok));
            }
        }

        Ok(None)
    }

    /// Attempts to start macro-expanding `tok`, returning whether expansion is now taking place.
    ///
    /// If `tok` names a macro that is currently being expanded, it is not expanded and is marked
    /// as disallowing expansion (§6.10.3.4p2).
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
                        name_tok,
                        def.name_tok,
                        params,
                        replacement,
                    );
                }
            }
        }

        Ok(false)
    }

    /// Pushes an object-like macro expansion replacing `name_tok` with `replacement_list`.
    fn push_object_macro(
        &mut self,
        name_tok: PpToken<Symbol>,
        replacement_list: &ReplacementList,
    ) -> DResult<()> {
        let tokens = match self.map_replacement_tokens(name_tok.map(|_| ()), replacement_list)? {
            Some(iter) => iter.collect(),
            None => return Ok(()),
        };
        self.replacements.push(Some(name_tok.data()), tokens);
        Ok(())
    }

    /// If the next token is an opening parenthesis, parses and pushes a function-like macro
    /// expansion replacing `name_tok`, returning `true`. Otherwise, returns `false`.
    ///
    /// `def_tok` should point to the name written in the macro definition.
    fn try_push_function_macro(
        &mut self,
        name_tok: PpToken<Symbol>,
        def_tok: Token<Symbol>,
        params: &[Symbol],
        replacement_list: &ReplacementList,
    ) -> DResult<bool> {
        let peeked = self.peek_token()?;

        if peeked.ppt.data() != TokenKind::Punct(PunctKind::LParen) {
            return Ok(false);
        }

        // Consume the peeked lparen.
        self.next_token()?;

        let args = match self.parse_macro_args(name_tok.tok, def_tok)? {
            Some(args) => args,
            None => return Ok(true),
        };

        if !self.check_arity(name_tok.tok, def_tok, params, &args)? {
            return Ok(true);
        }

        self.push_parsed_function_macro(name_tok, replacement_list, params, args)?;
        Ok(true)
    }

    /// Parses and returns arguments for a function-like macro invocation after an opening
    /// parenthesis has been consumed.
    ///
    /// The arguments are parsed according to the rules in §6.10.3p11, skipping nested pairs of
    /// balanced parentheses.
    ///
    /// Every returned argument will be terminated by an `Eof` token indicating where the argument
    /// was terminated, to simplify preexpansion and error reporting later.
    ///
    /// The returned argument list will always contain at least one (possibly empty, `Eof`-only)
    /// argument.
    fn parse_macro_args(
        &mut self,
        name_tok: Token<Symbol>,
        def_tok: Token<Symbol>,
    ) -> DResult<Option<Vec<VecDeque<ReplacementToken>>>> {
        let mut args = Vec::new();
        let mut cur_arg = VecDeque::new();
        let mut paren_level = 1; // We've already consumed the opening lparen.

        let mut finish_arg = |arg: &mut VecDeque<ReplacementToken>, mut tok: ReplacementToken| {
            tok.ppt = tok.ppt.map(|_| TokenKind::Eof);
            arg.push_back(tok);
            args.push(mem::take(arg))
        };

        loop {
            // Make sure that we don't consume the EOF token (if one exists), which could be crucial
            // when using directive lexers or pre-expanding macro arguments.
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
                        finish_arg(&mut cur_arg, tok);
                        break;
                    }
                    cur_arg.push_back(tok);
                }

                TokenKind::Punct(PunctKind::Comma) if paren_level == 1 => {
                    finish_arg(&mut cur_arg, tok);
                }

                _ => cur_arg.push_back(tok),
            }
        }

        Ok(Some(args))
    }

    /// Compares the number of arguments provided in `args` to the number of parameters in `params`,
    /// reporting errors on mismatch.
    ///
    /// Returns true if the arguments match the arity specified in `params`.
    fn check_arity(
        &mut self,
        name_tok: Token<Symbol>,
        def_tok: Token<Symbol>,
        params: &[Symbol],
        args: &[VecDeque<ReplacementToken>],
    ) -> DResult<bool> {
        // There is always at least one argument parsed, so if the macro takes no parameters just
        // make sure that there is exactly one empty argument.
        if args.len() != params.len()
            && !(params.is_empty() && args.len() == 1 && args[0].len() == 1)
        {
            let (quantifier, arg_tok) = if args.len() > params.len() {
                ("many", args[params.len()][0])
            } else {
                ("few", *args.last().unwrap().back().unwrap())
            };

            let note = self.macro_def_note(def_tok);

            self.ctx
                .reporter()
                .error(
                    name_tok.range,
                    format!("too {} arguments provided to macro invocation", quantifier),
                )
                .add_range(arg_tok.ppt.range().into())
                .add_note(note)
                .emit()?;
            return Ok(false);
        }

        Ok(true)
    }

    /// Pushes a function-like macro replacing `name_tok` with `replacement_list`.
    ///
    /// This also handles pre-expansion and substitution of macro arguments.
    fn push_parsed_function_macro(
        &mut self,
        name_tok: PpToken<Symbol>,
        replacement_list: &ReplacementList,
        params: &[Symbol],
        args: Vec<VecDeque<ReplacementToken>>,
    ) -> DResult<()> {
        enum ArgState {
            Raw(VecDeque<ReplacementToken>),
            PreExpanded(Vec<ReplacementToken>),
        }

        fn get_pre_expanded_arg<'c>(
            this: &mut ReplacementCtx<'_, '_, '_>,
            arg: &'c mut ArgState,
        ) -> DResult<impl Iterator<Item = ReplacementToken> + 'c> {
            if let ArgState::Raw(unexp) = arg {
                *arg = ArgState::PreExpanded(this.pre_expand_macro_arg(mem::take(unexp))?);
            }

            match arg {
                ArgState::PreExpanded(preexp) => Ok(preexp.iter().copied()),
                ArgState::Raw(_) => unreachable!(),
            }
        }

        let mut replacement_tok = name_tok.map(|_| ());
        replacement_tok.tok.range = self.get_function_replacement_range(name_tok, &args);

        let body_tokens = match self.map_replacement_tokens(replacement_tok, replacement_list)? {
            Some(iter) => iter,
            None => return Ok(()),
        };

        let mut args: Vec<_> = args.into_iter().map(ArgState::Raw).collect();
        let mut tokens = VecDeque::new();

        for tok in body_tokens {
            if let TokenKind::Ident(ident) = tok.ppt.data() {
                if let Some(idx) = params.iter().position(|&name| name == ident) {
                    let preexp = get_pre_expanded_arg(self, &mut args[idx])?;
                    tokens.extend(self.map_arg_tokens(tok.ppt.map(|_| ()), preexp)?);
                    continue;
                }
            }

            tokens.push_back(tok);
        }

        self.replacements.push(Some(name_tok.data()), tokens);
        Ok(())
    }

    /// Computes the [replacement range](../../../mrcc_source/smap/index.html#sources) for a
    /// function-like macro invocation of `name_tok` with arguments `args`.
    ///
    /// # Panics
    ///
    /// Panics if the tokens come from several different files. This should not be possible in a
    /// function-like macro invocation.
    fn get_function_replacement_range(
        &self,
        name_tok: PpToken<Symbol>,
        args: &[VecDeque<ReplacementToken>],
    ) -> SourceRange {
        let last_tok = args.last().unwrap().back().unwrap().ppt;

        self.ctx
            .smap
            .get_unfragmented_range(FragmentedSourceRange::new(
                name_tok.range().start(),
                last_tok.range().end(),
            ))
            .expect("macro invocation spans multiple files")
    }

    /// Expands the tokens in `arg` as if they form the remainder of the file.
    ///
    /// This step is performed before substituting the argument into the expansion of a
    /// function-like macro.
    ///
    /// It is critical here that `arg` have a trailing `Eof` (as added by `parse_macro_args`); it is
    /// used as a sentinel to stop expansion.
    /// The returned tokens will no longer have a trailing `Eof`.
    fn pre_expand_macro_arg(
        &mut self,
        arg: VecDeque<ReplacementToken>,
    ) -> DResult<Vec<ReplacementToken>> {
        self.replacements.push(None, arg);

        itertools::process_results(
            iter::from_fn(|| self.next_expansion_token().transpose()),
            |iter| {
                iter.take_while(|tok| tok.ppt.data() != TokenKind::Eof)
                    .collect()
            },
        )
    }

    /// Maps every token in `tokens` to a new one with a range indicating that it came from a macro
    /// argument expansion into `replacement_tok`.
    ///
    /// The tokens need not be contiguous or lie in the same source.
    fn map_arg_tokens(
        &mut self,
        replacement_tok: PpToken<()>,
        tokens: impl Iterator<Item = ReplacementToken>,
    ) -> DResult<Vec<ReplacementToken>> {
        fn lookup_tok_source(
            this: &ReplacementCtx<'_, '_, '_>,
            tok: &ReplacementToken,
        ) -> SourceId {
            this.ctx.smap.lookup_source_id(tok.ppt.range().start())
        }

        let mut tokens = tokens.peekable();
        let mut ret = Vec::new();
        let mut first = true;

        while let Some(tok) = tokens.next() {
            let mut run = vec![tok];
            let source = lookup_tok_source(self, &tok);

            run.extend(tokens.peeking_take_while(|tok| lookup_tok_source(self, tok) == source));

            let begin = run[0].ppt.range().start();
            let end = run.last().unwrap().ppt.range().end();

            let spelling_range = SourceRange::new(begin, end.offset_from(begin));

            ret.extend(self.map_tokens(
                replacement_tok,
                mem::replace(&mut first, false),
                run,
                spelling_range,
                ExpansionKind::MacroArg,
            )?);
        }

        Ok(ret)
    }

    /// Maps every token in `replacement_list` to a new one indicating that it came from a macro
    /// expansion into `replacement_tok`.
    ///
    /// # Panics
    ///
    /// May panic if the tokens in `replacement_list` do not cover a contiguous range from a single
    /// source.
    fn map_replacement_tokens<'c>(
        &mut self,
        replacement_tok: PpToken<()>,
        replacement_list: &'c ReplacementList,
    ) -> DResult<Option<impl Iterator<Item = ReplacementToken> + 'c>> {
        let spelling_range = match replacement_list.spelling_range() {
            Some(range) => range,
            None => return Ok(None),
        };

        self.map_tokens(
            replacement_tok,
            true,
            replacement_list.tokens().iter().copied().map(Into::into),
            spelling_range,
            ExpansionKind::Macro,
        )
        .map(Some)
    }

    /// Maps every token in `tokens` to a new one indicating that it came from an expansion of
    /// `spelling_range` into `replacement_tok`.
    ///
    /// If `first` is set, these tokens are assumed to be the first tokens expanded into
    /// `replacement_tok`; the first of them will inherit whitespace and line properties from
    /// `replacement_tok`.
    ///
    /// # Panics
    ///
    /// Panics if any of the tokens does not lie entirely within `spelling_range`.
    fn map_tokens<'c>(
        &mut self,
        replacement_tok: PpToken<()>,
        first: bool,
        tokens: impl IntoIterator<Item = ReplacementToken> + 'c,
        spelling_range: SourceRange,
        expansion_kind: ExpansionKind,
    ) -> DResult<impl Iterator<Item = ReplacementToken> + 'c> {
        fn move_subrange(
            subrange: SourceRange,
            old_range: SourceRange,
            new_range: SourceRange,
        ) -> SourceRange {
            new_range.subrange(
                old_range
                    .local_range(subrange)
                    .expect("range not in spelling range"),
            )
        }

        let ctx = &mut self.ctx;

        let exp_id = ctx
            .smap
            .create_expansion(spelling_range, replacement_tok.range(), expansion_kind)
            .map_err(|_| {
                ctx.reporter()
                    .fatal(
                        replacement_tok.range(),
                        "translation unit too large for macro expansion",
                    )
                    .emit()
                    .unwrap_err()
            })?;

        let exp_range = ctx.smap.get_source(exp_id).range;

        Ok(tokens.into_iter().enumerate().map(move |(idx, mut tok)| {
            let ppt = &mut tok.ppt;
            if first && idx == 0 {
                // The first replacement token inherits `line_start` and `leading_trivia`
                // from the replaced token.
                ppt.line_start = replacement_tok.line_start;
                ppt.leading_trivia = replacement_tok.leading_trivia;
            } else {
                ppt.line_start = false;
            }

            // Move every token to point into the newly-created expansion source.
            ppt.tok.range = move_subrange(ppt.tok.range, spelling_range, exp_range);

            tok
        }))
    }

    /// Creates a diagnostic note indicating the specified macro definition.
    fn macro_def_note(&self, def_tok: Token<Symbol>) -> RawSubDiagnostic {
        RawSubDiagnostic::new(
            format!("macro '{}' defined here", &self.ctx.interner[def_tok.data]),
            def_tok.range.into(),
        )
    }

    /// Advances to the next pending expansion token, falling back to the lexer if there is none.
    fn next_token(&mut self) -> DResult<ReplacementToken> {
        self.next_or_lex(
            |replacements| replacements.next_token(),
            |lexer, ctx| lexer.next(ctx),
        )
    }

    /// Peeks at the next pending expansion token, falling back to the lexer if there is none.
    fn peek_token(&mut self) -> DResult<ReplacementToken> {
        self.next_or_lex(
            |replacements| replacements.peek_token(),
            |lexer, ctx| lexer.peek(ctx),
        )
    }

    /// Invokes `next` to obtain the next expansion token, falling back to `lex` if it returns
    /// `None`.
    fn next_or_lex(
        &mut self,
        next: impl FnOnce(&mut PendingReplacements) -> Option<ReplacementToken>,
        lex: impl FnOnce(&mut dyn ReplacementLexer, &mut LexCtx<'_, '_>) -> DResult<PpToken>,
    ) -> DResult<ReplacementToken> {
        next(&mut self.replacements).map_or_else(|| lex(self.lexer, self.ctx).map(Into::into), Ok)
    }
}

/// Represents an in-flight macro replacement.
struct PendingReplacement {
    /// The name of the macro being replaced, if any. This is used to track which macros are
    /// currently being expanded.
    name: Option<Symbol>,
    /// The tokens remaining in this replacement.
    tokens: VecDeque<ReplacementToken>,
}

impl PendingReplacement {
    /// Advances to the next token in this replacement.
    fn next_token(&mut self) -> Option<ReplacementToken> {
        self.tokens.pop_front()
    }

    /// Peeks at the next token in this replacement.
    fn peek_token(&self) -> Option<ReplacementToken> {
        self.tokens.front().copied()
    }
}

/// A stack of the macro replacements currently in flight.
pub struct PendingReplacements {
    /// A stack of the active replacements - last is most recent.
    replacements: Vec<PendingReplacement>,
    /// Tracks which names are currently being expanded.
    active_names: FxHashSet<Symbol>,
}

impl PendingReplacements {
    /// Creates a new, empty replacement stack.
    pub fn new() -> Self {
        Self {
            replacements: Vec::new(),
            active_names: Default::default(),
        }
    }

    /// Checks whether `name` is currently being expanded.
    fn is_active(&self, name: Symbol) -> bool {
        self.active_names.contains(&name)
    }

    /// Advances to the next replacement token, transparently popping completed replacements.
    fn next_token(&mut self) -> Option<ReplacementToken> {
        self.next(PendingReplacement::next_token)
    }

    /// Peeks at the next replacement token, transparently popping completed replacements.
    fn peek_token(&mut self) -> Option<ReplacementToken> {
        self.next(|replacement| replacement.peek_token())
    }

    /// Pushes a new replacement onto the stack.
    ///
    /// If `name` is provided, it will be considered active until the replacement is popped.
    fn push(&mut self, name: Option<Symbol>, tokens: VecDeque<ReplacementToken>) {
        if let Some(name) = name {
            self.active_names.insert(name);
        }
        self.replacements.push(PendingReplacement { name, tokens });
    }

    /// Invokes `f` on the topmost replacement, popping replacements and retrying until it returns
    /// `Some` or there are no replacements left.
    ///
    /// Returns the value returned by `f`, if any.
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

    /// Pops the topmost replacement off the stack.
    fn pop(&mut self) {
        if let Some(replacement) = self.replacements.pop() {
            if let Some(name) = replacement.name {
                self.active_names.remove(&name);
            }
        }
    }
}
