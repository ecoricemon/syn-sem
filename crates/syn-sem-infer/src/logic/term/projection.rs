//! Logic rules and predicates for associated type projection solving.

use super::{
    atom::{def_id, term, type_id, var, LogicAtom, LogicClause, LogicTerm},
    equality::{same_type, SameTypeRules},
};
use crate::{
    AssocTypeImplFact, ImplSelfMatch, ProjectionMatch, ProjectionObligation, TraitBoundFact,
    TypeBindingFact, TypeId, TypeSubstitution,
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
const PRED_TRAIT_BOUND: &str = "trait_bound";
const PRED_TYPE_BINDING: &str = "type_binding";
const PRED_TYPE_SUBSTITUTION: &str = "type_substitution";

const VAR_ARG: &str = "$Arg";
const VAR_ASSOC: &str = "$Assoc";
const VAR_GENERIC: &str = "$Generic";
const VAR_IMPL_SELF: &str = "$ImplSelf";
const VAR_IMPL_TRAIT: &str = "$ImplTrait";
const VAR_PROJECTION: &str = "$Projection";
const VAR_SELF: &str = "$Self";
const VAR_SUBJECT: &str = "$Subject";
const VAR_SUBSTITUTED: &str = "$Substituted";
const VAR_TRAIT: &str = "$Trait";
const VAR_VALUE: &str = "$Value";

/// Projection needs reflexive and reverse type-shape equality, and does not need transitive
/// closure.
pub(in crate::logic) const PROJECTION_SAME_TYPE_RULES: SameTypeRules = SameTypeRules {
    reflexive: true,
    reverse: true,
    transitive: false,
};

/// * For `<T as Trait>::Assoc`,
/// * Output clause 0 - `projection_candidate($Projection, $Self, $Assoc, $Trait) :-
///   explicit_projection_obligation($Projection, $Self, $Assoc, $Trait).`
///
/// * For `<T>::Assoc` with `T: Trait`,
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
                var(ccx.intern(VAR_PROJECTION)),
                var(ccx.intern(VAR_SELF)),
                var(ccx.intern(VAR_ASSOC)),
                var(ccx.intern(VAR_TRAIT)),
            ),
            body: Some(Expr::Term(explicit_projection_obligation(
                ccx,
                var(ccx.intern(VAR_PROJECTION)),
                var(ccx.intern(VAR_SELF)),
                var(ccx.intern(VAR_ASSOC)),
                var(ccx.intern(VAR_TRAIT)),
            ))),
        },
        Clause {
            head: projection_candidate(
                ccx,
                var(ccx.intern(VAR_PROJECTION)),
                var(ccx.intern(VAR_SELF)),
                var(ccx.intern(VAR_ASSOC)),
                var(ccx.intern(VAR_TRAIT)),
            ),
            body: Some(Expr::And(vec![
                Expr::Term(projection_obligation(
                    ccx,
                    var(ccx.intern(VAR_PROJECTION)),
                    var(ccx.intern(VAR_SELF)),
                    var(ccx.intern(VAR_ASSOC)),
                )),
                Expr::Term(trait_bound(
                    ccx,
                    var(ccx.intern(VAR_SUBJECT)),
                    var(ccx.intern(VAR_TRAIT)),
                )),
                Expr::Term(same_type(
                    ccx,
                    var(ccx.intern(VAR_SELF)),
                    var(ccx.intern(VAR_SUBJECT)),
                )),
            ])),
        },
    ]
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
                var(ccx.intern(VAR_PROJECTION)),
                var(ccx.intern(VAR_SELF)),
                var(ccx.intern(VAR_ASSOC)),
                var(ccx.intern(VAR_TRAIT)),
                var(ccx.intern(VAR_VALUE)),
            ),
            body: Some(Expr::And(vec![
                Expr::Term(projection_match(
                    ccx,
                    var(ccx.intern(VAR_PROJECTION)),
                    var(ccx.intern(VAR_SELF)),
                    var(ccx.intern(VAR_ASSOC)),
                    var(ccx.intern(VAR_TRAIT)),
                )),
                Expr::Term(impl_assoc_type(
                    ccx,
                    var(ccx.intern(VAR_IMPL_SELF)),
                    var(ccx.intern(VAR_IMPL_TRAIT)),
                    var(ccx.intern(VAR_ASSOC)),
                    var(ccx.intern(VAR_VALUE)),
                )),
                Expr::Term(same_type(
                    ccx,
                    var(ccx.intern(VAR_SELF)),
                    var(ccx.intern(VAR_IMPL_SELF)),
                )),
                Expr::Term(same_type(
                    ccx,
                    var(ccx.intern(VAR_TRAIT)),
                    var(ccx.intern(VAR_IMPL_TRAIT)),
                )),
            ])),
        },
        Clause {
            head: projection_normalizes_to(
                ccx,
                var(ccx.intern(VAR_PROJECTION)),
                var(ccx.intern(VAR_SELF)),
                var(ccx.intern(VAR_ASSOC)),
                var(ccx.intern(VAR_TRAIT)),
                var(ccx.intern(VAR_SUBSTITUTED)),
            ),
            body: Some(Expr::And(vec![
                Expr::Term(projection_match(
                    ccx,
                    var(ccx.intern(VAR_PROJECTION)),
                    var(ccx.intern(VAR_SELF)),
                    var(ccx.intern(VAR_ASSOC)),
                    var(ccx.intern(VAR_TRAIT)),
                )),
                Expr::Term(impl_assoc_type(
                    ccx,
                    var(ccx.intern(VAR_IMPL_SELF)),
                    var(ccx.intern(VAR_IMPL_TRAIT)),
                    var(ccx.intern(VAR_ASSOC)),
                    var(ccx.intern(VAR_VALUE)),
                )),
                Expr::Term(same_type(
                    ccx,
                    var(ccx.intern(VAR_TRAIT)),
                    var(ccx.intern(VAR_IMPL_TRAIT)),
                )),
                Expr::Term(impl_self_match(
                    ccx,
                    var(ccx.intern(VAR_SELF)),
                    var(ccx.intern(VAR_IMPL_SELF)),
                )),
                Expr::Term(type_binding(
                    ccx,
                    var(ccx.intern(VAR_SELF)),
                    var(ccx.intern(VAR_IMPL_SELF)),
                    var(ccx.intern(VAR_GENERIC)),
                    var(ccx.intern(VAR_ARG)),
                )),
                Expr::Term(type_substitution(
                    ccx,
                    var(ccx.intern(VAR_SELF)),
                    var(ccx.intern(VAR_IMPL_SELF)),
                    var(ccx.intern(VAR_VALUE)),
                    var(ccx.intern(VAR_GENERIC)),
                    var(ccx.intern(VAR_ARG)),
                    var(ccx.intern(VAR_SUBSTITUTED)),
                )),
            ])),
        },
    ]
}

