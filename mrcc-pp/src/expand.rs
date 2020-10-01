use std::mem;

use mrcc_lex::{LexCtx, PunctKind, Symbol, Token, TokenKind};
use mrcc_source::DResult;

use crate::PpToken;

use def::MacroTable;
use replace::{PendingReplacements, ReplacementToken};

pub use def::{MacroDef, MacroDefKind, ReplacementList};

mod def;
mod replace;

pub trait ReplacementLexer {
    fn next(&mut self, ctx: &mut LexCtx<'_, '_>) -> DResult<PpToken>;
    fn next_macro_arg(&mut self, ctx: &mut LexCtx<'_, '_>) -> DResult<PpToken>;
    fn peek(&mut self, ctx: &mut LexCtx<'_, '_>) -> DResult<PpToken>;
}

pub struct MacroState {
    definitions: MacroTable,
    replacements: PendingReplacements,
}

impl MacroState {
    pub fn new() -> Self {
        Self {
            definitions: MacroTable::new(),
            replacements: PendingReplacements::new(),
        }
    }

    pub fn define(&mut self, def: MacroDef) -> Option<MacroDef> {
        self.definitions.define(def)
    }

    pub fn undef(&mut self, name: Symbol) {
        self.definitions.undef(name)
    }

    pub fn begin_expansion(
        &mut self,
        ctx: &mut LexCtx<'_, '_>,
        ppt: PpToken,
        lexer: &mut dyn ReplacementLexer,
    ) -> DResult<bool> {
        self.begin_repl_expansion(ctx, &mut ppt.into(), lexer)
    }

    pub fn next_expanded_token(
        &mut self,
        ctx: &mut LexCtx<'_, '_>,
        lexer: &mut dyn ReplacementLexer,
    ) -> DResult<Option<PpToken>> {
        self.next_expanded_repl_token(ctx, lexer)
            .map(|res| res.map(|tok| tok.ppt))
    }

    fn begin_repl_expansion(
        &mut self,
        ctx: &mut LexCtx<'_, '_>,
        tok: &mut ReplacementToken,
        lexer: &mut dyn ReplacementLexer,
    ) -> DResult<bool> {
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

        if let Some(def) = self.definitions.lookup(name) {
            match &def.kind {
                MacroDefKind::Object(replacement) => {
                    self.replacements.push(ctx, name_tok, replacement)?;
                }

                MacroDefKind::Function {
                    params,
                    replacement,
                } => {
                    let replacements = &mut self.replacements;
                    let peeked = next_or_lex(|| replacements.peek_token(), || lexer.peek(ctx))?;

                    if peeked.ppt.data() != TokenKind::Punct(PunctKind::LParen) {
                        return Ok(false);
                    }

                    let args = match parse_macro_args(replacements, ctx, name_tok.tok, lexer)? {
                        Some(args) => args,
                        None => return Ok(true),
                    };

                    unimplemented!("function-like macro expansion")
                }
            }

            return Ok(true);
        }

        Ok(false)
    }

    fn next_expanded_repl_token(
        &mut self,
        ctx: &mut LexCtx<'_, '_>,
        lexer: &mut dyn ReplacementLexer,
    ) -> DResult<Option<ReplacementToken>> {
        while let Some(mut tok) = self.replacements.next_token() {
            if !self.begin_repl_expansion(ctx, &mut tok, lexer)? {
                return Ok(Some(tok));
            }
        }

        Ok(None)
    }
}

fn parse_macro_args(
    replacements: &mut PendingReplacements,
    ctx: &mut LexCtx<'_, '_>,
    name_tok: Token<Symbol>,
    lexer: &mut dyn ReplacementLexer,
) -> DResult<Option<Vec<Vec<ReplacementToken>>>> {
    let mut args = Vec::new();
    let mut cur_arg = Vec::new();
    let mut paren_level = 0;

    loop {
        let tok = next_or_lex(|| replacements.next_token(), || lexer.next_macro_arg(ctx))?;

        match tok.ppt.data() {
            TokenKind::Punct(PunctKind::LParen) => {
                paren_level += 1;
                cur_arg.push(tok)
            }
            TokenKind::Punct(PunctKind::RParen) => {
                paren_level -= 1;
                if paren_level == 0 {
                    args.push(cur_arg);
                    break;
                }
                cur_arg.push(tok);
            }

            TokenKind::Punct(PunctKind::Comma) if paren_level == 1 => {
                args.push(mem::take(&mut cur_arg))
            }

            TokenKind::Eof => {
                let msg = format!(
                    "unterminated invocation of macro '{}'",
                    &ctx.interner[name_tok.data]
                );

                ctx.reporter().error(name_tok.range, msg).emit()?;
                return Ok(None);
            }

            _ => cur_arg.push(tok),
        }
    }

    Ok(Some(args))
}

fn next_or_lex(
    next: impl FnOnce() -> Option<ReplacementToken>,
    lex: impl FnOnce() -> DResult<PpToken>,
) -> DResult<ReplacementToken> {
    next().map_or_else(|| lex().map(|ppt| ppt.into()), Ok)
}
