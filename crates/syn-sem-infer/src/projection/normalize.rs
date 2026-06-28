//! Logic-backed associated type projection normalization.

use super::logic::ProjectionLogic;
use crate::{
    GenericArg, ImplAssocType, ImplSelfGenericBinding, ImplSelfMatch, InferConstFacts, InferTypes,
    PathType, PathTypeResolution, ProjectionDb, ProjectionMatch, ProjectionNormalization,
    ProjectionTypeSubstitution, TraitBound, Type, TypeId,
};
use syn_sem_common::{CommonCx, VecUniqueExt};
use syn_sem_name::{DefId, DefKind, NameDb, Namespace, ResolveResult};

/// Normalizes associated type projections through trait bounds and impl associated types.
///
/// Trait-less projections use collected bounds to choose a trait candidate. For example,
/// `<T>::Item` inside `struct S<T: Iterator>` is matched through the collected `T: Iterator` fact
/// before the same impl-self matching and substitution pipeline runs.
pub(crate) struct ProjectionNormalizer<'a, 'cx> {
    projections: &'a mut ProjectionDb,
    types: &'a mut InferTypes<'cx>,
    ccx: &'cx CommonCx,
    trait_bounds: &'a [TraitBound],
    impl_assoc_types: &'a [ImplAssocType],
    names: &'a NameDb<'cx>,
    const_facts: &'a InferConstFacts,
}

