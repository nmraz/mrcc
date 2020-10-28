use std::collections::hash_map::Entry;
use std::mem;

use rustc_hash::FxHashMap;

use mrcc_lex::{Symbol, Token};
use mrcc_source::SourceRange;

use crate::PpToken;

/// Represents a list of replacement tokens in a macro definition.
///
/// These tokens are assumed to span a contiguous portion of a single source.
#[derive(Debug, Clone)]
pub struct ReplacementList {
    tokens: Vec<PpToken>,
}

impl ReplacementList {
    /// Creates a new replacement list with the specified tokens.
    pub fn new(mut tokens: Vec<PpToken>) -> Self {
        if let Some(first) = tokens.first_mut() {
            first.leading_trivia = false;
        }

        Self { tokens }
    }

    /// Returns the tokens constituting this replacement list.
    ///
    /// The first token here will always have `leading_trivia` set to `false`, as specified in
    /// §6.10.3p7.
    pub fn tokens(&self) -> &[PpToken] {
        &self.tokens
    }

    /// Returns the range covered by this replacement list's tokens, or `None` if it is empty.
    pub fn spelling_range(&self) -> Option<SourceRange> {
        self.tokens.first().map(|first| {
            let last = self.tokens.last().unwrap();
            SourceRange::new(
                first.range().start(),
                last.range().end().offset_from(first.range().start()),
            )
        })
    }

    /// Determines whether this replacement list is identical to `rhs` using the rules laid out in
    /// §6.10.3p1 (same tokens and whitespace separation).
    ///
    /// This is used when checking for macro redefinitions.
    pub fn is_identical_to(&self, rhs: &ReplacementList) -> bool {
        let translate = |ppt: &PpToken| (ppt.data(), ppt.leading_trivia);

        self.tokens
            .iter()
            .map(translate)
            .eq(rhs.tokens.iter().map(translate))
    }
}

/// The data associated with a macro definition.
#[derive(Debug, Clone)]
pub enum MacroDefKind {
    Object(ReplacementList),
    Function {
        params: Vec<Symbol>,
        replacement: ReplacementList,
    },
}

impl MacroDefKind {
    /// Determines whether this definition is identical to `rhs` using the rules laid out in
    /// §6.10.3p2.
    pub fn is_identical_to(&self, rhs: &MacroDefKind) -> bool {
        match (self, rhs) {
            (MacroDefKind::Object(lhs), MacroDefKind::Object(rhs)) => lhs.is_identical_to(rhs),
            (
                MacroDefKind::Function {
                    params: lhs_params,
                    replacement: lhs_replacement,
                },
                MacroDefKind::Function {
                    params: rhs_params,
                    replacement: rhs_replacement,
                },
            ) => lhs_params == rhs_params && lhs_replacement.is_identical_to(rhs_replacement),
            _ => false,
        }
    }
}

/// Represents a macro definition read from the source code.
#[derive(Debug, Clone)]
pub struct MacroDef {
    /// The name of the macro and its location in the source code.
    pub name_tok: Token<Symbol>,

    /// The data associated with this definition.
    pub kind: MacroDefKind,
}

/// Holds a table of currently defined macros.
pub struct MacroTable {
    map: FxHashMap<Symbol, MacroDef>,
}

impl MacroTable {
    /// Creates a new table with no definitions.
    pub fn new() -> Self {
        Self {
            map: Default::default(),
        }
    }

    /// Adds `def` to the table.
    ///
    /// If `def` redefines an existing macro (using the rules in §6.10.3p2), the previous definition
    /// is returned.
    pub fn define(&mut self, def: MacroDef) -> Option<MacroDef> {
        match self.map.entry(def.name_tok.data) {
            Entry::Occupied(ent) => {
                let prev = ent.into_mut();
                let identical = prev.kind.is_identical_to(&def.kind);

                // The standard allows redefinition iff the replacement lists are identical - always
                // redefine here to try to make things more accurate later, but report the previous
                // definition if it is not identical.
                Some(mem::replace(prev, def)).filter(|_| !identical)
            }

            Entry::Vacant(ent) => {
                ent.insert(def);
                None
            }
        }
    }

    /// Removes any stored definition associated with `name`.
    ///
    /// This has no effect if `name` is not defined.
    pub fn undef(&mut self, name: Symbol) {
        self.map.remove(&name);
    }

    /// Looks up the definition assoicated with `name`.
    pub fn lookup(&self, name: Symbol) -> Option<&MacroDef> {
        self.map.get(&name)
    }
}
