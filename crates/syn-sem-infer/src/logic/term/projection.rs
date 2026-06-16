//! Logic rules and predicates for associated type projection solving.

use super::atom::{def_id, term, type_id, var, LogicAtom, LogicClause, LogicTerm};
use crate::{
    AssocTypeImplFact, ImplSelfMatch, ProjectionCandidate, ProjectionMatch, ProjectionObligation,
    TraitBoundFact, TypeBindingFact, TypeId, TypeSubstitution,
};
use logic_eval::{Clause, Expr};
use syn_sem_common::CommonCx;
use syn_sem_name::DefId;

// In examples below, `tyN` encodes `TypeId::new(N)` and `defN` encodes `DefId::new(N)`.
const PRED_EXPLICIT_PROJECTION_OBLIGATION: &str = "explicit_projection_obligation";
const PRED_IMPL_ASSOC_TYPE: &str = "impl_assoc_type";
const PRED_IMPL_SELF_MATCH: &str = "impl_self_match";
const PRED_PROJECTION_OBLIGATION: &str = "projection_obligation";
const PRED_PROJECTION_CANDIDATE: &str = "projection_candidate";
const PRED_PROJECTION_MATCH: &str = "projection_match";
const PRED_PROJECTION_NORMALIZES_TO: &str = "projection_normalizes_to";
const PRED_SAME_TYPE: &str = "same_type";
const PRED_TRAIT_MEMBER: &str = "trait_member";
const PRED_TRAIT_BOUND: &str = "trait_bound";
const PRED_TYPE_BINDING: &str = "type_binding";
const PRED_TYPE_SUBSTITUTION: &str = "type_substitution";

const VAR_ARG: &str = "Arg";
const VAR_ASSOC: &str = "Assoc";
const VAR_GENERIC: &str = "Generic";
const VAR_IMPL_SELF: &str = "ImplSelf";
const VAR_IMPL_TRAIT: &str = "ImplTrait";
const VAR_MEMBER_ASSOC: &str = "MemberAssoc";
const VAR_PROJECTION: &str = "Projection";
const VAR_REQUESTED_ASSOC: &str = "RequestedAssoc";
const VAR_SELF: &str = "Self";
const VAR_SUBJECT: &str = "Subject";
const VAR_SUBSTITUTED: &str = "Substituted";
const VAR_TRAIT: &str = "Trait";
const VAR_VALUE: &str = "Value";

/// * Rule 0 - `projection_candidate(P, Self, Assoc, Trait) :-
///   explicit_projection_obligation(P, Self, Assoc, Trait).`
/// * Rule 1 - `projection_candidate(P, Self, Assoc, Trait) :-
///   projection_obligation(P, Self, Assoc), trait_bound(Subject, Trait),
///   same_type(Self, Subject).`
///
/// # Examples
///
/// * Code - `<T as Trait>::Assoc` or `<T>::Assoc` with `T: Trait`
/// * Output clause 0 - `projection_candidate($Projection, $Self, $Assoc, $Trait) :-
///   explicit_projection_obligation($Projection, $Self, $Assoc, $Trait).`
/// * Output clause 1 - `projection_candidate($Projection, $Self, $Assoc, $Trait) :-
///   projection_obligation($Projection, $Self, $Assoc), trait_bound($Subject, $Trait),
///   same_type($Self, $Subject).`
pub(in crate::logic) fn projection_candidate_rules<'cx>(
    ccx: &'cx CommonCx,
) -> [LogicClause<'cx>; 2] {
    [
        Clause {
            head: projection_candidate(
                ccx,
                var(ccx, VAR_PROJECTION),
                var(ccx, VAR_SELF),
                var(ccx, VAR_ASSOC),
                var(ccx, VAR_TRAIT),
            ),
            body: Some(Expr::Term(explicit_projection_obligation(
                ccx,
                var(ccx, VAR_PROJECTION),
                var(ccx, VAR_SELF),
                var(ccx, VAR_ASSOC),
                var(ccx, VAR_TRAIT),
            ))),
        },
        Clause {
            head: projection_candidate(
                ccx,
                var(ccx, VAR_PROJECTION),
                var(ccx, VAR_SELF),
                var(ccx, VAR_ASSOC),
                var(ccx, VAR_TRAIT),
            ),
            body: Some(Expr::And(vec![
                Expr::Term(projection_obligation(
                    ccx,
                    var(ccx, VAR_PROJECTION),
                    var(ccx, VAR_SELF),
                    var(ccx, VAR_ASSOC),
                )),
                Expr::Term(trait_bound(ccx, var(ccx, VAR_SUBJECT), var(ccx, VAR_TRAIT))),
                Expr::Term(same_type(ccx, var(ccx, VAR_SELF), var(ccx, VAR_SUBJECT))),
            ])),
        },
    ]
}

