use crate::{FromSyn, Ident, Span, SyntaxCx};
use syn_sem_macros::CheckDropless;

#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct Path<'scx> {
    pub segments: &'scx [PathSegment<'scx>],
    pub span: Span<'scx>,
}

impl<'scx> Path<'scx> {
    pub fn from_str(scx: &'scx SyntaxCx, value: &str, span: Span<'scx>) -> Self {
        Self::from_iter(scx, value.split("::"), span)
    }

    pub fn from_iter<'a, I>(scx: &'scx SyntaxCx, mut iter: I, span: Span<'scx>) -> Self
    where
        I: Iterator<Item = &'a str> + Clone,
    {
        let len = iter.clone().count();
        let segments = scx.alloc_slice(len, |_| {
            PathSegment::from_str(scx, iter.next().unwrap(), Span::empty())
        });
        Self { segments, span }
    }

    pub fn get_ident(&self) -> Option<&Ident<'scx>> {
        if self.segments.len() == 1 {
            Some(&self.segments[0].ident)
        } else {
            None
        }
    }
}

impl<'scx> FromSyn<'scx, syn::Path> for Path<'scx> {
    fn from_syn(scx: &'scx SyntaxCx, input: &syn::Path) -> Self {
        Self {
            segments: FromSyn::from_syn(scx, &input.segments),
            span: Span::from_locatable(scx, input),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct PathSegment<'scx> {
    pub ident: Ident<'scx>,
    pub span: Span<'scx>,
}

impl<'scx> PathSegment<'scx> {
    pub fn from_str(scx: &'scx SyntaxCx, value: &str, span: Span<'scx>) -> Self {
        Self {
            ident: Ident::from_str(scx, value, span),
            span,
        }
    }
}

impl<'scx> FromSyn<'scx, syn::PathSegment> for PathSegment<'scx> {
    fn from_syn(scx: &'scx SyntaxCx, input: &syn::PathSegment) -> Self {
        Self {
            ident: Ident::from_syn(scx, &input.ident),
            span: Span::from_locatable(scx, input),
        }
    }
}
