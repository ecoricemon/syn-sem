use crate::{
    Block, Expr, Field, FromSyn, Ident, InputDesc, Pat, PatIdent, PatType, Span, SyntaxCx, Type,
    Variant, Visibility,
};
use std::iter;
use syn_sem_macros::CheckDropless;

#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub enum Item<'scx> {
    Const(ItemConst<'scx>),
    Enum(ItemEnum<'scx>),
    Fn(ItemFn<'scx>),
    Struct(ItemStruct<'scx>),
}

impl<'scx> FromSyn<'scx, syn::Item> for Item<'scx> {
    fn from_syn(scx: &'scx SyntaxCx, desc: InputDesc<'_, syn::Item>) -> Self {
        match desc.input {
            syn::Item::Const(v) => Item::Const(ItemConst::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: v,
                },
            )),
            syn::Item::Enum(v) => Item::Enum(ItemEnum::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: v,
                },
            )),
            syn::Item::Fn(v) => Item::Fn(ItemFn::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: v,
                },
            )),
            syn::Item::Struct(v) => Item::Struct(ItemStruct::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: v,
                },
            )),
            _ => todo!(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct ItemConst<'scx> {
    pub vis: Visibility<'scx>,
    pub ident: Ident<'scx>,
    pub ty: &'scx Type<'scx>,
    pub init: &'scx Expr<'scx>,
    pub span: Span<'scx>,
}

impl<'scx> FromSyn<'scx, syn::ItemConst> for ItemConst<'scx> {
    fn from_syn(scx: &'scx SyntaxCx, desc: InputDesc<'_, syn::ItemConst>) -> Self {
        Self {
            vis: Visibility::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.vis,
                },
            ),
            ident: Ident::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.ident,
                },
            ),
            ty: scx.alloc(Type::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.ty,
                },
            )),
            init: scx.alloc(Expr::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.expr,
                },
            )),
            span: Span::from_locatable(scx, desc.file_path, desc.input),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct ItemEnum<'scx> {
    pub vis: Visibility<'scx>,
    pub ident: Ident<'scx>,
    pub variants: &'scx [Variant<'scx>],
    pub span: Span<'scx>,
}

impl<'scx> FromSyn<'scx, syn::ItemEnum> for ItemEnum<'scx> {
    fn from_syn(scx: &'scx SyntaxCx, desc: InputDesc<'_, syn::ItemEnum>) -> Self {
        Self {
            vis: Visibility::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.vis,
                },
            ),
            ident: Ident::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.ident,
                },
            ),
            variants: FromSyn::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.variants,
                },
            ),
            span: Span::from_locatable(scx, desc.file_path, desc.input),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct ItemFn<'scx> {
    pub vis: Visibility<'scx>,
    pub sig: Signature<'scx>,
    pub block: Block<'scx>,
    pub span: Span<'scx>,
}

impl<'scx> FromSyn<'scx, syn::ItemFn> for ItemFn<'scx> {
    fn from_syn(scx: &'scx SyntaxCx, desc: InputDesc<'_, syn::ItemFn>) -> Self {
        Self {
            vis: Visibility::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.vis,
                },
            ),
            sig: Signature::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.sig,
                },
            ),
            block: Block::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.block,
                },
            ),
            span: Span::from_locatable(scx, desc.file_path, desc.input),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct ItemStruct<'scx> {
    pub vis: Visibility<'scx>,
    pub ident: Ident<'scx>,
    pub fields: &'scx [Field<'scx>],
    pub span: Span<'scx>,
}

impl<'scx> FromSyn<'scx, syn::ItemStruct> for ItemStruct<'scx> {
    fn from_syn(scx: &'scx SyntaxCx, desc: InputDesc<'_, syn::ItemStruct>) -> Self {
        Self {
            vis: Visibility::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.vis,
                },
            ),
            ident: Ident::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.ident,
                },
            ),
            fields: FromSyn::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.fields,
                },
            ),
            span: Span::from_locatable(scx, desc.file_path, desc.input),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct Signature<'scx> {
    pub ident: Ident<'scx>,
    pub params: &'scx [Parameter<'scx>],
    pub span: Span<'scx>,
}

