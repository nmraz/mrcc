use mrcc_lex::{LexCtx, Symbol};
use mrcc_source::DResult;

use data::MacroTable;
pub use data::{MacroDef, MacroDefKind, ReplacementList};

use crate::PpToken;

mod data;
mod replace;

pub trait ReplacementLexer {
    fn next(&mut self, ctx: &mut LexCtx<'_, '_>) -> DResult<PpToken>;
    fn next_macro_arg(&mut self, ctx: &mut LexCtx<'_, '_>) -> DResult<PpToken>;
    fn peek(&mut self, ctx: &mut LexCtx<'_, '_>) -> DResult<PpToken>;
}

pub struct MacroState {
    definitions: MacroTable,
}

impl MacroState {
    pub fn new() -> Self {
        Self {
            definitions: MacroTable::new(),
        }
    }

    pub fn define(&mut self, def: MacroDef) -> Option<MacroDef> {
        self.definitions.define(def)
    }

    pub fn undef(&mut self, name: Symbol) {
        self.definitions.undef(name)
    }

    pub fn begin_expand(
        &mut self,
        ctx: &mut LexCtx<'_, '_>,
        ppt: PpToken,
        lexer: &mut dyn ReplacementLexer,
    ) -> DResult<bool> {
        Ok(false)
    }

    pub fn next_expanded_token(
        &mut self,
        ctx: &mut LexCtx<'_, '_>,
        lexer: &mut dyn ReplacementLexer,
    ) -> DResult<Option<PpToken>> {
        Ok(None)
    }
}
