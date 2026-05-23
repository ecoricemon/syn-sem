use crate::{Expr, FromSyn, Ident, InputDesc, Path, Span, SyntaxCx, Type};
use syn_sem_macros::CheckDropless;

/// Generic parameters and where-clause information for an item or signature.
///
/// For example, `<T, const N: usize> where T: Clone`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct Generics<'scx> {
    pub params: &'scx [GenericParam<'scx>],
    pub where_clause: Option<WhereClause<'scx>>,
    pub span: Span<'scx>,
}

impl<'scx> FromSyn<'scx, syn::Generics> for Generics<'scx> {
    fn from_syn(scx: &'scx SyntaxCx, desc: InputDesc<'_, syn::Generics>) -> Self {
        Self {
            params: FromSyn::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.params,
                },
            ),
            where_clause: FromSyn::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.where_clause,
                },
            ),
            span: Span::from_locatable(scx, desc.file_path, desc.input),
        }
    }
}

/// One generic parameter declaration.
///
/// Examples include `T`, `T: Clone = i32`, and `const N: usize = 4`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub enum GenericParam<'scx> {
    Type(TypeParam<'scx>),
    Const(ConstParam<'scx>),
    Unsupported(Span<'scx>),
}

impl<'scx> FromSyn<'scx, syn::GenericParam> for GenericParam<'scx> {
    fn from_syn(scx: &'scx SyntaxCx, desc: InputDesc<'_, syn::GenericParam>) -> Self {
        match desc.input {
            syn::GenericParam::Type(v) => Self::Type(TypeParam::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: v,
                },
            )),
            syn::GenericParam::Const(v) => Self::Const(ConstParam::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: v,
                },
            )),
            syn::GenericParam::Lifetime(v) => {
                Self::Unsupported(Span::from_locatable(scx, desc.file_path, v))
            }
        }
    }
}

/// A type generic parameter.
///
/// For example, `T: Clone = i32`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct TypeParam<'scx> {
    pub ident: Ident<'scx>,
    pub bounds: &'scx [TypeParamBound<'scx>],
    pub default: Option<&'scx Type<'scx>>,
    pub span: Span<'scx>,
}

impl<'scx> FromSyn<'scx, syn::TypeParam> for TypeParam<'scx> {
    fn from_syn(scx: &'scx SyntaxCx, desc: InputDesc<'_, syn::TypeParam>) -> Self {
        Self {
            ident: Ident::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.ident,
                },
            ),
            bounds: FromSyn::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.bounds,
                },
            ),
            default: desc.input.default.as_ref().map(|ty| {
                scx.alloc(Type::from_syn(
                    scx,
                    InputDesc {
                        file_path: desc.file_path,
                        input: ty,
                    },
                ))
            }),
            span: Span::from_locatable(scx, desc.file_path, desc.input),
        }
    }
}

/// A const generic parameter.
///
/// For example, `const N: usize = 4`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct ConstParam<'scx> {
    pub ident: Ident<'scx>,
    pub ty: &'scx Type<'scx>,
    pub default: Option<&'scx Expr<'scx>>,
    pub span: Span<'scx>,
}

impl<'scx> FromSyn<'scx, syn::ConstParam> for ConstParam<'scx> {
    fn from_syn(scx: &'scx SyntaxCx, desc: InputDesc<'_, syn::ConstParam>) -> Self {
        Self {
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
            default: desc.input.default.as_ref().map(|expr| {
                scx.alloc(Expr::from_syn(
                    scx,
                    InputDesc {
                        file_path: desc.file_path,
                        input: expr,
                    },
                ))
            }),
            span: Span::from_locatable(scx, desc.file_path, desc.input),
        }
    }
}

/// A where-clause attached to generics.
///
/// For example, `where T: Clone, U: Display`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct WhereClause<'scx> {
    pub predicates: &'scx [WherePredicate<'scx>],
    pub span: Span<'scx>,
}

impl<'scx> FromSyn<'scx, syn::WhereClause> for WhereClause<'scx> {
    fn from_syn(scx: &'scx SyntaxCx, desc: InputDesc<'_, syn::WhereClause>) -> Self {
        Self {
            predicates: FromSyn::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.predicates,
                },
            ),
            span: Span::from_locatable(scx, desc.file_path, desc.input),
        }
    }
}

