use crate::{Expr, FromSyn, InputDesc, Item, Pat, Span, SyntaxCx};
use syn_sem_macros::CheckDropless;

/// A braced block of statements.
///
/// For example, `{ let x = 1; x }`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct Block<'scx> {
    pub stmts: &'scx [Stmt<'scx>],
    pub span: Span<'scx>,
}

impl<'scx> FromSyn<'scx, syn::Block> for Block<'scx> {
    fn from_syn(scx: &'scx SyntaxCx, desc: InputDesc<'_, syn::Block>) -> Self {
        Self {
            stmts: FromSyn::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.stmts,
                },
            ),
            span: Span::from_locatable(scx, desc.file_path, desc.input),
        }
    }
}

/// A statement inside a block.
///
/// Examples include `let x = 1;`, an item like `fn f() {}`, and an expression statement.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub enum Stmt<'scx> {
    Local(Local<'scx>),
    Item(Item<'scx>),
    Expr(Expr<'scx>),
}

impl<'scx> FromSyn<'scx, syn::Stmt> for Stmt<'scx> {
    fn from_syn(scx: &'scx SyntaxCx, desc: InputDesc<'_, syn::Stmt>) -> Self {
        match desc.input {
            syn::Stmt::Local(v) => Self::Local(Local::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: v,
                },
            )),
            syn::Stmt::Item(v) => Self::Item(Item::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: v,
                },
            )),
            syn::Stmt::Expr(v, _) => Self::Expr(Expr::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: v,
                },
            )),
            _ => todo!(),
        }
    }
}

/// A local `let` binding.
///
/// For example, `let x: i32 = 1;`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct Local<'scx> {
    pub pat: Pat<'scx>,
    pub init: Option<LocalInit<'scx>>,
    pub span: Span<'scx>,
}

impl<'scx> FromSyn<'scx, syn::Local> for Local<'scx> {
    fn from_syn(scx: &'scx SyntaxCx, desc: InputDesc<'_, syn::Local>) -> Self {
        Self {
            pat: Pat::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.pat,
                },
            ),
            init: FromSyn::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.init,
                },
            ),
            span: Span::from_locatable(scx, desc.file_path, desc.input),
        }
    }
}

/// The initializer expression of a local binding.
///
/// For example, `= 1` in `let x = 1;`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct LocalInit<'scx> {
    pub expr: &'scx Expr<'scx>,
    pub span: Span<'scx>,
}

impl<'scx> FromSyn<'scx, syn::LocalInit> for LocalInit<'scx> {
    fn from_syn(scx: &'scx SyntaxCx, desc: InputDesc<'_, syn::LocalInit>) -> Self {
        Self {
            expr: scx.alloc(Expr::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.expr,
                },
            )),
            span: Span::from_locatable(scx, desc.file_path, desc.input),
        }
    }
}
