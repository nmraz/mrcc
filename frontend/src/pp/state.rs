use std::collections::hash_map::Entry;
use std::mem;

use rustc_hash::FxHashMap;

use crate::lex::{Interner, LexCtx, Symbol, Token, TokenKind};

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
    tokens: Vec<MacroToken>,
}

impl ReplacementList {
    pub fn new(tokens: impl IntoIterator<Item = PpToken>) -> Self {
        let mut tokens: Vec<MacroToken> = tokens.into_iter().map(|ppt| ppt.into()).collect();
        if let Some(first) = tokens.first_mut() {
            first.data.leading_trivia = false;
        }

        Self { tokens }
    }

    pub fn tokens(&self) -> impl Iterator<Item = PpToken> + '_ {
        self.tokens.iter().copied().map(|tok| tok.into())
    }

    pub fn is_identical_to(&self, rhs: &ReplacementList) -> bool {
        self.tokens
            .iter()
            .map(|tok| tok.data)
            .eq(rhs.tokens.iter().map(|tok| tok.data))
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
struct MacroTokenData {
    kind: TokenKind,
    leading_trivia: bool,
}

type MacroToken = Token<MacroTokenData>;

impl From<PpToken> for MacroToken {
    fn from(ppt: PpToken) -> Self {
        Self {
            data: MacroTokenData {
                kind: ppt.data(),
                leading_trivia: ppt.leading_trivia,
            },
            range: ppt.range(),
        }
    }
}

impl From<MacroToken> for PpToken {
    fn from(macro_tok: MacroToken) -> Self {
        let leading_trivia = macro_tok.data.leading_trivia;

        PpToken {
            tok: macro_tok.map(|data| data.kind),
            line_start: false,
            leading_trivia,
        }
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
}
