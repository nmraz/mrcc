//! Raw tokens and tokenization.
//!
//! Raw tokens differ from "ordinary" tokens in that they are lossless and point back into the
//! original source string. Lexing them requires no auxiliary state (such as interners) and can
//! never fail. To validate  a raw token and convert it to a "real" token, use
//! [`convert_raw()`](../fn.convert_raw.html).

use std::borrow::Cow;
use std::convert::TryFrom;

use mrcc_source::{LocalOff, LocalRange};

use super::PunctKind;

/// Enum representing raw token types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RawTokenKind {
    Unknown,

    Eof,
    Newline,

    Ws,
    LineComment,

    BlockComment {
        terminated: bool,
    },

    Punct(PunctKind),
    Ident,

    /// A preprocessing number. Note that the definition of preprocessing numbers is rather lax and
    /// matches many invalid numeric literals as well. See §6.4.8 for details.
    Number,

    Str {
        terminated: bool,
    },
    Char {
        terminated: bool,
    },
}

/// A slice of the actual source string.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RawContent<'a> {
    /// The offset within the string at which the slice starts.
    pub off: LocalOff,
    /// The relevant slice of the source string.
    pub str: &'a str,
    /// Indicates whether the slice contains escaped newlines that should be deleted before use,
    /// as per translation phase 2.
    pub tainted: bool,
}

impl<'a> RawContent<'a> {
    /// Returns the string corresponding to this slice with escaped newlines deleted.
    pub fn cleaned_str(&self) -> Cow<'a, str> {
        if self.tainted {
            Cow::Owned(clean(self.str))
        } else {
            Cow::Borrowed(self.str)
        }
    }
}

/// Represents a raw token lexed from a string.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RawToken<'a> {
    /// The type of token.
    pub kind: RawTokenKind,
    /// The source contents of the token.
    pub content: RawContent<'a>,
}

/// Deletes escaped newlines (`\` immediately followed by a newline) from `tok`, as specified in
/// translation phase 2 (§5.1.1.2).
pub fn clean(tok: &str) -> String {
    tok.replace("\\\n", "")
}

/// Checks whether `c` is a non-newline whitespace character, as per §6.4.
fn is_line_ws(c: char) -> bool {
    [' ', '\t', '\x0b', '\x0c'].contains(&c)
}

/// Checks whether `c` is the start of an identifier (identifier-nondigit), as per §6.4.2.1.
fn is_ident_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_'
}

/// Checks whether `c` is an identifier continuation character, as per §6.4.2.1.
fn is_ident_continue(c: char) -> bool {
    is_ident_start(c) || c.is_ascii_digit()
}

/// An iterator through the characters of a string that skips escaped newlines within it.
///
/// The iterator also tracks whether it has seen any escaped newlines and become "tainted".
#[derive(Clone)]
struct SkipEscapedNewlines<'a> {
    input: &'a str,
    off: LocalOff,
    tainted: bool,
}

impl<'a> SkipEscapedNewlines<'a> {
    /// Creates a new iterator with the specified input string.
    pub fn new(input: &'a str) -> Self {
        Self {
            input,
            off: 0.into(),
            tainted: false,
        }
    }

    /// Returns the entire input string.
    pub fn input(&self) -> &'a str {
        self.input
    }

    /// Returns the portion of the input not yet iterated through.
    pub fn remaining(&self) -> &'a str {
        &self.input[self.off.into()..]
    }

    /// Returns the current offset within the input string.
    pub fn off(&self) -> LocalOff {
        self.off
    }

    /// Returns whether any escaped newlines have been encountered since the last call to
    /// `untaint()`.
    pub fn tainted(&self) -> bool {
        self.tainted
    }

    /// Resets the "tainted" flag to `false`.
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
            self.off += LocalOff::from(2);
        }

        let next = self.remaining().chars().next();
        if let Some(c) = next {
            self.off += LocalOff::try_from(c.len_utf8()).unwrap();
        }
        next
    }
}

/// A utility for reading content from a source string.
///
/// `Reader` also implements translation phase 2 (§5.1.1.2) and transparently skips any `\`
/// characters immediately followed by a newline in the source.
#[derive(Clone)]
pub struct Reader<'a> {
    /// The underlying character iterator.
    iter: SkipEscapedNewlines<'a>,
    /// The start of the current token being read.
    start: LocalOff,
}

