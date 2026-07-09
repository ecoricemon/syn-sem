//! Logic rules and predicates for associated type projection solving.

use super::type_shape_term::{type_shape, type_shape_mode, TypeShapeMode};
use crate::{
    logic::{
        atom, def_id, same_type,
        symbol::{Rel, Var},
        term, type_equal_clause, type_id, Clause, Expr, SameTypeRules, Term,
    },
    ImplAssocType, ImplSelfGenericBinding, ImplSelfMatch, ProjectionMatch, ProjectionObligation,
    ProjectionTypeSubstitution, TraitBound, TypeId,
};
use syn_sem_name::DefId;

// In examples below, `tyN` encodes `TypeId::new(N)` and `defN` encodes `DefId::new(N)`.

/// Projection needs reflexive and reverse type-shape equality, and does not need transitive
/// closure.
pub(crate) const PROJECTION_SAME_TYPE_RULES: SameTypeRules = SameTypeRules {
    reflexive: true,
    reverse: true,
    transitive: false,
};

/// * For `<T as Trait>::Assoc`,
/// * Output clause 0 - `#projection_candidate($Projection, $Self, $Assoc, $Trait) :-
///   #explicit_projection_obligation($Projection, $Self, $Assoc, $Trait).`
///
/// * For `<T>::Assoc` with `T: Trait`,
/// * Output clause 1 - `#projection_candidate($Projection, $Self, $Assoc, $Trait) :-
///   #projection_obligation($Projection, $Self, $Assoc), #trait_bound($Subject, $Trait),
///   #same_type($Self, $Subject).`
pub(crate) fn projection_candidate_rules() -> [Clause<'static>; 2] {
    [
        Clause::rule(
            projection_candidate(
                atom(Var::Projection),
                atom(Var::SelfTy),
                atom(Var::Assoc),
                atom(Var::Trait),
            ),
            Expr::Term(explicit_projection_obligation(
                atom(Var::Projection),
                atom(Var::SelfTy),
                atom(Var::Assoc),
                atom(Var::Trait),
            )),
        ),
        Clause::rule(
            projection_candidate(
                atom(Var::Projection),
                atom(Var::SelfTy),
                atom(Var::Assoc),
                atom(Var::Trait),
            ),
            Expr::And(vec![
                Expr::Term(projection_obligation(
                    atom(Var::Projection),
                    atom(Var::SelfTy),
                    atom(Var::Assoc),
                )),
                Expr::Term(trait_bound(atom(Var::Subject), atom(Var::Trait))),
                Expr::Term(same_type(atom(Var::SelfTy), atom(Var::Subject))),
            ]),
        ),
    ]
}

/// * Rule - `#impl_self_match_candidate(Self, ImplSelf) :-
///   #projection_match(Projection, Self, Assoc, Trait),
///   #impl_assoc_type(ImplSelf, ImplTrait, Assoc, Value),
///   #same_type(Trait, ImplTrait).`
///
/// # Examples
///
/// * Code - `<Vec<u32> as Iterator>::Item` with
///   `impl<T> Iterator for Vec<T> { type Item = T; }`
/// * Output clause - `#impl_self_match_candidate($Self, $ImplSelf) :-
///   #projection_match($Projection, $Self, $Assoc, $Trait),
///   #impl_assoc_type($ImplSelf, $ImplTrait, $Assoc, $Value),
///   #same_type($Trait, $ImplTrait).`
pub(crate) fn impl_self_match_candidate_rules() -> [Clause<'static>; 1] {
    [Clause::rule(
        impl_self_match_candidate(atom(Var::SelfTy), atom(Var::ImplSelf)),
        Expr::And(vec![
            Expr::Term(projection_match(
                atom(Var::Projection),
                atom(Var::SelfTy),
                atom(Var::Assoc),
                atom(Var::Trait),
            )),
            Expr::Term(impl_assoc_type(
                atom(Var::ImplSelf),
                atom(Var::ImplTrait),
                atom(Var::Assoc),
                atom(Var::Value),
            )),
            Expr::Term(same_type(atom(Var::Trait), atom(Var::ImplTrait))),
        ]),
    )]
}

