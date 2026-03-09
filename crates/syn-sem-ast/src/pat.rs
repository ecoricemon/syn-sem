use crate::{ExprLit, ExprPath, FromSyn, Span, SyntaxContext};
use syn_sem_macros::CheckDropless;

#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub enum Pat<'scx> {
    Lit(PatLit<'scx>),
    Path(PatPath<'scx>),
    Reference(PatReference<'scx>),
    Slice(PatSlice<'scx>),
    Tuple(PatTuple<'scx>),
}

impl<'scx> FromSyn<'scx, syn::Pat> for Pat<'scx> {
    fn from_syn(scx: &'scx SyntaxContext, input: &syn::Pat) -> Self {
        match input {
            syn::Pat::Lit(v) => Self::Lit(PatLit::from_syn(scx, v)),
            syn::Pat::Path(v) => Self::Path(PatPath::from_syn(scx, v)),
            syn::Pat::Reference(v) => Self::Reference(PatReference::from_syn(scx, v)),
            syn::Pat::Slice(v) => Self::Slice(PatSlice::from_syn(scx, v)),
            syn::Pat::Tuple(v) => Self::Tuple(PatTuple::from_syn(scx, v)),
            _ => todo!(),
        }
    }
}

pub type PatLit<'scx> = ExprLit<'scx>;
pub type PatPath<'scx> = ExprPath<'scx>;

#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct PatReference<'scx> {
    pub pat: &'scx Pat<'scx>,
    pub span: Span<'scx>,
}

impl<'scx> FromSyn<'scx, syn::PatReference> for PatReference<'scx> {
    fn from_syn(scx: &'scx SyntaxContext, input: &syn::PatReference) -> Self {
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
    fn from_syn(scx: &'scx SyntaxContext, input: &syn::PatSlice) -> Self {
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
    fn from_syn(scx: &'scx SyntaxContext, input: &syn::PatTuple) -> Self {
        Self {
            elems: FromSyn::from_syn(scx, &input.elems),
            span: Span::from_locatable(scx, input),
        }
    }
}