impl<'scx> FromSyn<'scx, syn::Signature> for Signature<'scx> {
    fn from_syn(scx: &'scx SyntaxCx, desc: InputDesc<'_, syn::Signature>) -> Self {
        let output =
            Parameter::from_return_type(scx, desc.file_path, &desc.input.output, ParameterCx::Fn);
        let output = iter::once(output);
        let inputs = desc.input.inputs.iter().map(|arg| {
            Parameter::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: arg,
                },
            )
        });
        let mut params = output.chain(inputs);
        let len = desc.input.inputs.len() + 1;

        Self {
            ident: Ident::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.ident,
                },
            ),
            params: scx.alloc_slice(len, |_| params.next().unwrap()),
            span: Span::from_locatable(scx, desc.file_path, desc.input),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct Parameter<'scx> {
    pub pat: PatType<'scx>,
    pub span: Span<'scx>,
}

impl<'scx> Parameter<'scx> {
    /// Creates a parameter with the ident `0`.
    pub fn from_return_type(
        scx: &'scx SyntaxCx,
        file_path: &str,
        ret_ty: &syn::ReturnType,
        cx: ParameterCx,
    ) -> Self {
        const IDENT: u32 = 0;

        let span = Span::from_locatable(scx, file_path, ret_ty);
        let ty = match ret_ty {
            syn::ReturnType::Default => match cx {
                ParameterCx::Fn => Type::unit(span),
                ParameterCx::Closure => Type::Infer(span),
            },
            syn::ReturnType::Type(_, ty) => Type::from_syn(
                scx,
                InputDesc {
                    file_path,
                    input: ty,
                },
            ),
        };
        let pat_ident = Pat::Ident(PatIdent::from_number(scx, IDENT, Span::empty()));
        let pat = PatType {
            pat: scx.alloc(pat_ident),
            ty,
            span,
        };
        Self { pat, span }
    }
}

impl<'scx> FromSyn<'scx, syn::FnArg> for Parameter<'scx> {
    fn from_syn(scx: &'scx SyntaxCx, desc: InputDesc<'_, syn::FnArg>) -> Self {
        let span = Span::from_locatable(scx, desc.file_path, desc.input);
        let pat = match desc.input {
            syn::FnArg::Receiver(v) => PatType::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: v,
                },
            ),
            syn::FnArg::Typed(v) => PatType::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: v,
                },
            ),
        };
        Self { pat, span }
    }
}

#[derive(PartialEq, Eq)]
pub enum ParameterCx {
    Fn,
    Closure,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::*;

    #[test]
    fn test_item_struct() {
        type T = syn::ItemStruct;
        type U<'a> = ItemStruct<'a>;
        let scx = SyntaxCx::default();

        // Empty struct
        let st = parse::<T, U>(&scx, "struct A;");
        assert_eq!(&*st.ident.inner, "A");
        assert!(st.fields.is_empty());

        // Tuple struct with zero, one, and two fields.
        let st = parse::<T, U>(&scx, "struct A();");
        assert!(st.fields.is_empty());
        let st = parse::<T, U>(&scx, "struct A(B);");
        assert_eq!(st.fields.len(), 1);
        let st = parse::<T, U>(&scx, "struct A(B, C);");
        assert_eq!(st.fields.len(), 2);

        // Struct with zero, one, and two fields.
        let st = parse::<T, U>(&scx, "struct A{}");
        assert!(st.fields.is_empty());
        let st = parse::<T, U>(&scx, "struct A{ f1: B }");
        assert_eq!(st.fields.len(), 1);
        let st = parse::<T, U>(&scx, "struct A{ f1: B, f2: C }");
        assert_eq!(st.fields.len(), 2);
    }
}
