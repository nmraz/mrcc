use lex::{PunctKind, Symbol};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    Unknown,
    Punct(PunctKind),

    Ident(Symbol),
    Number(Symbol),
    Str(Symbol),
    Char(Symbol),

    AlignofKw,
    AutoKw,
    BreakKw,
    CaseKw,
    CharKw,
    ConstKw,
    ContinueKw,
    DefaultKw,
    DoKw,
    DoubleKw,
    ElseKw,
    EnumKw,
    ExternKw,
    FloatKw,
    ForKw,
    GotoKw,
    IfKw,
    InlineKw,
    IntKw,
    LongKw,
    RegisterKw,
    RestrictKw,
    ReturnKw,
    ShortKw,
    SignedKw,
    SizeofKw,
    StaticKw,
    StructKw,
    SwitchKw,
    TypedefKw,
    UnionKw,
    UnsignedKw,
    VoidKw,
    VolatileKw,
    WhileKw,
    AlignasKw,
    AtomicKw,
    BoolKw,
    ComplexKw,
    GenericKw,
    ImaginaryKw,
    NoreturnKw,
    StaticAssertKw,
    ThreadLocalKw,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeKind {
    TranslationUnit,
}
