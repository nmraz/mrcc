use std::iter::Peekable;
use std::str::Chars;

use crate::{CommentKind, TokenKind};
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

        if self.eat_line_ws() {
            return self.tok(RawTokenKind::Ws, true);
        }

        let c = match self.bump() {
            None => return self.real_tok(Eof),
            Some(c) => c,
        };

        match c {
            '\n' => self.tok(RawTokenKind::Newline, true),
            ',' => self.real_tok(Comma),
            ':' => self.real_tok(Colon),
            ';' => self.real_tok(Semi),
            '[' => self.real_tok(LSquare),
            ']' => self.real_tok(RSquare),
            '(' => self.real_tok(LParen),
            ')' => self.real_tok(RParen),
            '~' => self.real_tok(Tilde),
            '?' => self.real_tok(Question),
            '#' => {
                if self.eat('#') {
                    self.real_tok(HashHash)
                } else {
                    self.real_tok(Hash)
                }
            }
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
            '*' => {
                if self.eat('=') {
                    self.real_tok(StarEq)
                } else {
                    self.real_tok(Star)
                }
            }
            '/' => {
                if self.eat('/') {
                    self.eat_while(|c| c != '\n');
                    self.real_tok(Comment(CommentKind::Line))
                } else if self.eat('*') {
                    self.handle_block_comment()
                } else if self.eat('=') {
                    self.real_tok(SlashEq)
                } else {
                    self.real_tok(Slash)
                }
            }
            '%' => {
                if self.eat(':') {
                    if self.eat_str("%:") {
                        self.real_tok(HashHash)
                    } else {
                        self.real_tok(Hash)
                    }
                } else if self.eat('=') {
                    self.real_tok(PercEq)
                } else {
                    self.real_tok(Perc)
                }
            }
            '&' => {
                if self.eat('&') {
                    self.real_tok(AmpAmp)
                } else if self.eat('=') {
                    self.real_tok(AmpEq)
                } else {
                    self.real_tok(Amp)
                }
            }
            '|' => {
                if self.eat('|') {
                    self.real_tok(PipePipe)
                } else if self.eat('=') {
                    self.real_tok(PipeEq)
                } else {
                    self.real_tok(Pipe)
                }
            }
            '^' => {
                if self.eat('=') {
                    self.real_tok(CaretEq)
                } else {
                    self.real_tok(Caret)
                }
            }
            '!' => {
                if self.eat('=') {
                    self.real_tok(ExclEq)
                } else {
                    self.real_tok(Excl)
                }
            }
            '<' => {
                if self.eat(':') {
                    self.real_tok(LSquare)
                } else if self.eat('%') {
                    self.real_tok(LCurly)
                } else if self.eat('<') {
                    if self.eat('=') {
                        self.real_tok(LessLessEq)
                    } else {
                        self.real_tok(LessLess)
                    }
                } else if self.eat('=') {
                    self.real_tok(LessEq)
                } else {
                    self.real_tok(Less)
                }
            }
            '>' => {
                if self.eat(':') {
                    self.real_tok(RSquare)
                } else if self.eat('%') {
                    self.real_tok(RCurly)
                } else if self.eat('>') {
                    if self.eat('=') {
                        self.real_tok(GreaterGreaterEq)
                    } else {
                        self.real_tok(GreaterGreater)
                    }
                } else if self.eat('=') {
                    self.real_tok(GreaterEq)
                } else {
                    self.real_tok(Greater)
                }
            }
            '=' => {
                if self.eat('=') {
                    self.real_tok(EqEq)
                } else {
                    self.real_tok(Eq)
                }
            }
            _ => self.real_tok(Unknown),
        }
    }

    fn handle_block_comment(&mut self) -> RawToken {
        let terminated = loop {
            self.eat_while(|c| c != '*');
            match self.bump() {
                None => break false,
                Some('/') => break true,
                _ => {}
            }
        };

        self.tok(
            RawTokenKind::Real(TokenKind::Comment(CommentKind::Block)),
            terminated,
        )
    }
}
