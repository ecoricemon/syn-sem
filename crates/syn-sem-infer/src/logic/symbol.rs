//! Centralized logic symbols used by solver terms.

use syn_sem_name::DefId;

/// Relation symbols used as the top-level predicate of facts, rules, and queries.
///
/// A `Rel` names rows in the logic database, such as `TypeShape` or `ProjectionMatch`. Structured
/// values inside relation arguments use [`Ctor`] instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Rel {
    ExplicitProjectionObligation,
    ImplAssocType,
    ImplAssocValueWithoutBindings,
    ImplSelfMatch,
    ImplSelfMatchCandidate,
    ProjectionCandidate,
    ProjectionMatch,
    ProjectionNormalizesTo,
    ProjectionObligation,
    SameType,
    TraitBound,
    TypeBinding,
    TypeClass,
    TypeShape,
    TypeSubstitution,
}

/// Constructor symbols used to build structured terms inside relation arguments.
///
/// A `Ctor` is not a database relation by itself. It is the lower-level functor for data terms such
/// as type shapes, const values, modes, and argument lists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Ctor {
    Arg,
    Array,
    AssocConstArg,
    AssocTypeArg,
    ConstBool,
    ConstFloat,
    ConstInt,
    ConstUsize,
    Def,
    GenericParam,
    Infer,
    LenConst,
    LenExpr,
    Mut,
    Name,
    Path,
    Primitive,
    PreserveGenerics,
    Ref,
    Slice,
    Tuple,
    VariableGenerics,
}

/// Logic variables used by rules and open queries.
///
/// `Var` atoms are the only symbols treated as unification variables by `logic-eval`.
/// `GenericParam` represents impl-self generic parameters that must behave as variables while
/// matching structural type shapes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Var {
    Arg,
    Assoc,
    Class,
    GenericParam(DefId),
    Generic,
    ImplSelf,
    ImplTrait,
    Left,
    Projection,
    Right,
    SelfTy,
    Subject,
    Substituted,
    Trait,
    Type,
    Value,
    Shape,
}
