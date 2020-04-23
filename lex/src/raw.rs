use std::borrow::Cow;

use crate::{CommentKind, PunctKind, TokenKind};
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

    fn punct(&self, kind: PunctKind) -> RawToken {
        self.real_tok(TokenKind::Punct(kind))
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
        use PunctKind::*;

        match c {
            ',' => self.punct(Comma),
            ':' => self.punct(Colon),
            ';' => self.punct(Semi),
            '[' => self.punct(LSquare),
            ']' => self.punct(RSquare),
            '(' => self.punct(LParen),
            ')' => self.punct(RParen),
            '~' => self.punct(Tilde),
            '?' => self.punct(Question),
            '#' => {
                if self.eat('#') {
                    self.punct(HashHash)
                } else {
                    self.punct(Hash)
                }
            }
            '+' => {
                if self.eat('+') {
                    self.punct(PlusPlus)
                } else if self.eat('=') {
                    self.punct(PlusEq)
                } else {
                    self.punct(Plus)
                }
            }
            '-' => {
                if self.eat('-') {
                    self.punct(MinusMinus)
                } else if self.eat('=') {
                    self.punct(MinusEq)
                } else if self.eat('>') {
                    self.punct(Arrow)
                } else {
                    self.punct(Minus)
                }
            }
            '*' => {
                if self.eat('=') {
                    self.punct(StarEq)
                } else {
                    self.punct(Star)
                }
            }
            '/' => {
                if self.eat('/') {
                    self.eat_while(|c| c != '\n');
                    self.real_tok(TokenKind::Comment(CommentKind::Line))
                } else if self.eat('*') {
                    self.handle_block_comment()
                } else if self.eat('=') {
                    self.punct(SlashEq)
                } else {
                    self.punct(Slash)
                }
            }
            '%' => {
                if self.eat(':') {
                    if self.eat_str("%:") {
                        self.punct(HashHash)
                    } else {
                        self.punct(Hash)
                    }
                } else if self.eat('=') {
                    self.punct(PercEq)
                } else {
                    self.punct(Perc)
                }
            }
            '&' => {
                if self.eat('&') {
                    self.punct(AmpAmp)
                } else if self.eat('=') {
                    self.punct(AmpEq)
                } else {
                    self.punct(Amp)
                }
            }
            '|' => {
                if self.eat('|') {
                    self.punct(PipePipe)
                } else if self.eat('=') {
                    self.punct(PipeEq)
                } else {
                    self.punct(Pipe)
                }
            }
            '^' => {
                if self.eat('=') {
                    self.punct(CaretEq)
                } else {
                    self.punct(Caret)
                }
            }
            '!' => {
                if self.eat('=') {
                    self.punct(ExclEq)
                } else {
                    self.punct(Excl)
                }
            }
            '<' => {
                if self.eat(':') {
                    self.punct(LSquare)
                } else if self.eat('%') {
                    self.punct(LCurly)
                } else if self.eat('<') {
                    if self.eat('=') {
                        self.punct(LessLessEq)
                    } else {
                        self.punct(LessLess)
                    }
                } else if self.eat('=') {
                    self.punct(LessEq)
                } else {
                    self.punct(Less)
                }
            }
            '>' => {
                if self.eat(':') {
                    self.punct(RSquare)
                } else if self.eat('%') {
                    self.punct(RCurly)
                } else if self.eat('>') {
                    if self.eat('=') {
                        self.punct(GreaterGreaterEq)
                    } else {
                        self.punct(GreaterGreater)
                    }
                } else if self.eat('=') {
                    self.punct(GreaterEq)
                } else {
                    self.punct(Greater)
                }
            }
            '=' => {
                if self.eat('=') {
                    self.punct(EqEq)
                } else {
                    self.punct(Eq)
                }
            }
            _ => self.real_tok(TokenKind::Unknown),
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
