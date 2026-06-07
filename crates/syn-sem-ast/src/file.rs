use crate::{FromSyn, InputDesc, Item, Span, SyntaxCx};
use syn_sem_common::{AstNode, AstNodeKind};
use syn_sem_macros::CheckDropless;

/// A parsed Rust source file.
///
/// For example, `mod a; fn main() {}` is represented as a file containing module and function
/// items.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct File<'cx> {
    /// Top-level items in the file.
    pub items: &'cx [Item<'cx>],
    /// Source span of the whole file.
    pub span: Span<'cx>,
}

impl AstNode for File<'_> {
    const KIND: AstNodeKind = AstNodeKind::File;
}

impl<'cx> FromSyn<'cx, syn::File> for File<'cx> {
    fn from_syn(scx: &'cx SyntaxCx<'cx>, desc: InputDesc<'cx, '_, syn::File>) -> Self {
        Self {
            items: FromSyn::from_syn(scx, desc.with_input(&*desc.input.items)),
            span: desc.span(desc.input),
        }
    }
}