/// # Examples
///
/// * Code - `<T as Trait>::Assoc`
/// * Input - `projection = ty0`, `self_ = ty1`, `assoc = def2`, `trait_ = ty3`
/// * Output - `explicit_projection_obligation(ty0, ty1, def2, ty3).`
/// * Code - `<T>::Assoc`
/// * Input - `projection = ty0`, `self_ = ty1`, `assoc = def2`, `trait_ = None`
/// * Output - `projection_obligation(ty0, ty1, def2).`
///
/// # Panics
///
/// Panics if `obligation` does not carry a projection self type. Callers should only pass
/// obligations that can be represented as solver facts.
pub(in crate::logic) fn projection_obligation_clause<'cx>(
    ccx: &'cx CommonCx,
    obligation: ProjectionObligation,
) -> LogicClause<'cx> {
    let projection = type_id(ccx, obligation.projection);
    let self_ = obligation
        .self_
        .expect("projection obligation clause requires a projection self type");
    let self_ = type_id(ccx, self_);
    let assoc = def_id(ccx, obligation.assoc);
    if let Some(trait_) = obligation.trait_ {
        Clause {
            head: explicit_projection_obligation(
                ccx,
                projection,
                self_,
                assoc,
                type_id(ccx, trait_),
            ),
            body: None,
        }
    } else {
        Clause {
            head: projection_obligation(ccx, projection, self_, assoc),
            body: None,
        }
    }
}

