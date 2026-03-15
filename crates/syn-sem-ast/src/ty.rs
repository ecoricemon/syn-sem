use crate::{Expr, FromSyn, Path, Span, SyntaxCx};
use syn_sem_macros::CheckDropless;

#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub enum Type<'scx> {
    Array(TypeArray<'scx>),
    Infer(Span<'scx>),
    Path(TypePath<'scx>),
    Reference(TypeReference<'scx>),
    Slice(TypeSlice<'scx>),
    Tuple(TypeTuple<'scx>),
}

impl<'scx> Type<'scx> {
    pub fn unit(span: Span<'scx>) -> Self {
        Self::Tuple(TypeTuple::unit(span))
    }
}

impl<'scx> FromSyn<'scx, syn::Type> for Type<'scx> {
    fn from_syn(scx: &'scx SyntaxCx, input: &syn::Type) -> Self {
        match input {
            syn::Type::Array(v) => Self::Array(TypeArray::from_syn(scx, v)),
            syn::Type::Infer(v) => Self::Infer(Span::from_locatable(scx, v)),
            syn::Type::Path(v) => Self::Path(TypePath::from_syn(scx, v)),
            syn::Type::Reference(v) => Self::Reference(TypeReference::from_syn(scx, v)),
            syn::Type::Slice(v) => Self::Slice(TypeSlice::from_syn(scx, v)),
            syn::Type::Tuple(v) => Self::Tuple(TypeTuple::from_syn(scx, v)),
            _ => todo!(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct TypeArray<'scx> {
    pub elem: &'scx Type<'scx>,
    pub len: Expr<'scx>,
    pub span: Span<'scx>,
}

impl<'scx> FromSyn<'scx, syn::TypeArray> for TypeArray<'scx> {
    fn from_syn(scx: &'scx SyntaxCx, input: &syn::TypeArray) -> Self {
        Self {
            elem: scx.alloc(Type::from_syn(scx, &input.elem)),
            len: Expr::from_syn(scx, &input.len),
            span: Span::from_locatable(scx, input),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct TypePath<'scx> {
    pub path: Path<'scx>,
    pub span: Span<'scx>,
}

impl<'scx> FromSyn<'scx, syn::TypePath> for TypePath<'scx> {
    fn from_syn(scx: &'scx SyntaxCx, input: &syn::TypePath) -> Self {
        Self {
            path: Path::from_syn(scx, &input.path),
            span: Span::from_locatable(scx, input),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct TypeReference<'scx> {
    pub elem: &'scx Type<'scx>,
    pub span: Span<'scx>,
}

impl<'scx> FromSyn<'scx, syn::TypeReference> for TypeReference<'scx> {
    fn from_syn(scx: &'scx SyntaxCx, input: &syn::TypeReference) -> Self {
        Self {
            elem: scx.alloc(Type::from_syn(scx, &input.elem)),
            span: Span::from_locatable(scx, input),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct TypeSlice<'scx> {
    pub elem: &'scx Type<'scx>,
    pub span: Span<'scx>,
}

impl<'scx> FromSyn<'scx, syn::TypeSlice> for TypeSlice<'scx> {
    fn from_syn(scx: &'scx SyntaxCx, input: &syn::TypeSlice) -> Self {
        Self {
            elem: scx.alloc(Type::from_syn(scx, &input.elem)),
            span: Span::from_locatable(scx, input),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct TypeTuple<'scx> {
    pub elems: &'scx [Type<'scx>],
    pub span: Span<'scx>,
}

impl<'scx> TypeTuple<'scx> {
    pub fn unit(span: Span<'scx>) -> Self {
        Self { elems: &[], span }
    }
}

impl<'scx> FromSyn<'scx, syn::TypeTuple> for TypeTuple<'scx> {
    fn from_syn(scx: &'scx SyntaxCx, input: &syn::TypeTuple) -> Self {
        Self {
            elems: FromSyn::from_syn(scx, &input.elems),
            span: Span::from_locatable(scx, input),
        }
    }
}
