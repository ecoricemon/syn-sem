use crate::{Expr, FromSyn, InputDesc, Item, Pat, Span, SyntaxCx};
use syn_sem_macros::CheckDropless;

/// A braced block of statements.
///
/// For example, `{ let x = 1; x }`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct Block<'cx> {
    /// Statements contained in the block.
    pub stmts: &'cx [Stmt<'cx>],
    /// Source span of the block.
    pub span: Span<'cx>,
}

impl<'cx> FromSyn<'cx, syn::Block> for Block<'cx> {
    fn from_syn(scx: &'cx SyntaxCx<'cx>, desc: InputDesc<'cx, syn::Block>) -> Self {
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
pub enum Stmt<'cx> {
    /// Local `let` binding.
    Local(Local<'cx>),
    /// Item statement.
    Item(Item<'cx>),
    /// Expression statement.
    Expr(Expr<'cx>),
}

impl<'cx> FromSyn<'cx, syn::Stmt> for Stmt<'cx> {
    fn from_syn(scx: &'cx SyntaxCx<'cx>, desc: InputDesc<'cx, syn::Stmt>) -> Self {
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
pub struct Local<'cx> {
    /// Binding pattern.
    pub pat: Pat<'cx>,
    /// Optional initializer.
    pub init: Option<LocalInit<'cx>>,
    /// Source span of the local binding.
    pub span: Span<'cx>,
}

impl<'cx> FromSyn<'cx, syn::Local> for Local<'cx> {
    fn from_syn(scx: &'cx SyntaxCx<'cx>, desc: InputDesc<'cx, syn::Local>) -> Self {
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
pub struct LocalInit<'cx> {
    /// Initializer expression.
    pub expr: &'cx Expr<'cx>,
    /// Source span of the initializer.
    pub span: Span<'cx>,
}

impl<'cx> FromSyn<'cx, syn::LocalInit> for LocalInit<'cx> {
    fn from_syn(scx: &'cx SyntaxCx<'cx>, desc: InputDesc<'cx, syn::LocalInit>) -> Self {
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