/// * Rule 0 - `projection_match(P, Self, MemberAssoc, Trait) :-
///   projection_candidate(P, Self, RequestedAssoc, Trait),
///   trait_member(Trait, RequestedAssoc, MemberAssoc).`
///
/// # Examples
///
/// * Code - `<T>::Assoc` with `T: Trait` and `Trait::Assoc`
/// * Output clause - `projection_match($Projection, $Self, $MemberAssoc, $Trait) :-
///   projection_candidate($Projection, $Self, $RequestedAssoc, $Trait),
///   trait_member($Trait, $RequestedAssoc, $MemberAssoc).`
pub(in crate::logic) fn projection_match_rules<'cx>(ccx: &'cx CommonCx) -> [LogicClause<'cx>; 1] {
    [Clause {
        head: projection_match(
            ccx,
            var(ccx, VAR_PROJECTION),
            var(ccx, VAR_SELF),
            var(ccx, VAR_MEMBER_ASSOC),
            var(ccx, VAR_TRAIT),
        ),
        body: Some(Expr::And(vec![
            Expr::Term(projection_candidate(
                ccx,
                var(ccx, VAR_PROJECTION),
                var(ccx, VAR_SELF),
                var(ccx, VAR_REQUESTED_ASSOC),
                var(ccx, VAR_TRAIT),
            )),
            Expr::Term(trait_member(
                ccx,
                var(ccx, VAR_TRAIT),
                var(ccx, VAR_REQUESTED_ASSOC),
                var(ccx, VAR_MEMBER_ASSOC),
            )),
        ])),
    }]
}

/// * Rule 0 - `projection_normalizes_to(P, Self, Assoc, Trait, Value) :-
///   projection_match(P, Self, Assoc, Trait),
///   impl_assoc_type(ImplSelf, ImplTrait, Assoc, Value),
///   same_type(Self, ImplSelf), same_type(Trait, ImplTrait).`
/// * Rule 1 - `projection_normalizes_to(P, Self, Assoc, Trait, Substituted) :-
///   projection_match(P, Self, Assoc, Trait),
///   impl_assoc_type(ImplSelf, ImplTrait, Assoc, Value),
///   same_type(Trait, ImplTrait), impl_self_match(Self, ImplSelf),
///   type_binding(Self, ImplSelf, Generic, Arg),
///   type_substitution(Self, ImplSelf, Value, Generic, Arg, Substituted).`
///
/// # Examples
///
/// * Code - `<Vec as Iterator>::Item` with `impl Iterator for Vec { type Item = u32; }`
/// * Output clause - `projection_normalizes_to($Projection, $Self, $Assoc, $Trait, $Value) :-
///   projection_match($Projection, $Self, $Assoc, $Trait),
///   impl_assoc_type($ImplSelf, $ImplTrait, $Assoc, $Value),
///   same_type($Self, $ImplSelf), same_type($Trait, $ImplTrait).`
/// * Code - `<Vec<u32> as Iterator>::Item` with
///   `impl<T> Iterator for Vec<T> { type Item = T; }`
/// * Output clause - `projection_normalizes_to($Projection, $Self, $Assoc, $Trait, $Substituted) :-
///   projection_match($Projection, $Self, $Assoc, $Trait),
///   impl_assoc_type($ImplSelf, $ImplTrait, $Assoc, $Value),
///   same_type($Trait, $ImplTrait), impl_self_match($Self, $ImplSelf),
///   type_binding($Self, $ImplSelf, $Generic, $Arg),
///   type_substitution($Self, $ImplSelf, $Value, $Generic, $Arg, $Substituted).`
pub(in crate::logic) fn projection_normalization_rules<'cx>(
    ccx: &'cx CommonCx,
) -> [LogicClause<'cx>; 2] {
    [
        Clause {
            head: projection_normalizes_to(
                ccx,
                var(ccx, VAR_PROJECTION),
                var(ccx, VAR_SELF),
                var(ccx, VAR_ASSOC),
                var(ccx, VAR_TRAIT),
                var(ccx, VAR_VALUE),
            ),
            body: Some(Expr::And(vec![
                Expr::Term(projection_match(
                    ccx,
                    var(ccx, VAR_PROJECTION),
                    var(ccx, VAR_SELF),
                    var(ccx, VAR_ASSOC),
                    var(ccx, VAR_TRAIT),
                )),
                Expr::Term(impl_assoc_type(
                    ccx,
                    var(ccx, VAR_IMPL_SELF),
                    var(ccx, VAR_IMPL_TRAIT),
                    var(ccx, VAR_ASSOC),
                    var(ccx, VAR_VALUE),
                )),
                Expr::Term(same_type(ccx, var(ccx, VAR_SELF), var(ccx, VAR_IMPL_SELF))),
                Expr::Term(same_type(
                    ccx,
                    var(ccx, VAR_TRAIT),
                    var(ccx, VAR_IMPL_TRAIT),
                )),
            ])),
        },
        Clause {
            head: projection_normalizes_to(
                ccx,
                var(ccx, VAR_PROJECTION),
                var(ccx, VAR_SELF),
                var(ccx, VAR_ASSOC),
                var(ccx, VAR_TRAIT),
                var(ccx, VAR_SUBSTITUTED),
            ),
            body: Some(Expr::And(vec![
                Expr::Term(projection_match(
                    ccx,
                    var(ccx, VAR_PROJECTION),
                    var(ccx, VAR_SELF),
                    var(ccx, VAR_ASSOC),
                    var(ccx, VAR_TRAIT),
                )),
                Expr::Term(impl_assoc_type(
                    ccx,
                    var(ccx, VAR_IMPL_SELF),
                    var(ccx, VAR_IMPL_TRAIT),
                    var(ccx, VAR_ASSOC),
                    var(ccx, VAR_VALUE),
                )),
                Expr::Term(same_type(
                    ccx,
                    var(ccx, VAR_TRAIT),
                    var(ccx, VAR_IMPL_TRAIT),
                )),
                Expr::Term(impl_self_match(
                    ccx,
                    var(ccx, VAR_SELF),
                    var(ccx, VAR_IMPL_SELF),
                )),
                Expr::Term(type_binding(
                    ccx,
                    var(ccx, VAR_SELF),
                    var(ccx, VAR_IMPL_SELF),
                    var(ccx, VAR_GENERIC),
                    var(ccx, VAR_ARG),
                )),
                Expr::Term(type_substitution(
                    ccx,
                    var(ccx, VAR_SELF),
                    var(ccx, VAR_IMPL_SELF),
                    var(ccx, VAR_VALUE),
                    var(ccx, VAR_GENERIC),
                    var(ccx, VAR_ARG),
                    var(ccx, VAR_SUBSTITUTED),
                )),
            ])),
        },
    ]
}

