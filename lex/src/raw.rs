use std::borrow::Cow;

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
    pub start: u32,
    pub len: u32,
    pub terminated: bool,
}

pub fn clean(tok: &str) -> String {
    tok.replace("\\\n", "")
}

fn is_line_ws(c: char) -> bool {
    match c {
        ' ' | '\t' | '\x0b' | '\x0c' => true,
        _ => false,
    }
}

#[derive(Clone)]
struct SkipEscapedNewlines<'a> {
    input: &'a str,
    pos: usize,
    tainted: bool,
}

impl<'a> SkipEscapedNewlines<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            input,
            pos: 0,
            tainted: false,
        }
    }

    pub fn input(&self) -> &'a str {
        self.input
    }

    pub fn remaining(&self) -> &'a str {
        &self.input[self.pos..]
    }

    pub fn pos(&self) -> usize {
        self.pos
    }

    pub fn tainted(&self) -> bool {
        self.tainted
    }

    pub fn untaint(&mut self) {
        self.tainted = false
    }
}

impl Iterator for SkipEscapedNewlines<'_> {
    type Item = char;

    #[inline]
    fn next(&mut self) -> Option<char> {
        while self.remaining().starts_with("\\\n") {
            self.tainted = true;
            self.pos += 2;
        }

        let next = self.remaining().chars().next();
        if let Some(c) = next {
            self.pos += c.len_utf8();
        }
        next
    }
}

#[derive(Clone)]
pub struct Reader<'a> {
    iter: SkipEscapedNewlines<'a>,
    start: usize,
}

impl<'a> Reader<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            iter: SkipEscapedNewlines::new(input),
            start: 0,
        }
    }

    pub fn pos(&self) -> usize {
        self.iter.pos()
    }

    pub fn start(&self) -> usize {
        self.start
    }

    pub fn cur_len(&self) -> usize {
        self.pos() - self.start
    }

    pub fn cur_str_raw(&self) -> &'a str {
        &self.iter.input()[self.start..self.pos()]
    }

    pub fn cur_str_cleaned(&self) -> Cow<'_, str> {
        let raw = self.cur_str_raw();

        if self.iter.tainted() {
            Cow::Owned(clean(raw))
        } else {
            Cow::Borrowed(raw)
        }
    }

    pub fn bump(&mut self) -> Option<char> {
        self.iter.next()
    }

    pub fn begin_tok(&mut self) {
        self.start = self.pos();
        self.iter.untaint();
    }

    pub fn eat(&mut self, c: char) -> bool {
        self.eat_if(|cur| cur == c)
    }

    pub fn eat_if(&mut self, mut pred: impl FnMut(char) -> bool) -> bool {
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
        let mut iter = self.iter.clone();
        for c in s.chars() {
            if iter.next() != Some(c) {
                return false;
            }
        }
        self.iter = iter;
        true
    }

    pub fn eat_line_ws(&mut self) -> bool {
        self.eat_while(is_line_ws) > 0
    }

    fn tok(&self, kind: RawTokenKind, terminated: bool) -> RawToken {
        RawToken {
            kind,
            start: self.start as u32,
            len: self.cur_len() as u32,
            terminated,
        }
    }

    fn real_tok(&self, kind: TokenKind) -> RawToken {
        self.tok(RawTokenKind::Real(kind), true)
    }

    pub fn next_token(&mut self, interner: &mut IdentInterner) -> RawToken {
        self.begin_tok();

        let c = match self.bump() {
            None => return self.real_tok(TokenKind::Eof),
            Some(c) => c,
        };

        match c {
            ws if is_line_ws(ws) => {
                self.eat_line_ws();
                self.tok(RawTokenKind::Ws, true)
            }
            '\n' => self.tok(RawTokenKind::Newline, true),
            c => self.handle_punct(c),
        }
    }

    fn handle_punct(&mut self, c: char) -> RawToken {
        use TokenKind::*;

        match c {
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
