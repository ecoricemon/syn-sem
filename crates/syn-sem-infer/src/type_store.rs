//! Inference-owned type storage shared by inference phases.
//!
//! [`InferTypes`] is the arena for lowered and synthesized inference types. HIR type occurrences
//! are bound to fresh type ids, while helper types created by later phases may be interned when
//! their shape is safe to share.

use crate::{
    GenericArg, Path, PathSegment, PathType, PathTypeResolution, ProjectionType, QSelf, Type,
    TypeId, TypeParamBound,
};
use std::ops::Index;
use syn_sem_common::{Map, VecUniqueExt};
use syn_sem_hir as hir;
use syn_sem_name::DefId;

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
    pub(crate) fn type_for_hir_type(&self, hir_ty_id: hir::TypeId) -> Option<TypeId> {
        self.hir_to_infer.get(&hir_ty_id).copied()
    }

    pub(crate) fn iter(&self) -> impl ExactSizeIterator<Item = (TypeId, &Type<'cx>)> + Clone {
        self.types.iter().enumerate().map(|(i, ty)| (TypeId(i), ty))
    }

    pub(crate) fn len(&self) -> usize {
        self.types.len()
    }

    pub(crate) fn bind_hir_type(&mut self, hir_ty_id: hir::TypeId, ty: TypeId) {
        self.hir_to_infer.insert(hir_ty_id, ty);
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

    pub(crate) fn can_share_types(&self, left: TypeId, right: TypeId) -> bool {
        TypeSharing::new(&self.types).can_share_type_ids(left, right, &mut Vec::new())
    }

    pub(crate) fn path_resolution(&self, ty: TypeId) -> Option<&PathTypeResolution> {
        let Type::Path(path) = &self[ty] else {
            return None;
        };
        Some(&path.resolution)
    }

    pub(crate) fn nominal_def(&self, ty: TypeId) -> Option<DefId> {
        let PathTypeResolution::Nominal(def) = self.path_resolution(ty)? else {
            return None;
        };
        Some(*def)
    }

    pub(crate) fn generic_def(&self, ty: TypeId) -> Option<DefId> {
        let PathTypeResolution::GenericParam(def) = self.path_resolution(ty)? else {
            return None;
        };
        Some(*def)
    }

    fn push_type(&mut self, ty: Type<'cx>) -> TypeId {
        let ty_id = TypeId::new(self.types.len());
        self.types.push(ty);
        ty_id
    }
}

impl<'cx> Index<TypeId> for InferTypes<'cx> {
    type Output = Type<'cx>;

    fn index(&self, ty: TypeId) -> &Self::Output {
        &self[ty.index()]
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
        if !seen.push_unique((left, right)) {
            return true;
        }
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
                    elem: left_elem,
                    len: left_len,
                },
                Type::Array {
                    elem: right_elem,
                    len: right_len,
                },
            ) => left_len == right_len && self.can_share_type_ids(*left_elem, *right_elem, seen),
            (Type::Infer, Type::Infer) => false,
            (Type::Primitive(left), Type::Primitive(right)) => left == right,
            (Type::Path(left), Type::Path(right)) => self.can_share_path_types(left, right, seen),
            (
                Type::Reference {
                    elem: left_elem,
                    is_mut: left_mut,
                },
                Type::Reference {
                    elem: right_elem,
                    is_mut: right_mut,
                },
            ) => left_mut == right_mut && self.can_share_type_ids(*left_elem, *right_elem, seen),
            (Type::Slice { elem: left_elem }, Type::Slice { elem: right_elem }) => {
                self.can_share_type_ids(*left_elem, *right_elem, seen)
            }
            (Type::Tuple { elems: left }, Type::Tuple { elems: right }) => {
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
        left.assoc == right.assoc
            && self.can_share_optional_type_ids(left.self_, right.self_, seen)
            && self.can_share_optional_type_ids(left.trait_, right.trait_, seen)
    }

    fn can_share_qself(&self, left: QSelf, right: QSelf, seen: &mut Vec<(TypeId, TypeId)>) -> bool {
        if !self.can_share_type_ids(left.self_, right.self_, seen) {
            return false;
        }
        match (left.trait_, right.trait_) {
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
                    ty: left_ty_id,
                },
                GenericArg::AssocType {
                    name: right_name,
                    ty: right_ty_id,
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