impl<'a, 'cx: 'a> ProjectionNormalizer<'a, 'cx> {
    pub(crate) fn new(
        projections: &'a mut ProjectionDb,
        types: &'a mut InferTypes<'cx>,
        ccx: &'cx CommonCx,
        trait_bounds: &'a [TraitBound],
        impl_assoc_types: &'a [ImplAssocType],
        names: &'a NameDb<'cx>,
        const_facts: &'a InferConstFacts,
    ) -> Self {
        Self {
            projections,
            types,
            ccx,
            trait_bounds,
            impl_assoc_types,
            names,
            const_facts,
        }
    }

    /// Runs the associated type projection normalization pipeline.
    ///
    /// For example, given:
    /// ```text
    /// impl<T> Iterator for Vec<T> {
    ///     type Item = T;
    /// }
    ///
    /// struct Output {
    ///     field: <Vec<u32> as Iterator>::Item,
    /// }
    /// ```
    ///
    /// this records:
    /// ```text
    /// projection match:     <Vec<u32> as Iterator>::Item uses Iterator::Item
    /// impl self match:      Vec<u32> matches Vec<T>
    /// generic binding:      T -> u32
    /// type substitution:    Item = T becomes u32
    /// normalization:        <Vec<u32> as Iterator>::Item -> u32
    /// ```
    pub(crate) fn normalize(&mut self) {
        let matches = self.match_projection_obligations();
        self.projections.projection_matches.extend(matches);

        let (impl_self_matches, impl_self_generic_bindings) = self.match_impl_self_types();
        self.projections.impl_self_matches.extend(impl_self_matches);
        self.projections
            .impl_self_generic_bindings
            .extend(impl_self_generic_bindings);

        let substitutions = self.build_type_substitutions();
        self.projections.type_substitutions.extend(substitutions);

        let normalizations = self.normalize_projection_matches();
        self.projections.normalizations.extend(normalizations);
    }

    fn match_projection_obligations(&self) -> Vec<ProjectionMatch> {
        let mut logic = ProjectionLogic::new(
            self.ccx,
            self.projections,
            self.types,
            self.trait_bounds,
            self.impl_assoc_types,
            self.const_facts,
        );
        logic.load_projection_candidates();

        let mut matches = Vec::new();
        for obligation in &self.projections.obligations {
            let self_ = obligation.self_;
            if let Some(trait_) = obligation.trait_ {
                if let Some(assoc) = self.trait_member_assoc(trait_, obligation.assoc) {
                    matches.push(ProjectionMatch {
                        projection: obligation.projection,
                        self_,
                        assoc,
                        trait_,
                    });
                }
                continue;
            }
            for bound in self.trait_bounds {
                if !logic.proves_candidate(
                    obligation.projection,
                    self_,
                    obligation.assoc,
                    bound.trait_,
                ) {
                    continue;
                }
                if let Some(assoc) = self.trait_member_assoc(bound.trait_, obligation.assoc) {
                    matches.push(ProjectionMatch {
                        projection: obligation.projection,
                        self_,
                        assoc,
                        trait_: bound.trait_,
                    });
                }
            }
        }
        matches
    }

    fn match_impl_self_types(&self) -> (Vec<ImplSelfMatch>, Vec<ImplSelfGenericBinding>) {
        let mut logic = ProjectionLogic::new(
            self.ccx,
            self.projections,
            self.types,
            self.trait_bounds,
            self.impl_assoc_types,
            self.const_facts,
        );
        logic.load_impl_self_matches();

        logic.impl_self_matches_and_generic_bindings()
    }

    fn build_type_substitutions(&mut self) -> Vec<ProjectionTypeSubstitution> {
        let mut substitutions = Vec::new();
        for impl_assoc_type in self.impl_assoc_types {
            let mut contexts = Vec::new();
            for binding in self
                .projections
                .impl_self_generic_bindings
                .iter()
                .filter(|binding| binding.impl_self == impl_assoc_type.impl_self)
            {
                let context = (binding.projection_self, binding.impl_self);
                contexts.push_unique(context);
            }

            for (projection_self, impl_self) in contexts {
                let impl_bindings = self
                    .projections
                    .impl_self_generic_bindings
                    .iter()
                    .filter(|binding| {
                        binding.projection_self == projection_self && binding.impl_self == impl_self
                    })
                    .copied()
                    .collect::<Vec<_>>();
                let Some((substituted, used_bindings)) =
                    Self::substitute_type(self.types, impl_assoc_type.value_ty, &impl_bindings)
                else {
                    continue;
                };
                for binding in used_bindings {
                    let substitution = ProjectionTypeSubstitution {
                        projection_self: binding.projection_self,
                        impl_self: binding.impl_self,
                        value_ty: impl_assoc_type.value_ty,
                        generic: binding.generic,
                        arg: binding.arg,
                        substituted,
                    };
                    substitutions.push_unique(substitution);
                }
            }
        }
        substitutions
    }

    fn normalize_projection_matches(&self) -> Vec<ProjectionNormalization> {
        let mut logic = ProjectionLogic::new(
            self.ccx,
            self.projections,
            self.types,
            self.trait_bounds,
            self.impl_assoc_types,
            self.const_facts,
        );
        logic.load_projection_normalizations();

        let mut normalizations = Vec::new();
        for projection_match in &self.projections.projection_matches {
            for impl_assoc_type in self.impl_assoc_types {
                let substituted_values = self
                    .projections
                    .type_substitutions
                    .iter()
                    .filter(|substitution| {
                        substitution.projection_self == projection_match.self_
                            && substitution.impl_self == impl_assoc_type.impl_self
                            && substitution.value_ty == impl_assoc_type.value_ty
                    })
                    .map(|substitution| substitution.substituted);
                for value_ty in std::iter::once(impl_assoc_type.value_ty).chain(substituted_values)
                {
                    if logic.proves_normalization(
                        projection_match.projection,
                        projection_match.self_,
                        projection_match.assoc,
                        projection_match.trait_,
                        value_ty,
                    ) || (value_ty == impl_assoc_type.value_ty
                        && self.matches_without_generic_bindings(projection_match, impl_assoc_type))
                    {
                        let normalization = ProjectionNormalization {
                            projection: projection_match.projection,
                            self_: projection_match.self_,
                            assoc: projection_match.assoc,
                            trait_: projection_match.trait_,
                            value_ty,
                        };
                        normalizations.push_unique(normalization);
                    }
                }
            }
        }
        normalizations
    }

    fn matches_without_generic_bindings(
        &self,
        projection_match: &ProjectionMatch,
        impl_assoc_type: &ImplAssocType,
    ) -> bool {
        if projection_match.trait_ != impl_assoc_type.trait_
            && self.types[projection_match.trait_] != self.types[impl_assoc_type.trait_]
        {
            return false;
        }
        let matched = self.projections.impl_self_matches.iter().any(|match_| {
            match_.projection_self == projection_match.self_
                && match_.impl_self == impl_assoc_type.impl_self
        });
        if !matched {
            return false;
        }
        !self
            .projections
            .impl_self_generic_bindings
            .iter()
            .any(|binding| {
                binding.projection_self == projection_match.self_
                    && binding.impl_self == impl_assoc_type.impl_self
            })
    }

    /// Returns the associated type member in `trait_` whose name matches
    /// `requested_assoc_type`.
    ///
    /// `requested_assoc_type` is the definition found by the projection path, and is used only as
    /// the source of the requested name. The returned definition is the concrete associated type
    /// member owned by the candidate trait, so the input and output [`DefId`]s may differ.
    fn trait_member_assoc(&self, trait_: TypeId, requested_assoc_type: DefId) -> Option<DefId> {
        let trait_def = self.types.nominal_def(trait_)?;
        if self.names[trait_def].kind != DefKind::Trait {
            return None;
        }
        let assoc_name = self.names[requested_assoc_type].name?;
        let ResolveResult::Found(member_assoc_type) =
            self.names.member(trait_def, Namespace::Type, assoc_name)
        else {
            return None;
        };
        if self.names[member_assoc_type].kind != DefKind::AssocType {
            return None;
        }
        Some(member_assoc_type)
    }

    fn substitute_type(
        types: &mut InferTypes<'cx>,
        ty: TypeId,
        bindings: &[ImplSelfGenericBinding],
    ) -> Option<(TypeId, Vec<ImplSelfGenericBinding>)> {
        if let Some(binding) = Self::binding_for_generic(types, ty, bindings) {
            return Some((binding.arg, vec![binding]));
        }

        match types[ty].clone() {
            Type::Array { elem, len } => {
                let (elem, used) = Self::substitute_type(types, elem, bindings)?;
                Some((types.intern_type(Type::Array { elem, len }), used))
            }
            Type::Infer | Type::Primitive(_) => None,
            Type::Path(path) => {
                let (path, used) = Self::substitute_path_type(types, path, bindings)?;
                Some((types.intern_type(Type::Path(path)), used))
            }
            Type::Reference { elem, is_mut } => {
                let (elem, used) = Self::substitute_type(types, elem, bindings)?;
                Some((types.intern_type(Type::Reference { elem, is_mut }), used))
            }
            Type::Slice { elem } => {
                let (elem, used) = Self::substitute_type(types, elem, bindings)?;
                Some((types.intern_type(Type::Slice { elem }), used))
            }
            Type::Tuple { elems } => {
                let (elems, used) = Self::substitute_type_ids(types, elems, bindings);
                if used.is_empty() {
                    return None;
                }
                Some((types.intern_type(Type::Tuple { elems }), used))
            }
        }
    }

    fn substitute_path_type(
        types: &InferTypes<'cx>,
        path: PathType<'cx>,
        bindings: &[ImplSelfGenericBinding],
    ) -> Option<(PathType<'cx>, Vec<ImplSelfGenericBinding>)> {
        let mut used = Vec::new();
        let qself = path.qself.map(|qself| {
            let (self_, self_used) = Self::substitute_type_id(types, qself.self_, bindings);
            used.extend(self_used);
            let trait_ = qself.trait_.map(|trait_| {
                let (trait_, trait_used) = Self::substitute_type_id(types, trait_, bindings);
                used.extend(trait_used);
                trait_
            });
            crate::QSelf { self_, trait_ }
        });
        let segments = path
            .path
            .segments
            .into_iter()
            .map(|segment| {
                let args = segment
                    .args
                    .into_iter()
                    .map(|arg| Self::substitute_generic_argument(types, arg, bindings, &mut used))
                    .collect();
                crate::PathSegment {
                    name: segment.name,
                    args,
                }
            })
            .collect();

        let used = Self::unique_bindings(used);
        if used.is_empty() {
            return None;
        }
        Some((
            PathType {
                qself,
                path: crate::Path { segments },
                resolution: path.resolution,
            },
            used,
        ))
    }

    fn substitute_generic_argument(
        types: &InferTypes<'cx>,
        arg: GenericArg<'cx>,
        bindings: &[ImplSelfGenericBinding],
        used: &mut Vec<ImplSelfGenericBinding>,
    ) -> GenericArg<'cx> {
        match arg {
            GenericArg::Type(ty) => {
                let (ty_id, ty_used) = Self::substitute_type_id(types, ty, bindings);
                used.extend(ty_used);
                GenericArg::Type(ty_id)
            }
            GenericArg::AssocType { name, ty } => {
                let (ty_id, ty_used) = Self::substitute_type_id(types, ty, bindings);
                used.extend(ty_used);
                GenericArg::AssocType { name, ty: ty_id }
            }
            GenericArg::Const(arg) => GenericArg::Const(arg),
            GenericArg::AssocConst { name, value } => GenericArg::AssocConst { name, value },
            GenericArg::Constraint { name, bounds } => {
                let (bounds, bounds_used) = Self::substitute_type_bounds(types, bounds, bindings);
                used.extend(bounds_used);
                GenericArg::Constraint { name, bounds }
            }
            GenericArg::Unsupported => GenericArg::Unsupported,
        }
    }

    fn substitute_type_bounds(
        types: &InferTypes<'cx>,
        bounds: Vec<crate::TypeParamBound<'cx>>,
        bindings: &[ImplSelfGenericBinding],
    ) -> (Vec<crate::TypeParamBound<'cx>>, Vec<ImplSelfGenericBinding>) {
        let mut used = Vec::new();
        let bounds = bounds
            .into_iter()
            .map(|bound| {
                let (bound, bound_used) = Self::substitute_type_param_bound(types, bound, bindings);
                used.extend(bound_used);
                bound
            })
            .collect();
        (bounds, Self::unique_bindings(used))
    }

    fn substitute_type_param_bound(
        types: &InferTypes<'cx>,
        bound: crate::TypeParamBound<'cx>,
        bindings: &[ImplSelfGenericBinding],
    ) -> (crate::TypeParamBound<'cx>, Vec<ImplSelfGenericBinding>) {
        match bound {
            crate::TypeParamBound::Trait(path) => {
                let (path, used) = Self::substitute_path(types, path, bindings);
                (crate::TypeParamBound::Trait(path), used)
            }
            crate::TypeParamBound::Unsupported => (crate::TypeParamBound::Unsupported, Vec::new()),
        }
    }

    fn substitute_path(
        types: &InferTypes<'cx>,
        path: crate::Path<'cx>,
        bindings: &[ImplSelfGenericBinding],
    ) -> (crate::Path<'cx>, Vec<ImplSelfGenericBinding>) {
        let mut used = Vec::new();
        let segments = path
            .segments
            .into_iter()
            .map(|segment| {
                let args = segment
                    .args
                    .into_iter()
                    .map(|arg| {
                        let mut arg_used = Vec::new();
                        let arg =
                            Self::substitute_generic_argument(types, arg, bindings, &mut arg_used);
                        used.extend(arg_used);
                        arg
                    })
                    .collect();
                crate::PathSegment {
                    name: segment.name,
                    args,
                }
            })
            .collect();
        (crate::Path { segments }, Self::unique_bindings(used))
    }

    fn substitute_type_ids(
        types: &InferTypes<'cx>,
        tys: Vec<TypeId>,
        bindings: &[ImplSelfGenericBinding],
    ) -> (Vec<TypeId>, Vec<ImplSelfGenericBinding>) {
        let mut used = Vec::new();
        let tys = tys
            .into_iter()
            .map(|ty| {
                let (ty_id, ty_used) = Self::substitute_type_id(types, ty, bindings);
                used.extend(ty_used);
                ty_id
            })
            .collect();
        (tys, Self::unique_bindings(used))
    }

    fn substitute_type_id(
        types: &InferTypes<'cx>,
        ty: TypeId,
        bindings: &[ImplSelfGenericBinding],
    ) -> (TypeId, Vec<ImplSelfGenericBinding>) {
        if let Some(binding) = Self::binding_for_generic(types, ty, bindings) {
            return (binding.arg, vec![binding]);
        }
        (ty, Vec::new())
    }

    fn binding_for_generic(
        types: &InferTypes<'cx>,
        ty: TypeId,
        bindings: &[ImplSelfGenericBinding],
    ) -> Option<ImplSelfGenericBinding> {
        let generic_def = Self::generic_def(types, ty)?;
        bindings
            .iter()
            .copied()
            .find(|binding| Self::generic_def(types, binding.generic) == Some(generic_def))
    }

    fn unique_bindings(bindings: Vec<ImplSelfGenericBinding>) -> Vec<ImplSelfGenericBinding> {
        let mut unique = Vec::new();
        for binding in bindings {
            unique.push_unique(binding);
        }
        unique
    }

    fn generic_def(types: &InferTypes<'cx>, ty: TypeId) -> Option<DefId> {
        let Type::Path(path) = &types[ty] else {
            return None;
        };
        let PathTypeResolution::GenericParam(def) = path.resolution else {
            return None;
        };
        Some(def)
    }
}
