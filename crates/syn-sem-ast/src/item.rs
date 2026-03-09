use crate::{Expr, Field, FromSyn, Ident, Span, SyntaxContext, Type};
use syn_sem_macros::CheckDropless;

#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub enum Item<'scx> {
    Const(ItemConst<'scx>),
    Struct(ItemStruct<'scx>),
}

impl<'scx> FromSyn<'scx, syn::Item> for Item<'scx> {
    fn from_syn(scx: &'scx SyntaxContext, input: &syn::Item) -> Self {
        match input {
            syn::Item::Const(v) => Item::Const(ItemConst::from_syn(scx, v)),
            syn::Item::Struct(v) => Item::Struct(ItemStruct::from_syn(scx, v)),
            _ => todo!(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct ItemConst<'scx> {
    ident: Ident<'scx>,
    ty: &'scx Type<'scx>,
    init: &'scx Expr<'scx>,
    span: Span<'scx>,
}

impl<'scx> FromSyn<'scx, syn::ItemConst> for ItemConst<'scx> {
    fn from_syn(scx: &'scx SyntaxContext, input: &syn::ItemConst) -> Self {
        Self {
            ident: Ident::from_syn(scx, &input.ident),
            ty: scx.alloc(Type::from_syn(scx, &input.ty)),
            init: scx.alloc(Expr::from_syn(scx, &input.expr)),
            span: Span::from_locatable(scx, input),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct ItemStruct<'scx> {
    ident: Ident<'scx>,
    fields: &'scx [Field<'scx>],
    span: Span<'scx>,
}

impl<'scx> FromSyn<'scx, syn::ItemStruct> for ItemStruct<'scx> {
    fn from_syn(scx: &'scx SyntaxContext, input: &syn::ItemStruct) -> Self {
        Self {
            ident: Ident::from_syn(scx, &input.ident),
            fields: FromSyn::from_syn(scx, &input.fields),
            span: Span::from_locatable(scx, input),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::*;

    #[test]
    fn test_item_struct() {
        type T = syn::ItemStruct;
        type U<'a> = ItemStruct<'a>;
        let scx = create_context();

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
