use std::fmt;

/// Enum representing punctuator types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PunctKind {
    Hash,
    HashHash,

    Comma,
    Colon,
    Semi,

    LSquare,
    RSquare,
    LParen,
    RParen,
    LCurly,
    RCurly,

    Dot,
    Ellipsis,
    Arrow,

    Plus,
    PlusPlus,
    Minus,
    MinusMinus,
    Star,
    Slash,
    Perc,
    Amp,
    AmpAmp,
    Pipe,
    PipePipe,
    Caret,
    Tilde,
    Bang,
    Question,
    Less,
    LessLess,
    LessEq,
    Greater,
    GreaterGreater,
    GreaterEq,

    Eq,
    EqEq,
    BangEq,
    PlusEq,
    MinusEq,
    StarEq,
    SlashEq,
    PercEq,
    AmpEq,
    PipeEq,
    CaretEq,
    LessLessEq,
    GreaterGreaterEq,
}

impl PunctKind {
    /// Returns the appropriate punctuator string corresponding to `self`.
    pub fn as_str(self) -> &'static str {
        use PunctKind::*;

        match self {
            Hash => "#",
            HashHash => "##",
            Comma => ",",
            Colon => ":",
            Semi => ";",
            LSquare => "[",
            RSquare => "]",
            LParen => "(",
            RParen => ")",
            LCurly => "{",
            RCurly => "}",
            Dot => ".",
            Ellipsis => "...",
            Arrow => "->",
            Plus => "+",
            PlusPlus => "++",
            Minus => "-",
            MinusMinus => "--",
            Star => "*",
            Slash => "/",
            Perc => "%",
            Amp => "&",
            AmpAmp => "&&",
            Pipe => "|",
            PipePipe => "||",
            Caret => "^",
            Tilde => "~",
            Bang => "!",
            BangEq => "!=",
            Question => "?",
            Less => "<",
            LessLess => "<<",
            LessEq => "<=",
            Greater => ">",
            GreaterGreater => ">>",
            GreaterEq => ">=",
            Eq => "=",
            EqEq => "==",
            PlusEq => "+=",
            MinusEq => "-=",
            StarEq => "*=",
            SlashEq => "/=",
            PercEq => "%=",
            AmpEq => "&=",
            PipeEq => "|=",
            CaretEq => "^=",
            LessLessEq => "<<=",
            GreaterGreaterEq => ">>=",
        }
    }
}

impl fmt::Display for PunctKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
