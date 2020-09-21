use mrcc_lex::Symbol;

use data::MacroTable;
pub use data::{MacroDef, MacroDefInfo, ReplacementList};

mod data;

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
}