/// * projection - Type occurrence for the whole projection, such as `<T>::Assoc`
/// * self_ty - Type written as the projection self type, such as `T`
/// * assoc_type - Definition of the associated type being projected, such as `Trait::Assoc`
/// * trait_ty - Explicit trait path type, such as `Trait` in `<T as Trait>::Assoc`
///
/// # Examples
///
/// * Code - `<T as Trait>::Assoc`
/// * Input - `projection = ty0`, `self_ty = ty1`, `assoc_type = def2`, `trait_ty = ty3`
/// * Output - `explicit_projection_obligation(ty0, ty1, def2, ty3).`
/// * Code - `<T>::Assoc`
/// * Input - `projection = ty0`, `self_ty = ty1`, `assoc_type = def2`, `trait_ty = None`
/// * Output - `projection_obligation(ty0, ty1, def2).`
pub(in crate::logic) fn projection_obligation_clause<'cx>(
    ccx: &'cx CommonCx,
    obligation: ProjectionObligation,
    self_ty: TypeId,
) -> LogicClause<'cx> {
    let projection = type_id(ccx, obligation.projection);
    let self_ty = type_id(ccx, self_ty);
    let assoc_type = def_id(ccx, obligation.assoc_type);
    if let Some(trait_ty) = obligation.trait_ty {
        Clause {
            head: explicit_projection_obligation(
                ccx,
                projection,
                self_ty,
                assoc_type,
                type_id(ccx, trait_ty),
            ),
            body: None,
        }
    } else {
        Clause {
            head: projection_obligation(ccx, projection, self_ty, assoc_type),
            body: None,
        }
    }
}

/// * subject - Type being constrained by the bound, such as `T`
/// * trait_ty - Trait required by the bound, such as `Trait`
///
/// # Examples
///
/// * Code - `T: Trait`
/// * Input - `subject = ty0`, `trait_ty = ty1`
/// * Output - `trait_bound(ty0, ty1).`
pub(in crate::logic) fn trait_bound_clause<'cx>(
    ccx: &'cx CommonCx,
    bound: TraitBoundFact,
) -> LogicClause<'cx> {
    Clause {
        head: trait_bound(
            ccx,
            type_id(ccx, bound.subject),
            type_id(ccx, bound.trait_ty),
        ),
        body: None,
    }
}

