use crate::{FromSyn, Ident, Span, SyntaxContext};
use syn_sem_macros::CheckDropless;

#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct Path<'scx> {
    pub segments: &'scx [PathSegment<'scx>],
    pub span: Span<'scx>,
}

impl<'scx> Path<'scx> {
    pub fn get_ident(&self) -> Option<&Ident<'scx>> {
        if self.segments.len() == 1 {
            Some(&self.segments[0].ident)
        } else {
            None
        }
    }
}

impl<'scx> FromSyn<'scx, syn::Path> for Path<'scx> {
    fn from_syn(scx: &'scx SyntaxContext, input: &syn::Path) -> Self {
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

impl<'scx> FromSyn<'scx, syn::PathSegment> for PathSegment<'scx> {
    fn from_syn(scx: &'scx SyntaxContext, input: &syn::PathSegment) -> Self {
        Self {
            ident: Ident::from_syn(scx, &input.ident),
            span: Span::from_locatable(scx, input),
        }
    }
}
