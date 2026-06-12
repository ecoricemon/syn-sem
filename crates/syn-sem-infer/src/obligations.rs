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

/// Impl self type pattern matched against a projection self type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImplSelfMatch {
    /// Self type from the projection, such as `Vec<u32>`.
    pub projection_self_ty: TypeId,
    /// Self type from the impl header, such as `Vec<T>`.
    pub impl_self_ty: TypeId,
}

/// Generic type binding discovered while matching an impl self type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TypeBindingFact {
    /// Self type from the projection, such as `Vec<u32>`.
    pub projection_self_ty: TypeId,
    /// Self type from the impl header, such as `Vec<T>`.
    pub impl_self_ty: TypeId,
    /// Generic type occurrence from the impl self type, such as `T`.
    pub generic_ty: TypeId,
    /// Type argument matched for the generic, such as `u32`.
    pub arg_ty: TypeId,
}

/// Type substitution fact used while normalizing associated type projections.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TypeSubstitution {
    /// Self type from the projection that requested the substitution.
    pub projection_self_ty: TypeId,
    /// Self type from the impl header whose value type is substituted.
    pub impl_self_ty: TypeId,
    /// Type before substitution, such as `T`.
    pub value_ty: TypeId,
    /// Generic type occurrence being substituted, such as `T`.
    pub generic_ty: TypeId,
    /// Type argument used for the generic, such as `u32`.
    pub arg_ty: TypeId,
    /// Type after substitution, such as `u32`.
    pub substituted_ty: TypeId,
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