impl<'a> Reader<'a> {
    /// Creates a new reader with the specified source string.
    #[inline]
    pub fn new(input: &'a str) -> Self {
        Self {
            iter: SkipEscapedNewlines::new(input),
            start: 0.into(),
        }
    }

    /// Returns the current offset of this reader within the source.
    #[inline]
    pub fn off(&self) -> LocalOff {
        self.iter.off()
    }

    /// Returns the current content of this reader.
    ///
    /// The current content consists of all characters consumed since the last call to
    /// [`begin_tok()`](#method.begin_tok).
    #[inline]
    pub fn cur_content(&self) -> RawContent<'a> {
        RawContent {
            off: self.start,
            str: &self.iter.input()[LocalRange::new(self.start, self.off())],
            tainted: self.iter.tainted(),
        }
    }

    /// Consumes and returns the next character from the source string.
    pub fn bump(&mut self) -> Option<char> {
        self.iter.next()
    }

    /// Consumes and returns the next character from the source if `pred` evaluates to `true` on it.
    pub fn bump_if(&mut self, mut pred: impl FnMut(char) -> bool) -> Option<char> {
        let mut iter = self.iter.clone();
        let c = iter.next();
        if c.map_or(false, &mut pred) {
            self.iter = iter;
            return c;
        }
        None
    }

    /// Marks the current offset as the start of a new token.
    ///
    /// Subsequent calls to [`cur_content()`](#method.cur_content) will return all characters
    /// consumed since this point.
    pub fn begin_tok(&mut self) {
        self.start = self.off();
        self.iter.untaint();
    }

    /// Consumes the next character from the source if it is exactly `c`.
    ///
    /// Returns whether a character was consumed.
    pub fn eat(&mut self, c: char) -> bool {
        self.eat_if(|cur| cur == c)
    }

    /// Consumes the next character from the source if `pred` evaluates to `true` on it.
    ///
    /// Returns whether a character was consumed.
    pub fn eat_if(&mut self, pred: impl FnMut(char) -> bool) -> bool {
        self.bump_if(pred).is_some()
    }

    /// Consumes characters from the source as long as `pred` evaluates to `true` on them.
    ///
    /// Returns the number of characters consumed (excluding escaped newlines).
    pub fn eat_while(&mut self, mut pred: impl FnMut(char) -> bool) -> u32 {
        let mut eaten = 0;
        while self.eat_if(&mut pred) {
            eaten += 1;
        }
        eaten
    }

    /// Consumes characters until just after the next occurrence of `term`.
    ///
    /// Returns `true` if `term` was found and consumed, and `false` if the end of the source
    /// was reached without seeing `term`.
    pub fn eat_to_after(&mut self, term: char) -> bool {
        while let Some(c) = self.bump() {
            if c == term {
                return true;
            }
        }

        false
    }

    /// Consumes the next `s.len()` characters if they match `s` exactly (ignoring escaped
    /// newlines).
    ///
    /// Returns `true` if there was a match, `false` otherwise.
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

    /// Consumes characters from the source as long as they are non-newline whitespace characters
    /// (using the definition of "whitespace" given in §6.4).
    pub fn eat_line_ws(&mut self) -> bool {
        self.eat_while(is_line_ws) > 0
    }
}

/// Reads raw tokens out of a string.
pub struct Tokenizer<'a> {
    /// The underlying reader used to tokenize the string.
    pub reader: Reader<'a>,
}

impl<'a> Tokenizer<'a> {
    /// Creates a new tokenizer with the specified source string.
    #[inline]
    pub fn new(input: &'a str) -> Self {
        Self {
            reader: Reader::new(input),
        }
    }

    /// Reads the next token using `self.reader`.
    pub fn next_token(&mut self) -> RawToken<'a> {
        self.reader.begin_tok();

        let c = match self.reader.bump() {
            None => return self.tok(RawTokenKind::Eof),
            Some(c) => c,
        };

