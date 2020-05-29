use std::borrow::Cow;

use crate::{CommentKind, PunctKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RawTokenKind {
    Unknown,
    Eof,

    Ws,
    Newline,
    Comment(CommentKind),

    Punct(PunctKind),

    Ident,
    Number,
    Str,
    Char,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RawContent<'a> {
    pub start: u32,
    pub str: &'a str,
    pub tainted: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RawToken<'a> {
    pub kind: RawTokenKind,
    pub content: RawContent<'a>,
    pub terminated: bool,
}

impl<'a> RawContent<'a> {
    pub fn cleaned_str(&self) -> Cow<'a, str> {
        if self.tainted {
            Cow::Owned(clean(self.str))
        } else {
            Cow::Borrowed(self.str)
        }
    }
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

fn is_ident_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_'
}

fn is_ident_continue(c: char) -> bool {
    is_ident_start(c) || c.is_ascii_digit()
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
    #[inline]
    pub fn new(input: &'a str) -> Self {
        Self {
            iter: SkipEscapedNewlines::new(input),
            start: 0,
        }
    }

    #[inline]
    pub fn pos(&self) -> usize {
        self.iter.pos()
    }

    #[inline]
    pub fn cur_content(&self) -> RawContent<'a> {
        RawContent {
            start: self.start as u32,
            str: &self.iter.input()[self.start..self.pos()],
            tainted: self.iter.tainted(),
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

    pub fn eat_to_after(&mut self, term: char) -> bool {
        while let Some(c) = self.bump() {
            if c == term {
                return true;
            }
        }

        false
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
}

pub struct Tokenizer<'a> {
    pub reader: Reader<'a>,
}

impl<'a> Tokenizer<'a> {
    #[inline]
    pub fn new(input: &'a str) -> Self {
        Self {
            reader: Reader::new(input),
        }
    }

    fn tok(&self, kind: RawTokenKind, terminated: bool) -> RawToken<'a> {
        RawToken {
            kind,
            content: self.reader.cur_content(),
            terminated,
        }
    }

    fn tok_term(&self, kind: RawTokenKind) -> RawToken<'a> {
        self.tok(kind, true)
    }

    fn punct(&self, kind: PunctKind) -> RawToken<'a> {
        self.tok_term(RawTokenKind::Punct(kind))
    }

    pub fn next_token(&mut self) -> RawToken<'a> {
        self.reader.begin_tok();

        let c = match self.reader.bump() {
            None => return self.tok_term(RawTokenKind::Eof),
            Some(c) => c,
        };

        match c {
            ws if is_line_ws(ws) => {
                self.reader.eat_line_ws();
                self.tok_term(RawTokenKind::Ws)
            }
            '\n' => self.tok_term(RawTokenKind::Newline),

            'U' | 'L' => self.handle_encoding_prefix(true),
            'u' => {
                let allow_char = !self.reader.eat('8');
                self.handle_encoding_prefix(allow_char)
            }

            '"' => self.handle_str_like('"', RawTokenKind::Str),
            '\'' => self.handle_str_like('\'', RawTokenKind::Char),

            '.' => {
                if self.reader.eat_if(|c| c.is_ascii_digit()) {
                    self.handle_number()
                } else if self.reader.eat_str("..") {
                    self.punct(PunctKind::Ellipsis)
                } else {
                    self.punct(PunctKind::Dot)
                }
            }

            c if is_ident_start(c) => self.handle_ident(),
            d if d.is_ascii_digit() => self.handle_number(),

            c => self.handle_punct(c),
        }
    }

    fn handle_ident(&mut self) -> RawToken<'a> {
        self.reader.eat_while(is_ident_continue);
        self.tok_term(RawTokenKind::Ident)
    }

    fn handle_number(&mut self) -> RawToken<'a> {
        while self.eat_number_char() {}
        self.tok_term(RawTokenKind::Number)
    }

    fn eat_number_char(&mut self) -> bool {
        // If any of these characters are followed by a sign, they designate an exponent. Otherwise,
        // they are a part of the pp-number anyway.
        if self.reader.eat_if(|c| ['e', 'E', 'p', 'P'].contains(&c)) {
            self.reader.eat_if(|c| c == '+' || c == '-');
            return true;
        }

        self.reader.eat_if(|c| c == '.' || is_ident_continue(c))
    }

    fn handle_encoding_prefix(&mut self, allow_char: bool) -> RawToken<'a> {
        if self.reader.eat('"') {
            self.handle_str_like('"', RawTokenKind::Str)
        } else if allow_char && self.reader.eat('\'') {
            self.handle_str_like('\'', RawTokenKind::Char)
        } else {
            self.handle_ident()
        }
    }

    fn handle_str_like(&mut self, term: char, kind: RawTokenKind) -> RawToken<'a> {
        let mut escaped = false;

        while let Some(c) = self.reader.bump() {
            match c {
                '\\' => escaped = !escaped,
                '\n' => break,
                c if c == term && !escaped => return self.tok_term(kind),
                _ => {}
            }
        }

        self.tok(kind, false)
    }

    fn handle_punct(&mut self, c: char) -> RawToken<'a> {
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
                if self.reader.eat('#') {
                    self.punct(HashHash)
                } else {
                    self.punct(Hash)
                }
            }
            '+' => {
                if self.reader.eat('+') {
                    self.punct(PlusPlus)
                } else if self.reader.eat('=') {
                    self.punct(PlusEq)
                } else {
                    self.punct(Plus)
                }
            }
            '-' => {
                if self.reader.eat('-') {
                    self.punct(MinusMinus)
                } else if self.reader.eat('=') {
                    self.punct(MinusEq)
                } else if self.reader.eat('>') {
                    self.punct(Arrow)
                } else {
                    self.punct(Minus)
                }
            }
            '*' => {
                if self.reader.eat('=') {
                    self.punct(StarEq)
                } else {
                    self.punct(Star)
                }
            }
            '/' => {
                if self.reader.eat('/') {
                    self.handle_line_comment()
                } else if self.reader.eat('*') {
                    self.handle_block_comment()
                } else if self.reader.eat('=') {
                    self.punct(SlashEq)
                } else {
                    self.punct(Slash)
                }
            }
            '%' => {
                if self.reader.eat(':') {
                    if self.reader.eat_str("%:") {
                        self.punct(HashHash)
                    } else {
                        self.punct(Hash)
                    }
                } else if self.reader.eat('=') {
                    self.punct(PercEq)
                } else {
                    self.punct(Perc)
                }
            }
            '&' => {
                if self.reader.eat('&') {
                    self.punct(AmpAmp)
                } else if self.reader.eat('=') {
                    self.punct(AmpEq)
                } else {
                    self.punct(Amp)
                }
            }
            '|' => {
                if self.reader.eat('|') {
                    self.punct(PipePipe)
                } else if self.reader.eat('=') {
                    self.punct(PipeEq)
                } else {
                    self.punct(Pipe)
                }
            }
            '^' => {
                if self.reader.eat('=') {
                    self.punct(CaretEq)
                } else {
                    self.punct(Caret)
                }
            }
            '!' => {
                if self.reader.eat('=') {
                    self.punct(ExclEq)
                } else {
                    self.punct(Excl)
                }
            }
            '<' => {
                if self.reader.eat(':') {
                    self.punct(LSquare)
                } else if self.reader.eat('%') {
                    self.punct(LCurly)
                } else if self.reader.eat('<') {
                    if self.reader.eat('=') {
                        self.punct(LessLessEq)
                    } else {
                        self.punct(LessLess)
                    }
                } else if self.reader.eat('=') {
                    self.punct(LessEq)
                } else {
                    self.punct(Less)
                }
            }
            '>' => {
                if self.reader.eat(':') {
                    self.punct(RSquare)
                } else if self.reader.eat('%') {
                    self.punct(RCurly)
                } else if self.reader.eat('>') {
                    if self.reader.eat('=') {
                        self.punct(GreaterGreaterEq)
                    } else {
                        self.punct(GreaterGreater)
                    }
                } else if self.reader.eat('=') {
                    self.punct(GreaterEq)
                } else {
                    self.punct(Greater)
                }
            }
            '=' => {
                if self.reader.eat('=') {
                    self.punct(EqEq)
                } else {
                    self.punct(Eq)
                }
            }
            _ => self.tok_term(RawTokenKind::Unknown),
        }
    }

    fn handle_line_comment(&mut self) -> RawToken<'a> {
        // Note: we intentionally don't consume the newline - it will be emitted as a separate
        // newline token.
        self.reader.eat_while(|c| c != '\n');
        self.tok_term(RawTokenKind::Comment(CommentKind::Line))
    }

    fn handle_block_comment(&mut self) -> RawToken<'a> {
        let terminated = loop {
            self.reader.eat_to_after('*');
            match self.reader.bump() {
                None => break false,
                Some('/') => break true,
                _ => {}
            }
        };

        self.tok(RawTokenKind::Comment(CommentKind::Block), terminated)
    }
}