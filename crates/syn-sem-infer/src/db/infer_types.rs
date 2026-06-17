//! Inference-owned type storage and HIR type lowering.

use crate::{
    ArrayLen, ConstArg, GenericArgument, Path, PathSegment, PathType, PathTypeResolution,
    PrimitiveType, ProjectionType, QSelf, TraitBound, Type, TypeBounds, TypeId, TypeParamBound,
};
use std::ops::Index;
use syn_sem_common::Map;
use syn_sem_hir as hir;
use syn_sem_name::{DefId, DefKind, NameDb, Namespace, ResolveResult, ScopeId};

/// Inference-owned type arena and HIR type occurrence map.
#[derive(Debug, Default)]
pub(crate) struct InferTypes<'cx> {
    types: Vec<Type<'cx>>,
    hir_types: Map<hir::TypeId, TypeId>,
}

impl<'cx> InferTypes<'cx> {
    pub(super) fn collect_hir_types(&mut self, hir: &hir::Hir<'cx>, names: &NameDb<'cx>) {
        let mut lowerer = TypeLowerer::new(hir, names, self);
        for ty in hir.types() {
            lowerer.lower_hir_type(ty.id);
        }
    }

    pub(super) fn type_for_hir_type(&self, hir_type: hir::TypeId) -> Option<TypeId> {
        self.hir_types.get(&hir_type).copied()
    }

    pub(super) fn as_slice(&self) -> &[Type<'cx>] {
        &self.types
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = &Type<'cx>> {
        self.types.iter()
    }

    pub(crate) fn len(&self) -> usize {
        self.types.len()
    }

    fn next_type_id(&self) -> TypeId {
        TypeId::new(self.types.len())
    }

    pub(super) fn push_type(&mut self, ty: Type<'cx>) -> TypeId {
        let id = self.next_type_id();
        self.types.push(ty);
        id
    }

    pub(super) fn intern_type(&mut self, ty: Type<'cx>) -> TypeId {
        if let Some(index) = self.types.iter().position(|existing| existing == &ty) {
            return TypeId::new(index);
        }
        self.push_type(ty)
    }
}

impl<'cx> Index<usize> for InferTypes<'cx> {
    type Output = Type<'cx>;

    fn index(&self, index: usize) -> &Self::Output {
        &self.types[index]
    }
}

pub(super) struct TypeLowerer<'a, 'cx> {
    hir: &'a hir::Hir<'cx>,
    names: &'a NameDb<'cx>,
    types: &'a mut InferTypes<'cx>,
}

impl<'a, 'cx> TypeLowerer<'a, 'cx> {
    pub(super) fn new(
        hir: &'a hir::Hir<'cx>,
        names: &'a NameDb<'cx>,
        types: &'a mut InferTypes<'cx>,
    ) -> Self {
        Self { hir, names, types }
    }

    pub(super) fn lower_hir_type(&mut self, hir_type: hir::TypeId) -> TypeId {
        if let Some(id) = self.types.type_for_hir_type(hir_type) {
            return id;
        }

        let ty = self.lower_type(hir_type, &self.hir[hir_type].kind);
        let id = self.types.push_type(ty);
        self.types.hir_types.insert(hir_type, id);
        id
    }

