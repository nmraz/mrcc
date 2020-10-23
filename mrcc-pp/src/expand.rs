use mrcc_lex::{LexCtx, Symbol};
use mrcc_source::DResult;

use crate::PpToken;

use def::MacroTable;
use replace::{PendingReplacements, ReplacementCtx};

pub use def::{MacroDef, MacroDefKind, ReplacementList};
pub use replace::ReplacementLexer;

mod def;
mod replace;

pub struct MacroState {
    defs: MacroTable,
    replacements: PendingReplacements,
}

impl MacroState {
    pub fn new() -> Self {
        Self {
            defs: MacroTable::new(),
            replacements: PendingReplacements::new(),
        }
    }

    pub fn define(&mut self, def: MacroDef) -> Option<MacroDef> {
        self.defs.define(def)
    }

    pub fn undef(&mut self, name: Symbol) {
        self.defs.undef(name)
    }

    pub fn next_expansion_token(
        &mut self,
        ctx: &mut LexCtx<'_, '_>,
        lexer: &mut dyn ReplacementLexer,
    ) -> DResult<Option<PpToken>> {
        ReplacementCtx::new(ctx, &self.defs, &mut self.replacements, lexer)
            .next_expansion_token()
            .map(|res| res.map(|tok| tok.ppt))
    }

    pub fn begin_expansion(
        &mut self,
        ctx: &mut LexCtx<'_, '_>,
        ppt: PpToken,
        lexer: &mut dyn ReplacementLexer,
    ) -> DResult<bool> {
        ReplacementCtx::new(ctx, &self.defs, &mut self.replacements, lexer)
            .begin_expansion(&mut ppt.into())
    }
}
