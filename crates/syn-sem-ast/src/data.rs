use crate::{FromSyn, Ident, Span, SyntaxContext, Visibility};
use syn_sem_macros::CheckDropless;

#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct Field<'scx> {
    pub vis: Visibility<'scx>,
    pub ident: Ident<'scx>,
    pub span: Span<'scx>,
}

impl<'scx> FromSyn<'scx, syn::Field> for Field<'scx> {
    fn from_syn(scx: &'scx SyntaxContext, input: &syn::Field) -> Self {
        Self {
            vis: Visibility::from_syn(scx, &input.vis),
            ident: input
                .ident
                .as_ref()
                .map(|ident| Ident::from_syn(scx, ident))
                .unwrap_or(Ident::empty(scx)),
            span: Span::from_locatable(scx, input),
        }
    }
}

impl<'scx> FromSyn<'scx, syn::Fields> for &'scx [Field<'scx>] {
    fn from_syn(scx: &'scx SyntaxContext, input: &syn::Fields) -> Self {
        match input {
            syn::Fields::Named(syn::FieldsNamed { named, .. }) => Self::from_syn(scx, named),
            syn::Fields::Unnamed(syn::FieldsUnnamed { unnamed, .. }) => {
                let len = unnamed.len();
                let mut fields = unnamed.iter().enumerate().map(|(i, field)| {
                    let span = Span::from_locatable(scx, field);
                    let mut field = Field::from_syn(scx, field);
                    field.ident = Ident::from_usize(scx, i, span);
                    field
                });
                scx.alloc_slice(len, |_| fields.next().unwrap())
            }
            syn::Fields::Unit => &[],
        }
    }
}
