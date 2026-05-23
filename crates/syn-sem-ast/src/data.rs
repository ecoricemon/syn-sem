use crate::{Expr, FromSyn, Ident, InputDesc, Span, SyntaxCx, Type, Visibility};
use syn_sem_macros::CheckDropless;

#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct Field<'scx> {
    pub vis: Visibility<'scx>,
    pub ident: Ident<'scx>,
    pub span: Span<'scx>,
}

impl<'scx> FromSyn<'scx, syn::Field> for Field<'scx> {
    fn from_syn(scx: &'scx SyntaxCx, desc: crate::InputDesc<'_, syn::Field>) -> Self {
        let vis = Visibility::from_syn(
            scx,
            InputDesc {
                file_path: desc.file_path,
                input: &desc.input.vis,
            },
        );
        let ident = desc
            .input
            .ident
            .as_ref()
            .map(|ident| {
                Ident::from_syn(
                    scx,
                    InputDesc {
                        file_path: desc.file_path,
                        input: ident,
                    },
                )
            })
            .unwrap_or(Ident::empty(scx));
        let span = Span::from_locatable(scx, desc.file_path, desc.input);
        Self { vis, ident, span }
    }
}

impl<'scx> FromSyn<'scx, syn::Fields> for &'scx [Field<'scx>] {
    fn from_syn(scx: &'scx SyntaxCx, desc: InputDesc<'_, syn::Fields>) -> Self {
        match desc.input {
            syn::Fields::Named(syn::FieldsNamed { named, .. }) => Self::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: named,
                },
            ),
            syn::Fields::Unnamed(syn::FieldsUnnamed { unnamed, .. }) => {
                let len = unnamed.len();
                let mut fields = unnamed.iter().enumerate().map(|(i, field)| {
                    let span = Span::from_locatable(scx, desc.file_path, field);
                    let mut field = Field::from_syn(
                        scx,
                        InputDesc {
                            file_path: desc.file_path,
                            input: field,
                        },
                    );
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
    fn from_syn(scx: &'scx SyntaxCx, desc: InputDesc<'_, syn::Variant>) -> Self {
        let ident = Ident::from_syn(
            scx,
            InputDesc {
                file_path: desc.file_path,
                input: &desc.input.ident,
            },
        );
        let kind = match &desc.input.fields {
            syn::Fields::Named(v) => VariantKind::Fields(FromSyn::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: v,
                },
            )),
            syn::Fields::Unnamed(v) => VariantKind::Fields(FromSyn::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: v,
                },
            )),
            syn::Fields::Unit => {
                if let Some((_, expr)) = &desc.input.discriminant {
                    VariantKind::Discriminant(Expr::from_syn(
                        scx,
                        InputDesc {
                            file_path: desc.file_path,
                            input: expr,
                        },
                    ))
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
    fn from_syn(scx: &'scx SyntaxCx, desc: InputDesc<'_, syn::FieldsNamed>) -> Self {
        let len = desc.input.named.len();
        let mut iter = desc.input.named.iter();
        scx.alloc_slice(len, |_| {
            let field = iter.next().unwrap();
            VariantField {
                ident: Ident::from_syn(
                    scx,
                    InputDesc {
                        file_path: desc.file_path,
                        input: field.ident.as_ref().unwrap(),
                    },
                ),
                ty: Type::from_syn(
                    scx,
                    InputDesc {
                        file_path: desc.file_path,
                        input: &field.ty,
                    },
                ),
                span: Span::from_locatable(scx, desc.file_path, field),
            }
        })
    }
}

impl<'scx> FromSyn<'scx, syn::FieldsUnnamed> for &'scx [VariantField<'scx>] {
    fn from_syn(scx: &'scx SyntaxCx, desc: InputDesc<'_, syn::FieldsUnnamed>) -> Self {
        let len = desc.input.unnamed.len();
        let mut iter = desc.input.unnamed.iter();
        scx.alloc_slice(len, |i| {
            let field = iter.next().unwrap();
            VariantField {
                ident: Ident::from_number(scx, i, Span::empty()),
                ty: Type::from_syn(
                    scx,
                    InputDesc {
                        file_path: desc.file_path,
                        input: &field.ty,
                    },
                ),
                span: Span::from_locatable(scx, desc.file_path, field),
            }
        })
    }
}
