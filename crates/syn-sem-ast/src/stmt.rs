use crate::{Expr, FromSyn, Item, Pat, Span, SyntaxCx};
use syn_sem_macros::CheckDropless;

#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct Block<'scx> {
    pub stmts: &'scx [Stmt<'scx>],
    pub span: Span<'scx>,
}

impl<'scx> FromSyn<'scx, syn::Block> for Block<'scx> {
    fn from_syn(scx: &'scx SyntaxCx, input: &syn::Block) -> Self {
        Self {
            stmts: FromSyn::from_syn(scx, &input.stmts),
            span: Span::from_locatable(scx, input),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub enum Stmt<'scx> {
    Local(Local<'scx>),
    Item(Item<'scx>),
    Expr(Expr<'scx>),
}

impl<'scx> FromSyn<'scx, syn::Stmt> for Stmt<'scx> {
    fn from_syn(scx: &'scx SyntaxCx, input: &syn::Stmt) -> Self {
        match input {
            syn::Stmt::Local(v) => Self::Local(Local::from_syn(scx, v)),
            syn::Stmt::Item(v) => Self::Item(Item::from_syn(scx, v)),
            syn::Stmt::Expr(v, _) => Self::Expr(Expr::from_syn(scx, v)),
            _ => todo!(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct Local<'scx> {
    pub pat: Pat<'scx>,
    pub init: Option<LocalInit<'scx>>,
    pub span: Span<'scx>,
}

impl<'scx> FromSyn<'scx, syn::Local> for Local<'scx> {
    fn from_syn(scx: &'scx SyntaxCx, input: &syn::Local) -> Self {
        Self {
            pat: Pat::from_syn(scx, &input.pat),
            init: FromSyn::from_syn(scx, &input.init),
            span: Span::from_locatable(scx, input),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct LocalInit<'scx> {
    pub expr: &'scx Expr<'scx>,
    pub span: Span<'scx>,
}

impl<'scx> FromSyn<'scx, syn::LocalInit> for LocalInit<'scx> {
    fn from_syn(scx: &'scx SyntaxCx, input: &syn::LocalInit) -> Self {
        Self {
            expr: scx.alloc(Expr::from_syn(scx, &input.expr)),
            span: Span::from_locatable(scx, input),
        }
    }
}
