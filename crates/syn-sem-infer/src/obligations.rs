use crate::TypeId;
use syn_sem_hir as hir;
use syn_sem_name::DefId;

/// One trait bound fact collected as solver input.
///
/// A type-bound predicate can contain multiple bounds, such as `T: Debug + Clone`;
/// inference flattens that into one fact per trait bound, e.g. `T: Debug` and `T: Clone`.
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

/// Lowered block fact consumed from HIR body lowering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BodyBlockFact {
    /// Source block represented by this body fact.
    pub(crate) block: hir::BlockId,
    /// Tail expression for the block, when present.
    pub(crate) tail_expr: Option<hir::ExprId>,
}

/// Lowered local fact consumed from HIR body lowering.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BodyLocalFact {
    /// Block containing this local statement.
    pub(crate) block: hir::BlockId,
    /// Source local binding.
    pub(crate) local: hir::LocalId,
    /// Local definitions introduced by the binding pattern.
    pub(crate) bindings: Vec<DefId>,
    /// Initializer expression, when present.
    pub(crate) init: Option<hir::ExprId>,
}

/// Subject whose type can participate in body-local type equality.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum TypeSubject {
    /// A definition such as a parameter or local binding.
    Def(DefId),
    /// A HIR expression occurrence.
    Expr(hir::ExprId),
    /// A concrete inference type.
    Type(TypeId),
}

/// Body-local type equality edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TypeEqualFact {
    /// Left side of the equality edge.
    pub(crate) left: TypeSubject,
    /// Right side of the equality edge.
    pub(crate) right: TypeSubject,
}

/// Resolved concrete type found for a body-local subject.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ResolvedTypeFact {
    /// Subject being resolved.
    pub(crate) subject: TypeSubject,
    /// Concrete inference type reachable from the subject through equality edges.
    pub(crate) ty: TypeId,
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
