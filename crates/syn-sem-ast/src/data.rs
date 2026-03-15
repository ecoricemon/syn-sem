use crate::{Expr, FromSyn, Ident, Span, SyntaxCx, Type, Visibility};
use syn_sem_macros::CheckDropless;

#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct Field<'scx> {
    pub vis: Visibility<'scx>,
    pub ident: Ident<'scx>,
    pub span: Span<'scx>,
}

impl<'scx> FromSyn<'scx, syn::Field> for Field<'scx> {
    fn from_syn(scx: &'scx SyntaxCx, input: &syn::Field) -> Self {
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
    fn from_syn(scx: &'scx SyntaxCx, input: &syn::Fields) -> Self {
        match input {
            syn::Fields::Named(syn::FieldsNamed { named, .. }) => Self::from_syn(scx, named),
            syn::Fields::Unnamed(syn::FieldsUnnamed { unnamed, .. }) => {
                let len = unnamed.len();
                let mut fields = unnamed.iter().enumerate().map(|(i, field)| {
                    let span = Span::from_locatable(scx, field);
                    let mut field = Field::from_syn(scx, field);
                    field.ident = Ident::from_number(scx, i, span);
                    field
                });
                scx.alloc_slice(len, |_| fields.next().unwrap())
            }
            syn::Fields::Unit => &[],
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct Variant<'scx> {
    pub ident: Ident<'scx>,
    pub kind: VariantKind<'scx>,
}

impl<'scx> FromSyn<'scx, syn::Variant> for Variant<'scx> {
    fn from_syn(scx: &'scx SyntaxCx, input: &syn::Variant) -> Self {
        let ident = Ident::from_syn(scx, &input.ident);
        let kind = match &input.fields {
            syn::Fields::Named(v) => VariantKind::Fields(FromSyn::from_syn(scx, v)),
            syn::Fields::Unnamed(v) => VariantKind::Fields(FromSyn::from_syn(scx, v)),
            syn::Fields::Unit => {
                if let Some((_, expr)) = &input.discriminant {
                    VariantKind::Discriminant(Expr::from_syn(scx, expr))
                } else {
                    VariantKind::Unit
                }
            }
        };
        Self { ident, kind }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub enum VariantKind<'scx> {
    Fields(&'scx [VariantField<'scx>]),
    Discriminant(Expr<'scx>),
    Unit,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct VariantField<'scx> {
    pub ident: Ident<'scx>,
    pub ty: Type<'scx>,
    pub span: Span<'scx>,
}

impl<'scx> FromSyn<'scx, syn::FieldsNamed> for &'scx [VariantField<'scx>] {
    fn from_syn(scx: &'scx SyntaxCx, input: &syn::FieldsNamed) -> Self {
        let len = input.named.len();
        let mut iter = input.named.iter();
        scx.alloc_slice(len, |_| {
            let field = iter.next().unwrap();
            VariantField {
                ident: Ident::from_syn(scx, field.ident.as_ref().unwrap()),
                ty: Type::from_syn(scx, &field.ty),
                span: Span::from_locatable(scx, field),
            }
        })
    }
}

impl<'scx> FromSyn<'scx, syn::FieldsUnnamed> for &'scx [VariantField<'scx>] {
    fn from_syn(scx: &'scx SyntaxCx, input: &syn::FieldsUnnamed) -> Self {
        let len = input.unnamed.len();
        let mut iter = input.unnamed.iter();
        scx.alloc_slice(len, |i| {
            let field = iter.next().unwrap();
            VariantField {
                ident: Ident::from_number(scx, i, Span::empty()),
                ty: Type::from_syn(scx, &field.ty),
                span: Span::from_locatable(scx, field),
            }
        })
    }
}
