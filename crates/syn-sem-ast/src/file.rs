use crate::{FromSyn, InputDesc, Item, Span, SyntaxCx};
use syn_sem_macros::CheckDropless;

#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct File<'scx> {
    items: &'scx [Item<'scx>],
    span: Span<'scx>,
}

impl<'scx> FromSyn<'scx, syn::File> for File<'scx> {
    fn from_syn(scx: &'scx SyntaxCx, desc: InputDesc<'_, syn::File>) -> Self {
        Self {
            items: FromSyn::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &*desc.input.items,
                },
            ),
            span: Span::from_locatable(scx, desc.file_path, desc.input),
        }
    }
}