/// * Rule - `#impl_self_match(Self, ImplSelf) :-
///   #impl_self_match_candidate(Self, ImplSelf),
///   #type_shape(Self, #preserve_generics, Shape),
///   #type_shape(ImplSelf, #variable_generics, Shape).`
///
/// The shared `Shape` variable lets logic unification validate the projection self type against
/// the impl-self pattern.
pub(crate) fn impl_self_match_rules() -> [Clause<'static>; 1] {
    [Clause::rule(
        impl_self_match(atom(Var::SelfTy), atom(Var::ImplSelf)),
        Expr::And(vec![
            Expr::Term(impl_self_match_candidate(
                atom(Var::SelfTy),
                atom(Var::ImplSelf),
            )),
            Expr::Term(type_shape(
                atom(Var::SelfTy),
                type_shape_mode(TypeShapeMode::PreserveGenerics),
                atom(Var::Shape),
            )),
            Expr::Term(type_shape(
                atom(Var::ImplSelf),
                type_shape_mode(TypeShapeMode::VariableGenerics),
                atom(Var::Shape),
            )),
        ]),
    )]
}

/// * Rule 0 - `#projection_normalizes_to(P, Self, Assoc, Trait, Value) :-
///   #projection_match(P, Self, Assoc, Trait),
///   #impl_assoc_type(ImplSelf, ImplTrait, Assoc, Value),
///   #same_type(Trait, ImplTrait), #impl_self_match(Self, ImplSelf),
///   #impl_assoc_value_without_bindings(ImplSelf, Value).`
/// * Rule 1 - `#projection_normalizes_to(P, Self, Assoc, Trait, Substituted) :-
///   #projection_match(P, Self, Assoc, Trait),
///   #impl_assoc_type(ImplSelf, ImplTrait, Assoc, Value),
///   #same_type(Trait, ImplTrait), #impl_self_match(Self, ImplSelf),
///   #type_binding(Self, ImplSelf, Generic, Arg),
///   #type_substitution(Self, ImplSelf, Value, Generic, Arg, Substituted).`
///
/// # Examples
///
/// * Code - `<Vec as Iterator>::Item` with `impl Iterator for Vec { type Item = u32; }`
/// * Output clause - `#projection_normalizes_to($Projection, $Self, $Assoc, $Trait, $Value) :-
///   #projection_match($Projection, $Self, $Assoc, $Trait),
///   #impl_assoc_type($ImplSelf, $ImplTrait, $Assoc, $Value),
///   #same_type($Trait, $ImplTrait), #impl_self_match($Self, $ImplSelf),
///   #impl_assoc_value_without_bindings($ImplSelf, $Value).`
/// * Code - `<Vec<u32> as Iterator>::Item` with
///   `impl<T> Iterator for Vec<T> { type Item = T; }`
/// * Output clause - `#projection_normalizes_to($Projection, $Self, $Assoc, $Trait, $Substituted) :-
///   #projection_match($Projection, $Self, $Assoc, $Trait),
///   #impl_assoc_type($ImplSelf, $ImplTrait, $Assoc, $Value),
///   #same_type($Trait, $ImplTrait), #impl_self_match($Self, $ImplSelf),
///   #type_binding($Self, $ImplSelf, $Generic, $Arg),
///   #type_substitution($Self, $ImplSelf, $Value, $Generic, $Arg, $Substituted).`
pub(crate) fn projection_normalization_rules() -> [Clause<'static>; 2] {
    [
        Clause::rule(
            projection_normalizes_to(
                atom(Var::Projection),
                atom(Var::SelfTy),
                atom(Var::Assoc),
                atom(Var::Trait),
                atom(Var::Value),
            ),
            Expr::And(vec![
                Expr::Term(projection_match(
                    atom(Var::Projection),
                    atom(Var::SelfTy),
                    atom(Var::Assoc),
                    atom(Var::Trait),
                )),
                Expr::Term(impl_assoc_type(
                    atom(Var::ImplSelf),
                    atom(Var::ImplTrait),
                    atom(Var::Assoc),
                    atom(Var::Value),
                )),
                Expr::Term(same_type(atom(Var::Trait), atom(Var::ImplTrait))),
                Expr::Term(impl_self_match(atom(Var::SelfTy), atom(Var::ImplSelf))),
                Expr::Term(impl_assoc_value_without_bindings(
                    atom(Var::ImplSelf),
                    atom(Var::Value),
                )),
            ]),
        ),
        Clause::rule(
            projection_normalizes_to(
                atom(Var::Projection),
                atom(Var::SelfTy),
                atom(Var::Assoc),
                atom(Var::Trait),
                atom(Var::Substituted),
            ),
            Expr::And(vec![
                Expr::Term(projection_match(
                    atom(Var::Projection),
                    atom(Var::SelfTy),
                    atom(Var::Assoc),
                    atom(Var::Trait),
                )),
                Expr::Term(impl_assoc_type(
                    atom(Var::ImplSelf),
                    atom(Var::ImplTrait),
                    atom(Var::Assoc),
                    atom(Var::Value),
                )),
                Expr::Term(same_type(atom(Var::Trait), atom(Var::ImplTrait))),
                Expr::Term(impl_self_match(atom(Var::SelfTy), atom(Var::ImplSelf))),
                Expr::Term(type_binding(
                    atom(Var::SelfTy),
                    atom(Var::ImplSelf),
                    atom(Var::Generic),
                    atom(Var::Arg),
                )),
                Expr::Term(type_substitution(
                    atom(Var::SelfTy),
                    atom(Var::ImplSelf),
                    atom(Var::Value),
                    atom(Var::Generic),
                    atom(Var::Arg),
                    atom(Var::Substituted),
                )),
            ]),
        ),
    ]
}

