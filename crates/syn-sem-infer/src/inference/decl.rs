//! Declaration type inference.
//!
//! This module lowers explicit source type occurrences from `syn-sem-pr` into inference
//! [`TypeId`](crate::TypeId)s, collects declaration-level facts such as trait bounds and impl
//! associated type assignments, and then invokes logic-backed derivation for projection solving.

use crate::{
    ArrayLen, GenericArgument, InferDb, Path, PathSegment, PathType, PathTypeResolution,
    PrimitiveType, ProjectionObligation, ProjectionType, QSelf, SourceConstArg, SourceTypeBounds,
    TraitBoundFact, Type, TypeId,
};
use syn_sem_common::CommonCx;
use syn_sem_name::{DefId, DefKind, NameDb, Namespace, ResolveResult, ScopeId};
use syn_sem_pr as pr;

pub(crate) fn analyze<'cx>(
    ccx: &'cx CommonCx,
    repr: &pr::ProgramRepr<'cx>,
    names: &NameDb<'cx>,
) -> InferDb<'cx> {
    let mut db = InferCx::new(repr, names).lower();
    crate::logic::derive(ccx, &mut db, names);
    db
}

struct InferCx<'a, 'cx> {
    repr: &'a pr::ProgramRepr<'cx>,
    names: &'a NameDb<'cx>,
    db: InferDb<'cx>,
}

impl<'a, 'cx> InferCx<'a, 'cx> {
    fn new(repr: &'a pr::ProgramRepr<'cx>, names: &'a NameDb<'cx>) -> Self {
        Self {
            repr,
            names,
            db: InferDb::default(),
        }
    }

