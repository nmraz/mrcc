use crate::lex::{Interner, LexCtx, Symbol};
use crate::SourcePos;

pub struct PendingIf {
    pub pos: SourcePos,
}

pub struct FileState {
    pub is_line_start: bool,
    pub pending_ifs: Vec<PendingIf>,
}

impl Default for FileState {
    fn default() -> Self {
        Self {
            is_line_start: true,
            pending_ifs: vec![],
        }
    }
}

pub struct State {
    pub known_idents: KnownIdents,
}

impl State {
    pub fn new(ctx: &mut LexCtx<'_, '_>) -> Self {
        Self {
            known_idents: KnownIdents::new(&mut ctx.ident_interner),
        }
    }
}

pub struct KnownIdents {
    pub dir_if: Symbol,
    pub dir_ifdef: Symbol,
    pub dir_ifndef: Symbol,
    pub dir_elif: Symbol,
    pub dir_else: Symbol,
    pub dir_endif: Symbol,
    pub dir_include: Symbol,
    pub dir_define: Symbol,
    pub dir_undef: Symbol,
    pub dir_error: Symbol,
}

impl KnownIdents {
    pub fn new(interner: &mut Interner) -> Self {
        Self {
            dir_if: interner.intern("if"),
            dir_ifdef: interner.intern("ifdef"),
            dir_ifndef: interner.intern("ifndef"),
            dir_elif: interner.intern("elif"),
            dir_else: interner.intern("else"),
            dir_endif: interner.intern("endif"),
            dir_include: interner.intern("include"),
            dir_define: interner.intern("define"),
            dir_undef: interner.intern("undef"),
            dir_error: interner.intern("error"),
        }
    }
}