/// * impl_self_ty - Implementing self type in `impl Trait for Self`
/// * trait_ty - Implemented trait type in `impl Trait for Self`
/// * assoc_type - Associated type definition assigned by the impl item
/// * value_ty - Type assigned by the impl item
///
/// # Examples
///
/// * Code - `impl Iterator for Vec { type Item = u32; }`
/// * Input - `impl_self_ty = ty0`, `trait_ty = ty1`, `assoc_type = def2`, `value_ty = ty3`
/// * Output - `impl_assoc_type(ty0, ty1, def2, ty3).`
pub(in crate::logic) fn impl_assoc_type_clause<'cx>(
    ccx: &'cx CommonCx,
    fact: AssocTypeImplFact,
) -> LogicClause<'cx> {
    Clause {
        head: impl_assoc_type(
            ccx,
            type_id(ccx, fact.impl_self_ty),
            type_id(ccx, fact.trait_ty),
            def_id(ccx, fact.assoc_type),
            type_id(ccx, fact.value_ty),
        ),
        body: None,
    }
}

/// * projection_self_ty - Self type from the projection, such as `Vec<u32>`
/// * impl_self_ty - Self type from the impl header, such as `Vec<T>`
///
/// # Examples
///
/// * Code - `<Vec<u32> as Iterator>::Item` matched against `impl<T> Iterator for Vec<T>`
/// * Input - `projection_self_ty = ty0`, `impl_self_ty = ty1`
/// * Output - `impl_self_match(ty0, ty1).`
pub(in crate::logic) fn impl_self_match_clause<'cx>(
    ccx: &'cx CommonCx,
    match_: ImplSelfMatch,
) -> LogicClause<'cx> {
    Clause {
        head: impl_self_match(
            ccx,
            type_id(ccx, match_.projection_self_ty),
            type_id(ccx, match_.impl_self_ty),
        ),
        body: None,
    }
}

/// * projection_self_ty - Self type from the projection, such as `Vec<u32>`
/// * impl_self_ty - Self type from the impl header, such as `Vec<T>`
/// * generic_ty - Generic type occurrence from the impl self type, such as `T`
/// * arg_ty - Type argument matched for the generic, such as `u32`
///
/// # Examples
///
/// * Code - `<Vec<u32> as Iterator>::Item` matched against `impl<T> Iterator for Vec<T>`
/// * Input - `projection_self_ty = ty0`, `impl_self_ty = ty1`, `generic_ty = ty2`, `arg_ty = ty3`
/// * Output - `type_binding(ty0, ty1, ty2, ty3).`
pub(in crate::logic) fn type_binding_clause<'cx>(
    ccx: &'cx CommonCx,
    binding: TypeBindingFact,
) -> LogicClause<'cx> {
    Clause {
        head: type_binding(
            ccx,
            type_id(ccx, binding.projection_self_ty),
            type_id(ccx, binding.impl_self_ty),
            type_id(ccx, binding.generic_ty),
            type_id(ccx, binding.arg_ty),
        ),
        body: None,
    }
}

/// * projection_self_ty - Self type from the projection that requested the substitution
/// * impl_self_ty - Self type from the impl header whose value type is substituted
/// * value_ty - Type before substitution, such as `T`
/// * generic_ty - Generic type occurrence being substituted, such as `T`
/// * arg_ty - Type argument used for the generic, such as `u32`
/// * substituted_ty - Type after substitution, such as `u32`
///
/// # Examples
///
/// * Code - `type Item = T` with `Vec<T>` matched against `Vec<u32>`
/// * Input - `projection_self_ty = ty0`, `impl_self_ty = ty1`, `value_ty = ty2`,
///   `generic_ty = ty2`, `arg_ty = ty3`, `substituted_ty = ty3`
/// * Output - `type_substitution(ty0, ty1, ty2, ty2, ty3, ty3).`
pub(in crate::logic) fn type_substitution_clause<'cx>(
    ccx: &'cx CommonCx,
    substitution: TypeSubstitution,
) -> LogicClause<'cx> {
    Clause {
        head: type_substitution(
            ccx,
            type_id(ccx, substitution.projection_self_ty),
            type_id(ccx, substitution.impl_self_ty),
            type_id(ccx, substitution.value_ty),
            type_id(ccx, substitution.generic_ty),
            type_id(ccx, substitution.arg_ty),
            type_id(ccx, substitution.substituted_ty),
        ),
        body: None,
    }
}

