use crate::{ProjectionCandidate, ProjectionObligation, TraitBoundFact, TypeId};
use logic_eval::{Clause, Expr, Term};
use syn_sem_common::{CommonCx, InternedStr};
use syn_sem_name::DefId;

// In examples below, `tyN` encodes `TypeId::new(N)` and `defN` encodes `DefId::new(N)`.
const PRED_EXPLICIT_PROJECTION_OBLIGATION: &str = "explicit_projection_obligation";
const PRED_PROJECTION_OBLIGATION: &str = "projection_obligation";
const PRED_PROJECTION_CANDIDATE: &str = "projection_candidate";
const PRED_PROJECTION_MATCH: &str = "projection_match";
const PRED_SAME_TYPE: &str = "same_type";
const PRED_TRAIT_MEMBER: &str = "trait_member";
const PRED_TRAIT_BOUND: &str = "trait_bound";

const VAR_ASSOC: &str = "Assoc";
const VAR_MEMBER_ASSOC: &str = "MemberAssoc";
const VAR_PROJECTION: &str = "Projection";
const VAR_REQUESTED_ASSOC: &str = "RequestedAssoc";
const VAR_SELF: &str = "Self";
const VAR_SUBJECT: &str = "Subject";
const VAR_TRAIT: &str = "Trait";

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
pub(super) fn projection_candidate_rules<'cx>(ccx: &'cx CommonCx) -> [LogicClause<'cx>; 2] {
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
pub(super) fn projection_match_rules<'cx>(ccx: &'cx CommonCx) -> [LogicClause<'cx>; 1] {
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
pub(super) fn projection_obligation_clause<'cx>(
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
pub(super) fn trait_bound_clause<'cx>(
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
pub(super) fn projection_candidate_clause<'cx>(
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

/// * trait_ty - Trait type that owns the member
/// * requested_assoc_type - Associated type definition requested by the projection path
/// * member_assoc_type - Associated type definition found inside the trait
///
/// # Examples
///
/// * Code - `trait Trait { type Assoc; }`
/// * Input - `trait_ty = ty0`, `requested_assoc_type = def1`, `member_assoc_type = def2`
/// * Output - `trait_member(ty0, def1, def2).`
pub(super) fn trait_member_clause<'cx>(
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
pub(super) fn same_type_clause<'cx>(
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
pub(super) fn projection_candidate_query<'cx>(
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
pub(super) fn projection_match_query<'cx>(
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

/// * id - Inference type id to encode as a logic atom
///
/// # Examples
///
/// * Input - `id = TypeId::new(0)`
/// * Output - `ty0`
fn type_id<'cx>(ccx: &'cx CommonCx, id: TypeId) -> LogicTerm<'cx> {
    let value = format!("ty{}", id.index());
    atom(ccx, &value)
}

/// def-id atom.
///
/// * id - Name definition id to encode as a logic atom
///
/// # Examples
///
/// * Input - `id = DefId::new(2)`
/// * Output - `def2`
fn def_id<'cx>(ccx: &'cx CommonCx, id: DefId) -> LogicTerm<'cx> {
    let value = format!("def{}", id.index());
    atom(ccx, &value)
}

/// * name - Logic variable name without `$`
///
/// # Examples
///
/// * Input - `name = "Self"`
/// * Output - `$Self`
fn var<'cx>(ccx: &'cx CommonCx, name: &str) -> LogicTerm<'cx> {
    let value = format!("${name}");
    atom(ccx, &value)
}

fn atom<'cx>(ccx: &'cx CommonCx, functor: &str) -> LogicTerm<'cx> {
    term(ccx, functor, Vec::new())
}

fn term<'cx>(ccx: &'cx CommonCx, functor: &str, args: Vec<LogicTerm<'cx>>) -> LogicTerm<'cx> {
    Term {
        functor: ccx.intern(functor),
        args,
    }
}

pub(super) type LogicAtom<'cx> = InternedStr<'cx>;
pub(super) type LogicTerm<'cx> = Term<LogicAtom<'cx>>;
pub(super) type LogicClause<'cx> = Clause<LogicAtom<'cx>>;
