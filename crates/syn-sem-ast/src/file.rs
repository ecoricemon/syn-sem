use crate::{FromSyn, InputDesc, Item, Span, SyntaxCx};
use syn_sem_macros::CheckDropless;

/// A parsed Rust source file.
///
/// For example, `mod a; fn main() {}` is represented as a file containing module and function
/// items.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct File<'scx> {
    pub items: &'scx [Item<'scx>],
    pub span: Span<'scx>,
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
