use std::ops::Index;
use syn_sem_common::Map;
use syn_sem_name::{DefId, DefKind, NameDb, ResolveResult, ScopeId};
use syn_sem_pr as pr;

/// Type information collected for upper semantic inference.
#[derive(Debug, Default)]
pub struct InferDb<'cx> {
    types: Vec<Type<'cx>>,
    repr_types: Map<pr::TypeId, TypeId>,
}

impl<'cx> InferDb<'cx> {
    /// Builds inference type facts from program representation and name-resolution data.
    pub fn analyze(repr: &pr::ProgramRepr<'cx>, names: &NameDb<'cx>) -> Self {
        InferCx::new(repr, names).lower()
    }

    /// Returns all collected inference types.
    pub fn types(&self) -> &[Type<'cx>] {
        &self.types
    }

    /// Returns the inference type linked to a represented type occurrence.
    pub fn type_for_repr_type(&self, repr_type: pr::TypeId) -> Option<TypeId> {
        self.repr_types.get(&repr_type).copied()
    }

    /// Returns all represented-type-to-inference-type links.
    pub fn repr_types(&self) -> &Map<pr::TypeId, TypeId> {
        &self.repr_types
    }
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
        self.lower_repr_types();
        self.db
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
        let id = self.next_type_id();
        self.db.types.push(ty);
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
        let id = self.next_type_id();
        self.db.types.push(ty);
        id
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
}

impl<'cx> Index<TypeId> for InferDb<'cx> {
    type Output = Type<'cx>;

    fn index(&self, id: TypeId) -> &Self::Output {
        &self.types[id.index()]
    }
}

/// Stable identity for one inference type.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TypeId(usize);

impl TypeId {
    /// Creates an id from a raw index.
    pub const fn new(index: usize) -> Self {
        Self(index)
    }

    /// Returns the raw index represented by this id.
    pub const fn index(self) -> usize {
        self.0
    }
}

/// Type shape used by inference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type<'cx> {
    /// Fixed-length array type.
    Array {
        /// Element type.
        elem: TypeId,
        /// Array length expression shape.
        len: ArrayLen,
    },
    /// Inferred type placeholder.
    Infer,
    /// Primitive Rust type.
    Primitive(PrimitiveType),
    /// Path type.
    Path(PathType<'cx>),
    /// Borrowed reference type.
    Reference {
        /// Referenced type.
        elem: TypeId,
        /// Whether the reference is mutable.
        is_mut: bool,
    },
    /// Dynamically sized slice type.
    Slice {
        /// Element type.
        elem: TypeId,
    },
    /// Tuple type.
    Tuple {
        /// Tuple element types.
        elems: Vec<TypeId>,
    },
}

/// Primitive Rust type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimitiveType {
    /// `bool`.
    Bool,
    /// `char`.
    Char,
    /// `str`.
    Str,
    /// `i8`.
    I8,
    /// `i16`.
    I16,
    /// `i32`.
    I32,
    /// `i64`.
    I64,
    /// `i128`.
    I128,
    /// `isize`.
    Isize,
    /// `u8`.
    U8,
    /// `u16`.
    U16,
    /// `u32`.
    U32,
    /// `u64`.
    U64,
    /// `u128`.
    U128,
    /// `usize`.
    Usize,
    /// `f32`.
    F32,
    /// `f64`.
    F64,
}

impl PrimitiveType {
    fn from_repr_path(path: &pr::TypePathValue<'_>) -> Option<Self> {
        let [segment] = path.segments.as_slice() else {
            return None;
        };
        if !segment.args.is_empty() {
            return None;
        }
        Self::from_str(segment.name.as_ref())
    }

    fn from_str(value: &str) -> Option<Self> {
        match value {
            "bool" => Some(Self::Bool),
            "char" => Some(Self::Char),
            "str" => Some(Self::Str),
            "i8" => Some(Self::I8),
            "i16" => Some(Self::I16),
            "i32" => Some(Self::I32),
            "i64" => Some(Self::I64),
            "i128" => Some(Self::I128),
            "isize" => Some(Self::Isize),
            "u8" => Some(Self::U8),
            "u16" => Some(Self::U16),
            "u32" => Some(Self::U32),
            "u64" => Some(Self::U64),
            "u128" => Some(Self::U128),
            "usize" => Some(Self::Usize),
            "f32" => Some(Self::F32),
            "f64" => Some(Self::F64),
            _ => None,
        }
    }
}

