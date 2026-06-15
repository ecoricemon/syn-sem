use syn_sem_ast as ast;
use syn_sem_name::Name;

pub(crate) fn generic_predicates<'cx>(
    generics: &'cx ast::Generics<'cx>,
) -> Vec<WherePredicate<'cx>> {
    let mut predicates = Vec::new();
    predicates.extend(generics.params.iter().filter_map(inline_bound_predicate));
    if let Some(where_clause) = &generics.where_clause {
        predicates.extend(where_clause.predicates.iter().map(source_where_predicate));
    }
    predicates
}

fn inline_bound_predicate<'cx>(param: &'cx ast::GenericParam<'cx>) -> Option<WherePredicate<'cx>> {
    let ast::GenericParam::Type(param) = param else {
        return None;
    };
    (!param.bounds.is_empty()).then_some(WherePredicate::TypeBound(TypeBoundPredicate {
        subject: PredicateSubject::TypeParam(param.ident.inner),
        bounds: param.bounds,
    }))
}

fn source_where_predicate<'cx>(predicate: &'cx ast::WherePredicate<'cx>) -> WherePredicate<'cx> {
    match predicate {
        ast::WherePredicate::Type(predicate) => WherePredicate::TypeBound(TypeBoundPredicate {
            subject: PredicateSubject::Type(predicate.bounded_ty),
            bounds: predicate.bounds,
        }),
        ast::WherePredicate::Unsupported(_) => WherePredicate::Unsupported,
    }
}

pub(crate) enum WherePredicate<'cx> {
    TypeBound(TypeBoundPredicate<'cx>),
    Unsupported,
}

pub(crate) struct TypeBoundPredicate<'cx> {
    pub(crate) subject: PredicateSubject<'cx>,
    pub(crate) bounds: &'cx [ast::TypeParamBound<'cx>],
}

pub(crate) enum PredicateSubject<'cx> {
    TypeParam(Name<'cx>),
    Type(&'cx ast::Type<'cx>),
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn_sem_ast::SyntaxCx;
    use syn_sem_common::CommonCx;

    fn parse_struct_generics<'cx>(
        ccx: &'cx CommonCx,
        scx: &'cx SyntaxCx<'cx>,
        code: &str,
    ) -> &'cx ast::Generics<'cx> {
        let file_path = ccx.intern("lower_test.rs");
        let text = ccx.intern(code);
        scx.parse_virtual_file(file_path, text)
            .expect("test input should parse");
        let file = scx.lookup_source(file_path).unwrap().ast();
        let Some(ast::Item::Struct(item)) = file.items.first() else {
            panic!("expected struct item");
        };
        &item.generics
    }

    #[test]
    fn lowers_inline_type_param_bounds_into_predicates() {
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let generics = parse_struct_generics(&ccx, &scx, "struct S<T: Clone>;");

        let predicates = generic_predicates(generics);

        assert_eq!(predicates.len(), 1);
        let WherePredicate::TypeBound(predicate) = &predicates[0] else {
            panic!("expected type-bound predicate");
        };
        let PredicateSubject::TypeParam(name) = predicate.subject else {
            panic!("expected type parameter subject");
        };
        assert_eq!(name.as_ref(), "T");
        assert_eq!(predicate.bounds.len(), 1);
    }

    #[test]
    fn appends_source_where_clause_predicates_after_inline_bounds() {
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let generics = parse_struct_generics(&ccx, &scx, "struct S<T: Clone> where T: Iterator;");

        let predicates = generic_predicates(generics);

        assert_eq!(predicates.len(), 2);
        let WherePredicate::TypeBound(inline) = &predicates[0] else {
            panic!("expected inline type-bound predicate");
        };
        let PredicateSubject::TypeParam(name) = inline.subject else {
            panic!("expected type parameter subject");
        };
        assert_eq!(name.as_ref(), "T");

        let WherePredicate::TypeBound(where_predicate) = &predicates[1] else {
            panic!("expected where-clause type-bound predicate");
        };
        let PredicateSubject::Type(ty) = where_predicate.subject else {
            panic!("expected source type subject");
        };
        assert!(matches!(ty, ast::Type::Path(_)));
    }
}