/// One predicate inside a where-clause.
///
/// For example, `T: Clone` in `where T: Clone`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub enum WherePredicate<'scx> {
    Type(PredicateType<'scx>),
    Unsupported(Span<'scx>),
}

impl<'scx> FromSyn<'scx, syn::WherePredicate> for WherePredicate<'scx> {
    fn from_syn(scx: &'scx SyntaxCx, desc: InputDesc<'_, syn::WherePredicate>) -> Self {
        match desc.input {
            syn::WherePredicate::Type(v) => {
                if v.lifetimes.is_some() {
                    Self::Unsupported(Span::from_locatable(scx, desc.file_path, v))
                } else {
                    Self::Type(PredicateType::from_syn(
                        scx,
                        InputDesc {
                            file_path: desc.file_path,
                            input: v,
                        },
                    ))
                }
            }
            syn::WherePredicate::Lifetime(v) => {
                Self::Unsupported(Span::from_locatable(scx, desc.file_path, v))
            }
            _ => Self::Unsupported(Span::from_locatable(scx, desc.file_path, desc.input)),
        }
    }
}

/// A type-bounds predicate inside a where-clause.
///
/// For example, `T: Clone + Display`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct PredicateType<'scx> {
    pub bounded_ty: &'scx Type<'scx>,
    pub bounds: &'scx [TypeParamBound<'scx>],
    pub span: Span<'scx>,
}

impl<'scx> FromSyn<'scx, syn::PredicateType> for PredicateType<'scx> {
    fn from_syn(scx: &'scx SyntaxCx, desc: InputDesc<'_, syn::PredicateType>) -> Self {
        Self {
            bounded_ty: scx.alloc(Type::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.bounded_ty,
                },
            )),
            bounds: FromSyn::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.bounds,
                },
            ),
            span: Span::from_locatable(scx, desc.file_path, desc.input),
        }
    }
}

/// A bound on a type parameter or associated type constraint.
///
/// For example, `Clone` in `T: Clone`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub enum TypeParamBound<'scx> {
    Trait(TraitBound<'scx>),
    Unsupported(Span<'scx>),
}

impl<'scx> FromSyn<'scx, syn::TypeParamBound> for TypeParamBound<'scx> {
    fn from_syn(scx: &'scx SyntaxCx, desc: InputDesc<'_, syn::TypeParamBound>) -> Self {
        match desc.input {
            syn::TypeParamBound::Trait(v) => {
                if v.paren_token.is_some()
                    || v.lifetimes.is_some()
                    || !matches!(v.modifier, syn::TraitBoundModifier::None)
                {
                    Self::Unsupported(Span::from_locatable(scx, desc.file_path, v))
                } else {
                    Self::Trait(TraitBound::from_syn(
                        scx,
                        InputDesc {
                            file_path: desc.file_path,
                            input: v,
                        },
                    ))
                }
            }
            _ => Self::Unsupported(Span::from_locatable(scx, desc.file_path, desc.input)),
        }
    }
}

/// A trait bound represented by its trait path.
///
/// For example, `Clone` or `std::fmt::Display`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct TraitBound<'scx> {
    pub path: Path<'scx>,
    pub span: Span<'scx>,
}

