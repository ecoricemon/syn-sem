//! Inference-owned type storage and HIR type lowering.

use crate::{
    ArrayLen, ConstArg, GenericArg, Path, PathSegment, PathType, PathTypeResolution, PrimitiveType,
    ProjectionType, QSelf, Type, TypeId, TypeParamBound,
};
use std::ops::Index;
use syn_sem_common::Map;
use syn_sem_hir as hir;
use syn_sem_name::{DefId, DefKind, NameDb, Namespace, ResolveResult, ScopeId};

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

    pub(super) fn lower_hir_type(&mut self, hir_ty_id: hir::TypeId) -> TypeId {
        if let Some(ty_id) = self.types.type_for_hir_type(hir_ty_id) {
            return ty_id;
        }

        // HIR types always lower to fresh infer types.
        let ty = self.lower_type(hir_ty_id, &self.hir[hir_ty_id].kind);
        let ty_id = self.types.insert_fresh_type(ty);
        self.types.bind_hir_type(hir_ty_id, ty_id);
        ty_id
    }

    fn lower_type(&mut self, hir_ty_id: hir::TypeId, kind: &hir::TypeKind<'cx>) -> Type<'cx> {
        match kind {
            hir::TypeKind::Array { elem_id, len } => Type::Array {
                elem_id: self.lower_hir_type(*elem_id),
                len: ArrayLen::from_hir(*len),
            },
            hir::TypeKind::Infer => Type::Infer,
            hir::TypeKind::Path(path) => self.lower_path_type(hir_ty_id, path),
            hir::TypeKind::Reference { elem_id, is_mut } => Type::Reference {
                elem_id: self.lower_hir_type(*elem_id),
                is_mut: *is_mut,
            },
            hir::TypeKind::Slice { elem_id } => Type::Slice {
                elem_id: self.lower_hir_type(*elem_id),
            },
            hir::TypeKind::Tuple { elem_ids } => Type::Tuple {
                elem_ids: elem_ids
                    .iter()
                    .map(|elem| self.lower_hir_type(*elem))
                    .collect(),
            },
        }
    }

    fn lower_path_type(&mut self, hir_ty_id: hir::TypeId, path: &hir::Path<'cx>) -> Type<'cx> {
        if path.qself.is_none() {
            if let Some(primitive) = PrimitiveType::from_hir_path(&path.segments) {
                return Type::Primitive(primitive);
            }
        }

        let scope = self.hir[hir_ty_id].scope;
        let qself = self.lower_qself(path.qself.as_ref(), scope);
        let resolution = self.resolve_path_type(qself.as_ref(), &path.segments, scope);

        Type::Path(PathType {
            qself,
            path: self.lower_plain_path(&path.segments),
            resolution,
        })
    }

    fn lower_qself(
        &mut self,
        qself: Option<&hir::QSelf<'cx>>,
        scope: Option<ScopeId>,
    ) -> Option<QSelf> {
        let qself = qself?;
        let trait_ty_id = (!qself.trait_path.is_empty())
            .then(|| self.lower_plain_path_as_type(&qself.trait_path, scope));
        Some(QSelf {
            self_ty_id: self.lower_hir_type(qself.self_ty_id),
            trait_ty_id,
        })
    }

    pub(super) fn lower_plain_path_as_type(
        &mut self,
        path: &[hir::PathSegment<'cx>],
        scope: Option<ScopeId>,
    ) -> TypeId {
        let resolution = self.resolve_path_type(None, path, scope);
        let ty = Type::Path(PathType {
            qself: None,
            path: self.lower_plain_path(path),
            resolution,
        });
        self.types.intern_type(ty)
    }

    fn resolve_path_type(
        &self,
        qself: Option<&QSelf>,
        path: &[hir::PathSegment<'cx>],
        scope: Option<ScopeId>,
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
        let trait_ty_id = qself.trait_ty_id?;
        let trait_def = self.trait_def_for_type(trait_ty_id)?;
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
            self_ty_id: Some(qself.self_ty_id),
            trait_ty_id: Some(trait_ty_id),
        }))
    }

    pub(super) fn trait_def_for_type(&self, ty_id: TypeId) -> Option<DefId> {
        let Type::Path(path) = &self.types[ty_id.index()] else {
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
                self_ty_id: qself.map(|qself| qself.self_ty_id),
                trait_ty_id: qself.and_then(|qself| qself.trait_ty_id),
            }),
            _ => PathTypeResolution::Unresolved,
        }
    }

    fn lower_plain_path(&mut self, path: &[hir::PathSegment<'cx>]) -> Path<'cx> {
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

    fn lower_generic_arg(&mut self, arg: &hir::GenericArg<'cx>) -> GenericArg<'cx> {
        match arg {
            hir::GenericArg::Type(ty_id) => GenericArg::Type(self.lower_hir_type(*ty_id)),
            hir::GenericArg::Const(value) => GenericArg::Const(self.lower_const_arg(value)),
            hir::GenericArg::AssocType { name, ty_id } => GenericArg::AssocType {
                name: *name,
                ty_id: self.lower_hir_type(*ty_id),
            },
            hir::GenericArg::AssocConst { name, value } => GenericArg::AssocConst {
                name: *name,
                value: self.lower_const_arg(value),
            },
            hir::GenericArg::Constraint { name, bounds } => GenericArg::Constraint {
                name: *name,
                bounds: self.lower_type_bounds(bounds),
            },
            hir::GenericArg::Unsupported => GenericArg::Unsupported,
        }
    }

    fn lower_const_arg(&mut self, arg: &hir::ConstArg<'cx>) -> ConstArg<'cx> {
        match arg {
            hir::ConstArg::Lit(lit) => ConstArg::Lit(crate::Lit::from_hir(lit)),
            hir::ConstArg::Path(path) => ConstArg::Path(self.lower_plain_path(&path.segments)),
            hir::ConstArg::Expr(expr) => ConstArg::Expr(*expr),
        }
    }

    fn lower_type_bounds(
        &mut self,
        bounds: &[hir::TypeParamBound<'cx>],
    ) -> Vec<TypeParamBound<'cx>> {
        bounds
            .iter()
            .map(|bound| self.lower_type_param_bound(bound))
            .collect()
    }

    fn lower_type_param_bound(&mut self, bound: &hir::TypeParamBound<'cx>) -> TypeParamBound<'cx> {
        match bound {
            hir::TypeParamBound::Trait(path) => TypeParamBound::Trait(self.lower_plain_path(path)),
            hir::TypeParamBound::Unsupported => TypeParamBound::Unsupported,
        }
    }
}

/// Inference-owned type arena and HIR type occurrence map.
///
/// HIR type ids name source occurrences, so lowering keeps each HIR occurrence mapped to its own
/// inference type id. Occurrence-insensitive helper types may still be interned when that is safe.
/// Distinct inference type ids can end up denoting the same final type after solving; for example,
/// a generic parameter path `T` and a concrete path `u32` remain distinct stored types even if
/// later obligations infer `T = u32`.
#[derive(Debug, Default)]
pub(crate) struct InferTypes<'cx> {
    types: Vec<Type<'cx>>,
    hir_to_infer: Map<hir::TypeId, TypeId>,
}

impl<'cx> InferTypes<'cx> {
    pub(super) fn collect_hir_types(&mut self, hir: &hir::Hir<'cx>, names: &NameDb<'cx>) {
        let mut ty_lowerer = TypeLowerer::new(hir, names, self);
        for ty in hir.types() {
            ty_lowerer.lower_hir_type(ty.id);
        }
    }

    pub(super) fn type_for_hir_type(&self, hir_ty_id: hir::TypeId) -> Option<TypeId> {
        self.hir_to_infer.get(&hir_ty_id).copied()
    }

    pub(crate) fn iter(&self) -> impl ExactSizeIterator<Item = (TypeId, &Type<'cx>)> + Clone {
        self.types.iter().enumerate().map(|(i, ty)| (TypeId(i), ty))
    }

    pub(crate) fn len(&self) -> usize {
        self.types.len()
    }

    pub(crate) fn bind_hir_type(&mut self, hir_ty_id: hir::TypeId, ty_id: TypeId) {
        self.hir_to_infer.insert(hir_ty_id, ty_id);
    }

    pub(crate) fn insert_fresh_type(&mut self, ty: Type<'cx>) -> TypeId {
        self.push_type(ty)
    }

    pub(crate) fn intern_type(&mut self, ty: Type<'cx>) -> TypeId {
        let sharing = TypeSharing::new(&self.types);
        for index in 0..self.types.len() {
            let existing = TypeId::new(index);
            if sharing.can_share_type_with(existing, &ty) {
                return existing;
            }
        }

        self.push_type(ty)
    }

    fn push_type(&mut self, ty: Type<'cx>) -> TypeId {
        let ty_id = TypeId::new(self.types.len());
        self.types.push(ty);
        ty_id
    }
}

impl<'cx> Index<usize> for InferTypes<'cx> {
    type Output = Type<'cx>;

    fn index(&self, index: usize) -> &Self::Output {
        &self.types[index]
    }
}

struct TypeSharing<'a, 'cx> {
    types: &'a [Type<'cx>],
}

impl<'a, 'cx> TypeSharing<'a, 'cx> {
    fn new(types: &'a [Type<'cx>]) -> Self {
        Self { types }
    }

    fn can_share_type_with(&self, existing_id: TypeId, candidate: &Type<'cx>) -> bool {
        self.can_share_type_with_inner(existing_id, candidate, &mut Vec::new())
    }

    fn can_share_type_ids(
        &self,
        left: TypeId,
        right: TypeId,
        seen: &mut Vec<(TypeId, TypeId)>,
    ) -> bool {
        if left == right {
            return true;
        }
        if seen.contains(&(left, right)) {
            return true;
        }
        seen.push((left, right));
        self.can_share_type_with_inner(left, &self.types[right.index()], seen)
    }

    fn can_share_type_with_inner(
        &self,
        existing_id: TypeId,
        candidate: &Type<'cx>,
        seen: &mut Vec<(TypeId, TypeId)>,
    ) -> bool {
        match (&self.types[existing_id.index()], candidate) {
            (
                Type::Array {
                    elem_id: left_elem,
                    len: left_len,
                },
                Type::Array {
                    elem_id: right_elem,
                    len: right_len,
                },
            ) => left_len == right_len && self.can_share_type_ids(*left_elem, *right_elem, seen),
            (Type::Infer, Type::Infer) => false,
            (Type::Primitive(left), Type::Primitive(right)) => left == right,
            (Type::Path(left), Type::Path(right)) => self.can_share_path_types(left, right, seen),
            (
                Type::Reference {
                    elem_id: left_elem,
                    is_mut: left_mut,
                },
                Type::Reference {
                    elem_id: right_elem,
                    is_mut: right_mut,
                },
            ) => left_mut == right_mut && self.can_share_type_ids(*left_elem, *right_elem, seen),
            (
                Type::Slice { elem_id: left_elem },
                Type::Slice {
                    elem_id: right_elem,
                },
            ) => self.can_share_type_ids(*left_elem, *right_elem, seen),
            (Type::Tuple { elem_ids: left }, Type::Tuple { elem_ids: right }) => {
                left.len() == right.len()
                    && left
                        .iter()
                        .zip(right)
                        .all(|(left, right)| self.can_share_type_ids(*left, *right, seen))
            }
            _ => false,
        }
    }

    fn can_share_path_types(
        &self,
        left: &PathType<'cx>,
        right: &PathType<'cx>,
        seen: &mut Vec<(TypeId, TypeId)>,
    ) -> bool {
        if !self.can_share_paths(&left.path, &right.path, seen)
            || !self.can_share_path_resolutions(&left.resolution, &right.resolution, seen)
        {
            return false;
        }
        match (left.qself, right.qself) {
            (None, None) => true,
            (Some(left), Some(right)) => self.can_share_qself(left, right, seen),
            _ => false,
        }
    }

    fn can_share_path_resolutions(
        &self,
        left: &PathTypeResolution,
        right: &PathTypeResolution,
        seen: &mut Vec<(TypeId, TypeId)>,
    ) -> bool {
        match (left, right) {
            (PathTypeResolution::Nominal(left), PathTypeResolution::Nominal(right)) => {
                left == right
            }
            (PathTypeResolution::GenericParam(left), PathTypeResolution::GenericParam(right)) => {
                left == right
            }
            (PathTypeResolution::Projection(left), PathTypeResolution::Projection(right)) => {
                self.can_share_projection_types(left, right, seen)
            }
            (
                PathTypeResolution::Ambiguous(_) | PathTypeResolution::Unresolved,
                PathTypeResolution::Ambiguous(_) | PathTypeResolution::Unresolved,
            ) => false,
            _ => false,
        }
    }

    fn can_share_projection_types(
        &self,
        left: &ProjectionType,
        right: &ProjectionType,
        seen: &mut Vec<(TypeId, TypeId)>,
    ) -> bool {
        left.assoc_type == right.assoc_type
            && self.can_share_optional_type_ids(left.self_ty_id, right.self_ty_id, seen)
            && self.can_share_optional_type_ids(left.trait_ty_id, right.trait_ty_id, seen)
    }

    fn can_share_qself(&self, left: QSelf, right: QSelf, seen: &mut Vec<(TypeId, TypeId)>) -> bool {
        if !self.can_share_type_ids(left.self_ty_id, right.self_ty_id, seen) {
            return false;
        }
        match (left.trait_ty_id, right.trait_ty_id) {
            (None, None) => true,
            (Some(left), Some(right)) => self.can_share_type_ids(left, right, seen),
            _ => false,
        }
    }

    fn can_share_optional_type_ids(
        &self,
        left: Option<TypeId>,
        right: Option<TypeId>,
        seen: &mut Vec<(TypeId, TypeId)>,
    ) -> bool {
        match (left, right) {
            (None, None) => true,
            (Some(left), Some(right)) => self.can_share_type_ids(left, right, seen),
            _ => false,
        }
    }

    fn can_share_paths(
        &self,
        left: &Path<'cx>,
        right: &Path<'cx>,
        seen: &mut Vec<(TypeId, TypeId)>,
    ) -> bool {
        left.segments.len() == right.segments.len()
            && left
                .segments
                .iter()
                .zip(&right.segments)
                .all(|(left, right)| self.can_share_path_segments(left, right, seen))
    }

    fn can_share_path_segments(
        &self,
        left: &PathSegment<'cx>,
        right: &PathSegment<'cx>,
        seen: &mut Vec<(TypeId, TypeId)>,
    ) -> bool {
        left.name == right.name
            && left.args.len() == right.args.len()
            && left
                .args
                .iter()
                .zip(&right.args)
                .all(|(left, right)| self.can_share_generic_args(left, right, seen))
    }

    fn can_share_generic_args(
        &self,
        left: &GenericArg<'cx>,
        right: &GenericArg<'cx>,
        seen: &mut Vec<(TypeId, TypeId)>,
    ) -> bool {
        match (left, right) {
            (GenericArg::Type(left), GenericArg::Type(right)) => {
                self.can_share_type_ids(*left, *right, seen)
            }
            (GenericArg::Const(left), GenericArg::Const(right)) => left == right,
            (
                GenericArg::AssocType {
                    name: left_name,
                    ty_id: left_ty_id,
                },
                GenericArg::AssocType {
                    name: right_name,
                    ty_id: right_ty_id,
                },
            ) => {
                left_name == right_name && self.can_share_type_ids(*left_ty_id, *right_ty_id, seen)
            }
            (
                GenericArg::AssocConst {
                    name: left_name,
                    value: left_value,
                },
                GenericArg::AssocConst {
                    name: right_name,
                    value: right_value,
                },
            ) => left_name == right_name && left_value == right_value,
            (
                GenericArg::Constraint {
                    name: left_name,
                    bounds: left_bounds,
                },
                GenericArg::Constraint {
                    name: right_name,
                    bounds: right_bounds,
                },
            ) => {
                left_name == right_name
                    && left_bounds.len() == right_bounds.len()
                    && left_bounds
                        .iter()
                        .zip(right_bounds)
                        .all(|(left, right)| self.can_share_type_param_bounds(left, right, seen))
            }
            (GenericArg::Unsupported, GenericArg::Unsupported) => false,
            _ => false,
        }
    }

    fn can_share_type_param_bounds(
        &self,
        left: &TypeParamBound<'cx>,
        right: &TypeParamBound<'cx>,
        seen: &mut Vec<(TypeId, TypeId)>,
    ) -> bool {
        match (left, right) {
            (TypeParamBound::Trait(left), TypeParamBound::Trait(right)) => {
                self.can_share_paths(left, right, seen)
            }
            (TypeParamBound::Unsupported, TypeParamBound::Unsupported) => false,
            _ => false,
        }
    }
}
