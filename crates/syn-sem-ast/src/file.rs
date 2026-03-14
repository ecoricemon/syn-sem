use crate::{FromSyn, Item, Span, SyntaxContext};
use syn_sem_macros::CheckDropless;

#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct File<'scx> {
    items: &'scx [Item<'scx>],
    span: Span<'scx>,
}

impl<'scx> FromSyn<'scx, syn::File> for File<'scx> {
    fn from_syn(scx: &'scx SyntaxContext, input: &syn::File) -> Self {
        Self {
            items: FromSyn::from_syn(scx, &*input.items),
            span: Span::from_locatable(scx, input),
        }
    }
}
