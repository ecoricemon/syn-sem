use crate::{ProjectionCandidate, ProjectionMatch, ProjectionObligation, TraitBoundFact};
use smallvec::SmallVec;
use std::ops::Index;
use syn_sem_common::{CommonCx, Map};
use syn_sem_name::{DefId, NameDb};
use syn_sem_pr as pr;

/// Type information collected for upper semantic inference.
#[derive(Debug, Default)]
pub struct InferDb<'cx> {
    pub(crate) types: Vec<Type<'cx>>,
    pub(crate) repr_types: Map<pr::TypeId, TypeId>,
    pub(crate) projection_obligations: Vec<ProjectionObligation>,
    pub(crate) trait_bound_facts: Vec<TraitBoundFact>,
    pub(crate) projection_candidates: Vec<ProjectionCandidate>,
    pub(crate) projection_matches: Vec<ProjectionMatch>,
}

impl<'cx> InferDb<'cx> {
    /// Builds inference type facts from program representation and name-resolution data.
    pub fn analyze(ccx: &'cx CommonCx, repr: &pr::ProgramRepr<'cx>, names: &NameDb<'cx>) -> Self {
        crate::lower::analyze(ccx, repr, names)
    }

    /// Returns all collected inference types.
    pub fn types(&self) -> &[Type<'cx>] {
        &self.types
    }

    /// Returns the inference type linked to a represented type occurrence.
    pub fn type_for_repr_type(&self, repr_type: pr::TypeId) -> Option<TypeId> {
        self.repr_types.get(&repr_type).copied()
    }

    /// Returns all represented-type-to-inference-type links.
    pub fn repr_types(&self) -> &Map<pr::TypeId, TypeId> {
        &self.repr_types
    }

    /// Returns associated type projections that still need solver work.
    pub fn projection_obligations(&self) -> &[ProjectionObligation] {
        &self.projection_obligations
    }

    /// Returns trait bounds collected as solver input facts.
    pub fn trait_bound_facts(&self) -> &[TraitBoundFact] {
        &self.trait_bound_facts
    }

    /// Returns projection candidates derived from obligations and known trait bounds.
    pub fn projection_candidates(&self) -> &[ProjectionCandidate] {
        &self.projection_candidates
    }

    /// Returns projections matched against concrete associated type members.
    pub fn projection_matches(&self) -> &[ProjectionMatch] {
        &self.projection_matches
    }
}

impl<'cx> Index<TypeId> for InferDb<'cx> {
    type Output = Type<'cx>;

    fn index(&self, id: TypeId) -> &Self::Output {
        &self.types[id.index()]
    }
}

/// Stable identity for one inference type.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TypeId(usize);

impl TypeId {
    /// Creates an id from a raw index.
    pub const fn new(index: usize) -> Self {
        Self(index)
    }

    /// Returns the raw index represented by this id.
    pub const fn index(self) -> usize {
        self.0
    }
}

/// Type shape used by inference.
//
// Allowing `large_enum_variant`: Path types are the common semantic payload here. Boxing `PathType`
// would shrink scalar variants but add one heap allocation for each normal path type.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type<'cx> {
    /// Fixed-length array type.
    Array {
        /// Element type.
        elem: TypeId,
        /// Array length expression shape.
        len: ArrayLen,
    },
    /// Inferred type placeholder.
    Infer,
    /// Primitive Rust type.
    Primitive(PrimitiveType),
    /// Path type.
    Path(PathType<'cx>),
    /// Borrowed reference type.
    Reference {
        /// Referenced type.
        elem: TypeId,
        /// Whether the reference is mutable.
        is_mut: bool,
    },
    /// Dynamically sized slice type.
    Slice {
        /// Element type.
        elem: TypeId,
    },
    /// Tuple type.
    Tuple {
        /// Tuple element types.
        elems: Vec<TypeId>,
    },
}

/// Primitive Rust type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimitiveType {
    /// `bool`.
    Bool,
    /// `char`.
    Char,
    /// `str`.
    Str,
    /// `i8`.
    I8,
    /// `i16`.
    I16,
    /// `i32`.
    I32,
    /// `i64`.
    I64,
    /// `i128`.
    I128,
    /// `isize`.
    Isize,
    /// `u8`.
    U8,
    /// `u16`.
    U16,
    /// `u32`.
    U32,
    /// `u64`.
    U64,
    /// `u128`.
    U128,
    /// `usize`.
    Usize,
    /// `f32`.
    F32,
    /// `f64`.
    F64,
}

