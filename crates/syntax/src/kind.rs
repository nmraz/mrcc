#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Keyword {
    Alignof,
    Auto,
    Break,
    Case,
    Char,
    Const,
    Continue,
    Default,
    Do,
    Double,
    Else,
    Enum,
    Extern,
    Float,
    For,
    Goto,
    If,
    Inline,
    Int,
    Long,
    Register,
    Restrict,
    Return,
    Short,
    Signed,
    Sizeof,
    Static,
    Struct,
    Switch,
    Typedef,
    Union,
    Unsigned,
    Void,
    Volatile,
    While,
    Alignas,
    Atomic,
    Bool,
    Complex,
    Generic,
    Imaginary,
    Noreturn,
    StaticAssert,
    ThreadLocal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    Plain(lex::TokenKind),
    Keyword(Keyword),
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

    BlockStmt,
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
