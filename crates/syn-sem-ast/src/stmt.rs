use crate::{AstNode, AstNodeKind, Expr, FromSyn, InputDesc, Item, Pat, Span, SyntaxCx};
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

impl AstNode for Block<'_> {
    const KIND: AstNodeKind = AstNodeKind::Block;
}

impl<'cx> FromSyn<'cx, syn::Block> for Block<'cx> {
    fn from_syn(scx: &'cx SyntaxCx<'cx>, desc: InputDesc<'cx, '_, syn::Block>) -> Self {
        Self {
            stmts: FromSyn::from_syn(scx, desc.with_input(&desc.input.stmts)),
            span: desc.span(desc.input),
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
    Expr {
        /// Expression being evaluated.
        expr: Expr<'cx>,
        /// Whether this expression statement has a trailing semicolon.
        has_semi: bool,
    },
}

impl<'cx> FromSyn<'cx, syn::Stmt> for Stmt<'cx> {
    fn from_syn(scx: &'cx SyntaxCx<'cx>, desc: InputDesc<'cx, '_, syn::Stmt>) -> Self {
        match desc.input {
            syn::Stmt::Local(v) => Self::Local(Local::from_syn(scx, desc.with_input(v))),
            syn::Stmt::Item(v) => Self::Item(Item::from_syn(scx, desc.with_input(v))),
            syn::Stmt::Expr(v, semi) => Self::Expr {
                expr: Expr::from_syn(scx, desc.with_input(v)),
                has_semi: semi.is_some(),
            },
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
    fn from_syn(scx: &'cx SyntaxCx<'cx>, desc: InputDesc<'cx, '_, syn::Local>) -> Self {
        Self {
            pat: Pat::from_syn(scx, desc.with_input(&desc.input.pat)),
            init: FromSyn::from_syn(scx, desc.with_input(&desc.input.init)),
            span: desc.span(desc.input),
        }
    }
}

/// The initializer expression of a local binding.
///
/// For example, `= 1` in `let x = 1;`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct LocalInit<'cx> {
    /// Initializer expression.
    ///
    /// Stored by reference to break the recursive statement/expression shape.
    pub expr: &'cx Expr<'cx>,
    /// Source span of the initializer.
    pub span: Span<'cx>,
}

impl<'cx> FromSyn<'cx, syn::LocalInit> for LocalInit<'cx> {
    fn from_syn(scx: &'cx SyntaxCx<'cx>, desc: InputDesc<'cx, '_, syn::LocalInit>) -> Self {
        Self {
            expr: scx.alloc(Expr::from_syn(scx, desc.with_input(&desc.input.expr))),
            span: desc.span(desc.input),
        }
    }
}
