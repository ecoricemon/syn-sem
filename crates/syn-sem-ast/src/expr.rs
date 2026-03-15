use crate::{Block, FromSyn, Ident, Lit, Path, Span, SyntaxCx, Type};
use syn_sem_macros::CheckDropless;

#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub enum Expr<'scx> {
    Array(ExprArray<'scx>),
    Assign(ExprAssign<'scx>),
    Binary(ExprBinary<'scx>),
    Block(ExprBlock<'scx>),
    Call(ExprCall<'scx>),
    Cast(ExprCast<'scx>),
    Field(ExprField<'scx>),
    Index(ExprIndex<'scx>),
    Lit(ExprLit<'scx>),
    MethodCall(ExprMethodCall<'scx>),
    Paren(ExprParen<'scx>),
    Path(ExprPath<'scx>),
    Reference(ExprReference<'scx>),
}

impl<'scx> FromSyn<'scx, syn::Expr> for Expr<'scx> {
    fn from_syn(scx: &'scx SyntaxCx, input: &syn::Expr) -> Self {
        match input {
            syn::Expr::Array(v) => Self::Array(ExprArray::from_syn(scx, v)),
            syn::Expr::Assign(v) => Self::Assign(ExprAssign::from_syn(scx, v)),
            syn::Expr::Binary(v) => Self::Binary(ExprBinary::from_syn(scx, v)),
            syn::Expr::Block(v) => Self::Block(ExprBlock::from_syn(scx, v)),
            syn::Expr::Call(v) => Self::Call(ExprCall::from_syn(scx, v)),
            syn::Expr::Cast(v) => Self::Cast(ExprCast::from_syn(scx, v)),
            syn::Expr::Field(v) => Self::Field(ExprField::from_syn(scx, v)),
            syn::Expr::Index(v) => Self::Index(ExprIndex::from_syn(scx, v)),
            syn::Expr::Lit(v) => Self::Lit(ExprLit::from_syn(scx, v)),
            syn::Expr::MethodCall(v) => Self::MethodCall(ExprMethodCall::from_syn(scx, v)),
            syn::Expr::Paren(v) => Self::Paren(ExprParen::from_syn(scx, v)),
            syn::Expr::Path(v) => Self::Path(ExprPath::from_syn(scx, v)),
            syn::Expr::Reference(v) => Self::Reference(ExprReference::from_syn(scx, v)),
            o => todo!("{o:?}"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct ExprArray<'scx> {
    pub elems: &'scx [Expr<'scx>],
    pub span: Span<'scx>,
}

impl<'scx> FromSyn<'scx, syn::ExprArray> for ExprArray<'scx> {
    fn from_syn(scx: &'scx SyntaxCx, input: &syn::ExprArray) -> Self {
        Self {
            elems: FromSyn::from_syn(scx, &input.elems),
            span: Span::from_locatable(scx, input),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct ExprAssign<'scx> {
    pub left: &'scx Expr<'scx>,
    pub right: &'scx Expr<'scx>,
    pub span: Span<'scx>,
}

impl<'scx> FromSyn<'scx, syn::ExprAssign> for ExprAssign<'scx> {
    fn from_syn(scx: &'scx SyntaxCx, input: &syn::ExprAssign) -> Self {
        Self {
            left: scx.alloc(Expr::from_syn(scx, &input.left)),
            right: scx.alloc(Expr::from_syn(scx, &input.right)),
            span: Span::from_locatable(scx, input),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct ExprBinary<'scx> {
    pub left: &'scx Expr<'scx>,
    pub right: &'scx Expr<'scx>,
    pub span: Span<'scx>,
}

impl<'scx> FromSyn<'scx, syn::ExprBinary> for ExprBinary<'scx> {
    fn from_syn(scx: &'scx SyntaxCx, input: &syn::ExprBinary) -> Self {
        Self {
            left: scx.alloc(Expr::from_syn(scx, &input.left)),
            right: scx.alloc(Expr::from_syn(scx, &input.right)),
            span: Span::from_locatable(scx, input),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct ExprBlock<'scx> {
    pub block: Block<'scx>,
    pub span: Span<'scx>,
}

impl<'scx> FromSyn<'scx, syn::ExprBlock> for ExprBlock<'scx> {
    fn from_syn(scx: &'scx SyntaxCx, input: &syn::ExprBlock) -> Self {
        Self {
            block: Block::from_syn(scx, &input.block),
            span: Span::from_locatable(scx, input),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct ExprCall<'scx> {
    pub func: &'scx Expr<'scx>,
    pub args: &'scx [Expr<'scx>],
    pub span: Span<'scx>,
}

impl<'scx> FromSyn<'scx, syn::ExprCall> for ExprCall<'scx> {
    fn from_syn(scx: &'scx SyntaxCx, input: &syn::ExprCall) -> Self {
        Self {
            func: scx.alloc(Expr::from_syn(scx, &input.func)),
            args: FromSyn::from_syn(scx, &input.args),
            span: Span::from_locatable(scx, input),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct ExprCast<'scx> {
    pub expr: &'scx Expr<'scx>,
    pub ty: &'scx Type<'scx>,
    pub span: Span<'scx>,
}

impl<'scx> FromSyn<'scx, syn::ExprCast> for ExprCast<'scx> {
    fn from_syn(scx: &'scx SyntaxCx, input: &syn::ExprCast) -> Self {
        Self {
            expr: scx.alloc(Expr::from_syn(scx, &input.expr)),
            ty: scx.alloc(Type::from_syn(scx, &input.ty)),
            span: Span::from_locatable(scx, input),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct ExprField<'scx> {
    pub base: &'scx Expr<'scx>,
    pub member: Ident<'scx>,
    pub span: Span<'scx>,
}

impl<'scx> FromSyn<'scx, syn::ExprField> for ExprField<'scx> {
    fn from_syn(scx: &'scx SyntaxCx, input: &syn::ExprField) -> Self {
        let member = match &input.member {
            syn::Member::Named(ident) => Ident::from_syn(scx, ident),
            syn::Member::Unnamed(idx) => {
                Ident::from_number(scx, idx.index, Span::from_locatable(scx, idx))
            }
        };
        Self {
            base: scx.alloc(Expr::from_syn(scx, &input.base)),
            member,
            span: Span::from_locatable(scx, input),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct ExprIndex<'scx> {
    pub expr: &'scx Expr<'scx>,
    pub index: &'scx Expr<'scx>,
    pub span: Span<'scx>,
}

impl<'scx> FromSyn<'scx, syn::ExprIndex> for ExprIndex<'scx> {
    fn from_syn(scx: &'scx SyntaxCx, input: &syn::ExprIndex) -> Self {
        Self {
            expr: scx.alloc(Expr::from_syn(scx, &input.expr)),
            index: scx.alloc(Expr::from_syn(scx, &input.index)),
            span: Span::from_locatable(scx, input),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct ExprLit<'scx> {
    pub lit: Lit<'scx>,
    pub span: Span<'scx>,
}

impl<'scx> FromSyn<'scx, syn::ExprLit> for ExprLit<'scx> {
    fn from_syn(scx: &'scx SyntaxCx, input: &syn::ExprLit) -> Self {
        Self {
            lit: Lit::from_syn(scx, &input.lit),
            span: Span::from_locatable(scx, input),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct ExprMethodCall<'scx> {
    pub receiver: &'scx Expr<'scx>,
    pub method: Ident<'scx>,
    pub args: &'scx [Expr<'scx>],
    pub span: Span<'scx>,
}

impl<'scx> FromSyn<'scx, syn::ExprMethodCall> for ExprMethodCall<'scx> {
    fn from_syn(scx: &'scx SyntaxCx, input: &syn::ExprMethodCall) -> Self {
        Self {
            receiver: scx.alloc(Expr::from_syn(scx, &input.receiver)),
            method: Ident::from_syn(scx, &input.method),
            args: FromSyn::from_syn(scx, &input.args),
            span: Span::from_locatable(scx, input),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct ExprParen<'scx> {
    pub expr: &'scx Expr<'scx>,
    pub span: Span<'scx>,
}

impl<'scx> FromSyn<'scx, syn::ExprParen> for ExprParen<'scx> {
    fn from_syn(scx: &'scx SyntaxCx, input: &syn::ExprParen) -> Self {
        Self {
            expr: scx.alloc(Expr::from_syn(scx, &input.expr)),
            span: Span::from_locatable(scx, input),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct ExprPath<'scx> {
    pub path: Path<'scx>,
    pub span: Span<'scx>,
}

impl<'scx> FromSyn<'scx, syn::ExprPath> for ExprPath<'scx> {
    fn from_syn(scx: &'scx SyntaxCx, input: &syn::ExprPath) -> Self {
        Self {
            path: Path::from_syn(scx, &input.path),
            span: Span::from_locatable(scx, input),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct ExprReference<'scx> {
    pub expr: &'scx Expr<'scx>,
    pub span: Span<'scx>,
}

impl<'scx> FromSyn<'scx, syn::ExprReference> for ExprReference<'scx> {
    fn from_syn(scx: &'scx SyntaxCx, input: &syn::ExprReference) -> Self {
        Self {
            expr: scx.alloc(Expr::from_syn(scx, &input.expr)),
            span: Span::from_locatable(scx, input),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::*;

    #[test]
    fn text_expr_call() {
        let scx = create_context();
        let expr = parse::<syn::ExprCall, ExprCall>(&scx, "invoke(a, b)");
        let Expr::Path(path) = expr.func else {
            panic!()
        };
        assert_eq!(&**path.path.get_ident().unwrap(), "invoke");
        assert_eq!(expr.args.len(), 2);
    }

    #[test]
    fn test_expr_lit() {
        let scx = create_context();
        let expr = parse::<syn::ExprLit, ExprLit>(&scx, "1");
        let Lit::Int(v) = expr.lit else { panic!() };
        assert_eq!(v.base10_parse::<i32>().unwrap(), 1);
    }
}