/// # Examples
///
/// * Code - `<T as Trait>::Assoc`
/// * Input - `projection = ty0`, `self_ = ty1`, `assoc = def2`, `trait_ = ty3`
/// * Output - `#explicit_projection_obligation(ty0, ty1, def2, ty3).`
/// * Code - `<T>::Assoc`
/// * Input - `projection = ty0`, `self_ = ty1`, `assoc = def2`, `trait_ = None`
/// * Output - `#projection_obligation(ty0, ty1, def2).`
///
pub(crate) fn projection_obligation_clause(obligation: ProjectionObligation) -> Clause<'static> {
    let projection = type_id(obligation.projection);
    let self_ = type_id(obligation.self_);
    let assoc = def_id(obligation.assoc);
    if let Some(trait_) = obligation.trait_ {
        Clause::fact(explicit_projection_obligation(
            projection,
            self_,
            assoc,
            type_id(trait_),
        ))
    } else {
        Clause::fact(projection_obligation(projection, self_, assoc))
    }
}

/// # Examples
///
/// * Code - `T: Trait`
/// * Input - `subject = ty0`, `trait_ = ty1`
/// * Output - `#trait_bound(ty0, ty1).`
pub(crate) fn trait_bound_clause(bound: TraitBound) -> Clause<'static> {
    Clause::fact(trait_bound(type_id(bound.subject), type_id(bound.trait_)))
}

/// * impl_self - Implementing self type in `impl Trait for Self`
/// * trait_ - Implemented trait type in `impl Trait for Self`
/// * assoc - Associated type definition assigned by the impl item
/// * value_ty - Type assigned by the impl item
///
/// # Examples
///
/// * Code - `impl Iterator for Vec { type Item = u32; }`
/// * Input - `impl_self = ty0`, `trait_ = ty1`, `assoc = def2`, `value_ty = ty3`
/// * Output - `#impl_assoc_type(ty0, ty1, def2, ty3).`
pub(crate) fn impl_assoc_type_clause(assoc_type: ImplAssocType) -> Clause<'static> {
    Clause::fact(impl_assoc_type(
        type_id(assoc_type.impl_self),
        type_id(assoc_type.trait_),
        def_id(assoc_type.assoc),
        type_id(assoc_type.value_ty),
    ))
}

