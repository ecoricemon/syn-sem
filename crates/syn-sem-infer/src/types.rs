use crate::TypeId;
use syn_sem_hir as hir;
use syn_sem_name::DefId;

/// Type shape used by inference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type<'cx> {
    /// Fixed-length array type.
    Array {
        /// Element type.
        elem_tid: TypeId,
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
        elem_tid: TypeId,
        /// Whether the reference is mutable.
        is_mut: bool,
    },
    /// Dynamically sized slice type.
    Slice {
        /// Element type.
        elem_tid: TypeId,
    },
    /// Tuple type.
    Tuple {
        /// Tuple element types.
        elem_tids: Vec<TypeId>,
    },
}

/// Primitive Rust type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimitiveType {
    /// Unsuffixed integer literal type before it is constrained to a concrete integer primitive.
    AbstractInt,
    /// Unsuffixed floating-point literal type before it is constrained to a concrete float primitive.
    AbstractFloat,
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
    pub(crate) fn is_abstract_numeric(self) -> bool {
        matches!(self, Self::AbstractInt | Self::AbstractFloat)
    }

    pub(crate) fn is_abstract_of(self, concrete: Self) -> bool {
        matches!(
            (self, concrete),
            (
                Self::AbstractInt,
                Self::I8
                    | Self::I16
                    | Self::I32
                    | Self::I64
                    | Self::I128
                    | Self::Isize
                    | Self::U8
                    | Self::U16
                    | Self::U32
                    | Self::U64
                    | Self::U128
                    | Self::Usize
            ) | (Self::AbstractFloat, Self::F32 | Self::F64)
        )
    }

    pub(crate) fn from_hir_path(path: &[hir::PathSegment<'_>]) -> Option<Self> {
        let [segment] = path else {
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
    pub self_tid: TypeId,
    /// Trait path in `<Self as Trait>`, when present.
    pub trait_tid: Option<TypeId>,
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
    Ambiguous(Vec<DefId>),
    /// No target is known for the path yet.
    Unresolved,
}

/// Associated type projection known to path type resolution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectionType {
    /// Associated type definition selected for the projection.
    pub assoc_type: DefId,
    /// Self type for the projection, when represented.
    pub self_tid: Option<TypeId>,
    /// Trait type for the projection, when represented.
    pub trait_tid: Option<TypeId>,
}

/// Plain path segments used by inference.
///
/// Qualified self metadata belongs to [`PathType`]; this payload only stores the source-order
/// segments shared by qualified and unqualified type paths.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Path<'cx> {
    /// Path segments in source order.
    pub segments: Vec<PathSegment<'cx>>,
}

/// One type path segment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathSegment<'cx> {
    /// Segment name.
    pub name: syn_sem_name::Name<'cx>,
    /// Generic arguments on this segment.
    pub args: Vec<GenericArg<'cx>>,
}

/// Generic argument shape used by inference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GenericArg<'cx> {
    /// Type argument.
    Type(TypeId),
    /// Const expression argument.
    Const(ConstArg<'cx>),
    /// Associated type equality.
    AssocType {
        /// Associated type name.
        name: syn_sem_name::Name<'cx>,
        /// Assigned type.
        tid: TypeId,
    },
    /// Associated const equality.
    AssocConst {
        /// Associated const name.
        name: syn_sem_name::Name<'cx>,
        /// Assigned const value.
        value: ConstArg<'cx>,
    },
    /// Associated type constraint.
    Constraint {
        /// Associated type name.
        name: syn_sem_name::Name<'cx>,
        /// Source bounds.
        bounds: Vec<TypeParamBound<'cx>>,
    },
    /// Unsupported argument form.
    Unsupported,
}

/// Array length expression shape used by inference.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArrayLen {
    /// Length is still a HIR expression.
    Expr(hir::ExprId),
}

impl ArrayLen {
    pub(crate) fn from_hir(len: hir::ArrayLen) -> Self {
        match len {
            hir::ArrayLen::Expr(expr) => Self::Expr(expr),
        }
    }
}

/// Const argument shape used by inference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConstArg<'cx> {
    /// Literal const argument.
    Lit(Lit<'cx>),
    /// Path const argument.
    Path(Path<'cx>),
    /// Const expression argument.
    Expr(hir::ExprId),
}

/// Literal shape used by inference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Lit<'cx> {
    /// Integer literal stored as normalized base-10 digits.
    Int(syn_sem_common::InternedStr<'cx>),
    /// Floating-point literal stored as normalized base-10 digits.
    Float(syn_sem_common::InternedStr<'cx>),
    /// Boolean literal.
    Bool(bool),
}

impl<'cx> Lit<'cx> {
    pub(crate) fn from_hir(lit: &hir::Lit<'cx>) -> Self {
        match lit {
            hir::Lit::Int(value) => Self::Int(*value),
            hir::Lit::Float(value) => Self::Float(*value),
            hir::Lit::Bool(value) => Self::Bool(*value),
        }
    }
}

/// Type parameter bound shape used by inference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeParamBound<'cx> {
    /// Trait bound.
    Trait(Path<'cx>),
    /// Unsupported bound form.
    Unsupported,
}