impl<'scx> FromSyn<'scx, syn::TraitBound> for TraitBound<'scx> {
    fn from_syn(scx: &'scx SyntaxCx, desc: InputDesc<'_, syn::TraitBound>) -> Self {
        Self {
            path: Path::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.path,
                },
            ),
            span: Span::from_locatable(scx, desc.file_path, desc.input),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::*;

    #[test]
    fn test_generics() {
        // Proves empty generics preserve no params and no where clause.
        let scx = SyntaxCx::default();

        let item = parse::<syn::ItemStruct, crate::ItemStruct>(&scx, "struct S;");
        assert!(item.generics.params.is_empty());
        assert!(item.generics.where_clause.is_none());
    }

    #[test]
    fn test_generic_param() {
        // Proves type and const generic params preserve names, defaults, and types.
        let scx = SyntaxCx::default();

        let item = parse::<syn::ItemStruct, crate::ItemStruct>(&scx, "struct S<T, U = i32>;");
        assert_eq!(item.generics.params.len(), 2);

        let GenericParam::Type(param) = &item.generics.params[0] else {
            panic!()
        };
        assert_eq!(&*param.ident, "T");
        assert!(param.default.is_none());

        let GenericParam::Type(param) = &item.generics.params[1] else {
            panic!()
        };
        assert_eq!(&*param.ident, "U");
        assert!(matches!(param.default, Some(Type::Path(_))));

        let item =
            parse::<syn::ItemStruct, crate::ItemStruct>(&scx, "struct S<const N: usize = 4>;");
        assert_eq!(item.generics.params.len(), 1);

        let GenericParam::Const(param) = &item.generics.params[0] else {
            panic!()
        };
        assert_eq!(&*param.ident, "N");
        assert!(matches!(param.ty, Type::Path(_)));
        assert!(param.default.is_some());
    }

    #[test]
    fn test_type_param_bound() {
        // Proves trait bounds preserve their path, including path generic arguments.
        let scx = SyntaxCx::default();

        let item = parse::<syn::ItemStruct, crate::ItemStruct>(&scx, "struct S<T: Clone>;");
        assert_eq!(item.generics.params.len(), 1);

        let GenericParam::Type(param) = &item.generics.params[0] else {
            panic!()
        };
        assert_eq!(&*param.ident, "T");
        assert_eq!(param.bounds.len(), 1);
        let TypeParamBound::Trait(bound) = &param.bounds[0] else {
            panic!()
        };
        assert_eq!(&**bound.path.get_ident().unwrap(), "Clone");

        let item =
            parse::<syn::ItemStruct, crate::ItemStruct>(&scx, "struct S<T: Iterator<Item = U>>;");
        let GenericParam::Type(param) = &item.generics.params[0] else {
            panic!()
        };
        let TypeParamBound::Trait(bound) = &param.bounds[0] else {
            panic!()
        };
        assert_eq!(&*bound.path.segments[0].ident, "Iterator");
        assert!(bound.path.segments[0].has_args());
    }

    #[test]
    fn test_where_clause() {
        // Proves type where-predicates preserve bounded type and bounds.
        let scx = SyntaxCx::default();

        let item = parse::<syn::ItemStruct, crate::ItemStruct>(&scx, "struct S<T> where T: Clone;");
        let where_clause = item.generics.where_clause.as_ref().unwrap();
        assert_eq!(where_clause.predicates.len(), 1);

        let WherePredicate::Type(pred) = &where_clause.predicates[0] else {
            panic!()
        };
        assert!(matches!(pred.bounded_ty, Type::Path(_)));
        assert_eq!(pred.bounds.len(), 1);
    }

    #[test]
    fn test_unsupported() {
        // Proves unsupported generic forms recover as `Unsupported` instead of panicking.
        let scx = SyntaxCx::default();

        let item = parse::<syn::ItemStruct, crate::ItemStruct>(&scx, "struct S<'a>;");
        assert!(matches!(
            item.generics.params[0],
            GenericParam::Unsupported(_)
        ));

        let item = parse::<syn::ItemStruct, crate::ItemStruct>(&scx, "struct S<T: ?Sized>;");
        let GenericParam::Type(param) = &item.generics.params[0] else {
            panic!()
        };
        assert!(matches!(param.bounds[0], TypeParamBound::Unsupported(_)));

        let item = parse::<syn::ItemStruct, crate::ItemStruct>(&scx, "struct S where 'a: 'static;");
        let where_clause = item.generics.where_clause.as_ref().unwrap();
        assert!(matches!(
            where_clause.predicates[0],
            WherePredicate::Unsupported(_)
        ));

        let item = parse::<syn::ItemStruct, crate::ItemStruct>(
            &scx,
            "struct S<T> where for<'a> T: Trait<'a>;",
        );
        let where_clause = item.generics.where_clause.as_ref().unwrap();
        assert!(matches!(
            where_clause.predicates[0],
            WherePredicate::Unsupported(_)
        ));
    }
}