/// # Examples
///
/// * Code - `T: Trait`
/// * Input - `subject = ty0`, `trait_ = ty1`
/// * Output - `trait_bound(ty0, ty1).`
pub(in crate::logic) fn trait_bound_clause<'cx>(
    ccx: &'cx CommonCx,
    bound: TraitBoundFact,
) -> LogicClause<'cx> {
    Clause {
        head: trait_bound(ccx, type_id(ccx, bound.subject), type_id(ccx, bound.trait_)),
        body: None,
    }
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
/// * Output - `impl_assoc_type(ty0, ty1, def2, ty3).`
pub(in crate::logic) fn impl_assoc_type_clause<'cx>(
    ccx: &'cx CommonCx,
    fact: AssocTypeImplFact,
) -> LogicClause<'cx> {
    Clause {
        head: impl_assoc_type(
            ccx,
            type_id(ccx, fact.impl_self),
            type_id(ccx, fact.trait_),
            def_id(ccx, fact.assoc),
            type_id(ccx, fact.value_ty),
        ),
        body: None,
    }
}

/// * projection_self - Self type from the projection, such as `Vec<u32>`
/// * impl_self - Self type from the impl header, such as `Vec<T>`
///
/// # Examples
///
/// * Code - `<Vec<u32> as Iterator>::Item` matched against `impl<T> Iterator for Vec<T>`
/// * Input - `projection_self = ty0`, `impl_self = ty1`
/// * Output - `impl_self_match(ty0, ty1).`
pub(in crate::logic) fn impl_self_match_clause<'cx>(
    ccx: &'cx CommonCx,
    match_: ImplSelfMatch,
) -> LogicClause<'cx> {
    Clause {
        head: impl_self_match(
            ccx,
            type_id(ccx, match_.projection_self),
            type_id(ccx, match_.impl_self),
        ),
        body: None,
    }
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
/// * Output - `type_binding(ty0, ty1, ty2, ty3).`
pub(in crate::logic) fn type_binding_clause<'cx>(
    ccx: &'cx CommonCx,
    binding: TypeBindingFact,
) -> LogicClause<'cx> {
    Clause {
        head: type_binding(
            ccx,
            type_id(ccx, binding.projection_self),
            type_id(ccx, binding.impl_self),
            type_id(ccx, binding.generic),
            type_id(ccx, binding.arg),
        ),
        body: None,
    }
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
/// * Output - `type_substitution(ty0, ty1, ty2, ty2, ty3, ty3).`
pub(in crate::logic) fn type_substitution_clause<'cx>(
    ccx: &'cx CommonCx,
    substitution: TypeSubstitution,
) -> LogicClause<'cx> {
    Clause {
        head: type_substitution(
            ccx,
            type_id(ccx, substitution.projection_self),
            type_id(ccx, substitution.impl_self),
            type_id(ccx, substitution.value_ty),
            type_id(ccx, substitution.generic),
            type_id(ccx, substitution.arg),
            type_id(ccx, substitution.substituted),
        ),
        body: None,
    }
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
/// * Output - `projection_match(ty0, ty1, def2, ty3).`
pub(in crate::logic) fn projection_match_clause<'cx>(
    ccx: &'cx CommonCx,
    match_: ProjectionMatch,
) -> LogicClause<'cx> {
    Clause {
        head: projection_match(
            ccx,
            type_id(ccx, match_.projection),
            type_id(ccx, match_.self_),
            def_id(ccx, match_.assoc),
            type_id(ccx, match_.trait_),
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
/// * Output - `type_equal(ty0, ty1).`
pub(in crate::logic) fn projection_type_equal_clause<'cx>(
    ccx: &'cx CommonCx,
    left: TypeId,
    right: TypeId,
) -> LogicClause<'cx> {
    super::equality::type_equal_clause(ccx, type_id(ccx, left), type_id(ccx, right))
}

