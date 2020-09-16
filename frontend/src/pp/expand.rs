use std::collections::hash_map::Entry;
use std::mem;

use rustc_hash::FxHashMap;

use crate::lex::Symbol;

pub use data::{MacroDef, MacroInfo, ReplacementList};

mod data;

pub struct MacroState {
    definitions: FxHashMap<Symbol, MacroDef>,
}

impl MacroState {
    pub fn new() -> Self {
        Self {
            definitions: Default::default(),
        }
    }

    pub fn define(&mut self, def: MacroDef) -> Option<MacroDef> {
        match self.definitions.entry(def.name_tok.data) {
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
        self.definitions.remove(&name);
    }
}
