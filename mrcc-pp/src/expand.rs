use mrcc_lex::{LexCtx, Symbol, TokenKind};
use mrcc_source::DResult;

use crate::PpToken;

use data::MacroTable;
use replace::PendingReplacements;

pub use data::{MacroDef, MacroDefKind, ReplacementList};

mod data;
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
        let name_tok = match ppt.maybe_map(|kind| match kind {
            TokenKind::Ident(name) => Some(name),
            _ => None,
        }) {
            Some(tok) => tok,
            None => return Ok(false),
        };

        let name = name_tok.data();

        if self.replacements.is_active(name) {
            return Ok(false);
        }

        if let Some(def) = self.definitions.lookup(name) {
            match &def.kind {
                MacroDefKind::Object(replacement) => {
                    self.replacements.push(ctx, name_tok, replacement)?;
                }
                MacroDefKind::Function { .. } => unimplemented!("function-like macro expansion"),
            }

            return Ok(true);
        }

        Ok(false)
    }

    pub fn next_expanded_token(
        &mut self,
        ctx: &mut LexCtx<'_, '_>,
        lexer: &mut dyn ReplacementLexer,
    ) -> DResult<Option<PpToken>> {
        while let Some(ppt) = self.replacements.next_token() {
            if !self.begin_expansion(ctx, ppt, lexer)? {
                return Ok(Some(ppt));
            }
        }

        Ok(None)
    }
}