/// * projection - Type occurrence for the whole projection, such as `<T>::Assoc`
/// * self_ - Type written as the projection self type, such as `T`
/// * assoc - Definition of the associated type being projected, such as `Trait::Assoc`
/// * trait_ - Candidate trait type that may provide the associated type
///
/// # Examples
///
/// * Code - can `<T>::Assoc` use `Trait`?
/// * Input - `projection = ty0`, `self_ = ty1`, `assoc = def2`, `trait_ = ty3`
/// * Output - `Expr::Term(projection_candidate(ty0, ty1, def2, ty3))`
pub(in crate::logic) fn projection_candidate_query<'cx>(
    ccx: &'cx CommonCx,
    projection: TypeId,
    self_: TypeId,
    assoc: DefId,
    trait_: TypeId,
) -> Expr<LogicAtom<'cx>> {
    Expr::Term(projection_candidate(
        ccx,
        type_id(ccx, projection),
        type_id(ccx, self_),
        def_id(ccx, assoc),
        type_id(ccx, trait_),
    ))
}

/// * projection - Type occurrence for the whole projection, such as `<Vec as Iterator>::Item`
/// * self_ - Type written as the projection self type, such as `Vec`
/// * assoc - Associated type member used for normalization, such as `Iterator::Item`
/// * trait_ - Trait type that provides the associated type, such as `Iterator`
/// * value_ty - Type assigned by the matching impl item, such as `u32`
///
/// # Examples
///
/// * Code - can `<Vec as Iterator>::Item` normalize to `u32`?
/// * Input - `projection = ty0`, `self_ = ty1`, `assoc = def2`, `trait_ = ty3`, `value_ty = ty4`
/// * Output - `Expr::Term(projection_normalizes_to(ty0, ty1, def2, ty3, ty4))`
pub(in crate::logic) fn projection_normalization_query<'cx>(
    ccx: &'cx CommonCx,
    projection: TypeId,
    self_: TypeId,
    assoc: DefId,
    trait_: TypeId,
    value_ty: TypeId,
) -> Expr<LogicAtom<'cx>> {
    Expr::Term(projection_normalizes_to(
        ccx,
        type_id(ccx, projection),
        type_id(ccx, self_),
        def_id(ccx, assoc),
        type_id(ccx, trait_),
        type_id(ccx, value_ty),
    ))
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
/// * Output - `projection_candidate(ty0, ty1, def2, ty3)`
fn projection_candidate<'cx>(
    ccx: &'cx CommonCx,
    projection: LogicTerm<'cx>,
    self_: LogicTerm<'cx>,
    assoc: LogicTerm<'cx>,
    trait_: LogicTerm<'cx>,
) -> LogicTerm<'cx> {
    term(
        ccx.intern(PRED_PROJECTION_CANDIDATE),
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
/// * Output - `projection_match(ty0, ty1, def2, ty3)`
fn projection_match<'cx>(
    ccx: &'cx CommonCx,
    projection: LogicTerm<'cx>,
    self_: LogicTerm<'cx>,
    assoc: LogicTerm<'cx>,
    trait_: LogicTerm<'cx>,
) -> LogicTerm<'cx> {
    term(
        ccx.intern(PRED_PROJECTION_MATCH),
        vec![projection, self_, assoc, trait_],
    )
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
/// * Output - `projection_normalizes_to(ty0, ty1, def2, ty3, ty4)`
fn projection_normalizes_to<'cx>(
    ccx: &'cx CommonCx,
    projection: LogicTerm<'cx>,
    self_: LogicTerm<'cx>,
    assoc: LogicTerm<'cx>,
    trait_: LogicTerm<'cx>,
    value_ty: LogicTerm<'cx>,
) -> LogicTerm<'cx> {
    term(
        ccx.intern(PRED_PROJECTION_NORMALIZES_TO),
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
/// * Output - `explicit_projection_obligation(ty0, ty1, def2, ty3)`
fn explicit_projection_obligation<'cx>(
    ccx: &'cx CommonCx,
    projection: LogicTerm<'cx>,
    self_: LogicTerm<'cx>,
    assoc: LogicTerm<'cx>,
    trait_: LogicTerm<'cx>,
) -> LogicTerm<'cx> {
    term(
        ccx.intern(PRED_EXPLICIT_PROJECTION_OBLIGATION),
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
/// * Output - `impl_assoc_type(ty0, ty1, def2, ty3)`
fn impl_assoc_type<'cx>(
    ccx: &'cx CommonCx,
    impl_self: LogicTerm<'cx>,
    trait_: LogicTerm<'cx>,
    assoc: LogicTerm<'cx>,
    value_ty: LogicTerm<'cx>,
) -> LogicTerm<'cx> {
    term(
        ccx.intern(PRED_IMPL_ASSOC_TYPE),
        vec![impl_self, trait_, assoc, value_ty],
    )
}

