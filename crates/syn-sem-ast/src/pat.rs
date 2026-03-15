use crate::{ExprLit, ExprPath, FromSyn, Ident, Span, SyntaxCx, Type};
use num_traits::ToPrimitive;
use syn_sem_macros::CheckDropless;

#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub enum Pat<'scx> {
    Ident(PatIdent<'scx>),
    Lit(PatLit<'scx>),
    Path(PatPath<'scx>),
    Reference(PatReference<'scx>),
    Slice(PatSlice<'scx>),
    Tuple(PatTuple<'scx>),
    Type(PatType<'scx>),
}

impl<'scx> FromSyn<'scx, syn::Pat> for Pat<'scx> {
    fn from_syn(scx: &'scx SyntaxCx, input: &syn::Pat) -> Self {
        match input {
            syn::Pat::Ident(v) => Self::Ident(PatIdent::from_syn(scx, v)),
            syn::Pat::Lit(v) => Self::Lit(PatLit::from_syn(scx, v)),
            syn::Pat::Path(v) => Self::Path(PatPath::from_syn(scx, v)),
            syn::Pat::Reference(v) => Self::Reference(PatReference::from_syn(scx, v)),
            syn::Pat::Slice(v) => Self::Slice(PatSlice::from_syn(scx, v)),
            syn::Pat::Tuple(v) => Self::Tuple(PatTuple::from_syn(scx, v)),
            syn::Pat::Type(v) => Self::Type(PatType::from_syn(scx, v)),
            _ => todo!(),
        }
    }
}

pub type PatLit<'scx> = ExprLit<'scx>;
pub type PatPath<'scx> = ExprPath<'scx>;

#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct PatIdent<'scx> {
    pub ident: Ident<'scx>,
    pub span: Span<'scx>,
}

impl<'scx> PatIdent<'scx> {
    pub fn from_number<T: ToPrimitive>(scx: &'scx SyntaxCx, value: T, span: Span<'scx>) -> Self {
        Self {
            ident: Ident::from_number(scx, value, span),
            span,
        }
    }
}

impl<'scx> FromSyn<'scx, syn::PatIdent> for PatIdent<'scx> {
    fn from_syn(scx: &'scx SyntaxCx, input: &syn::PatIdent) -> Self {
        Self {
            ident: Ident::from_syn(scx, &input.ident),
            span: Span::from_locatable(scx, input),
        }
    }
}

impl<'scx> FromSyn<'scx, syn::Token![self]> for PatIdent<'scx> {
    fn from_syn(scx: &'scx SyntaxCx, input: &syn::Token![self]) -> Self {
        Self {
            ident: Ident::from_syn(scx, input),
            span: Span::from_locatable(scx, input),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct PatReference<'scx> {
    pub pat: &'scx Pat<'scx>,
    pub span: Span<'scx>,
}

impl<'scx> FromSyn<'scx, syn::PatReference> for PatReference<'scx> {
    fn from_syn(scx: &'scx SyntaxCx, input: &syn::PatReference) -> Self {
        Self {
            pat: scx.alloc(Pat::from_syn(scx, &input.pat)),
            span: Span::from_locatable(scx, input),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct PatSlice<'scx> {
    pub elems: &'scx [Pat<'scx>],
    pub span: Span<'scx>,
}

impl<'scx> FromSyn<'scx, syn::PatSlice> for PatSlice<'scx> {
    fn from_syn(scx: &'scx SyntaxCx, input: &syn::PatSlice) -> Self {
        Self {
            elems: FromSyn::from_syn(scx, &input.elems),
            span: Span::from_locatable(scx, input),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct PatTuple<'scx> {
    pub elems: &'scx [Pat<'scx>],
    pub span: Span<'scx>,
}

impl<'scx> FromSyn<'scx, syn::PatTuple> for PatTuple<'scx> {
    fn from_syn(scx: &'scx SyntaxCx, input: &syn::PatTuple) -> Self {
        Self {
            elems: FromSyn::from_syn(scx, &input.elems),
            span: Span::from_locatable(scx, input),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct PatType<'scx> {
    pub pat: &'scx Pat<'scx>,
    pub ty: Type<'scx>,
    pub span: Span<'scx>,
}

impl<'scx> FromSyn<'scx, syn::PatType> for PatType<'scx> {
    fn from_syn(scx: &'scx SyntaxCx, input: &syn::PatType) -> Self {
        Self {
            pat: scx.alloc(Pat::from_syn(scx, &input.pat)),
            ty: Type::from_syn(scx, &input.ty),
            span: Span::from_locatable(scx, input),
        }
    }
}

impl<'scx> FromSyn<'scx, syn::Receiver> for PatType<'scx> {
    fn from_syn(scx: &'scx SyntaxCx, input: &syn::Receiver) -> Self {
        Self {
            pat: scx.alloc(Pat::Ident(PatIdent::from_syn(scx, &input.self_token))),
            ty: Type::from_syn(scx, &input.ty),
            span: Span::from_locatable(scx, input),
        }
    }
}
