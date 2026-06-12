use crate::TypeId;
use syn_sem_name::DefId;

/// Associated type projection that needs solver work.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProjectionObligation {
    /// Type occurrence whose value is the projection result.
    pub projection: TypeId,
    /// Associated type definition selected by name lookup.
    pub assoc_type: DefId,
    /// Self type for the projection, when represented.
    pub self_ty: Option<TypeId>,
    /// Trait type for the projection, when represented.
    pub trait_ty: Option<TypeId>,
}

/// Trait bound fact collected as solver input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TraitBoundFact {
    /// Type constrained by the trait bound.
    pub subject: TypeId,
    /// Trait type required by the bound.
    pub trait_ty: TypeId,
}

/// Associated type value assigned by a trait implementation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AssocTypeImplFact {
    /// Implementing self type in `impl Trait for Self`.
    pub impl_self_ty: TypeId,
    /// Implemented trait type in `impl Trait for Self`.
    pub trait_ty: TypeId,
    /// Associated type definition assigned by the impl item.
    pub assoc_type: DefId,
    /// Type assigned by the impl item.
    pub value_ty: TypeId,
}

/// Candidate trait selected for an associated type projection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProjectionCandidate {
    /// Type occurrence whose value is the projection result.
    pub projection: TypeId,
    /// Self type for the projection.
    pub self_ty: TypeId,
    /// Associated type definition selected by name lookup.
    pub assoc_type: DefId,
    /// Candidate trait type that may provide the associated type.
    pub trait_ty: TypeId,
}

/// Associated type projection matched against a concrete trait member.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProjectionMatch {
    /// Type occurrence whose value is the projection result.
    pub projection: TypeId,
    /// Self type for the projection.
    pub self_ty: TypeId,
    /// Associated type member found in the candidate trait.
    pub assoc_type: DefId,
    /// Trait type that provides the associated type member.
    pub trait_ty: TypeId,
}

/// Associated type projection normalized to an impl-provided value type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProjectionNormalization {
    /// Type occurrence whose value is the projection result.
    pub projection: TypeId,
    /// Self type for the projection.
    pub self_ty: TypeId,
    /// Associated type member used for normalization.
    pub assoc_type: DefId,
    /// Trait type that provides the associated type member.
    pub trait_ty: TypeId,
    /// Type assigned by the matching impl item.
    pub value_ty: TypeId,
}