/// * projection_self - Self type from the projection, such as `Vec<u32>`
/// * impl_self - Self type from the impl header, such as `Vec<T>`
///
/// # Examples
///
/// * Code - `<Vec<u32> as Iterator>::Item` matched against `impl<T> Iterator for Vec<T>`
/// * Input - `projection_self = ty0`, `impl_self = ty1`
/// * Output - `impl_self_match(ty0, ty1)`
fn impl_self_match<'cx>(
    ccx: &'cx CommonCx,
    projection_self: LogicTerm<'cx>,
    impl_self: LogicTerm<'cx>,
) -> LogicTerm<'cx> {
    term(
        ccx.intern(PRED_IMPL_SELF_MATCH),
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
/// * Output - `type_binding(ty0, ty1, ty2, ty3)`
fn type_binding<'cx>(
    ccx: &'cx CommonCx,
    projection_self: LogicTerm<'cx>,
    impl_self: LogicTerm<'cx>,
    generic: LogicTerm<'cx>,
    arg: LogicTerm<'cx>,
) -> LogicTerm<'cx> {
    term(
        ccx.intern(PRED_TYPE_BINDING),
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
/// * Output - `type_substitution(ty0, ty1, ty2, ty2, ty3, ty3)`
fn type_substitution<'cx>(
    ccx: &'cx CommonCx,
    projection_self: LogicTerm<'cx>,
    impl_self: LogicTerm<'cx>,
    value_ty: LogicTerm<'cx>,
    generic: LogicTerm<'cx>,
    arg: LogicTerm<'cx>,
    substituted: LogicTerm<'cx>,
) -> LogicTerm<'cx> {
    term(
        ccx.intern(PRED_TYPE_SUBSTITUTION),
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
/// * Output - `projection_obligation(ty0, ty1, def2)`
fn projection_obligation<'cx>(
    ccx: &'cx CommonCx,
    projection: LogicTerm<'cx>,
    self_: LogicTerm<'cx>,
    assoc: LogicTerm<'cx>,
) -> LogicTerm<'cx> {
    term(
        ccx.intern(PRED_PROJECTION_OBLIGATION),
        vec![projection, self_, assoc],
    )
}

/// * subject - Type being constrained by the bound, such as `T`
/// * trait_ - Trait required by the bound, such as `Trait`
///
/// # Examples
///
/// * Code - `T: Trait`
/// * Input - `subject = ty0`, `trait_ = ty1`
/// * Output - `trait_bound(ty0, ty1)`
fn trait_bound<'cx>(
    ccx: &'cx CommonCx,
    subject: LogicTerm<'cx>,
    trait_: LogicTerm<'cx>,
) -> LogicTerm<'cx> {
    term(ccx.intern(PRED_TRAIT_BOUND), vec![subject, trait_])
}