    fn lower(mut self) -> InferDb<'cx> {
        self.lower_trait_bound_facts();
        self.lower_repr_types();
        self.lower_assoc_type_impl_facts();
        self.db
    }

    fn lower_trait_bound_facts(&mut self) {
        for item in self.repr.items() {
            let generics = match &item.kind {
                pr::ItemKind::Enum { generics, .. }
                | pr::ItemKind::Fn { generics, .. }
                | pr::ItemKind::Impl { generics, .. }
                | pr::ItemKind::Struct { generics, .. }
                | pr::ItemKind::Trait { generics, .. }
                | pr::ItemKind::Type { generics, .. } => Some(generics.clone()),
                pr::ItemKind::Const { .. } | pr::ItemKind::Mod { .. } | pr::ItemKind::Use => None,
            };
            let Some(generics) = generics else {
                continue;
            };
            self.lower_generics(&generics);
        }
    }

    fn lower_generics(&mut self, generics: &pr::Generics<'cx>) {
        for param in &generics.params {
            let pr::GenericParam::Type(param) = param else {
                continue;
            };
            let subject = self.lower_name_as_type(param.name, generics.scope);
            for bound in &param.bounds {
                let pr::TypeParamBound::Trait(bound) = bound else {
                    continue;
                };
                let trait_ty = self.lower_path_value_as_type(&bound.path, generics.scope);
                self.db
                    .trait_bound_facts
                    .push(TraitBoundFact { subject, trait_ty });
            }
        }
    }

    fn lower_repr_types(&mut self) {
        for ty in self.repr.types() {
            self.lower_repr_type(ty.id);
        }
    }

    fn lower_assoc_type_impl_facts(&mut self) {
        for item in self.repr.items() {
            let pr::ItemKind::Impl {
                trait_,
                self_ty,
                items,
                ..
            } = &item.kind
            else {
                continue;
            };
            let Some(trait_) = trait_ else {
                continue;
            };
            let impl_self_ty = self.lower_repr_type(*self_ty);
            let trait_ty = self.lower_path_as_type(trait_, item.parent_scope);
            let Some(trait_def) = self.trait_def_for_type(trait_ty) else {
                continue;
            };
            for assoc_item in items.iter().map(|id| &self.repr[*id]) {
                let pr::AssocItemKind::ImplType { ty } = assoc_item.kind else {
                    continue;
                };
                let ResolveResult::Found(assoc_type) =
                    self.names
                        .member(trait_def, Namespace::Type, assoc_item.name)
                else {
                    continue;
                };
                let value_ty = self.lower_repr_type(ty);
                self.db
                    .assoc_type_impl_facts
                    .push(crate::AssocTypeImplFact {
                        impl_self_ty,
                        trait_ty,
                        assoc_type,
                        value_ty,
                    });
            }
        }
    }

    fn trait_def_for_type(&self, ty: TypeId) -> Option<DefId> {
        let Type::Path(path) = &self.db.types[ty.index()] else {
            return None;
        };
        let PathTypeResolution::Nominal(def) = path.resolution else {
            return None;
        };
        if self.names[def].kind != DefKind::Trait {
            return None;
        }
        Some(def)
    }

    fn lower_repr_type(&mut self, repr_type: pr::TypeId) -> TypeId {
        if let Some(id) = self.db.type_for_repr_type(repr_type) {
            return id;
        }

        let ty = self.lower_type(repr_type, &self.repr[repr_type].kind);
        let id = self.push_type(ty);
        self.db.repr_types.insert(repr_type, id);
        id
    }

    fn lower_type(&mut self, repr_type: pr::TypeId, kind: &pr::TypeKind<'cx>) -> Type<'cx> {
        match kind {
            pr::TypeKind::Array { elem, len } => Type::Array {
                elem: self.lower_repr_type(*elem),
                len: ArrayLen::from_repr(*len),
            },
            pr::TypeKind::Infer => Type::Infer,
            pr::TypeKind::Path(path) => self.lower_path_type(repr_type, path),
            pr::TypeKind::Reference { elem, is_mut } => Type::Reference {
                elem: self.lower_repr_type(*elem),
                is_mut: *is_mut,
            },
            pr::TypeKind::Slice { elem } => Type::Slice {
                elem: self.lower_repr_type(*elem),
            },
            pr::TypeKind::Tuple { elems } => Type::Tuple {
                elems: elems
                    .iter()
                    .map(|elem| self.lower_repr_type(*elem))
                    .collect(),
            },
        }
    }

    fn lower_path_type(&mut self, repr_type: pr::TypeId, path: &pr::TypePath<'cx>) -> Type<'cx> {
        if path.qself.is_none() {
            if let Some(primitive) = PrimitiveType::from_repr_path(&path.path) {
                return Type::Primitive(primitive);
            }
        }

        let scope = self.repr[repr_type].scope;
        let qself = self.lower_qself(path.qself.as_ref(), scope);
        let resolution = self.resolve_path_value(scope, &path.path, qself.as_ref());

        Type::Path(PathType {
            qself,
            path: self.lower_path_value(&path.path),
            resolution,
        })
    }

    fn lower_qself(
        &mut self,
        qself: Option<&pr::QSelf<'cx>>,
        scope: Option<ScopeId>,
    ) -> Option<QSelf> {
        let qself = qself?;
        Some(QSelf {
            self_ty: self.lower_repr_type(qself.self_ty),
            trait_ty: qself
                .trait_path
                .as_ref()
                .map(|path| self.lower_path_value_as_type(path, scope)),
        })
    }

    fn lower_path_value_as_type(
        &mut self,
        path: &pr::TypePathValue<'cx>,
        scope: Option<ScopeId>,
    ) -> TypeId {
        let ty = Type::Path(PathType {
            qself: None,
            path: self.lower_path_value(path),
            resolution: self.resolve_path_value(scope, path, None),
        });
        self.push_type(ty)
    }

    fn lower_path_as_type(&mut self, path: &pr::Path<'cx>, scope: Option<ScopeId>) -> TypeId {
        let path = pr::TypePathValue {
            segments: path
                .segments
                .iter()
                .map(|name| pr::TypePathSegment {
                    name: *name,
                    args: Default::default(),
                })
                .collect(),
        };
        self.lower_path_value_as_type(&path, scope)
    }

    fn lower_name_as_type(
        &mut self,
        name: syn_sem_name::Name<'cx>,
        scope: Option<ScopeId>,
    ) -> TypeId {
        let path = pr::TypePathValue {
            segments: std::iter::once(pr::TypePathSegment {
                name,
                args: Default::default(),
            })
            .collect(),
        };
        self.lower_path_value_as_type(&path, scope)
    }

    fn resolve_path_value(
        &self,
        scope: Option<ScopeId>,
        path: &pr::TypePathValue<'cx>,
        qself: Option<&QSelf>,
    ) -> PathTypeResolution {
        if let Some(projection) = self.resolve_qself_trait_member(path, qself) {
            return projection;
        }

        let Some(scope) = scope else {
            return PathTypeResolution::Unresolved;
        };
        match self
            .names
            .resolve_type_path(scope, path.segments.iter().map(|segment| segment.name))
        {
            ResolveResult::Found(def) => self.classify_path_target(def, qself),
            ResolveResult::Ambiguous(defs) => {
                PathTypeResolution::Ambiguous(defs.into_iter().collect())
            }
            ResolveResult::NotFound => PathTypeResolution::Unresolved,
        }
    }

    fn resolve_qself_trait_member(
        &self,
        path: &pr::TypePathValue<'cx>,
        qself: Option<&QSelf>,
    ) -> Option<PathTypeResolution> {
        let qself = qself?;
        let trait_ty = qself.trait_ty?;
        let trait_def = self.trait_def_for_type(trait_ty)?;
        let assoc_name = path.segments.last()?.name;
        let ResolveResult::Found(assoc_type) =
            self.names.member(trait_def, Namespace::Type, assoc_name)
        else {
            return None;
        };
        if self.names[assoc_type].kind != DefKind::AssocType {
            return None;
        }
        Some(PathTypeResolution::Projection(ProjectionType {
            assoc_type,
            self_ty: Some(qself.self_ty),
            trait_ty: Some(trait_ty),
        }))
    }

    fn classify_path_target(&self, def: DefId, qself: Option<&QSelf>) -> PathTypeResolution {
        match self.names[def].kind {
            DefKind::Struct
            | DefKind::Enum
            | DefKind::Variant
            | DefKind::Trait
            | DefKind::TypeAlias => PathTypeResolution::Nominal(def),
            DefKind::GenericType => PathTypeResolution::GenericParam(def),
            DefKind::AssocType => PathTypeResolution::Projection(ProjectionType {
                assoc_type: def,
                self_ty: qself.map(|qself| qself.self_ty),
                trait_ty: qself.and_then(|qself| qself.trait_ty),
            }),
            _ => PathTypeResolution::Unresolved,
        }
    }

    fn lower_path_value(&mut self, path: &pr::TypePathValue<'cx>) -> Path<'cx> {
        Path {
            segments: path
                .segments
                .iter()
                .map(|segment| self.lower_path_segment(segment))
                .collect(),
        }
    }

    fn lower_path_segment(&mut self, segment: &pr::TypePathSegment<'cx>) -> PathSegment<'cx> {
        PathSegment {
            name: segment.name,
            args: segment
                .args
                .iter()
                .map(|arg| self.lower_generic_arg(arg))
                .collect(),
        }
    }

    fn lower_generic_arg(&mut self, arg: &pr::GenericArgument<'cx>) -> GenericArgument<'cx> {
        match arg {
            pr::GenericArgument::Type(ty) => GenericArgument::Type(self.lower_repr_type(*ty)),
            pr::GenericArgument::Const(_) => GenericArgument::Const(SourceConstArg),
            pr::GenericArgument::AssocType { name, ty } => GenericArgument::AssocType {
                name: *name,
                ty: self.lower_repr_type(*ty),
            },
            pr::GenericArgument::AssocConst { name, .. } => GenericArgument::AssocConst {
                name: *name,
                value: SourceConstArg,
            },
            pr::GenericArgument::Constraint { name, .. } => GenericArgument::Constraint {
                name: *name,
                bounds: SourceTypeBounds,
            },
            pr::GenericArgument::Unsupported => GenericArgument::Unsupported,
        }
    }

    fn next_type_id(&self) -> TypeId {
        TypeId::new(self.db.types.len())
    }

    fn push_type(&mut self, ty: Type<'cx>) -> TypeId {
        let id = self.next_type_id();
        self.collect_projection_obligation(id, &ty);
        self.db.types.push(ty);
        id
    }

    fn collect_projection_obligation(&mut self, id: TypeId, ty: &Type<'cx>) {
        let Type::Path(path) = ty else {
            return;
        };
        let PathTypeResolution::Projection(projection) = &path.resolution else {
            return;
        };
        self.db.projection_obligations.push(ProjectionObligation {
            projection: id,
            assoc_type: projection.assoc_type,
            self_ty: projection.self_ty,
            trait_ty: projection.trait_ty,
        });
    }
}

#[cfg(test)]
mod tests {
    use crate::*;
    use syn_sem_ast::{self as ast, SyntaxCx};
    use syn_sem_common::CommonCx;
    use syn_sem_name::{
        collect::{collect_names, FileInput},
        AstNodeId, DefKind, NameDb, Origin, ScopeKind, Visibility,
    };
    use syn_sem_pr as pr;

    fn infer_types<'cx>(
        ccx: &'cx CommonCx,
        scx: &'cx SyntaxCx<'cx>,
        code: &str,
    ) -> (pr::ProgramRepr<'cx>, InferDb<'cx>) {
        let file_path = ccx.intern("test.rs");
        let text = ccx.intern(code);
        scx.parse_virtual_file(file_path, text).unwrap();
        let file = scx.lookup_source(file_path).unwrap().ast();
        let names = NameDb::default();
        let repr = pr::ProgramReprBuilder::new(&names).build(file_path, file);
        let infer = InferDb::analyze(ccx, &repr, &names);
        (repr, infer)
    }

    fn infer_types_with_names<'cx>(
        ccx: &'cx CommonCx,
        scx: &'cx SyntaxCx<'cx>,
        code: &str,
        names: &NameDb<'cx>,
    ) -> (pr::ProgramRepr<'cx>, InferDb<'cx>) {
        let file_path = ccx.intern("test.rs");
        let text = ccx.intern(code);
        scx.parse_virtual_file(file_path, text).unwrap();
        let file = scx.lookup_source(file_path).unwrap().ast();
        let repr = pr::ProgramReprBuilder::new(names).build(file_path, file);
        let infer = InferDb::analyze(ccx, &repr, names);
        (repr, infer)
    }

    fn infer_collected_types<'cx>(
        ccx: &'cx CommonCx,
        scx: &'cx SyntaxCx<'cx>,
        code: &str,
    ) -> (pr::ProgramRepr<'cx>, NameDb<'cx>, InferDb<'cx>) {
        let file_path = ccx.intern("test.rs");
        let text = ccx.intern(code);
        scx.parse_virtual_file(file_path, text).unwrap();
        let file = scx.lookup_source(file_path).unwrap().ast();
        let names = collect_names([FileInput { file_path, file }], file_path).unwrap();
        let repr = pr::ProgramReprBuilder::new(&names).build(file_path, file);
        let infer = InferDb::analyze(ccx, &repr, &names);
        (repr, names, infer)
    }

    fn struct_field_path_type<'a, 'cx>(
        repr: &'a pr::ProgramRepr<'cx>,
        infer: &'a InferDb<'cx>,
    ) -> &'a PathType<'cx> {
        let id = struct_field_type_id(repr, infer);
        let Type::Path(path) = &infer[id] else {
            panic!("struct field type should lower to path type");
        };
        path
    }

    fn struct_field_type_id<'cx>(repr: &pr::ProgramRepr<'cx>, infer: &InferDb<'cx>) -> TypeId {
        let repr_type = repr
            .types()
            .iter()
            .find(|source| matches!(source.source, pr::TypeSource::StructField))
            .unwrap();
        infer.type_for_repr_type(repr_type.id).unwrap()
    }

    fn struct_field_repr_types<'cx>(
        repr: &pr::ProgramRepr<'cx>,
        struct_name: &str,
    ) -> Vec<pr::TypeId> {
        let item = repr
            .items()
            .iter()
            .find(|item| item.name.is_some_and(|name| name.as_ref() == struct_name))
            .expect("struct item should be represented");
        let pr::ItemKind::Struct { fields, .. } = &item.kind else {
            panic!("item should be represented as a struct");
        };
        fields.iter().map(|field| repr[*field].ty).collect()
    }

    fn struct_field_path_resolution<'cx>(
        repr: &pr::ProgramRepr<'cx>,
        infer: &InferDb<'cx>,
    ) -> PathTypeResolution {
        struct_field_path_type(repr, infer).resolution.clone()
    }

    #[test]
    fn lowers_single_segment_primitives_to_primitive_types() {
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let (repr, infer) = infer_types(
            &ccx,
            &scx,
            "fn f(a: bool, b: char, c: str, d: i32, e: usize, f: f64) {}",
        );

        let lowered: Vec<_> = repr
            .types()
            .iter()
            .filter_map(|repr_type| infer.type_for_repr_type(repr_type.id))
            .map(|id| &infer[id])
            .filter_map(|ty| match ty {
                Type::Primitive(primitive) => Some(*primitive),
                _ => None,
            })
            .collect();

        assert!(lowered.contains(&PrimitiveType::Bool));
        assert!(lowered.contains(&PrimitiveType::Char));
        assert!(lowered.contains(&PrimitiveType::Str));
        assert!(lowered.contains(&PrimitiveType::I32));
        assert!(lowered.contains(&PrimitiveType::Usize));
        assert!(lowered.contains(&PrimitiveType::F64));
    }

    #[test]
    fn keeps_non_primitive_paths_as_paths() {
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let (repr, infer) = infer_types(&ccx, &scx, "struct S { field: crate::usize }");

        let repr_type = repr
            .types()
            .iter()
            .find(|source| matches!(source.source, pr::TypeSource::StructField))
            .unwrap();
        let id = infer.type_for_repr_type(repr_type.id).unwrap();
        let Type::Path(path) = &infer[id] else {
            panic!("qualified primitive-looking path should remain a path");
        };

        assert_eq!(path.path.segments.len(), 2);
        assert_eq!(path.path.segments[0].name.as_ref(), "crate");
        assert_eq!(path.path.segments[1].name.as_ref(), "usize");
        assert_eq!(path.resolution, PathTypeResolution::Unresolved);
    }

    #[test]
    fn classifies_nominal_path_targets() {
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let local = ccx.intern("Local");
        let mut names = NameDb::default();
        let local_def = names.add_def(
            names.root_scope(),
            DefKind::Struct,
            Some(local),
            Visibility::Private,
            Origin::Untracked,
        );
        let (repr, infer) = infer_types_with_names(&ccx, &scx, "struct S { field: Local }", &names);

        assert_eq!(
            struct_field_path_resolution(&repr, &infer),
            PathTypeResolution::Nominal(local_def)
        );
    }

    #[test]
    fn classifies_generic_type_parameters_separately_from_nominal_types() {
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let t = ccx.intern("T");
        let mut names = NameDb::default();
        let t_def = names.add_def(
            names.root_scope(),
            DefKind::GenericType,
            Some(t),
            Visibility::Private,
            Origin::Untracked,
        );
        let (repr, infer) = infer_types_with_names(&ccx, &scx, "struct S { field: T }", &names);

        assert_eq!(
            struct_field_path_resolution(&repr, &infer),
            PathTypeResolution::GenericParam(t_def)
        );
    }

    #[test]
    fn classifies_associated_type_targets_as_projection_candidates() {
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let item = ccx.intern("Item");
        let mut names = NameDb::default();
        let item_def = names.add_def(
            names.root_scope(),
            DefKind::AssocType,
            Some(item),
            Visibility::Private,
            Origin::Untracked,
        );
        let (repr, infer) = infer_types_with_names(&ccx, &scx, "struct S { field: Item }", &names);

        assert_eq!(
            struct_field_path_resolution(&repr, &infer),
            PathTypeResolution::Projection(ProjectionType {
                assoc_type: item_def,
                self_ty: None,
                trait_ty: None,
            })
        );
    }

    #[test]
    fn lowers_qualified_associated_type_paths_for_projection_solving() {
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let t = ccx.intern("T");
        let a = ccx.intern("a");
        let b = ccx.intern("b");
        let trait_name = ccx.intern("Trait");
        let item = ccx.intern("Item");
        let mut names = NameDb::default();
        let root = names.root_scope();
        let t_def = names.add_def(
            root,
            DefKind::GenericType,
            Some(t),
            Visibility::Private,
            Origin::Untracked,
        );
        let a_def = names.add_def(
            root,
            DefKind::Module,
            Some(a),
            Visibility::Public,
            Origin::Untracked,
        );
        let a_scope = names.add_scope(syn_sem_name::ScopeKind::Module, Some(root));
        names.set_path_scope(a_def, a_scope);
        let b_def = names.add_def(
            a_scope,
            DefKind::Module,
            Some(b),
            Visibility::Public,
            Origin::Untracked,
        );
        let b_scope = names.add_scope(syn_sem_name::ScopeKind::Module, Some(a_scope));
        names.set_path_scope(b_def, b_scope);
        let trait_def = names.add_def(
            b_scope,
            DefKind::Trait,
            Some(trait_name),
            Visibility::Public,
            Origin::Untracked,
        );
        let trait_scope = names.add_scope(syn_sem_name::ScopeKind::Trait, Some(b_scope));
        names.set_path_scope(trait_def, trait_scope);
        let item_def = names.add_def(
            trait_scope,
            DefKind::AssocType,
            Some(item),
            Visibility::Public,
            Origin::Untracked,
        );
        let (repr, infer) = infer_types_with_names(
            &ccx,
            &scx,
            "struct S { field: <T as a::b::Trait>::Item }",
            &names,
        );

        let path = struct_field_path_type(&repr, &infer);
        let projection = struct_field_type_id(&repr, &infer);
        let qself = path.qself.expect("qualified path should lower qself");
        let trait_ty = qself
            .trait_ty
            .expect("qualified path should lower trait path");

        assert_eq!(path.path.segments.len(), 4);
        assert_eq!(path.path.segments[0].name.as_ref(), "a");
        assert_eq!(path.path.segments[1].name.as_ref(), "b");
        assert_eq!(path.path.segments[2].name.as_ref(), "Trait");
        assert_eq!(path.path.segments[3].name.as_ref(), "Item");
        assert!(matches!(
            infer[qself.self_ty],
            Type::Path(PathType {
                resolution: PathTypeResolution::GenericParam(def),
                ..
            }) if def == t_def
        ));
        let Type::Path(lowered_trait_path) = &infer[trait_ty] else {
            panic!("qself trait type should lower to path type");
        };
        assert_eq!(lowered_trait_path.path.segments.len(), 3);
        assert_eq!(lowered_trait_path.path.segments[0].name.as_ref(), "a");
        assert_eq!(lowered_trait_path.path.segments[1].name.as_ref(), "b");
        assert_eq!(lowered_trait_path.path.segments[2].name.as_ref(), "Trait");
        assert!(matches!(
            infer[trait_ty],
            Type::Path(PathType {
                resolution: PathTypeResolution::Nominal(def),
                ..
            }) if def == trait_def
        ));
        assert_eq!(
            path.resolution,
            PathTypeResolution::Projection(ProjectionType {
                assoc_type: item_def,
                self_ty: Some(qself.self_ty),
                trait_ty: Some(trait_ty),
            })
        );
        assert_eq!(
            infer.projection_candidates(),
            &[ProjectionCandidate {
                projection,
                self_ty: qself.self_ty,
                assoc_type: item_def,
                trait_ty,
            }]
        );
        assert_eq!(
            infer.projection_matches(),
            &[ProjectionMatch {
                projection,
                self_ty: qself.self_ty,
                assoc_type: item_def,
                trait_ty,
            }]
        );
    }

    #[test]
    fn lowers_traitless_qualified_associated_type_paths_as_projection_candidates() {
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let t = ccx.intern("T");
        let iterator = ccx.intern("Iterator");
        let item = ccx.intern("Item");
        let mut names = NameDb::default();
        let root = names.root_scope();
        let t_def = names.add_def(
            root,
            DefKind::GenericType,
            Some(t),
            Visibility::Private,
            Origin::Untracked,
        );
        let iterator_def = names.add_def(
            root,
            DefKind::Trait,
            Some(iterator),
            Visibility::Private,
            Origin::Untracked,
        );
        let iterator_scope = names.add_scope(syn_sem_name::ScopeKind::Trait, Some(root));
        names.set_path_scope(iterator_def, iterator_scope);
        let iterator_item_def = names.add_def(
            iterator_scope,
            DefKind::AssocType,
            Some(item),
            Visibility::Private,
            Origin::Untracked,
        );
        let item_def = names.add_def(
            root,
            DefKind::AssocType,
            Some(item),
            Visibility::Private,
            Origin::Untracked,
        );
        let (repr, infer) = infer_types_with_names(
            &ccx,
            &scx,
            "struct S<T: Iterator> { field: <T>::Item }",
            &names,
        );

        let path = struct_field_path_type(&repr, &infer);
        let projection = struct_field_type_id(&repr, &infer);
        let qself = path
            .qself
            .expect("traitless qualified path should lower qself");

        assert_eq!(path.path.segments.len(), 1);
        assert_eq!(path.path.segments[0].name.as_ref(), "Item");
        assert_eq!(qself.trait_ty, None);
        assert!(matches!(
            infer[qself.self_ty],
            Type::Path(PathType {
                resolution: PathTypeResolution::GenericParam(def),
                ..
            }) if def == t_def
        ));
        assert_eq!(
            path.resolution,
            PathTypeResolution::Projection(ProjectionType {
                assoc_type: item_def,
                self_ty: Some(qself.self_ty),
                trait_ty: None,
            })
        );
        assert_eq!(
            infer.projection_obligations(),
            &[ProjectionObligation {
                projection,
                assoc_type: item_def,
                self_ty: Some(qself.self_ty),
                trait_ty: None,
            }]
        );
        let [bound] = infer.trait_bound_facts() else {
            panic!("expected one trait bound fact");
        };
        assert!(matches!(
            infer[bound.subject],
            Type::Path(PathType {
                resolution: PathTypeResolution::GenericParam(def),
                ..
            }) if def == t_def
        ));
        assert!(matches!(
            infer[bound.trait_ty],
            Type::Path(PathType {
                resolution: PathTypeResolution::Nominal(def),
                ..
            }) if def == iterator_def
        ));
        assert_eq!(
            infer.projection_candidates(),
            &[ProjectionCandidate {
                projection,
                self_ty: qself.self_ty,
                assoc_type: item_def,
                trait_ty: bound.trait_ty,
            }]
        );
        assert_eq!(
            infer.projection_matches(),
            &[ProjectionMatch {
                projection,
                self_ty: qself.self_ty,
                assoc_type: iterator_item_def,
                trait_ty: bound.trait_ty,
            }]
        );
    }

    #[test]
    fn skips_projection_matches_when_candidate_trait_has_no_associated_type_member() {
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let t = ccx.intern("T");
        let display = ccx.intern("Display");
        let item = ccx.intern("Item");
        let mut names = NameDb::default();
        let root = names.root_scope();
        let t_def = names.add_def(
            root,
            DefKind::GenericType,
            Some(t),
            Visibility::Private,
            Origin::Untracked,
        );
        let display_def = names.add_def(
            root,
            DefKind::Trait,
            Some(display),
            Visibility::Private,
            Origin::Untracked,
        );
        let display_scope = names.add_scope(syn_sem_name::ScopeKind::Trait, Some(root));
        names.set_path_scope(display_def, display_scope);
        let item_def = names.add_def(
            root,
            DefKind::AssocType,
            Some(item),
            Visibility::Private,
            Origin::Untracked,
        );
        let (repr, infer) = infer_types_with_names(
            &ccx,
            &scx,
            "struct S<T: Display> { field: <T>::Item }",
            &names,
        );

        let projection = struct_field_type_id(&repr, &infer);
        let path = struct_field_path_type(&repr, &infer);
        let qself = path
            .qself
            .expect("traitless qualified path should lower qself");
        assert!(matches!(
            infer[qself.self_ty],
            Type::Path(PathType {
                resolution: PathTypeResolution::GenericParam(def),
                ..
            }) if def == t_def
        ));
        let [bound] = infer.trait_bound_facts() else {
            panic!("expected one trait bound fact");
        };
        assert!(matches!(
            infer[bound.trait_ty],
            Type::Path(PathType {
                resolution: PathTypeResolution::Nominal(def),
                ..
            }) if def == display_def
        ));
        assert_eq!(
            infer.projection_candidates(),
            &[ProjectionCandidate {
                projection,
                self_ty: qself.self_ty,
                assoc_type: item_def,
                trait_ty: bound.trait_ty,
            }]
        );
        assert_eq!(infer.projection_matches(), &[]);
    }

    #[test]
    fn lowers_impl_associated_type_assignments_as_solver_input() {
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let file_path = ccx.intern("test.rs");
        let code = "struct Vec; trait Iterator { type Item; } impl Iterator for Vec { type Item = u32; } struct Output { field: <Vec as Iterator>::Item }";
        let text = ccx.intern(code);
        scx.parse_virtual_file(file_path, text).unwrap();
        let file = scx.lookup_source(file_path).unwrap().ast();

        let iterator = ccx.intern("Iterator");
        let vec = ccx.intern("Vec");
        let item = ccx.intern("Item");
        let mut names = NameDb::default();
        let root = names.root_scope();
        let vec_def = names.add_def(
            root,
            DefKind::Struct,
            Some(vec),
            Visibility::Private,
            Origin::Untracked,
        );
        let iterator_def = names.add_def(
            root,
            DefKind::Trait,
            Some(iterator),
            Visibility::Private,
            Origin::Untracked,
        );
        let iterator_scope = names.add_scope(ScopeKind::Trait, Some(root));
        names.set_path_scope(iterator_def, iterator_scope);
        let trait_assoc_def = names.add_def(
            iterator_scope,
            DefKind::AssocType,
            Some(item),
            Visibility::Private,
            Origin::Untracked,
        );
        let impl_item_def = names.add_def(
            root,
            DefKind::AssocType,
            Some(item),
            Visibility::Private,
            Origin::Untracked,
        );
        let ast::Item::Struct(_) = &file.items[0] else {
            panic!("expected struct item");
        };
        names.set_def_ast_node(vec_def, AstNodeId::from_ref(&file.items[0]));
        let ast::Item::Trait(trait_item) = &file.items[1] else {
            panic!("expected trait item");
        };
        names.set_def_ast_node(iterator_def, AstNodeId::from_ref(&file.items[1]));
        let ast::TraitItem::Type(_) = &trait_item.items[0] else {
            panic!("expected trait associated type");
        };
        let ast::Item::Impl(impl_item) = &file.items[2] else {
            panic!("expected impl item");
        };
        let ast::ImplItem::Type(_) = &impl_item.items[0] else {
            panic!("expected impl associated type");
        };
        names.set_def_ast_node(impl_item_def, AstNodeId::from_ref(&impl_item.items[0]));

        let repr = pr::ProgramReprBuilder::new(&names).build(file_path, file);
        let infer = InferDb::analyze(&ccx, &repr, &names);
        let [fact] = infer.assoc_type_impl_facts() else {
            panic!("expected one impl associated type fact");
        };

        let pr::ItemKind::Impl {
            trait_,
            self_ty,
            items,
            ..
        } = &repr.items()[2].kind
        else {
            panic!("expected represented impl");
        };
        assert!(trait_.is_some());
        let [assoc_item] = items.as_slice() else {
            panic!("expected one represented impl item");
        };
        assert!(matches!(
            repr[*assoc_item].kind,
            pr::AssocItemKind::ImplType { .. }
        ));
        assert_eq!(repr[*assoc_item].def, Some(impl_item_def));
        assert_eq!(fact.assoc_type, trait_assoc_def);
        assert_eq!(
            fact.impl_self_ty,
            infer.type_for_repr_type(*self_ty).unwrap()
        );
        assert!(matches!(
            infer[fact.impl_self_ty],
            Type::Path(PathType {
                resolution: PathTypeResolution::Nominal(def),
                ..
            }) if def == vec_def
        ));
        assert!(matches!(
            infer[fact.trait_ty],
            Type::Path(PathType {
                resolution: PathTypeResolution::Nominal(def),
                ..
            }) if def == iterator_def
        ));
        assert_eq!(infer[fact.value_ty], Type::Primitive(PrimitiveType::U32));

        let projection_path = struct_field_path_type(&repr, &infer);
        let qself = projection_path
            .qself
            .expect("projection field should lower qself");
        let projection = struct_field_type_id(&repr, &infer);
        assert_eq!(
            infer.projection_normalizations(),
            &[ProjectionNormalization {
                projection,
                self_ty: qself.self_ty,
                assoc_type: trait_assoc_def,
                trait_ty: qself.trait_ty.unwrap(),
                value_ty: fact.value_ty,
            }]
        );
        let normalizations = infer
            .normalizations_for_projection(projection)
            .collect::<Vec<_>>();
        assert_eq!(normalizations.len(), 1);
        assert_eq!(
            infer[normalizations[0].value_ty],
            Type::Primitive(PrimitiveType::U32)
        );
    }

    #[test]
    fn derives_generic_impl_self_binding_and_substitution() {
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let (repr, _names, infer) = infer_collected_types(
            &ccx,
            &scx,
            r#"
            struct Vec<T>;

            trait Iterator {
                type Item;
            }

            impl<T> Iterator for Vec<T> {
                type Item = T;
            }

            struct Output {
                field: <Vec<u32> as Iterator>::Item,
            }
            "#,
        );

        let fields = struct_field_repr_types(&repr, "Output");
        let [field_ty] = fields.as_slice() else {
            panic!("Output should have one field type");
        };
        let projection = infer.type_for_repr_type(*field_ty).unwrap();
        let [impl_self_match] = infer.impl_self_matches() else {
            panic!("generic impl self should match projection self once");
        };
        let [binding] = infer.type_binding_facts() else {
            panic!("generic impl self should create one type binding");
        };
        let [substitution] = infer.type_substitutions() else {
            panic!("generic impl associated type should create one substitution");
        };

        assert_eq!(
            binding.projection_self_ty,
            impl_self_match.projection_self_ty
        );
        assert_eq!(binding.impl_self_ty, impl_self_match.impl_self_ty);
        assert_eq!(substitution.projection_self_ty, binding.projection_self_ty);
        assert_eq!(substitution.impl_self_ty, binding.impl_self_ty);
        assert_eq!(substitution.generic_ty, binding.generic_ty);
        assert_eq!(substitution.arg_ty, binding.arg_ty);
        assert_eq!(substitution.substituted_ty, binding.arg_ty);
        assert_eq!(
            infer.normalized_projection_type(projection),
            Some(substitution.substituted_ty)
        );
        assert_eq!(
            infer[substitution.substituted_ty],
            Type::Primitive(PrimitiveType::U32)
        );
        assert_eq!(
            infer.projection_normalization(projection),
            ProjectionNormalizationResult::Known(substitution.substituted_ty)
        );
    }

    #[test]
    fn classifies_projection_normalization_query_results() {
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let (repr, _names, mut infer) = infer_collected_types(
            &ccx,
            &scx,
            r#"
            struct Vec<T>;

            trait Iterator {
                type Item;
            }

            impl<T> Iterator for Vec<T> {
                type Item = T;
            }

            struct Result {
                projected: <Vec<u32> as Iterator>::Item,
                plain: u32,
            }
            "#,
        );

        let fields = struct_field_repr_types(&repr, "Result");
        let [projected_ty, plain_ty] = fields.as_slice() else {
            panic!("Result should have two field types");
        };
        let projection = infer.type_for_repr_type(*projected_ty).unwrap();
        let plain = infer.type_for_repr_type(*plain_ty).unwrap();
        let known = infer
            .normalized_projection_type(projection)
            .expect("projection should have one normalization");

        assert_eq!(
            infer.projection_normalization(projection),
            ProjectionNormalizationResult::Known(known)
        );
        assert_eq!(
            infer.projection_normalization(plain),
            ProjectionNormalizationResult::NotProjection
        );

        let ambiguous_value = infer.intern_type(Type::Primitive(PrimitiveType::Bool));
        let ProjectionNormalizationResult::Known(_) = infer.projection_normalization(projection)
        else {
            panic!("projection should start with one known normalization");
        };
        let existing = infer.projection_normalizations()[0];
        infer
            .projection_normalizations
            .push(ProjectionNormalization {
                value_ty: ambiguous_value,
                ..existing
            });
        assert_eq!(
            infer.projection_normalization(projection),
            ProjectionNormalizationResult::Ambiguous
        );
        assert_eq!(infer.normalized_projection_type(projection), None);
        assert_eq!(infer.normalized_type(projection), projection);
    }

    #[test]
    fn reports_projection_without_known_normalization() {
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let (repr, _names, mut infer) = infer_collected_types(
            &ccx,
            &scx,
            r#"
            struct Vec<T>;

            trait Iterator {
                type Item;
            }

            struct Result {
                field: <Vec<u32> as Iterator>::Item,
            }
            "#,
        );

        let fields = struct_field_repr_types(&repr, "Result");
        let [field_ty] = fields.as_slice() else {
            panic!("Result should have one field type");
        };
        let projection = infer.type_for_repr_type(*field_ty).unwrap();

        assert_eq!(
            infer.projection_normalization(projection),
            ProjectionNormalizationResult::NoNormalization
        );
        assert_eq!(infer.normalized_projection_type(projection), None);
        assert_eq!(infer.normalized_type(projection), projection);
    }

    #[test]
    fn derives_selected_substitution_from_multiple_generic_bindings() {
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let (repr, _names, infer) = infer_collected_types(
            &ccx,
            &scx,
            r#"
            struct Pair<K, V>;

            trait Select {
                type Output;
            }

            impl<K, V> Select for Pair<K, V> {
                type Output = V;
            }

            struct Result {
                field: <Pair<u32, bool> as Select>::Output,
            }
            "#,
        );

        let fields = struct_field_repr_types(&repr, "Result");
        let [field_ty] = fields.as_slice() else {
            panic!("Result should have one field type");
        };
        let [_first_binding, _second_binding] = infer.type_binding_facts() else {
            panic!("Pair<K, V> should create two type bindings");
        };
        let [substitution] = infer.type_substitutions() else {
            panic!("type Output = V should create one selected substitution");
        };

        assert!(infer.type_binding_facts().iter().any(|binding| {
            substitution.projection_self_ty == binding.projection_self_ty
                && substitution.impl_self_ty == binding.impl_self_ty
                && substitution.generic_ty == binding.generic_ty
                && substitution.arg_ty == binding.arg_ty
        }));
        assert_eq!(
            infer.normalized_projection_type_for_repr_type(*field_ty),
            Some(substitution.substituted_ty)
        );
        assert_eq!(
            infer[substitution.substituted_ty],
            Type::Primitive(PrimitiveType::Bool)
        );
    }

    #[test]
    fn derives_nested_generic_value_substitution() {
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let (repr, _names, infer) = infer_collected_types(
            &ccx,
            &scx,
            r#"
            struct Vec<T>;
            struct Option<T>;

            trait Wrap {
                type Output;
            }

            impl<T> Wrap for Vec<T> {
                type Output = Option<T>;
            }

            struct Result {
                field: <Vec<u32> as Wrap>::Output,
            }
            "#,
        );

        let fields = struct_field_repr_types(&repr, "Result");
        let [field_ty] = fields.as_slice() else {
            panic!("Result should have one field type");
        };
        let [_binding] = infer.type_binding_facts() else {
            panic!("Vec<T> should create one type binding");
        };
        let [substitution] = infer.type_substitutions() else {
            panic!("nested generic associated type should create one substitution");
        };

        assert_eq!(
            substitution.projection_self_ty,
            infer.type_binding_facts()[0].projection_self_ty
        );
        assert_eq!(
            substitution.impl_self_ty,
            infer.type_binding_facts()[0].impl_self_ty
        );
        assert_eq!(
            infer.normalized_projection_type_for_repr_type(*field_ty),
            Some(substitution.substituted_ty)
        );
        assert_option_of(&infer, substitution.substituted_ty, PrimitiveType::U32);
    }

    #[test]
    fn recursively_normalizes_projection_inside_generic_argument() {
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let (repr, _names, mut infer) = infer_collected_types(
            &ccx,
            &scx,
            r#"
            struct Vec<T>;
            struct Option<T>;

            trait Iterator {
                type Item;
            }

            impl<T> Iterator for Vec<T> {
                type Item = T;
            }

            struct Result {
                field: Option<<Vec<u32> as Iterator>::Item>,
            }
            "#,
        );

        let fields = struct_field_repr_types(&repr, "Result");
        let [field_ty] = fields.as_slice() else {
            panic!("Result should have one field type");
        };
        let shallow = infer
            .shallow_normalized_type_for_repr_type(*field_ty)
            .expect("Result.field type should be lowered");
        let normalized = infer
            .normalized_type_for_repr_type(*field_ty)
            .expect("Result.field type should be lowered");

        assert_ne!(shallow, normalized);
        assert_option_of(&infer, normalized, PrimitiveType::U32);
    }

    #[test]
    fn recursively_normalizes_projection_inside_type_containers() {
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let (repr, _names, mut infer) = infer_collected_types(
            &ccx,
            &scx,
            r#"
            struct Vec<T>;

            trait Iterator {
                type Item;
            }

            impl<T> Iterator for Vec<T> {
                type Item = T;
            }

            struct Result {
                reference: &<Vec<u32> as Iterator>::Item,
                tuple: (<Vec<u32> as Iterator>::Item, bool),
                array: [<Vec<u32> as Iterator>::Item; 1],
                slice_ref: &[<Vec<u32> as Iterator>::Item],
            }
            "#,
        );

        let fields = struct_field_repr_types(&repr, "Result");
        let [reference_ty, tuple_ty, array_ty, slice_ref_ty] = fields.as_slice() else {
            panic!("Result should have four field types");
        };

        let reference = infer.normalized_type_for_repr_type(*reference_ty).unwrap();
        let Type::Reference { elem, is_mut } = infer[reference] else {
            panic!("reference field should normalize to a reference type");
        };
        assert!(!is_mut);
        assert_primitive_type(&infer, elem, PrimitiveType::U32);

        let tuple = infer.normalized_type_for_repr_type(*tuple_ty).unwrap();
        let Type::Tuple { elems } = &infer[tuple] else {
            panic!("tuple field should normalize to a tuple type");
        };
        let [first, second] = elems.as_slice() else {
            panic!("tuple field should have two elements");
        };
        assert_primitive_type(&infer, *first, PrimitiveType::U32);
        assert_primitive_type(&infer, *second, PrimitiveType::Bool);

        let array = infer.normalized_type_for_repr_type(*array_ty).unwrap();
        let Type::Array { elem, .. } = infer[array] else {
            panic!("array field should normalize to an array type");
        };
        assert_primitive_type(&infer, elem, PrimitiveType::U32);

        let slice_ref = infer.normalized_type_for_repr_type(*slice_ref_ty).unwrap();
        let Type::Reference { elem: slice, .. } = infer[slice_ref] else {
            panic!("slice_ref field should normalize to a reference type");
        };
        let Type::Slice { elem } = infer[slice] else {
            panic!("slice_ref element should normalize to a slice type");
        };
        assert_primitive_type(&infer, elem, PrimitiveType::U32);
    }

    #[test]
    fn caches_recursive_normalization_results() {
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let (repr, _names, mut infer) = infer_collected_types(
            &ccx,
            &scx,
            r#"
            struct Vec<T>;
            struct Option<T>;

            trait Iterator {
                type Item;
            }

            impl<T> Iterator for Vec<T> {
                type Item = T;
            }

            struct Result {
                field: Option<<Vec<u32> as Iterator>::Item>,
            }
            "#,
        );

        let fields = struct_field_repr_types(&repr, "Result");
        let [field_ty] = fields.as_slice() else {
            panic!("Result should have one field type");
        };
        let before = infer.types().len();
        let first = infer.normalized_type_for_repr_type(*field_ty).unwrap();
        let after_first = infer.types().len();
        let second = infer.normalized_type_for_repr_type(*field_ty).unwrap();
        let after_second = infer.types().len();

        assert_eq!(first, second);
        assert!(after_first >= before);
        assert_eq!(after_first, after_second);
    }

    #[test]
    fn keeps_generic_substitutions_tied_to_impl_self_match() {
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let (repr, _names, infer) = infer_collected_types(
            &ccx,
            &scx,
            r#"
            struct Vec<T>;
            struct Box<T>;
            struct Option<T>;

            trait Wrap {
                type Output;
            }

            impl<T> Wrap for Vec<T> {
                type Output = Option<T>;
            }

            impl<U> Wrap for Box<U> {
                type Output = Option<U>;
            }

            struct Result {
                vec_field: <Vec<u32> as Wrap>::Output,
                box_field: <Box<bool> as Wrap>::Output,
            }
            "#,
        );

        let fields = struct_field_repr_types(&repr, "Result");
        let [vec_field_ty, box_field_ty] = fields.as_slice() else {
            panic!("Result should have two field types");
        };
        let [_vec_substitution, _box_substitution] = infer.type_substitutions() else {
            panic!("each generic impl associated type should create one substitution");
        };

        for substitution in infer.type_substitutions() {
            assert!(infer.type_binding_facts().iter().any(|binding| {
                substitution.projection_self_ty == binding.projection_self_ty
                    && substitution.impl_self_ty == binding.impl_self_ty
                    && substitution.generic_ty == binding.generic_ty
                    && substitution.arg_ty == binding.arg_ty
            }));
        }
        let vec_normalized_ty = infer
            .normalized_projection_type_for_repr_type(*vec_field_ty)
            .expect("Vec projection should have one context-matched normalization");
        let box_normalized_ty = infer
            .normalized_projection_type_for_repr_type(*box_field_ty)
            .expect("Box projection should have one context-matched normalization");
        assert_option_of(&infer, vec_normalized_ty, PrimitiveType::U32);
        assert_option_of(&infer, box_normalized_ty, PrimitiveType::Bool);
    }

    fn assert_option_of(infer: &InferDb<'_>, ty: TypeId, expected: PrimitiveType) {
        let Type::Path(path) = &infer[ty] else {
            panic!("normalized projection value should lower to path type");
        };
        let [segment] = path.path.segments.as_slice() else {
            panic!("Option<T> should have one path segment");
        };
        assert_eq!(segment.name.as_ref(), "Option");
        let [GenericArgument::Type(arg)] = segment.args.as_slice() else {
            panic!("Option<T> should have one type argument");
        };
        assert_eq!(infer[*arg], Type::Primitive(expected));
    }

    fn assert_primitive_type(infer: &InferDb<'_>, ty: TypeId, expected: PrimitiveType) {
        assert_eq!(infer[ty], Type::Primitive(expected));
    }

    #[test]
    fn preserves_ambiguous_and_unresolved_path_states() {
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let maybe = ccx.intern("Maybe");
        let mut names = NameDb::default();
        let first = names.add_def(
            names.root_scope(),
            DefKind::Struct,
            Some(maybe),
            Visibility::Private,
            Origin::Untracked,
        );
        let second = names.add_def(
            names.root_scope(),
            DefKind::Enum,
            Some(maybe),
            Visibility::Private,
            Origin::Untracked,
        );
        let (repr, infer) = infer_types_with_names(&ccx, &scx, "struct S { field: Maybe }", &names);

        assert_eq!(
            struct_field_path_resolution(&repr, &infer),
            PathTypeResolution::Ambiguous(vec![first, second])
        );

        let (repr, infer) = infer_types(&ccx, &scx, "struct S { field: Missing }");

        assert_eq!(
            struct_field_path_resolution(&repr, &infer),
            PathTypeResolution::Unresolved
        );
    }
}