/// * projection_self - Self type from the projection, such as `Vec<u32>`
/// * impl_self - Self type from the impl header, such as `Vec<T>`
///
/// # Examples
///
/// * Code - `<Vec<u32> as Iterator>::Item` matched against `impl<T> Iterator for Vec<T>`
/// * Input - `projection_self = ty0`, `impl_self = ty1`
/// * Output - `#impl_self_match(ty0, ty1).`
pub(crate) fn impl_self_match_clause(match_: ImplSelfMatch) -> Clause<'static> {
    Clause::fact(impl_self_match(
        type_id(match_.projection_self),
        type_id(match_.impl_self),
    ))
}

/// * projection_self - Self type from the projection, such as `Vec<u32>`
/// * impl_self - Self type from the impl header, such as `Vec<T>`
/// * generic - Generic type occurrence from the impl self type, such as `T`
/// * arg - Type argument matched for the generic, such as `u32`
///
/// # Examples
///
/// * Code - `<Vec<u32> as Iterator>::Item` matched against `impl<T> Iterator for Vec<T>`
/// * Input - `projection_self = ty0`, `impl_self = ty1`, `generic = ty2`, `arg = ty3`
/// * Output - `#type_binding(ty0, ty1, ty2, ty3).`
pub(crate) fn type_binding_clause(binding: ImplSelfGenericBinding) -> Clause<'static> {
    Clause::fact(type_binding(
        type_id(binding.projection_self),
        type_id(binding.impl_self),
        type_id(binding.generic),
        type_id(binding.arg),
    ))
}

/// * projection_self - Self type from the projection that requested the substitution
/// * impl_self - Self type from the impl header whose value type is substituted
/// * value_ty - Type before substitution, such as `T`
/// * generic - Generic type occurrence being substituted, such as `T`
/// * arg - Type argument used for the generic, such as `u32`
/// * substituted - Type after substitution, such as `u32`
///
/// # Examples
///
/// * Code - `type Item = T` with `Vec<T>` matched against `Vec<u32>`
/// * Input - `projection_self = ty0`, `impl_self = ty1`, `value_ty = ty2`,
///   `generic = ty2`, `arg = ty3`, `substituted = ty3`
/// * Output - `#type_substitution(ty0, ty1, ty2, ty2, ty3, ty3).`
pub(crate) fn type_substitution_clause(
    substitution: ProjectionTypeSubstitution,
) -> Clause<'static> {
    Clause::fact(type_substitution(
        type_id(substitution.projection_self),
        type_id(substitution.impl_self),
        type_id(substitution.value_ty),
        type_id(substitution.generic),
        type_id(substitution.arg),
        type_id(substitution.substituted),
    ))
}

/// * impl_self - Implementing self type in `impl Trait for Self`
/// * value_ty - Associated type value that does not depend on impl-self generic bindings
///
/// # Examples
///
/// * Code - `impl Add<&usize> for &usize { type Output = usize; }`
/// * Output - `#impl_assoc_value_without_bindings(impl_self, usize).`
pub(crate) fn impl_assoc_value_without_bindings_clause(
    impl_self: TypeId,
    value_ty: TypeId,
) -> Clause<'static> {
    Clause::fact(impl_assoc_value_without_bindings(
        type_id(impl_self),
        type_id(value_ty),
    ))
}

/// * projection - Type occurrence for the whole projection, such as `<T>::Assoc`
/// * self_ - Type written as the projection self type, such as `T`
/// * assoc - Associated type definition found inside the candidate trait
/// * trait_ - Candidate trait type that provides the associated type
///
/// # Examples
///
/// * Code - `<T>::Assoc` matched against `Trait::Assoc`
/// * Input - `projection = ty0`, `self_ = ty1`, `assoc = def2`, `trait_ = ty3`
/// * Output - `#projection_match(ty0, ty1, def2, ty3).`
pub(crate) fn projection_match_clause(match_: ProjectionMatch) -> Clause<'static> {
    Clause::fact(projection_match(
        type_id(match_.projection),
        type_id(match_.self_),
        def_id(match_.assoc),
        type_id(match_.trait_),
    ))
}