/// * projection - Type occurrence for the whole projection, such as `<T>::Assoc`
/// * self_ty - Type written as the projection self type, such as `T`
/// * assoc_type - Definition requested by the projection path
/// * trait_ty - Candidate trait type that may provide the associated type
///
/// # Examples
///
/// * Code - `<T>::Assoc` with candidate `Trait`
/// * Input - `projection = ty0`, `self_ty = ty1`, `assoc_type = def2`, `trait_ty = ty3`
/// * Output - `projection_candidate(ty0, ty1, def2, ty3).`
pub(in crate::logic) fn projection_candidate_clause<'cx>(
    ccx: &'cx CommonCx,
    candidate: ProjectionCandidate,
) -> LogicClause<'cx> {
    Clause {
        head: projection_candidate(
            ccx,
            type_id(ccx, candidate.projection),
            type_id(ccx, candidate.self_ty),
            def_id(ccx, candidate.assoc_type),
            type_id(ccx, candidate.trait_ty),
        ),
        body: None,
    }
}

/// * projection - Type occurrence for the whole projection, such as `<T>::Assoc`
/// * self_ty - Type written as the projection self type, such as `T`
/// * assoc_type - Associated type definition found inside the candidate trait
/// * trait_ty - Candidate trait type that provides the associated type
///
/// # Examples
///
/// * Code - `<T>::Assoc` matched against `Trait::Assoc`
/// * Input - `projection = ty0`, `self_ty = ty1`, `assoc_type = def2`, `trait_ty = ty3`
/// * Output - `projection_match(ty0, ty1, def2, ty3).`
pub(in crate::logic) fn projection_match_clause<'cx>(
    ccx: &'cx CommonCx,
    match_: ProjectionMatch,
) -> LogicClause<'cx> {
    Clause {
        head: projection_match(
            ccx,
            type_id(ccx, match_.projection),
            type_id(ccx, match_.self_ty),
            def_id(ccx, match_.assoc_type),
            type_id(ccx, match_.trait_ty),
        ),
        body: None,
    }
}

/// * trait_ty - Trait type that owns the member
/// * requested_assoc_type - Associated type definition requested by the projection path
/// * member_assoc_type - Associated type definition found inside the trait
///
/// # Examples
///
/// * Code - `trait Trait { type Assoc; }`
/// * Input - `trait_ty = ty0`, `requested_assoc_type = def1`, `member_assoc_type = def2`
/// * Output - `trait_member(ty0, def1, def2).`
pub(in crate::logic) fn trait_member_clause<'cx>(
    ccx: &'cx CommonCx,
    trait_ty: TypeId,
    requested_assoc_type: DefId,
    member_assoc_type: DefId,
) -> LogicClause<'cx> {
    Clause {
        head: trait_member(
            ccx,
            type_id(ccx, trait_ty),
            def_id(ccx, requested_assoc_type),
            def_id(ccx, member_assoc_type),
        ),
        body: None,
    }
}

/// * left - One lowered inference type id
/// * right - Another lowered inference type id with the same stored [`crate::Type`] shape
///
/// # Examples
///
/// * Code - two lowered ids store the same type shape
/// * Input - `left = ty0`, `right = ty1`
/// * Output - `same_type(ty0, ty1).`
pub(in crate::logic) fn same_type_clause<'cx>(
    ccx: &'cx CommonCx,
    left: TypeId,
    right: TypeId,
) -> LogicClause<'cx> {
    Clause {
        head: same_type(ccx, type_id(ccx, left), type_id(ccx, right)),
        body: None,
    }
}

/// * projection - Type occurrence for the whole projection, such as `<T>::Assoc`
/// * self_ty - Type written as the projection self type, such as `T`
/// * assoc_type - Definition of the associated type being projected, such as `Trait::Assoc`
/// * trait_ty - Candidate trait type that may provide the associated type
///
/// # Examples
///
/// * Code - can `<T>::Assoc` use `Trait`?
/// * Input - `projection = ty0`, `self_ty = ty1`, `assoc_type = def2`, `trait_ty = ty3`
/// * Output - `Expr::Term(projection_candidate(ty0, ty1, def2, ty3))`
pub(in crate::logic) fn projection_candidate_query<'cx>(
    ccx: &'cx CommonCx,
    projection: TypeId,
    self_ty: TypeId,
    assoc_type: DefId,
    trait_ty: TypeId,
) -> Expr<LogicAtom<'cx>> {
    Expr::Term(projection_candidate(
        ccx,
        type_id(ccx, projection),
        type_id(ccx, self_ty),
        def_id(ccx, assoc_type),
        type_id(ccx, trait_ty),
    ))
}

