use std::collections::hash_map::Entry;

use rustc_hash::FxHashMap;

use crate::lex::{Interner, LexCtx, Symbol, Token};

use super::PpToken;

pub struct State {
    pub known_idents: KnownIdents,
    pub macro_table: MacroTable,
}

impl State {
    pub fn new(ctx: &mut LexCtx<'_, '_>) -> Self {
        Self {
            known_idents: KnownIdents::new(&mut ctx.interner),
            macro_table: MacroTable::new(),
        }
    }
}

pub struct KnownIdents {
    pub dir_if: Symbol,
    pub dir_ifdef: Symbol,
    pub dir_ifndef: Symbol,
    pub dir_elif: Symbol,
    pub dir_else: Symbol,
    pub dir_endif: Symbol,
    pub dir_include: Symbol,
    pub dir_define: Symbol,
    pub dir_undef: Symbol,
    pub dir_error: Symbol,
}

impl KnownIdents {
    pub fn new(interner: &mut Interner) -> Self {
        Self {
            dir_if: interner.intern("if"),
            dir_ifdef: interner.intern("ifdef"),
            dir_ifndef: interner.intern("ifndef"),
            dir_elif: interner.intern("elif"),
            dir_else: interner.intern("else"),
            dir_endif: interner.intern("endif"),
            dir_include: interner.intern("include"),
            dir_define: interner.intern("define"),
            dir_undef: interner.intern("undef"),
            dir_error: interner.intern("error"),
        }
    }
}

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

    pub fn is_identical_to(&self, rhs: &ReplacementList) -> bool {
        let translate = |ppt: &PpToken| (ppt.kind(), ppt.leading_trivia);

        self.tokens
            .iter()
            .map(translate)
            .eq(rhs.tokens.iter().map(translate))
    }
}

pub enum MacroInfo {
    Object(ReplacementList),
    Function {
        args: Vec<Symbol>,
        replacement: ReplacementList,
    },
}

impl MacroInfo {
    pub fn is_identical_to(&self, rhs: &MacroInfo) -> bool {
        match (self, rhs) {
            (MacroInfo::Object(lhs), MacroInfo::Object(rhs)) => lhs.is_identical_to(rhs),
            (
                MacroInfo::Function {
                    args: lhs_args,
                    replacement: lhs_replacement,
                },
                MacroInfo::Function {
                    args: rhs_args,
                    replacement: rhs_replacement,
                },
            ) => lhs_args == rhs_args && lhs_replacement.is_identical_to(rhs_replacement),
            _ => false,
        }
    }
}

pub struct MacroDef {
    pub name_tok: Token<Symbol>,
    pub info: MacroInfo,
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

    pub fn lookup(&self, name: Symbol) -> Option<&MacroDef> {
        self.map.get(&name)
    }

    pub fn define(&mut self, def: MacroDef) -> Option<&MacroDef> {
        match self.map.entry(def.name_tok.data) {
            Entry::Occupied(ent) => {
                let prev = ent.into_mut();

                // The standard allows redefinition iff the replacement lists are identical.
                if prev.info.is_identical_to(&def.info) {
                    *prev = def;
                    None
                } else {
                    Some(prev)
                }
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
}
