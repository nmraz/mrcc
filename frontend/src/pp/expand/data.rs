use std::collections::hash_map::Entry;
use std::mem;

use rustc_hash::FxHashMap;

use crate::lex::{Symbol, Token};
use crate::pp::PpToken;

#[derive(Debug, Clone)]
pub struct ReplacementList {
    tokens: Vec<PpToken>,
}

impl ReplacementList {
    pub fn new(mut tokens: Vec<PpToken>) -> Self {
        if let Some(first) = tokens.first_mut() {
            first.leading_trivia = false;
        }

        for ppt in &mut tokens {
            ppt.line_start = false;
        }

        Self { tokens }
    }

    pub fn tokens(&self) -> &[PpToken] {
        &self.tokens
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
pub enum MacroDefInfo {
    Object(ReplacementList),
    Function {
        args: Vec<Symbol>,
        replacement: ReplacementList,
    },
}

impl MacroDefInfo {
    pub fn is_identical_to(&self, rhs: &MacroDefInfo) -> bool {
        match (self, rhs) {
            (MacroDefInfo::Object(lhs), MacroDefInfo::Object(rhs)) => lhs.is_identical_to(rhs),
            (
                MacroDefInfo::Function {
                    args: lhs_args,
                    replacement: lhs_replacement,
                },
                MacroDefInfo::Function {
                    args: rhs_args,
                    replacement: rhs_replacement,
                },
            ) => lhs_args == rhs_args && lhs_replacement.is_identical_to(rhs_replacement),
            _ => false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MacroDef {
    pub name_tok: Token<Symbol>,
    pub info: MacroDefInfo,
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
                let identical = prev.info.is_identical_to(&def.info);

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

    pub fn lookup_mut(&mut self, name: Symbol) -> Option<&mut MacroDef> {
        self.map.get_mut(&name)
    }
}
