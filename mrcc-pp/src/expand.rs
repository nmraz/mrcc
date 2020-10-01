use mrcc_lex::{LexCtx, PunctKind, Symbol, TokenKind};
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
                    let is_lparen = |kind| kind == TokenKind::Punct(PunctKind::LParen);

                    if !self.replacements.eat_or_lex(ctx, lexer, is_lparen)? {
                        return Ok(false);
                    }

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
