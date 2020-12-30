use mrcc_lex::{LexCtx, Symbol};
use mrcc_source::DResult;

use crate::PpToken;

use def::MacroTable;
use replace::{PendingReplacements, ReplacementCtx};

pub use def::{MacroDef, MacroDefKind, ReplacementList};
pub use replace::ReplacementLexer;

mod def;
mod replace;

/// Tracks macro definitions and expansion state.
pub struct MacroState {
    defs: MacroTable,
    replacements: PendingReplacements,
}

impl MacroState {
    /// Creates a new state with no definitions and no pending expansion tokens.
    pub fn new() -> Self {
        Self {
            defs: MacroTable::new(),
            replacements: PendingReplacements::new(),
        }
    }

    /// Records the specified macro definition.
    ///
    /// If `def` redefines an existing macro (using the rules in §6.10.3p2), the previous definition
    /// is returned.
    pub fn define(&mut self, def: MacroDef) -> Option<MacroDef> {
        self.defs.define(def)
    }

    /// Removes any macro definition associated with `name`.
    ///
    /// This has no effect if `name` is not defined as a macro.
    pub fn undef(&mut self, name: Symbol) {
        self.defs.undef(name)
    }

    /// Returns the next pending macro expansion token, if any.
    ///
    /// The tokens returned by this function have already been (recursively)
    /// rescanned as defined in §6.10.3.4, and should not be passed again to `begin_expansion`.
    ///
    /// `lexer` may be necessary in certain edge cases when a recursive expansion produces a call to
    /// a function-like macro and additional argument tokens need to be lexed.
    pub fn next_expansion_token(
        &mut self,
        ctx: &mut LexCtx<'_, '_>,
        mut lexer: impl ReplacementLexer,
    ) -> DResult<Option<PpToken>> {
        ReplacementCtx::new(ctx, &self.defs, &mut self.replacements, &mut lexer)
            .next_expansion_token()
            .map(|res| res.map(|tok| tok.ppt))
    }

    /// Attempts to start macro-expanding `ppt`, returning whether expansion is now taking place.
    ///
    /// If this function returns `true`, `ppt` should be discarded as it is being replaced; the
    /// replacement tokens can be retrieved by repeatedly calling `next_expansion_token`. This
    /// function should not be called again until `next_expansion_token` has returned `None` - it
    /// may behave unexpectedly otherwise.
    ///
    /// `lexer` will be used to read function-like macro arguments, if necessary.
    pub fn begin_expansion(
        &mut self,
        ctx: &mut LexCtx<'_, '_>,
        ppt: PpToken,
        mut lexer: impl ReplacementLexer,
    ) -> DResult<bool> {
        ReplacementCtx::new(ctx, &self.defs, &mut self.replacements, &mut lexer)
            .begin_expansion(&mut ppt.into())
    }
}