        match c {
            ws if is_line_ws(ws) => {
                self.reader.eat_line_ws();
                self.tok(RawTokenKind::Ws)
            }
            '\n' => self.tok(RawTokenKind::Newline),

            'U' | 'L' => self.handle_encoding_prefix(true),
            'u' => {
                let allow_char = !self.reader.eat('8');
                self.handle_encoding_prefix(allow_char)
            }

            '"' => self.handle_str(),
            '\'' => self.handle_char(),

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

    /// Finishes consuming and returns an identifier token.
    fn handle_ident(&mut self) -> RawToken<'a> {
        self.reader.eat_while(is_ident_continue);
        self.tok(RawTokenKind::Ident)
    }

    /// Finishes consuming and returns a preprocessing number token.
    fn handle_number(&mut self) -> RawToken<'a> {
        while self.eat_number_char() {}
        self.tok(RawTokenKind::Number)
    }

    /// Consumes the next character or pair of characters if they form a part of a preprocessing
    /// number; see §6.4.8 for details.
    ///
    /// Returns `true` if characters were consumed.
    fn eat_number_char(&mut self) -> bool {
        // If any of these characters are followed by a sign, they designate an exponent. Otherwise,
        // they are a part of the pp-number anyway.
        if self.reader.eat_if(|c| ['e', 'E', 'p', 'P'].contains(&c)) {
            self.reader.eat_if(|c| c == '+' || c == '-');
            return true;
        }

        self.reader.eat_if(|c| c == '.' || is_ident_continue(c))
    }

    /// Reacts to a possible encoding prefix (`L`, `u8`, etc.) and returns either a string,
    /// character (if `allow_char` is `true`) or identifier token, as appropriate.
    fn handle_encoding_prefix(&mut self, allow_char: bool) -> RawToken<'a> {
        if self.reader.eat('"') {
            self.handle_str()
        } else if allow_char && self.reader.eat('\'') {
            self.handle_char()
        } else {
            self.handle_ident()
        }
    }

    /// Finishes consuming and emits a string token.
    fn handle_str(&mut self) -> RawToken<'a> {
        self.handle_str_like('"', |terminated| RawTokenKind::Str { terminated })
    }

    /// Finishes consuming and emits a character token.
    fn handle_char(&mut self) -> RawToken<'a> {
        self.handle_str_like('\'', |terminated| RawTokenKind::Char { terminated })
    }

    /// Consumes characters until after `delim` or the nearest newline, using `f` create a token
    /// type based on whether the token was terminated.
    ///
    /// This function correctly handles escaping of `delim`.
    ///
    /// If a newline is encountered, it is _not_ consumed, to allow it to be emitted as a separate
    /// token. This is important for clients that must react specially to newlines, such as the
    /// preprocessor.
    fn handle_str_like(
        &mut self,
        delim: char,
        f: impl FnOnce(bool) -> RawTokenKind,
    ) -> RawToken<'a> {
        let mut escaped = false;

        while let Some(c) = self.reader.bump_if(|c| c != '\n') {
            match c {
                '\\' => escaped = !escaped,
                c if c == delim && !escaped => return self.tok(f(true)),
                _ => {}
            }
        }

        self.tok(f(false))
    }

    /// Handles a suspected punctuator character `c`, and returns either the appropriate punctuator
    /// or an `Unknown` token.
    #[allow(clippy::cognitive_complexity)]
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
                    self.punct(BangEq)
                } else {
                    self.punct(Bang)
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
            _ => self.tok(RawTokenKind::Unknown),
        }
    }

    /// Consumes and emits a line comment token.
    ///
    /// The terminating newline is not consumed, to allow it to be emitted as a separate token.
    fn handle_line_comment(&mut self) -> RawToken<'a> {
        self.reader.eat_while(|c| c != '\n');
        self.tok(RawTokenKind::LineComment)
    }

    /// Consumes and emits a block comment token.
    fn handle_block_comment(&mut self) -> RawToken<'a> {
        let terminated = loop {
            self.reader.eat_to_after('*');
            match self.reader.bump() {
                None => break false,
                Some('/') => break true,
                _ => {}
            }
        };

        self.tok(RawTokenKind::BlockComment { terminated })
    }

    /// Returns a punctuator token with the current content and the specified type.
    fn punct(&self, kind: PunctKind) -> RawToken<'a> {
        self.tok(RawTokenKind::Punct(kind))
    }

    /// Returns a token with the current content and the specified type.
    fn tok(&self, kind: RawTokenKind) -> RawToken<'a> {
        RawToken {
            kind,
            content: self.reader.cur_content(),
        }
    }
}