impl PrimitiveType {
    pub(crate) fn from_repr_path(path: &pr::TypePathValue<'_>) -> Option<Self> {
        let [segment] = path.segments.as_slice() else {
            return None;
        };
        if !segment.args.is_empty() {
            return None;
        }
        Self::from_str(segment.name.as_ref())
    }

    fn from_str(value: &str) -> Option<Self> {
        match value {
            "bool" => Some(Self::Bool),
            "char" => Some(Self::Char),
            "str" => Some(Self::Str),
            "i8" => Some(Self::I8),
            "i16" => Some(Self::I16),
            "i32" => Some(Self::I32),
            "i64" => Some(Self::I64),
            "i128" => Some(Self::I128),
            "isize" => Some(Self::Isize),
            "u8" => Some(Self::U8),
            "u16" => Some(Self::U16),
            "u32" => Some(Self::U32),
            "u64" => Some(Self::U64),
            "u128" => Some(Self::U128),
            "usize" => Some(Self::Usize),
            "f32" => Some(Self::F32),
            "f64" => Some(Self::F64),
            _ => None,
        }
    }
}

/// Path type used by inference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathType<'cx> {
    /// Qualified self type, when the source used qualified path syntax.
    pub qself: Option<QSelf>,
    /// Path naming the type.
    pub path: Path<'cx>,
    /// Current resolution state for this type path.
    pub resolution: PathTypeResolution,
}

/// Qualified self type for an inference path type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QSelf {
    /// Self type written inside `<...>`.
    pub self_ty: TypeId,
    /// Trait path in `<Self as Trait>`, when present.
    pub trait_ty: Option<TypeId>,
}

/// Resolution state for a non-primitive path type.
///
/// This records the best current classification without pretending that name lookup is full Rust
/// type resolution. Solver-backed generic substitution, qualified paths, and projection
/// normalization can refine this later.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathTypeResolution {
    /// Path names a nominal type definition.
    Nominal(DefId),
    /// Path names a generic type parameter.
    GenericParam(DefId),
    /// Path denotes an associated type projection.
    Projection(ProjectionType),
    /// Multiple candidates matched the path.
    Ambiguous(SmallVec<[DefId; 2]>),
    /// No target is known for the path yet.
    Unresolved,
}

/// Associated type projection known to path type resolution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectionType {
    /// Associated type definition selected for the projection.
    pub assoc_type: DefId,
    /// Self type for the projection, when represented.
    pub self_ty: Option<TypeId>,
    /// Trait type for the projection, when represented.
    pub trait_ty: Option<TypeId>,
}

/// Type path used by inference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Path<'cx> {
    /// Path segments in source order.
    pub segments: SmallVec<[PathSegment<'cx>; 3]>,
}

/// One type path segment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathSegment<'cx> {
    /// Segment name.
    pub name: syn_sem_name::Name<'cx>,
    /// Generic arguments on this segment.
    pub args: SmallVec<[GenericArgument<'cx>; 2]>,
}

/// Generic argument shape used by inference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GenericArgument<'cx> {
    /// Type argument.
    Type(TypeId),
    /// Const expression argument.
    Const(SourceConstArg),
    /// Associated type equality.
    AssocType {
        /// Associated type name.
        name: syn_sem_name::Name<'cx>,
        /// Assigned type.
        ty: TypeId,
    },
    /// Associated const equality.
    AssocConst {
        /// Associated const name.
        name: syn_sem_name::Name<'cx>,
        /// Assigned const value.
        value: SourceConstArg,
    },
    /// Associated type constraint.
    Constraint {
        /// Associated type name.
        name: syn_sem_name::Name<'cx>,
        /// Source bounds.
        bounds: SourceTypeBounds,
    },
    /// Unsupported argument form.
    Unsupported,
}

/// Array length represented before expression lowering exists.
// TODO: Replace this with expression-backed or evaluated array length facts once const
// expression representation is available.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArrayLen {
    /// Length is still a source expression.
    SourceExpr,
}

impl ArrayLen {
    pub(crate) fn from_repr(len: pr::ArrayLen) -> Self {
        match len {
            pr::ArrayLen::SourceExpr => Self::SourceExpr,
        }
    }
}

/// Const argument represented before expression lowering exists.
// TODO: Replace this with expression-backed const argument facts once const expression
// representation is available.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceConstArg;

/// Type bounds represented before bound lowering exists.
// TODO: Replace this with bound-backed facts once type bound representation is available.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceTypeBounds;