/// * projection - Type occurrence for the whole projection, such as `<T>::Assoc`
/// * self_ty - Type written as the projection self type, such as `T`
/// * assoc_type - Associated type definition found inside the candidate trait
/// * trait_ty - Candidate trait type that provides the associated type
///
/// # Examples
///
/// * Code - can `<T>::Assoc` match `Trait::Assoc`?
/// * Input - `projection = ty0`, `self_ty = ty1`, `assoc_type = def2`, `trait_ty = ty3`
/// * Output - `Expr::Term(projection_match(ty0, ty1, def2, ty3))`
pub(in crate::logic) fn projection_match_query<'cx>(
    ccx: &'cx CommonCx,
    projection: TypeId,
    self_ty: TypeId,
    assoc_type: DefId,
    trait_ty: TypeId,
) -> Expr<LogicAtom<'cx>> {
    Expr::Term(projection_match(
        ccx,
        type_id(ccx, projection),
        type_id(ccx, self_ty),
        def_id(ccx, assoc_type),
        type_id(ccx, trait_ty),
    ))
}

/// * projection - Type occurrence for the whole projection, such as `<Vec as Iterator>::Item`
/// * self_ty - Type written as the projection self type, such as `Vec`
/// * assoc_type - Associated type member used for normalization, such as `Iterator::Item`
/// * trait_ty - Trait type that provides the associated type, such as `Iterator`
/// * value_ty - Type assigned by the matching impl item, such as `u32`
///
/// # Examples
///
/// * Code - can `<Vec as Iterator>::Item` normalize to `u32`?
/// * Input - `projection = ty0`, `self_ty = ty1`, `assoc_type = def2`, `trait_ty = ty3`, `value_ty = ty4`
/// * Output - `Expr::Term(projection_normalizes_to(ty0, ty1, def2, ty3, ty4))`
pub(in crate::logic) fn projection_normalization_query<'cx>(
    ccx: &'cx CommonCx,
    projection: TypeId,
    self_ty: TypeId,
    assoc_type: DefId,
    trait_ty: TypeId,
    value_ty: TypeId,
) -> Expr<LogicAtom<'cx>> {
    Expr::Term(projection_normalizes_to(
        ccx,
        type_id(ccx, projection),
        type_id(ccx, self_ty),
        def_id(ccx, assoc_type),
        type_id(ccx, trait_ty),
        type_id(ccx, value_ty),
    ))
}

/// * projection - Type occurrence for the whole projection, such as `<T>::Assoc`
/// * self_ty - Type written as the projection self type, such as `T`
/// * assoc_type - Definition of the associated type being projected, such as `Trait::Assoc`
/// * trait_ty - Candidate trait type that may provide the associated type
///
/// # Examples
///
/// * Code - `<T>::Assoc` with candidate `Trait`
/// * Input - `projection = ty0`, `self_ty = ty1`, `assoc_type = def2`, `trait_ty = ty3`
/// * Output - `projection_candidate(ty0, ty1, def2, ty3)`
fn projection_candidate<'cx>(
    ccx: &'cx CommonCx,
    projection: LogicTerm<'cx>,
    self_ty: LogicTerm<'cx>,
    assoc_type: LogicTerm<'cx>,
    trait_ty: LogicTerm<'cx>,
) -> LogicTerm<'cx> {
    term(
        ccx,
        PRED_PROJECTION_CANDIDATE,
        vec![projection, self_ty, assoc_type, trait_ty],
    )
}

/// * projection - Type occurrence for the whole projection, such as `<T>::Assoc`
/// * self_ty - Type written as the projection self type, such as `T`
/// * assoc_type - Associated type definition found inside the candidate trait
/// * trait_ty - Candidate trait type that provides the associated type
///
/// # Examples
///
/// * Code - `<T>::Assoc` matched against `Trait::Assoc`
/// * Input - `projection = ty0`, `self_ty = ty1`, `assoc_type = def2`, `trait_ty = ty3`
/// * Output - `projection_match(ty0, ty1, def2, ty3)`
fn projection_match<'cx>(
    ccx: &'cx CommonCx,
    projection: LogicTerm<'cx>,
    self_ty: LogicTerm<'cx>,
    assoc_type: LogicTerm<'cx>,
    trait_ty: LogicTerm<'cx>,
) -> LogicTerm<'cx> {
    term(
        ccx,
        PRED_PROJECTION_MATCH,
        vec![projection, self_ty, assoc_type, trait_ty],
    )
}

