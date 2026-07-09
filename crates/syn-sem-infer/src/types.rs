//! Inference-owned type model.
//!
//! These types preserve the source shape of lowered HIR type occurrences while adding inference
//! metadata such as path resolution, associated type projection metadata, and abstract numeric
//! literal primitives.

use crate::TypeId;
use syn_sem_hir as hir;
use syn_sem_name::DefId;

/// Type shape used by inference.
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PrimitiveType {
    /// Unsuffixed integer literal type before it is constrained to a concrete integer primitive.
    AbstractInt,
    /// Unsuffixed floating-point literal type before it is constrained to a concrete float
    /// primitive.
    AbstractFloat,
    /// Boolean primitive type.
    Bool,
    /// Unicode scalar value primitive type.
    Char,
    /// String slice primitive type.
    Str,
    /// Signed 8-bit integer primitive type.
    I8,
    /// Signed 16-bit integer primitive type.
    I16,
    /// Signed 32-bit integer primitive type.
    I32,
    /// Signed 64-bit integer primitive type.
    I64,
    /// Signed 128-bit integer primitive type.
    I128,
    /// Pointer-sized signed integer primitive type.
    Isize,
    /// Unsigned 8-bit integer primitive type.
    U8,
    /// Unsigned 16-bit integer primitive type.
    U16,
    /// Unsigned 32-bit integer primitive type.
    U32,
    /// Unsigned 64-bit integer primitive type.
    U64,
    /// Unsigned 128-bit integer primitive type.
    U128,
    /// Pointer-sized unsigned integer primitive type.
    Usize,
    /// 32-bit floating-point primitive type.
    F32,
    /// 64-bit floating-point primitive type.
    F64,
}

impl PrimitiveType {
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

    /// Returns the primitive type named by a plain single-segment HIR path.
    pub fn from_hir_path(path: &[hir::PathSegment<'_>]) -> Option<Self> {
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
    pub self_: TypeId,
    /// Trait path in `<Self as Trait>`, when present.
    pub trait_: Option<TypeId>,
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
    pub assoc: DefId,
    /// Self type for the projection, when represented.
    pub self_: Option<TypeId>,
    /// Trait type for the projection, when represented.
    pub trait_: Option<TypeId>,
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
        ty: TypeId,
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
    /// Length is a known `usize` const value.
    ConstUsize(usize),
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
    Path {
        /// Source path.
        path: Path<'cx>,
        /// Resolved const item definition, when name resolution found one.
        def: Option<syn_sem_name::DefId>,
    },
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
            hir::Lit::Int(value) => Self::Int(value.digits),
            hir::Lit::Float(value) => Self::Float(value.digits),
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