/// * left - One lowered inference type id
/// * right - Another lowered inference type id with the same stored [`crate::Type`] shape
///
/// # Examples
///
/// * Code - two lowered ids store the same type shape
/// * Input - `left = ty0`, `right = ty1`
/// * Output - `#type_equal(ty0, ty1).`
pub(crate) fn projection_type_equal_clause(left: TypeId, right: TypeId) -> Clause<'static> {
    type_equal_clause(type_id(left), type_id(right))
}

/// * projection - Type occurrence for the whole projection, such as `<T>::Assoc`
/// * self_ - Type written as the projection self type, such as `T`
/// * assoc - Definition of the associated type being projected, such as `Trait::Assoc`
///
/// # Examples
///
/// * Code - what trait can `<T>::Assoc` use?
/// * Input - `projection = ty0`, `self_ = ty1`, `assoc = def2`
/// * Output - `Expr::Term(#projection_candidate(ty0, ty1, def2, $Trait))`
pub(crate) fn projection_candidate_trait_query<'cx>(
    projection: TypeId,
    self_: TypeId,
    assoc: DefId,
) -> Expr<'cx> {
    Expr::Term(projection_candidate(
        type_id(projection),
        type_id(self_),
        def_id(assoc),
        atom(Var::Trait),
    ))
}

/// Query for all currently provable projection normalizations.
///
/// # Examples
///
/// * Code - what projection normalizations are known?
/// * Output - `Expr::Term(#projection_normalizes_to($Projection, $Self, $Assoc, $Trait, $Value))`
pub(crate) fn projection_normalization_query<'cx>() -> Expr<'cx> {
    Expr::Term(projection_normalizes_to(
        atom(Var::Projection),
        atom(Var::SelfTy),
        atom(Var::Assoc),
        atom(Var::Trait),
        atom(Var::Value),
    ))
}

/// * Output - `#impl_self_match($Self, $ImplSelf),
///   #type_shape($Self, #preserve_generics, $Shape),
///   #type_shape($ImplSelf, #variable_generics, $Shape)`
pub(crate) fn impl_self_match_query<'cx>() -> Expr<'cx> {
    Expr::And(vec![
        Expr::Term(impl_self_match(atom(Var::SelfTy), atom(Var::ImplSelf))),
        Expr::Term(type_shape(
            atom(Var::SelfTy),
            type_shape_mode(TypeShapeMode::PreserveGenerics),
            atom(Var::Shape),
        )),
        Expr::Term(type_shape(
            atom(Var::ImplSelf),
            type_shape_mode(TypeShapeMode::VariableGenerics),
            atom(Var::Shape),
        )),
    ])
}

/// * projection - Type occurrence for the whole projection, such as `<T>::Assoc`
/// * self_ - Type written as the projection self type, such as `T`
/// * assoc - Definition of the associated type being projected, such as `Trait::Assoc`
/// * trait_ - Candidate trait type that may provide the associated type
///
/// # Examples
///
/// * Code - `<T>::Assoc` with candidate `Trait`
/// * Input - `projection = ty0`, `self_ = ty1`, `assoc = def2`, `trait_ = ty3`
/// * Output - `#projection_candidate(ty0, ty1, def2, ty3)`
fn projection_candidate<'cx>(
    projection: Term<'cx>,
    self_: Term<'cx>,
    assoc: Term<'cx>,
    trait_: Term<'cx>,
) -> Term<'cx> {
    term(
        Rel::ProjectionCandidate,
        vec![projection, self_, assoc, trait_],
    )
}