/// * projection - Type occurrence for the whole projection, such as `<Vec as Iterator>::Item`
/// * self_ty - Type written as the projection self type, such as `Vec`
/// * assoc_type - Associated type member used for normalization, such as `Iterator::Item`
/// * trait_ty - Trait type that provides the associated type, such as `Iterator`
/// * value_ty - Type assigned by the matching impl item, such as `u32`
///
/// # Examples
///
/// * Code - `<Vec as Iterator>::Item` normalized through `impl Iterator for Vec { type Item = u32; }`
/// * Input - `projection = ty0`, `self_ty = ty1`, `assoc_type = def2`, `trait_ty = ty3`, `value_ty = ty4`
/// * Output - `projection_normalizes_to(ty0, ty1, def2, ty3, ty4)`
fn projection_normalizes_to<'cx>(
    ccx: &'cx CommonCx,
    projection: LogicTerm<'cx>,
    self_ty: LogicTerm<'cx>,
    assoc_type: LogicTerm<'cx>,
    trait_ty: LogicTerm<'cx>,
    value_ty: LogicTerm<'cx>,
) -> LogicTerm<'cx> {
    term(
        ccx,
        PRED_PROJECTION_NORMALIZES_TO,
        vec![projection, self_ty, assoc_type, trait_ty, value_ty],
    )
}

/// * projection - Type occurrence for the whole projection, such as `<T as Trait>::Assoc`
/// * self_ty - Type written as the projection self type, such as `T`
/// * assoc_type - Definition of the associated type being projected, such as `Trait::Assoc`
/// * trait_ty - Explicit trait path type, such as `Trait`
///
/// # Examples
///
/// * Code - `<T as Trait>::Assoc`
/// * Input - `projection = ty0`, `self_ty = ty1`, `assoc_type = def2`, `trait_ty = ty3`
/// * Output - `explicit_projection_obligation(ty0, ty1, def2, ty3)`
fn explicit_projection_obligation<'cx>(
    ccx: &'cx CommonCx,
    projection: LogicTerm<'cx>,
    self_ty: LogicTerm<'cx>,
    assoc_type: LogicTerm<'cx>,
    trait_ty: LogicTerm<'cx>,
) -> LogicTerm<'cx> {
    term(
        ccx,
        PRED_EXPLICIT_PROJECTION_OBLIGATION,
        vec![projection, self_ty, assoc_type, trait_ty],
    )
}

/// * impl_self_ty - Implementing self type in `impl Trait for Self`
/// * trait_ty - Implemented trait type in `impl Trait for Self`
/// * assoc_type - Associated type definition assigned by the impl item
/// * value_ty - Type assigned by the impl item
///
/// # Examples
///
/// * Code - `impl Iterator for Vec { type Item = u32; }`
/// * Input - `impl_self_ty = ty0`, `trait_ty = ty1`, `assoc_type = def2`, `value_ty = ty3`
/// * Output - `impl_assoc_type(ty0, ty1, def2, ty3)`
fn impl_assoc_type<'cx>(
    ccx: &'cx CommonCx,
    impl_self_ty: LogicTerm<'cx>,
    trait_ty: LogicTerm<'cx>,
    assoc_type: LogicTerm<'cx>,
    value_ty: LogicTerm<'cx>,
) -> LogicTerm<'cx> {
    term(
        ccx,
        PRED_IMPL_ASSOC_TYPE,
        vec![impl_self_ty, trait_ty, assoc_type, value_ty],
    )
}

/// * projection_self_ty - Self type from the projection, such as `Vec<u32>`
/// * impl_self_ty - Self type from the impl header, such as `Vec<T>`
///
/// # Examples
///
/// * Code - `<Vec<u32> as Iterator>::Item` matched against `impl<T> Iterator for Vec<T>`
/// * Input - `projection_self_ty = ty0`, `impl_self_ty = ty1`
/// * Output - `impl_self_match(ty0, ty1)`
fn impl_self_match<'cx>(
    ccx: &'cx CommonCx,
    projection_self_ty: LogicTerm<'cx>,
    impl_self_ty: LogicTerm<'cx>,
) -> LogicTerm<'cx> {
    term(
        ccx,
        PRED_IMPL_SELF_MATCH,
        vec![projection_self_ty, impl_self_ty],
    )
}

/// * projection_self_ty - Self type from the projection, such as `Vec<u32>`
/// * impl_self_ty - Self type from the impl header, such as `Vec<T>`
/// * generic_ty - Generic type occurrence from the impl self type, such as `T`
/// * arg_ty - Type argument matched for the generic, such as `u32`
///
/// # Examples
///
/// * Code - `<Vec<u32> as Iterator>::Item` matched against `impl<T> Iterator for Vec<T>`
/// * Input - `projection_self_ty = ty0`, `impl_self_ty = ty1`, `generic_ty = ty2`, `arg_ty = ty3`
/// * Output - `type_binding(ty0, ty1, ty2, ty3)`
fn type_binding<'cx>(
    ccx: &'cx CommonCx,
    projection_self_ty: LogicTerm<'cx>,
    impl_self_ty: LogicTerm<'cx>,
    generic_ty: LogicTerm<'cx>,
    arg_ty: LogicTerm<'cx>,
) -> LogicTerm<'cx> {
    term(
        ccx,
        PRED_TYPE_BINDING,
        vec![projection_self_ty, impl_self_ty, generic_ty, arg_ty],
    )
}

