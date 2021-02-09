use lex::Interner;

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

impl TokenKind {
    pub fn from_plain(plain: lex::TokenKind, interner: &Interner) -> Self {
        let ident = match plain {
            lex::TokenKind::Ident(ident) => ident,
            _ => return Self::Plain(plain),
        };

        let kw = match &interner[ident] {
            "alignof" => Keyword::Alignof,
            "auto" => Keyword::Auto,
            "break" => Keyword::Break,
            "case" => Keyword::Case,
            "char" => Keyword::Char,
            "const" => Keyword::Const,
            "continue" => Keyword::Continue,
            "default" => Keyword::Default,
            "do" => Keyword::Do,
            "double" => Keyword::Double,
            "else" => Keyword::Else,
            "enum" => Keyword::Enum,
            "extern" => Keyword::Extern,
            "float" => Keyword::Float,
            "for" => Keyword::For,
            "goto" => Keyword::Goto,
            "if" => Keyword::If,
            "inline" => Keyword::Inline,
            "int" => Keyword::Int,
            "long" => Keyword::Long,
            "register" => Keyword::Register,
            "restrict" => Keyword::Restrict,
            "return" => Keyword::Return,
            "short" => Keyword::Short,
            "signed" => Keyword::Signed,
            "sizeof" => Keyword::Sizeof,
            "static" => Keyword::Static,
            "struct" => Keyword::Struct,
            "switch" => Keyword::Switch,
            "typedef" => Keyword::Typedef,
            "union" => Keyword::Union,
            "unsigned" => Keyword::Unsigned,
            "void" => Keyword::Void,
            "volatile" => Keyword::Volatile,
            "while" => Keyword::While,
            "_Alignas" => Keyword::Alignas,
            "_Atomic" => Keyword::Atomic,
            "_Bool" => Keyword::Bool,
            "_Complex" => Keyword::Complex,
            "_Generic" => Keyword::Generic,
            "_Imaginary" => Keyword::Imaginary,
            "_Noreturn" => Keyword::Noreturn,
            "_Static_assert" => Keyword::StaticAssert,
            "_Thread_local" => Keyword::ThreadLocal,
            _ => return Self::Plain(plain),
        };

        Self::Keyword(kw)
    }
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

#[cfg(test)]
mod tests {
    use lex::PunctKind;

    use super::*;

    #[test]
    fn kind_from_plain() {
        fn check(plain: lex::TokenKind, expected: TokenKind, interner: &Interner) {
            assert_eq!(TokenKind::from_plain(plain, interner), expected);
        }

        fn check_plain(plain: lex::TokenKind, interner: &Interner) {
            check(plain, TokenKind::Plain(plain), interner);
        }

        fn check_kw(text: &str, kw: Keyword, interner: &mut Interner) {
            check(
                lex::TokenKind::Ident(interner.intern(text)),
                TokenKind::Keyword(kw),
                interner,
            );
        }

        let mut interner = Interner::new();

        check_plain(lex::TokenKind::Unknown, &interner);
        check_plain(lex::TokenKind::Eof, &interner);
        check_plain(lex::TokenKind::Punct(PunctKind::Comma), &interner);

        check_plain(lex::TokenKind::Number(interner.intern("3")), &interner);
        check_plain(lex::TokenKind::Str(interner.intern(r#""hi""#)), &interner);
        check_plain(lex::TokenKind::Char(interner.intern("'c'")), &interner);

        check_kw("if", Keyword::If, &mut interner);
        check_kw("while", Keyword::While, &mut interner);
        check_kw("for", Keyword::For, &mut interner);
    }
}