/// * projection - Type occurrence for the whole projection, such as `<T>::Assoc`
/// * self_ - Type written as the projection self type, such as `T`
/// * assoc - Associated type definition found inside the candidate trait
/// * trait_ - Candidate trait type that provides the associated type
///
/// # Examples
///
/// * Code - `<T>::Assoc` matched against `Trait::Assoc`
/// * Input - `projection = ty0`, `self_ = ty1`, `assoc = def2`, `trait_ = ty3`
/// * Output - `#projection_match(ty0, ty1, def2, ty3)`
fn projection_match<'cx>(
    projection: Term<'cx>,
    self_: Term<'cx>,
    assoc: Term<'cx>,
    trait_: Term<'cx>,
) -> Term<'cx> {
    term(Rel::ProjectionMatch, vec![projection, self_, assoc, trait_])
}

/// * projection - Type occurrence for the whole projection, such as `<Vec as Iterator>::Item`
/// * self_ - Type written as the projection self type, such as `Vec`
/// * assoc - Associated type member used for normalization, such as `Iterator::Item`
/// * trait_ - Trait type that provides the associated type, such as `Iterator`
/// * value_ty - Type assigned by the matching impl item, such as `u32`
///
/// # Examples
///
/// * Code - `<Vec as Iterator>::Item` normalized through `impl Iterator for Vec { type Item = u32; }`
/// * Input - `projection = ty0`, `self_ = ty1`, `assoc = def2`, `trait_ = ty3`, `value_ty = ty4`
/// * Output - `#projection_normalizes_to(ty0, ty1, def2, ty3, ty4)`
fn projection_normalizes_to<'cx>(
    projection: Term<'cx>,
    self_: Term<'cx>,
    assoc: Term<'cx>,
    trait_: Term<'cx>,
    value_ty: Term<'cx>,
) -> Term<'cx> {
    term(
        Rel::ProjectionNormalizesTo,
        vec![projection, self_, assoc, trait_, value_ty],
    )
}

/// * projection - Type occurrence for the whole projection, such as `<T as Trait>::Assoc`
/// * self_ - Type written as the projection self type, such as `T`
/// * assoc - Definition of the associated type being projected, such as `Trait::Assoc`
/// * trait_ - Explicit trait path type, such as `Trait`
///
/// # Examples
///
/// * Code - `<T as Trait>::Assoc`
/// * Input - `projection = ty0`, `self_ = ty1`, `assoc = def2`, `trait_ = ty3`
/// * Output - `#explicit_projection_obligation(ty0, ty1, def2, ty3)`
fn explicit_projection_obligation<'cx>(
    projection: Term<'cx>,
    self_: Term<'cx>,
    assoc: Term<'cx>,
    trait_: Term<'cx>,
) -> Term<'cx> {
    term(
        Rel::ExplicitProjectionObligation,
        vec![projection, self_, assoc, trait_],
    )
}

/// * impl_self - Implementing self type in `impl Trait for Self`
/// * trait_ - Implemented trait type in `impl Trait for Self`
/// * assoc - Associated type definition assigned by the impl item
/// * value_ty - Type assigned by the impl item
///
/// # Examples
///
/// * Code - `impl Iterator for Vec { type Item = u32; }`
/// * Input - `impl_self = ty0`, `trait_ = ty1`, `assoc = def2`, `value_ty = ty3`
/// * Output - `#impl_assoc_type(ty0, ty1, def2, ty3)`
fn impl_assoc_type<'cx>(
    impl_self: Term<'cx>,
    trait_: Term<'cx>,
    assoc: Term<'cx>,
    value_ty: Term<'cx>,
) -> Term<'cx> {
    term(Rel::ImplAssocType, vec![impl_self, trait_, assoc, value_ty])
}

fn impl_assoc_value_without_bindings<'cx>(impl_self: Term<'cx>, value_ty: Term<'cx>) -> Term<'cx> {
    term(
        Rel::ImplAssocValueWithoutBindings,
        vec![impl_self, value_ty],
    )
}

/// * projection_self - Self type from the projection, such as `Vec<u32>`
/// * impl_self - Self type from the impl header, such as `Vec<T>`
///
/// # Examples
///
/// * Code - `<Vec<u32> as Iterator>::Item` matched against `impl<T> Iterator for Vec<T>`
/// * Input - `projection_self = ty0`, `impl_self = ty1`
/// * Output - `#impl_self_match(ty0, ty1)`
fn impl_self_match<'cx>(projection_self: Term<'cx>, impl_self: Term<'cx>) -> Term<'cx> {
    term(Rel::ImplSelfMatch, vec![projection_self, impl_self])
}