/// * projection_self_ty - Self type from the projection that requested the substitution
/// * impl_self_ty - Self type from the impl header whose value type is substituted
/// * value_ty - Type before substitution, such as `T`
/// * generic_ty - Generic type occurrence being substituted, such as `T`
/// * arg_ty - Type argument used for the generic, such as `u32`
/// * substituted_ty - Type after substitution, such as `u32`
///
/// # Examples
///
/// * Code - `type Item = T` with `Vec<T>` matched against `Vec<u32>`
/// * Input - `projection_self_ty = ty0`, `impl_self_ty = ty1`, `value_ty = ty2`,
///   `generic_ty = ty2`, `arg_ty = ty3`, `substituted_ty = ty3`
/// * Output - `type_substitution(ty0, ty1, ty2, ty2, ty3, ty3)`
fn type_substitution<'cx>(
    ccx: &'cx CommonCx,
    projection_self_ty: LogicTerm<'cx>,
    impl_self_ty: LogicTerm<'cx>,
    value_ty: LogicTerm<'cx>,
    generic_ty: LogicTerm<'cx>,
    arg_ty: LogicTerm<'cx>,
    substituted_ty: LogicTerm<'cx>,
) -> LogicTerm<'cx> {
    term(
        ccx,
        PRED_TYPE_SUBSTITUTION,
        vec![
            projection_self_ty,
            impl_self_ty,
            value_ty,
            generic_ty,
            arg_ty,
            substituted_ty,
        ],
    )
}

/// * projection - Type occurrence for the whole projection, such as `<T>::Assoc`
/// * self_ty - Type written as the projection self type, such as `T`
/// * assoc_type - Definition of the associated type being projected, such as `Assoc`
///
/// # Examples
///
/// * Code - `<T>::Assoc`
/// * Input - `projection = ty0`, `self_ty = ty1`, `assoc_type = def2`
/// * Output - `projection_obligation(ty0, ty1, def2)`
fn projection_obligation<'cx>(
    ccx: &'cx CommonCx,
    projection: LogicTerm<'cx>,
    self_ty: LogicTerm<'cx>,
    assoc_type: LogicTerm<'cx>,
) -> LogicTerm<'cx> {
    term(
        ccx,
        PRED_PROJECTION_OBLIGATION,
        vec![projection, self_ty, assoc_type],
    )
}

/// * subject - Type being constrained by the bound, such as `T`
/// * trait_ty - Trait required by the bound, such as `Trait`
///
/// # Examples
///
/// * Code - `T: Trait`
/// * Input - `subject = ty0`, `trait_ty = ty1`
/// * Output - `trait_bound(ty0, ty1)`
fn trait_bound<'cx>(
    ccx: &'cx CommonCx,
    subject: LogicTerm<'cx>,
    trait_ty: LogicTerm<'cx>,
) -> LogicTerm<'cx> {
    term(ccx, PRED_TRAIT_BOUND, vec![subject, trait_ty])
}

/// * trait_ty - Trait type that owns the member
/// * requested_assoc_type - Associated type definition requested by the projection path
/// * member_assoc_type - Associated type definition found inside the trait
///
/// # Examples
///
/// * Code - `trait Trait { type Assoc; }`
/// * Input - `trait_ty = ty0`, `requested_assoc_type = def1`, `member_assoc_type = def2`
/// * Output - `trait_member(ty0, def1, def2)`
fn trait_member<'cx>(
    ccx: &'cx CommonCx,
    trait_ty: LogicTerm<'cx>,
    requested_assoc_type: LogicTerm<'cx>,
    member_assoc_type: LogicTerm<'cx>,
) -> LogicTerm<'cx> {
    term(
        ccx,
        PRED_TRAIT_MEMBER,
        vec![trait_ty, requested_assoc_type, member_assoc_type],
    )
}

/// * left - One lowered inference type id
/// * right - Another lowered inference type id with the same stored [`crate::Type`] shape
///
/// # Examples
///
/// * Code - two lowered ids store the same type shape
/// * Input - `left = ty0`, `right = ty1`
/// * Output - `same_type(ty0, ty1)`
fn same_type<'cx>(
    ccx: &'cx CommonCx,
    left: LogicTerm<'cx>,
    right: LogicTerm<'cx>,
) -> LogicTerm<'cx> {
    term(ccx, PRED_SAME_TYPE, vec![left, right])
}
