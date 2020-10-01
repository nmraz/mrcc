use std::collections::hash_map::Entry;
use std::mem;

use rustc_hash::FxHashMap;

use mrcc_lex::{Symbol, Token};
use mrcc_source::SourceRange;

use crate::PpToken;

#[derive(Debug, Clone)]
pub struct ReplacementList {
    tokens: Vec<PpToken>,
}

impl ReplacementList {
    pub fn new(mut tokens: Vec<PpToken>) -> Self {
        if let Some(first) = tokens.first_mut() {
            first.leading_trivia = false;
        }

        Self { tokens }
    }

    pub fn tokens(&self) -> &[PpToken] {
        &self.tokens
    }

    pub fn spelling_range(&self) -> Option<SourceRange> {
        self.tokens.first().map(|first| {
            let last = self.tokens.last().unwrap();
            SourceRange::new(
                first.range().start(),
                last.range().end().offset_from(first.range().start()),
            )
        })
    }

    pub fn is_identical_to(&self, rhs: &ReplacementList) -> bool {
        let translate = |ppt: &PpToken| (ppt.data(), ppt.leading_trivia);

        self.tokens
            .iter()
            .map(translate)
            .eq(rhs.tokens.iter().map(translate))
    }
}

#[derive(Debug, Clone)]
pub enum MacroDefKind {
    Object(ReplacementList),
    Function {
        params: Vec<Symbol>,
        replacement: ReplacementList,
    },
}

impl MacroDefKind {
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

#[derive(Debug, Clone)]
pub struct MacroDef {
    pub name_tok: Token<Symbol>,
    pub kind: MacroDefKind,
}

pub struct MacroTable {
    map: FxHashMap<Symbol, MacroDef>,
}

impl MacroTable {
    pub fn new() -> Self {
        Self {
            map: Default::default(),
        }
    }

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

    pub fn undef(&mut self, name: Symbol) {
        self.map.remove(&name);
    }

    pub fn lookup(&self, name: Symbol) -> Option<&MacroDef> {
        self.map.get(&name)
    }
}