/// * projection_self - Candidate self type from a projection match
/// * impl_self - Candidate self type from an impl-associated-type fact
///
/// # Examples
///
/// * Input - `projection_self = ty0`, `impl_self = ty1`
/// * Output - `#impl_self_match_candidate(ty0, ty1)`
fn impl_self_match_candidate<'cx>(projection_self: Term<'cx>, impl_self: Term<'cx>) -> Term<'cx> {
    term(
        Rel::ImplSelfMatchCandidate,
        vec![projection_self, impl_self],
    )
}

/// * projection_self - Self type from the projection, such as `Vec<u32>`
/// * impl_self - Self type from the impl header, such as `Vec<T>`
/// * generic - Generic type occurrence from the impl self type, such as `T`
/// * arg - Type argument matched for the generic, such as `u32`
///
/// # Examples
///
/// * Code - `<Vec<u32> as Iterator>::Item` matched against `impl<T> Iterator for Vec<T>`
/// * Input - `projection_self = ty0`, `impl_self = ty1`, `generic = ty2`, `arg = ty3`
/// * Output - `#type_binding(ty0, ty1, ty2, ty3)`
fn type_binding<'cx>(
    projection_self: Term<'cx>,
    impl_self: Term<'cx>,
    generic: Term<'cx>,
    arg: Term<'cx>,
) -> Term<'cx> {
    term(
        Rel::TypeBinding,
        vec![projection_self, impl_self, generic, arg],
    )
}

/// * projection_self - Self type from the projection that requested the substitution
/// * impl_self - Self type from the impl header whose value type is substituted
/// * value_ty - Type before substitution, such as `T`
/// * generic - Generic type occurrence being substituted, such as `T`
/// * arg - Type argument used for the generic, such as `u32`
/// * substituted - Type after substitution, such as `u32`
///
/// # Examples
///
/// * Code - `type Item = T` with `Vec<T>` matched against `Vec<u32>`
/// * Input - `projection_self = ty0`, `impl_self = ty1`, `value_ty = ty2`,
///   `generic = ty2`, `arg = ty3`, `substituted = ty3`
/// * Output - `#type_substitution(ty0, ty1, ty2, ty2, ty3, ty3)`
fn type_substitution<'cx>(
    projection_self: Term<'cx>,
    impl_self: Term<'cx>,
    value_ty: Term<'cx>,
    generic: Term<'cx>,
    arg: Term<'cx>,
    substituted: Term<'cx>,
) -> Term<'cx> {
    term(
        Rel::TypeSubstitution,
        vec![
            projection_self,
            impl_self,
            value_ty,
            generic,
            arg,
            substituted,
        ],
    )
}

/// * projection - Type occurrence for the whole projection, such as `<T>::Assoc`
/// * self_ - Type written as the projection self type, such as `T`
/// * assoc - Definition of the associated type being projected, such as `Assoc`
///
/// # Examples
///
/// * Code - `<T>::Assoc`
/// * Input - `projection = ty0`, `self_ = ty1`, `assoc = def2`
/// * Output - `#projection_obligation(ty0, ty1, def2)`
fn projection_obligation<'cx>(
    projection: Term<'cx>,
    self_: Term<'cx>,
    assoc: Term<'cx>,
) -> Term<'cx> {
    term(Rel::ProjectionObligation, vec![projection, self_, assoc])
}

/// * subject - Type being constrained by the bound, such as `T`
/// * trait_ - Trait required by the bound, such as `Trait`
///
/// # Examples
///
/// * Code - `T: Trait`
/// * Input - `subject = ty0`, `trait_ = ty1`
/// * Output - `#trait_bound(ty0, ty1)`
fn trait_bound<'cx>(subject: Term<'cx>, trait_: Term<'cx>) -> Term<'cx> {
    term(Rel::TraitBound, vec![subject, trait_])
}
