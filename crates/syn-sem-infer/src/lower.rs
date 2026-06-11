use crate::{
    ArrayLen, GenericArgument, InferDb, Path, PathSegment, PathType, PathTypeResolution,
    PrimitiveType, ProjectionObligation, ProjectionType, QSelf, SourceConstArg, SourceTypeBounds,
    TraitBoundFact, Type, TypeId,
};
use smallvec::{smallvec, SmallVec};
use syn_sem_common::CommonCx;
use syn_sem_name::{DefId, DefKind, NameDb, ResolveResult, ScopeId};
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

    fn lower_name_as_type(
        &mut self,
        name: syn_sem_name::Name<'cx>,
        scope: Option<ScopeId>,
    ) -> TypeId {
        let path = pr::TypePathValue {
            segments: smallvec![pr::TypePathSegment {
                name,
                args: SmallVec::new(),
            }],
        };
        self.lower_path_value_as_type(&path, scope)
    }

    fn resolve_path_value(
        &self,
        scope: Option<ScopeId>,
        path: &pr::TypePathValue<'cx>,
        qself: Option<&QSelf>,
    ) -> PathTypeResolution {
        let Some(scope) = scope else {
            return PathTypeResolution::Unresolved;
        };
        match self
            .names
            .resolve_type_path(scope, path.segments.iter().map(|segment| segment.name))
        {
            ResolveResult::Found(def) => self.classify_path_target(def, qself),
            ResolveResult::Ambiguous(defs) => PathTypeResolution::Ambiguous(defs),
            ResolveResult::NotFound => PathTypeResolution::Unresolved,
        }
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
    use smallvec::smallvec;
    use syn_sem_ast::SyntaxCx;
    use syn_sem_common::CommonCx;
    use syn_sem_name::{DefKind, NameDb, Origin, Visibility};
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
            PathTypeResolution::Ambiguous(smallvec![first, second])
        );

        let (repr, infer) = infer_types(&ccx, &scx, "struct S { field: Missing }");

        assert_eq!(
            struct_field_path_resolution(&repr, &infer),
            PathTypeResolution::Unresolved
        );
    }
}
