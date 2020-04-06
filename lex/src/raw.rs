use std::iter::Peekable;
use std::str::Chars;

use crate::TokenKind;
use crate::{IdentInterner, IdentSym};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RawTokenKind {
    Real(TokenKind),
    Ws,
    Newline,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RawToken {
    pub kind: RawTokenKind,
    pub len: u32,
    pub terminated: bool,
}

pub fn clean(tok: &str) -> String {
    tok.replace("\\\n", "")
}

#[derive(Clone)]
pub struct Reader<'a> {
    input: &'a str,
    iter: Chars<'a>,
}

impl<'a> Reader<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            input,
            iter: input.chars(),
        }
    }

    pub fn cur_len(&self) -> usize {
        self.input.len() - self.iter.as_str().len()
    }

    pub fn cur_str_raw(&self) -> &'a str {
        &self.input[..self.cur_len()]
    }

    pub fn cur_str_cleaned(&self) -> String {
        clean(self.cur_str_raw())
    }

    fn eat_escaped_newlines(&mut self) {
        fn consume_escaped(iter: &mut Chars<'_>) -> bool {
            iter.next() == Some('\\') && iter.next() == Some('\n')
        }

        let mut iter = self.iter.clone();
        while consume_escaped(&mut iter) {
            self.iter = iter.clone();
        }
    }

    pub fn peek(&mut self) -> Option<char> {
        self.eat_escaped_newlines();
        self.iter.clone().next()
    }

    pub fn bump(&mut self) -> Option<char> {
        self.eat_escaped_newlines();
        self.iter.next()
    }

    pub fn finish_cur(&mut self) {
        self.input = self.iter.as_str();
    }

    pub fn eat(&mut self, c: char) -> bool {
        self.eat_if(|cur| cur == c)
    }

    pub fn eat_if(&mut self, mut pred: impl FnMut(char) -> bool) -> bool {
        self.eat_escaped_newlines();
        let mut iter = self.iter.clone();
        if iter.next().map_or(false, &mut pred) {
            self.iter = iter;
            return true;
        }
        false
    }

    pub fn eat_while(&mut self, mut pred: impl FnMut(char) -> bool) -> u32 {
        let mut eaten = 0;
        while self.eat_if(&mut pred) {
            eaten += 1;
        }
        eaten
    }

    pub fn eat_str(&mut self, s: &str) -> bool {
        let mut tmp = self.clone();
        for c in s.chars() {
            if !tmp.eat(c) {
                return false;
            }
        }
        *self = tmp;
        true
    }

    pub fn eat_line_ws(&mut self) -> bool {
        self.eat_while(|c| c.is_ascii_whitespace() && c != '\n') > 0
    }

    fn tok(&mut self, kind: RawTokenKind, terminated: bool) -> RawToken {
        let len = self.cur_len() as u32;
        self.finish_cur();

        RawToken {
            kind,
            len,
            terminated,
        }
    }

    fn real_tok(&mut self, kind: TokenKind) -> RawToken {
        self.tok(RawTokenKind::Real(kind), true)
    }

    pub fn next_token(&mut self, interner: &mut IdentInterner) -> RawToken {
        use TokenKind::*;

        let c = match self.bump() {
            None => return self.real_tok(Eof),
            Some(c) => c,
        };

        match c {
            '#' => {
                if self.eat('#') {
                    self.real_tok(HashHash)
                } else {
                    self.real_tok(Hash)
                }
            }
            ',' => self.real_tok(Comma),
            ':' => self.real_tok(Colon),
            ';' => self.real_tok(Semi),
            '[' => self.real_tok(LSquare),
            ']' => self.real_tok(RSquare),
            '(' => self.real_tok(LParen),
            ')' => self.real_tok(RParen),
            '+' => {
                if self.eat('+') {
                    self.real_tok(PlusPlus)
                } else if self.eat('=') {
                    self.real_tok(PlusEq)
                } else {
                    self.real_tok(Plus)
                }
            }
            '-' => {
                if self.eat('-') {
                    self.real_tok(MinusMinus)
                } else if self.eat('=') {
                    self.real_tok(MinusEq)
                } else if self.eat('>') {
                    self.real_tok(Arrow)
                } else {
                    self.real_tok(Minus)
                }
            }
            _ => self.real_tok(Unknown),
        }
    }
}