    fn lower_type(&mut self, hir_type: hir::TypeId, kind: &hir::TypeKind<'cx>) -> Type<'cx> {
        match kind {
            hir::TypeKind::Array { elem, len } => Type::Array {
                elem: self.lower_hir_type(*elem),
                len: ArrayLen::from_hir(*len),
            },
            hir::TypeKind::Infer => Type::Infer,
            hir::TypeKind::Path(path) => self.lower_path_type(hir_type, path),
            hir::TypeKind::Reference { elem, is_mut } => Type::Reference {
                elem: self.lower_hir_type(*elem),
                is_mut: *is_mut,
            },
            hir::TypeKind::Slice { elem } => Type::Slice {
                elem: self.lower_hir_type(*elem),
            },
            hir::TypeKind::Tuple { elems } => Type::Tuple {
                elems: elems
                    .iter()
                    .map(|elem| self.lower_hir_type(*elem))
                    .collect(),
            },
        }
    }

    fn lower_path_type(&mut self, hir_type: hir::TypeId, path: &hir::Path<'cx>) -> Type<'cx> {
        if path.qself.is_none() {
            if let Some(primitive) = PrimitiveType::from_hir_path(&path.segments) {
                return Type::Primitive(primitive);
            }
        }

        let scope = self.hir[hir_type].scope;
        let qself = self.lower_qself(path.qself.as_ref(), scope);
        let resolution = self.resolve_path_value(scope, &path.segments, qself.as_ref());

        Type::Path(PathType {
            qself,
            path: self.lower_path_value(&path.segments),
            resolution,
        })
    }

    fn lower_qself(
        &mut self,
        qself: Option<&hir::QSelf<'cx>>,
        scope: Option<ScopeId>,
    ) -> Option<QSelf> {
        let qself = qself?;
        let trait_ty = (!qself.trait_path.is_empty())
            .then(|| self.lower_path_value_as_type(&qself.trait_path, scope));
        Some(QSelf {
            self_ty: self.lower_hir_type(qself.self_ty),
            trait_ty,
        })
    }

    pub(super) fn lower_path_value_as_type(
        &mut self,
        path: &[hir::PathSegment<'cx>],
        scope: Option<ScopeId>,
    ) -> TypeId {
        let ty = Type::Path(PathType {
            qself: None,
            path: self.lower_path_value(path),
            resolution: self.resolve_path_value(scope, path, None),
        });
        self.types.push_type(ty)
    }

    fn resolve_path_value(
        &self,
        scope: Option<ScopeId>,
        path: &[hir::PathSegment<'cx>],
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
            .resolve_type_path(scope, path.iter().map(|segment| segment.name))
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
        path: &[hir::PathSegment<'cx>],
        qself: Option<&QSelf>,
    ) -> Option<PathTypeResolution> {
        let qself = qself?;
        let trait_ty = qself.trait_ty?;
        let trait_def = self.trait_def_for_type(trait_ty)?;
        let assoc_name = path.last()?.name;
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

    pub(super) fn trait_def_for_type(&self, ty: TypeId) -> Option<DefId> {
        let Type::Path(path) = &self.types[ty.index()] else {
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

    fn lower_path_value(&mut self, path: &[hir::PathSegment<'cx>]) -> Path<'cx> {
        Path {
            segments: path
                .iter()
                .map(|segment| self.lower_path_segment(segment))
                .collect(),
        }
    }

    fn lower_path_segment(&mut self, segment: &hir::PathSegment<'cx>) -> PathSegment<'cx> {
        PathSegment {
            name: segment.name,
            args: segment
                .args
                .iter()
                .map(|arg| self.lower_generic_arg(arg))
                .collect(),
        }
    }

    fn lower_generic_arg(&mut self, arg: &hir::GenericArg<'cx>) -> GenericArgument<'cx> {
        match arg {
            hir::GenericArg::Type(ty) => GenericArgument::Type(self.lower_hir_type(*ty)),
            hir::GenericArg::Const(value) => GenericArgument::Const(self.lower_const_arg(value)),
            hir::GenericArg::AssocType { name, ty } => GenericArgument::AssocType {
                name: *name,
                ty: self.lower_hir_type(*ty),
            },
            hir::GenericArg::AssocConst { name, value } => GenericArgument::AssocConst {
                name: *name,
                value: self.lower_const_arg(value),
            },
            hir::GenericArg::Constraint { name, bounds } => GenericArgument::Constraint {
                name: *name,
                bounds: self.lower_type_bounds(bounds),
            },
            hir::GenericArg::Unsupported => GenericArgument::Unsupported,
        }
    }

    fn lower_const_arg(&mut self, arg: &hir::ConstArg<'cx>) -> ConstArg<'cx> {
        match arg {
            hir::ConstArg::Lit(lit) => ConstArg::Lit(crate::Lit::from_hir(lit)),
            hir::ConstArg::Path(path) => ConstArg::Path(self.lower_path_value(&path.segments)),
            hir::ConstArg::Expr(expr) => ConstArg::Expr(*expr),
        }
    }

    fn lower_type_bounds(&mut self, bounds: &[hir::TypeParamBound<'cx>]) -> TypeBounds<'cx> {
        TypeBounds {
            bounds: bounds
                .iter()
                .map(|bound| self.lower_type_param_bound(bound))
                .collect(),
        }
    }

    fn lower_type_param_bound(&mut self, bound: &hir::TypeParamBound<'cx>) -> TypeParamBound<'cx> {
        match bound {
            hir::TypeParamBound::Trait(bound) => TypeParamBound::Trait(TraitBound {
                path: self.lower_path_value(&bound.path),
            }),
            hir::TypeParamBound::Unsupported => TypeParamBound::Unsupported,
        }
    }
}
