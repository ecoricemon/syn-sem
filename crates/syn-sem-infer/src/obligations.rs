use crate::TypeId;
use syn_sem_name::DefId;

/// Associated type projection that needs solver work.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ProjectionObligation {
    /// Type occurrence whose value is the projection result.
    pub(crate) projection: TypeId,
    /// Associated type definition selected by name lookup.
    pub(crate) assoc_type: DefId,
    /// Self type for the projection, when represented.
    pub(crate) self_ty: Option<TypeId>,
    /// Trait type for the projection, when represented.
    pub(crate) trait_ty: Option<TypeId>,
}

/// Trait bound fact collected as solver input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TraitBoundFact {
    /// Type constrained by the trait bound.
    pub(crate) subject: TypeId,
    /// Trait type required by the bound.
    pub(crate) trait_ty: TypeId,
}

/// Associated type value assigned by a trait implementation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AssocTypeImplFact {
    /// Implementing self type in `impl Trait for Self`.
    pub(crate) impl_self_ty: TypeId,
    /// Implemented trait type in `impl Trait for Self`.
    pub(crate) trait_ty: TypeId,
    /// Associated type definition assigned by the impl item.
    pub(crate) assoc_type: DefId,
    /// Type assigned by the impl item.
    pub(crate) value_ty: TypeId,
}

/// Impl self type pattern matched against a projection self type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ImplSelfMatch {
    /// Self type from the projection, such as `Vec<u32>`.
    pub(crate) projection_self_ty: TypeId,
    /// Self type from the impl header, such as `Vec<T>`.
    pub(crate) impl_self_ty: TypeId,
}

/// Generic type binding discovered while matching an impl self type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TypeBindingFact {
    /// Self type from the projection, such as `Vec<u32>`.
    pub(crate) projection_self_ty: TypeId,
    /// Self type from the impl header, such as `Vec<T>`.
    pub(crate) impl_self_ty: TypeId,
    /// Generic type occurrence from the impl self type, such as `T`.
    pub(crate) generic_ty: TypeId,
    /// Type argument matched for the generic, such as `u32`.
    pub(crate) arg_ty: TypeId,
}

/// Type substitution fact used while normalizing associated type projections.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TypeSubstitution {
    /// Self type from the projection that requested the substitution.
    pub(crate) projection_self_ty: TypeId,
    /// Self type from the impl header whose value type is substituted.
    pub(crate) impl_self_ty: TypeId,
    /// Type before substitution, such as `T`.
    pub(crate) value_ty: TypeId,
    /// Generic type occurrence being substituted, such as `T`.
    pub(crate) generic_ty: TypeId,
    /// Type argument used for the generic, such as `u32`.
    pub(crate) arg_ty: TypeId,
    /// Type after substitution, such as `u32`.
    pub(crate) substituted_ty: TypeId,
}

/// Candidate trait selected for an associated type projection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ProjectionCandidate {
    /// Type occurrence whose value is the projection result.
    pub(crate) projection: TypeId,
    /// Self type for the projection.
    pub(crate) self_ty: TypeId,
    /// Associated type definition selected by name lookup.
    pub(crate) assoc_type: DefId,
    /// Candidate trait type that may provide the associated type.
    pub(crate) trait_ty: TypeId,
}

/// Associated type projection matched against a concrete trait member.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ProjectionMatch {
    /// Type occurrence whose value is the projection result.
    pub(crate) projection: TypeId,
    /// Self type for the projection.
    pub(crate) self_ty: TypeId,
    /// Associated type member found in the candidate trait.
    pub(crate) assoc_type: DefId,
    /// Trait type that provides the associated type member.
    pub(crate) trait_ty: TypeId,
}

/// Associated type projection normalized to an impl-provided value type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ProjectionNormalization {
    /// Type occurrence whose value is the projection result.
    pub(crate) projection: TypeId,
    /// Self type for the projection.
    pub(crate) self_ty: TypeId,
    /// Associated type member used for normalization.
    pub(crate) assoc_type: DefId,
    /// Trait type that provides the associated type member.
    pub(crate) trait_ty: TypeId,
    /// Type assigned by the matching impl item.
    pub(crate) value_ty: TypeId,
}
