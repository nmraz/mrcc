use crate::lex::{Symbol, Token, TokenKind};
use crate::pp::PpToken;

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

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
pub struct MacroDef {
    pub name_tok: Token<Symbol>,
    pub info: MacroInfo,
}
