//! Logic-backed associated type projection derivation.

use crate::{
    logic::{
        term::{self, symbol::var, TypeShapeMode},
        visit_left_var, TypeShape, TypeShapeEncoder,
    },
    AssocTypeImplFact, GenericArg, ImplSelfMatch, ImplSelfTypeArgBinding, InferTypes, PathType,
    PathTypeResolution, ProjectionDb, ProjectionMatch, ProjectionNormalization, TraitBoundFact,
    Type, TypeId, TypeSubstitution,
};
use logic_eval::Database;
use syn_sem_common::{CommonCx, Map, VecUniqueExt};
use syn_sem_name::{DefId, DefKind, NameDb, Namespace, ResolveResult};

type LogicTerm<'cx> = term::LogicTerm<'cx>;

/// Uses [`ProjectionLogic`] at each solver-backed step, then stores the derived projection data.
pub(crate) struct ProjectionDeriver<'a, 'cx> {
    projections: &'a mut ProjectionDb,
    types: &'a mut InferTypes<'cx>,
    ccx: &'cx CommonCx,
    trait_bound_facts: &'a [TraitBoundFact],
    assoc_type_impl_facts: &'a [AssocTypeImplFact],
    names: &'a NameDb<'cx>,
}

impl<'a, 'cx: 'a> ProjectionDeriver<'a, 'cx> {
    pub(crate) fn new(
        projections: &'a mut ProjectionDb,
        types: &'a mut InferTypes<'cx>,
        ccx: &'cx CommonCx,
        trait_bound_facts: &'a [TraitBoundFact],
        assoc_type_impl_facts: &'a [AssocTypeImplFact],
        names: &'a NameDb<'cx>,
    ) -> Self {
        Self {
            projections,
            types,
            ccx,
            trait_bound_facts,
            assoc_type_impl_facts,
            names,
        }
    }

    pub(crate) fn derive(&mut self) {
        let matches = self.derive_projection_matches();
        self.projections.matches.extend(matches);

        let (impl_self_matches, type_bindings) = self.derive_impl_self_matches();
        self.projections.impl_self_matches.extend(impl_self_matches);
        self.projections.type_bindings.extend(type_bindings);

        let substitutions = self.derive_type_substitutions();
        self.projections.type_substitutions.extend(substitutions);

        let normalizations = self.derive_projection_normalizations();
        self.projections.normalizations.extend(normalizations);
    }

    fn derive_projection_matches(&self) -> Vec<ProjectionMatch> {
        let mut logic = ProjectionLogic::new(
            self.ccx,
            self.projections,
            self.types,
            self.trait_bound_facts,
            self.assoc_type_impl_facts,
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
            for bound in self.trait_bound_facts {
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

    fn derive_impl_self_matches(&self) -> (Vec<ImplSelfMatch>, Vec<ImplSelfTypeArgBinding>) {
        let mut logic = ProjectionLogic::new(
            self.ccx,
            self.projections,
            self.types,
            self.trait_bound_facts,
            self.assoc_type_impl_facts,
        );
        logic.load_impl_self_matches();

        logic.impl_self_matches_and_type_bindings()
    }

    fn derive_type_substitutions(&mut self) -> Vec<TypeSubstitution> {
        let mut substitutions = Vec::new();
        for impl_fact in self.assoc_type_impl_facts {
            let mut contexts = Vec::new();
            for binding in self
                .projections
                .type_bindings
                .iter()
                .filter(|binding| binding.impl_self == impl_fact.impl_self)
            {
                let context = (binding.projection_self, binding.impl_self);
                contexts.push_unique(context);
            }

            for (projection_self, impl_self) in contexts {
                let impl_bindings = self
                    .projections
                    .type_bindings
                    .iter()
                    .filter(|binding| {
                        binding.projection_self == projection_self && binding.impl_self == impl_self
                    })
                    .copied()
                    .collect::<Vec<_>>();
                let Some((substituted, used_bindings)) =
                    Self::substitute_type(self.types, impl_fact.value_ty, &impl_bindings)
                else {
                    continue;
                };
                for binding in used_bindings {
                    let substitution = TypeSubstitution {
                        projection_self: binding.projection_self,
                        impl_self: binding.impl_self,
                        value_ty: impl_fact.value_ty,
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

    fn derive_projection_normalizations(&self) -> Vec<ProjectionNormalization> {
        let mut logic = ProjectionLogic::new(
            self.ccx,
            self.projections,
            self.types,
            self.trait_bound_facts,
            self.assoc_type_impl_facts,
        );
        logic.load_projection_normalizations();

        let mut normalizations = Vec::new();
        for projection_match in &self.projections.matches {
            for impl_fact in self.assoc_type_impl_facts {
                let substituted_values = self
                    .projections
                    .type_substitutions
                    .iter()
                    .filter(|substitution| {
                        substitution.projection_self == projection_match.self_
                            && substitution.impl_self == impl_fact.impl_self
                            && substitution.value_ty == impl_fact.value_ty
                    })
                    .map(|substitution| substitution.substituted);
                for value_ty in std::iter::once(impl_fact.value_ty).chain(substituted_values) {
                    if logic.proves_normalization(
                        projection_match.projection,
                        projection_match.self_,
                        projection_match.assoc,
                        projection_match.trait_,
                        value_ty,
                    ) {
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
        bindings: &[ImplSelfTypeArgBinding],
    ) -> Option<(TypeId, Vec<ImplSelfTypeArgBinding>)> {
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
        bindings: &[ImplSelfTypeArgBinding],
    ) -> Option<(PathType<'cx>, Vec<ImplSelfTypeArgBinding>)> {
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
        bindings: &[ImplSelfTypeArgBinding],
        used: &mut Vec<ImplSelfTypeArgBinding>,
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
        bindings: &[ImplSelfTypeArgBinding],
    ) -> (Vec<crate::TypeParamBound<'cx>>, Vec<ImplSelfTypeArgBinding>) {
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
        bindings: &[ImplSelfTypeArgBinding],
    ) -> (crate::TypeParamBound<'cx>, Vec<ImplSelfTypeArgBinding>) {
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
        bindings: &[ImplSelfTypeArgBinding],
    ) -> (crate::Path<'cx>, Vec<ImplSelfTypeArgBinding>) {
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
        bindings: &[ImplSelfTypeArgBinding],
    ) -> (Vec<TypeId>, Vec<ImplSelfTypeArgBinding>) {
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
        bindings: &[ImplSelfTypeArgBinding],
    ) -> (TypeId, Vec<ImplSelfTypeArgBinding>) {
        if let Some(binding) = Self::binding_for_generic(types, ty, bindings) {
            return (binding.arg, vec![binding]);
        }
        (ty, Vec::new())
    }

    fn binding_for_generic(
        types: &InferTypes<'cx>,
        ty: TypeId,
        bindings: &[ImplSelfTypeArgBinding],
    ) -> Option<ImplSelfTypeArgBinding> {
        let generic_def = Self::generic_def(types, ty)?;
        bindings
            .iter()
            .copied()
            .find(|binding| Self::generic_def(types, binding.generic) == Some(generic_def))
    }

    fn unique_bindings(bindings: Vec<ImplSelfTypeArgBinding>) -> Vec<ImplSelfTypeArgBinding> {
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

/// Performs projection logic operations:
///
/// * Loads trait-candidate or normalization rules
/// * Loads projection and trait facts needed by the selected rule set
/// * Loads Rust-side matching and substitution facts
/// * Queries trait-candidate and normalization predicates
struct ProjectionLogic<'a, 'cx> {
    ccx: &'cx CommonCx,
    projections: &'a ProjectionDb,
    types: &'a InferTypes<'cx>,
    trait_bound_facts: &'a [TraitBoundFact],
    assoc_type_impl_facts: &'a [AssocTypeImplFact],
    concrete_shapes: Vec<(TypeId, TypeShape<'cx>)>,
    impl_pattern_shapes: Vec<(TypeId, TypeShape<'cx>)>,
    db: Database<term::LogicAtom<'cx>>,
}

impl<'a, 'cx> ProjectionLogic<'a, 'cx> {
    fn new(
        ccx: &'cx CommonCx,
        projections: &'a ProjectionDb,
        types: &'a InferTypes<'cx>,
        trait_bound_facts: &'a [TraitBoundFact],
        assoc_type_impl_facts: &'a [AssocTypeImplFact],
    ) -> Self {
        Self {
            ccx,
            projections,
            types,
            trait_bound_facts,
            assoc_type_impl_facts,
            concrete_shapes: Vec::new(),
            impl_pattern_shapes: Vec::new(),
            db: Database::default(),
        }
    }

    /// #projection_candidate($Projection, $Self, $Assoc, $Trait) :-
    ///   #explicit_projection_obligation($Projection, $Self, $Assoc, $Trait).
    /// #projection_candidate($Projection, $Self, $Assoc, $Trait) :-
    ///   #projection_obligation($Projection, $Self, $Assoc),
    ///   #trait_bound($Subject, $Trait), #same_type($Self, $Subject).
    /// #same_type($A, $A).
    /// #same_type($A, $B) :- #type_equal($A, $B).
    /// #same_type($A, $B) :- #type_equal($B, $A).
    ///
    /// #explicit_projection_obligation(projection, self, assoc, trait).
    /// #projection_obligation(projection, self, assoc).
    /// #trait_bound(subject, trait).
    /// #type_equal(a, b).
    fn load_projection_candidates(&mut self) {
        self.insert_candidate_rules();
        self.insert_same_type_rules();

        self.insert_projection_obligations();
        self.insert_trait_bounds();
        self.insert_type_equalities();
    }

    /// #projection_normalizes_to($Projection, $Self, $Assoc, $Trait, $Value) :-
    ///   #projection_match($Projection, $Self, $Assoc, $Trait),
    ///   #impl_assoc_type($ImplSelf, $ImplTrait, $Assoc, $Value),
    ///   #same_type($Self, $ImplSelf), #same_type($Trait, $ImplTrait).
    /// #projection_normalizes_to($Projection, $Self, $Assoc, $Trait, $Substituted) :-
    ///   #projection_match($Projection, $Self, $Assoc, $Trait),
    ///   #impl_assoc_type($ImplSelf, $ImplTrait, $Assoc, $Value),
    ///   #same_type($Trait, $ImplTrait), #impl_self_match($Self, $ImplSelf),
    ///   #type_binding($Self, $ImplSelf, $Generic, $Arg),
    ///   #type_substitution($Self, $ImplSelf, $Value, $Generic, $Arg, $Substituted).
    /// #same_type($A, $A).
    /// #same_type($A, $B) :- #type_equal($A, $B).
    /// #same_type($A, $B) :- #type_equal($B, $A).
    ///
    /// #projection_match(projection, self, assoc, trait).
    /// #impl_assoc_type(impl_self, impl_trait, assoc, value).
    /// #impl_self_match(self, impl_self).
    /// #type_binding(self, impl_self, generic, arg).
    /// #type_substitution(self, impl_self, value, generic, arg, substituted).
    /// #type_equal(a, b).
    fn load_projection_normalizations(&mut self) {
        self.insert_normalization_rules();
        self.insert_same_type_rules();

        self.insert_projection_matches();
        self.insert_impl_assoc_types();
        self.insert_impl_self_matches();
        self.insert_type_binding_facts();
        self.insert_type_substitutions();
        self.insert_type_equalities();
    }

    /// #impl_self_match($Self, $ImplSelf) :-
    ///   #impl_self_match_candidate($Self, $ImplSelf),
    ///   #type_shape($Self, #concrete, $Shape),
    ///   #type_shape($ImplSelf, #impl_pattern, $Shape).
    /// #impl_self_match_candidate($Self, $ImplSelf) :-
    ///   #projection_match($Projection, $Self, $Assoc, $Trait),
    ///   #impl_assoc_type($ImplSelf, $ImplTrait, $Assoc, $Value),
    ///   #same_type($Trait, $ImplTrait).
    /// #same_type($A, $A).
    /// #same_type($A, $B) :- #type_equal($A, $B).
    /// #same_type($A, $B) :- #type_equal($B, $A).
    ///
    /// #projection_match(projection, self, assoc, trait).
    /// #impl_assoc_type(impl_self, impl_trait, assoc, value).
    /// #type_shape(self, #concrete, shape).
    /// #type_shape(impl_self, #impl_pattern, shape).
    /// #type_equal(a, b).
    fn load_impl_self_matches(&mut self) {
        self.insert_impl_self_match_rules();
        self.insert_impl_self_match_candidate_rules();
        self.insert_same_type_rules();

        self.insert_projection_matches();
        self.insert_impl_assoc_types();
        self.insert_type_shapes();
        self.insert_type_equalities();
    }

    fn insert_same_type_rules(&mut self) {
        for clause in term::same_type_rules(self.ccx, term::PROJECTION_SAME_TYPE_RULES) {
            self.db.insert_clause(clause);
        }
    }

    fn insert_candidate_rules(&mut self) {
        for clause in term::projection_candidate_rules(self.ccx) {
            self.db.insert_clause(clause);
        }
    }

    fn insert_normalization_rules(&mut self) {
        for clause in term::projection_normalization_rules(self.ccx) {
            self.db.insert_clause(clause);
        }
    }

    fn insert_impl_self_match_candidate_rules(&mut self) {
        for clause in term::impl_self_match_candidate_rules(self.ccx) {
            self.db.insert_clause(clause);
        }
    }

    fn insert_impl_self_match_rules(&mut self) {
        for clause in term::impl_self_match_rules(self.ccx) {
            self.db.insert_clause(clause);
        }
    }

    fn insert_projection_obligations(&mut self) {
        for obligation in &self.projections.obligations {
            self.db
                .insert_clause(term::projection_obligation_clause(self.ccx, *obligation));
        }
    }

    fn insert_trait_bounds(&mut self) {
        for bound in self.trait_bound_facts {
            self.db
                .insert_clause(term::trait_bound_clause(self.ccx, *bound));
        }
    }

    fn insert_type_equalities(&mut self) {
        for left_index in 0..self.types.len() {
            let left = TypeId::new(left_index);
            for right in (left_index + 1)..self.types.len() {
                let right = TypeId::new(right);
                if self.types[left] != self.types[right] {
                    continue;
                }
                self.db
                    .insert_clause(term::projection_type_equal_clause(self.ccx, left, right));
            }
        }
    }

    fn insert_projection_matches(&mut self) {
        for projection_match in &self.projections.matches {
            self.db
                .insert_clause(term::projection_match_clause(self.ccx, *projection_match));
        }
    }

    fn insert_impl_assoc_types(&mut self) {
        for fact in self.assoc_type_impl_facts {
            self.db
                .insert_clause(term::impl_assoc_type_clause(self.ccx, *fact));
        }
    }

    fn insert_impl_self_matches(&mut self) {
        for match_ in &self.projections.impl_self_matches {
            self.db
                .insert_clause(term::impl_self_match_clause(self.ccx, *match_));
        }
    }

    fn insert_type_binding_facts(&mut self) {
        for binding in &self.projections.type_bindings {
            self.db
                .insert_clause(term::type_binding_clause(self.ccx, *binding));
        }
    }

    fn insert_type_substitutions(&mut self) {
        for substitution in &self.projections.type_substitutions {
            self.db
                .insert_clause(term::type_substitution_clause(self.ccx, *substitution));
        }
    }

    fn insert_type_shapes(&mut self) {
        let encoder = TypeShapeEncoder::new(self.ccx, self.types);
        let mut concrete_tys = Vec::new();
        for projection_match in &self.projections.matches {
            concrete_tys.push_unique(projection_match.self_);
        }
        for ty in concrete_tys {
            let Some(shape) = encoder.encode(ty, TypeShapeMode::Concrete) else {
                continue;
            };
            let shape_term = shape.shape.clone();
            self.concrete_shapes
                .push_unique_by_key((ty, shape), |(ty, _)| *ty);
            self.db.insert_clause(term::type_shape_clause(
                self.ccx,
                ty,
                TypeShapeMode::Concrete,
                shape_term,
            ));
        }

        let mut impl_self_tys = Vec::new();
        for fact in self.assoc_type_impl_facts {
            impl_self_tys.push_unique(fact.impl_self);
        }
        for impl_self in impl_self_tys {
            let Some(shape) = encoder.encode(impl_self, TypeShapeMode::ImplPattern) else {
                continue;
            };
            let shape_term = shape.shape.clone();
            self.impl_pattern_shapes
                .push_unique_by_key((impl_self, shape), |(ty, _)| *ty);
            self.db.insert_clause(term::type_shape_clause(
                self.ccx,
                impl_self,
                TypeShapeMode::ImplPattern,
                shape_term,
            ));
        }
    }

    fn proves_candidate(
        &mut self,
        projection: TypeId,
        self_: TypeId,
        assoc: DefId,
        trait_: TypeId,
    ) -> bool {
        self.db
            .query(term::projection_candidate_query(
                self.ccx, projection, self_, assoc, trait_,
            ))
            .is_true()
    }

    fn proves_normalization(
        &mut self,
        projection: TypeId,
        self_: TypeId,
        assoc: DefId,
        trait_: TypeId,
        value_ty: TypeId,
    ) -> bool {
        self.db
            .query(term::projection_normalization_query(
                self.ccx, projection, self_, assoc, trait_, value_ty,
            ))
            .is_true()
    }

    fn impl_self_matches_and_type_bindings(
        &mut self,
    ) -> (Vec<ImplSelfMatch>, Vec<ImplSelfTypeArgBinding>) {
        let mut impl_self_matches = Vec::new();
        let mut type_bindings = Vec::new();
        let mut qcx = self.db.query(term::impl_self_match_query(self.ccx));
        while let Some(answer) = qcx.prove_next() {
            let projection_self = answer
                .get(var::SELF)
                .and_then(|term| term::type_id_from_term(&term));
            let impl_self = answer
                .get(var::IMPL_SELF)
                .and_then(|term| term::type_id_from_term(&term));
            let concrete_shape = answer.get(var::SHAPE);
            let (Some(projection_self), Some(impl_self), Some(concrete_shape)) =
                (projection_self, impl_self, concrete_shape)
            else {
                continue;
            };

            let match_ = ImplSelfMatch {
                projection_self,
                impl_self,
            };
            impl_self_matches.push_unique(match_);
            for binding in self.impl_self_type_arg_bindings(match_, &concrete_shape) {
                type_bindings.push_unique(binding);
            }
        }
        (impl_self_matches, type_bindings)
    }

    /// Materializes impl-self generic bindings from a successful self-type shape match.
    ///
    /// For `<Vec<u32> as Iterator>::Item` matched against `impl<T> Iterator for Vec<T>`, this finds
    /// that the impl generic `T` in the `Vec<T>` is bound to the projection argument `u32`.
    fn impl_self_type_arg_bindings(
        &self,
        match_: ImplSelfMatch,
        concrete_shape: &LogicTerm<'cx>,
    ) -> Vec<ImplSelfTypeArgBinding> {
        let Some(concrete_shape_info) = self
            .concrete_shapes
            .iter()
            .find_map(|(self_, shape)| (*self_ == match_.projection_self).then_some(shape))
        else {
            return Vec::new();
        };
        let Some(impl_pattern_shape_info) = self
            .impl_pattern_shapes
            .iter()
            .find_map(|(impl_self, shape)| (*impl_self == match_.impl_self).then_some(shape))
        else {
            return Vec::new();
        };

        let mut var_bindings = Vec::new();
        visit_left_var(
            &impl_pattern_shape_info.shape,
            concrete_shape,
            &mut |var, rhs| {
                var_bindings.push((var, rhs));
            },
        );

        let mut bindings = Vec::new();
        for (var, rhs) in var_bindings {
            let var_term = term::atom(var);
            let Some(generic) =
                Self::type_id_for_logic_term(&var_term, &impl_pattern_shape_info.term_types)
            else {
                continue;
            };
            let Some(arg) = Self::type_id_for_logic_term(rhs, &concrete_shape_info.term_types)
            else {
                continue;
            };
            let binding = ImplSelfTypeArgBinding {
                projection_self: match_.projection_self,
                impl_self: match_.impl_self,
                generic,
                arg,
            };
            bindings.push_unique(binding);
        }
        bindings
    }

    fn type_id_for_logic_term(
        term: &LogicTerm<'cx>,
        term_types: &Map<LogicTerm<'cx>, TypeId>,
    ) -> Option<TypeId> {
        term::type_id_from_term(term).or_else(|| term_types.get(term).copied())
    }
}
