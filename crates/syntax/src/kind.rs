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

    // (External) Declarations
    FunctionDef,
    PlainDecl,
    StaticAssertDecl,

    InitDeclarator,

    // Specifiers
    StorageSpecifier,

    PlainTypeSpecifier,
    AtomicTypeSpecifier,
    StructSpecifier,
    UnionSpecifier,
    EnumSpecifier,
    TypedefName,

    TypeQualifier,
    FunctionSpecifier,
    AlignmentSpecifier,

    SpecifierQualifierList,
    TypeQualifierList,

    // Struct/Union Contents
    StructDeclList,
    StructFieldDecl,
    BitfieldDeclarator,

    // Enum Contents
    EnumeratorList,
    Enumerator,

    // Declarators
    IdentDeclarator,
    ParenDeclarator,
    ArrayDeclarator,
    FunctionDeclarator,

    ParamList,

    // Initializers
    StructInitList,
    DesignatorList,
    FieldDesignator,
    ArrayDesignator,

    // Statements
    LabeledStmt,
    CaseStmt,
    DefaultCaseStmt,

    ExprStmt,

    IfStmt,
    SwitchStmt,

    WhileStmt,
    DoWhileStmt,
    ForStmt,

    GotoStmt,
    ContinueStmt,
    BreakStmt,
    ReturnStmt,

    // Expressions
    IdentExpr,
    NumberLiteralExpr,
    CharLiteralExpr,
    StrLiteralExpr,
    ParenExpr,

    IndexExpr,
    CallExpr,
    MemberExpr,
    DerefMemberExpr,
    PostIncrExpr,
    CompoundLiteralExpr,

    PreIncrExpr,
    UnaryExpr,
    SizeofValExpr,
    SizeofTypeExpr,
    AlignofExpr,

    CastExpr,
    BinExpr,
    ConditionalExpr,
    AssignmentExpr,

    ArgList,
}
