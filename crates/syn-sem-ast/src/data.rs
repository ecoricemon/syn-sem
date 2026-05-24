use crate::{Expr, FromSyn, Ident, InputDesc, Span, SyntaxCx, Type, Visibility};
use syn_sem_macros::CheckDropless;

/// A struct or union field.
///
/// For example, `pub x: i32` in `struct S { pub x: i32 }`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct Field<'cx> {
    /// Field visibility.
    pub vis: Visibility<'cx>,
    /// Field name, or a synthesized tuple-field index.
    pub ident: Ident<'cx>,
    /// Field type.
    pub ty: Type<'cx>,
    /// Source span of the field.
    pub span: Span<'cx>,
}

impl<'cx> FromSyn<'cx, syn::Field> for Field<'cx> {
    fn from_syn(scx: &'cx SyntaxCx<'cx>, desc: crate::InputDesc<'cx, syn::Field>) -> Self {
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
        let ty = Type::from_syn(
            scx,
            InputDesc {
                file_path: desc.file_path,
                input: &desc.input.ty,
            },
        );
        Self {
            vis,
            ident,
            ty,
            span,
        }
    }
}

impl<'cx> FromSyn<'cx, syn::Fields> for &'cx [Field<'cx>] {
    fn from_syn(scx: &'cx SyntaxCx<'cx>, desc: InputDesc<'cx, syn::Fields>) -> Self {
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

/// An enum variant.
///
/// Examples include `A`, `B(i32)`, `C { value: i32 }`, and `D = 1`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct Variant<'cx> {
    /// Variant name.
    pub ident: Ident<'cx>,
    /// Variant payload or discriminant.
    pub kind: VariantKind<'cx>,
}

impl<'cx> FromSyn<'cx, syn::Variant> for Variant<'cx> {
    fn from_syn(scx: &'cx SyntaxCx<'cx>, desc: InputDesc<'cx, syn::Variant>) -> Self {
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

/// The payload shape of an enum variant.
///
/// For example, `Some(T)` has fields, `None` is unit, and `A = 1` has a discriminant.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub enum VariantKind<'cx> {
    /// Variant with named or unnamed fields.
    Fields(&'cx [VariantField<'cx>]),
    /// Unit variant with an explicit discriminant expression.
    Discriminant(Expr<'cx>),
    /// Unit variant without a discriminant.
    Unit,
}

/// A field inside an enum variant payload.
///
/// For example, `value: i32` in `Variant { value: i32 }`, or synthesized index `0` in `Variant(i32)`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct VariantField<'cx> {
    /// Field name, or a synthesized tuple-field index.
    pub ident: Ident<'cx>,
    /// Field type.
    pub ty: Type<'cx>,
    /// Source span of the field.
    pub span: Span<'cx>,
}

impl<'cx> FromSyn<'cx, syn::FieldsNamed> for &'cx [VariantField<'cx>] {
    fn from_syn(scx: &'cx SyntaxCx<'cx>, desc: InputDesc<'cx, syn::FieldsNamed>) -> Self {
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

impl<'cx> FromSyn<'cx, syn::FieldsUnnamed> for &'cx [VariantField<'cx>] {
    fn from_syn(scx: &'cx SyntaxCx<'cx>, desc: InputDesc<'cx, syn::FieldsUnnamed>) -> Self {
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