/// Path type used by inference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathType<'cx> {
    /// Qualified self type, when the source used qualified path syntax.
    pub qself: Option<QSelf>,
    /// Path naming the type.
    pub path: Path<'cx>,
    /// Current resolution state for this type path.
    pub resolution: PathTypeResolution,
}

/// Qualified self type for an inference path type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QSelf {
    /// Self type written inside `<...>`.
    pub self_ty: TypeId,
    /// Trait path in `<Self as Trait>`, when present.
    pub trait_ty: Option<TypeId>,
}

/// Resolution state for a non-primitive path type.
///
/// This records the best current classification without pretending that name lookup is full Rust
/// type resolution. Solver-backed generic substitution, qualified paths, and projection
/// normalization can refine this later.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathTypeResolution {
    /// Path names a nominal type definition.
    Nominal(DefId),
    /// Path names a generic type parameter.
    GenericParam(DefId),
    /// Path denotes an associated type projection.
    Projection(ProjectionType),
    /// Multiple candidates matched the path.
    Ambiguous(Vec<DefId>),
    /// No target is known for the path yet.
    Unresolved,
}

/// Associated type projection known to path type resolution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectionType {
    /// Associated type definition selected for the projection.
    pub assoc_type: DefId,
    /// Self type for the projection, when represented.
    pub self_ty: Option<TypeId>,
    /// Trait type for the projection, when represented.
    pub trait_ty: Option<TypeId>,
}

/// Type path used by inference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Path<'cx> {
    /// Path segments in source order.
    pub segments: Vec<PathSegment<'cx>>,
}

/// One type path segment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathSegment<'cx> {
    /// Segment name.
    pub name: syn_sem_name::Name<'cx>,
    /// Generic arguments on this segment.
    pub args: Vec<GenericArgument<'cx>>,
}

/// Generic argument shape used by inference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GenericArgument<'cx> {
    /// Type argument.
    Type(TypeId),
    /// Const expression argument.
    Const(SourceConstArg),
    /// Associated type equality.
    AssocType {
        /// Associated type name.
        name: syn_sem_name::Name<'cx>,
        /// Assigned type.
        ty: TypeId,
    },
    /// Associated const equality.
    AssocConst {
        /// Associated const name.
        name: syn_sem_name::Name<'cx>,
        /// Assigned const value.
        value: SourceConstArg,
    },
    /// Associated type constraint.
    Constraint {
        /// Associated type name.
        name: syn_sem_name::Name<'cx>,
        /// Source bounds.
        bounds: SourceTypeBounds,
    },
    /// Unsupported argument form.
    Unsupported,
}

/// Array length represented before expression lowering exists.
// TODO: Replace this with expression-backed or evaluated array length facts once const
// expression representation is available.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArrayLen {
    /// Length is still a source expression.
    SourceExpr,
}

impl ArrayLen {
    fn from_repr(len: pr::ArrayLen) -> Self {
        match len {
            pr::ArrayLen::SourceExpr => Self::SourceExpr,
        }
    }
}

/// Const argument represented before expression lowering exists.
// TODO: Replace this with expression-backed const argument facts once const expression
// representation is available.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceConstArg;

/// Type bounds represented before bound lowering exists.
// TODO: Replace this with bound-backed facts once type bound representation is available.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceTypeBounds;

#[cfg(test)]
mod tests {
    use super::*;
    use syn_sem_ast::SyntaxCx;
    use syn_sem_common::CommonCx;
    use syn_sem_name::{DefKind, NameDb, Origin, Visibility};

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
        let infer = InferDb::analyze(&repr, &names);
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
        let infer = InferDb::analyze(&repr, names);
        (repr, infer)
    }

    fn struct_field_path_type<'a, 'cx>(
        repr: &'a pr::ProgramRepr<'cx>,
        infer: &'a InferDb<'cx>,
    ) -> &'a PathType<'cx> {
        let repr_type = repr
            .types()
            .iter()
            .find(|source| matches!(source.source, pr::TypeSource::StructField))
            .unwrap();
        let id = infer.type_for_repr_type(repr_type.id).unwrap();
        let Type::Path(path) = &infer[id] else {
            panic!("struct field type should lower to path type");
        };
        path
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
