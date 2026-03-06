use super::PathId;
use crate::ds::vec::BoxedSlice;
use any_intern::Interned;
use logic_eval_util::unique::{PairIter, UniqueContainer};
use std::{
    cell::Cell,
    fmt::{self, Write},
    hash::{Hash, Hasher},
    iter, ops,
    ptr::NonNull,
};

#[derive(Debug, Clone)]
pub enum Type<'gcx> {
    Scalar(TypeScalar),
    Path(TypePath<'gcx>),
    Tuple(TypeTuple),
    Array(TypeArray),
    Ref(TypeRef),
    Mut(TypeMut),
    Unit,
}

impl<'gcx> Hash for Type<'gcx> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        #[repr(u8)]
        enum TypeKind {
            Scalar,
            Path,
            Tuple,
            Array,
            Ref,
            Mut,
            Unit,
        }

        match self {
            Self::Scalar(scalar) => {
                state.write_u8(TypeKind::Scalar as u8);
                scalar.hash(state);
            }
            Self::Path(TypePath { pid, params: _ }) => {
                // We don't have to hash `params` because `pid` is sufficient to identify a type.
                state.write_u8(TypeKind::Path as u8);
                pid.hash(state);
            }
            Self::Tuple(elems) => {
                state.write_u8(TypeKind::Tuple as u8);
                elems.hash(state);
            }
            Self::Array(arr) => {
                state.write_u8(TypeKind::Array as u8);
                arr.hash(state);
            }
            Self::Ref(ref_) => {
                state.write_u8(TypeKind::Ref as u8);
                ref_.hash(state);
            }
            Self::Mut(mut_) => {
                state.write_u8(TypeKind::Mut as u8);
                mut_.hash(state);
            }
            Self::Unit => state.write_u8(TypeKind::Unit as u8),
        }
    }
}

impl<'gcx> PartialEq for Type<'gcx> {
    fn eq(&self, other: &Self) -> bool {
        match self {
            Self::Scalar(l) => matches!(other, Self::Scalar(r) if l == r),
            Self::Path(l) => {
                // We don't have to compare `params` because `pid` is sufficient to identify a
                // type.
                matches!(other, Self::Path(r) if l.pid == r.pid)
            }
            Self::Tuple(l) => matches!(other, Self::Tuple(r) if l == r),
            Self::Array(l) => matches!(other, Self::Array(r) if l == r),
            Self::Ref(l) => matches!(other, Self::Ref(r) if l == r),
            Self::Mut(l) => matches!(other, Self::Mut(r) if l == r),
            Self::Unit => matches!(other, Self::Unit),
        }
    }
}

impl Eq for Type<'_> {}

#[derive(PartialEq, Eq, Hash, Debug, Clone, Copy)]
pub enum TypeScalar {
    Int,
    Float,
    I8,
    I16,
    I32,
    I64,
    I128,
    Isize,
    U8,
    U16,
    U32,
    U64,
    U128,
    Usize,
    F32,
    F64,
    Bool,
}

impl TypeScalar {
    pub(crate) fn from_type_name(name: &str) -> Option<Self> {
        match name {
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
            "bool" => Some(Self::Bool),
            _ => None,
        }
    }

    pub(crate) fn to_type_name(self) -> &'static str {
        match self {
            Self::Int => "i32",
            Self::Float => "f32",
            Self::I8 => "i8",
            Self::I16 => "i16",
            Self::I32 => "i32",
            Self::I64 => "i64",
            Self::I128 => "i128",
            Self::Isize => "isize",
            Self::U8 => "u8",
            Self::U16 => "u16",
            Self::U32 => "u32",
            Self::U64 => "u64",
            Self::U128 => "u128",
            Self::Usize => "usize",
            Self::F32 => "f32",
            Self::F64 => "f64",
            Self::Bool => "bool",
        }
    }
}

#[derive(Debug, Clone)]
pub struct TypePath<'gcx> {
    pub pid: PathId,
    pub params: BoxedSlice<Param<'gcx>>,
}

#[derive(Debug, Clone)]
pub enum Param<'gcx> {
    Self_,
    Other {
        name: Interned<'gcx, str>,
        tid: TypeId,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TypeTuple {
    pub elems: BoxedSlice<TypeId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TypeArray {
    pub elem: TypeId,
    pub len: ArrayLen,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ArrayLen {
    Fixed(usize),
    Dynamic,
    Generic,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TypeRef {
    pub elem: TypeId,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TypeMut {
    pub elem: TypeId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TypeId(pub(crate) usize);

impl TypeId {
    /// # Safety
    ///
    /// Undefined behavior if the given index is out of bounds.
    pub unsafe fn new(index: usize) -> Self {
        Self(index)
    }
}

/// Unlike [`Type`], this type owns the whole type information, which means you don't need to refer
/// to other data structures to investigate the type.
#[derive(PartialEq, Eq, Hash, Clone, Default)]
pub enum OwnedType {
    /// Corresponds to [`Type::Scalar`] and [`Type::Path`].
    Path {
        name: String,
        params: BoxedSlice<OwnedParam>,
    },
    /// Corresponds to [`Type::Tuple`].
    Tuple(BoxedSlice<OwnedType>),
    /// Corresponds to [`Type::Array`].
    Array { elem: Box<OwnedType>, len: ArrayLen },
    /// Corresponds to [`Type::Ref`].
    Ref { elem: Box<OwnedType> },
    /// Corresponds to [`Type::Mut`].
    Mut { elem: Box<OwnedType> },
    /// Corresponds to [`Type::Unit`].
    #[default]
    Unit,
}

impl fmt::Debug for OwnedType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Path { name, .. } if name == "int" => f.write_str("int"),
            Self::Path { name, .. } if name == "float" => f.write_str("float"),
            Self::Path { name, .. } if name == "i8" => f.write_str("i8"),
            Self::Path { name, .. } if name == "i16" => f.write_str("i16"),
            Self::Path { name, .. } if name == "i32" => f.write_str("i32"),
            Self::Path { name, .. } if name == "i64" => f.write_str("i64"),
            Self::Path { name, .. } if name == "i128" => f.write_str("i128"),
            Self::Path { name, .. } if name == "isize" => f.write_str("isize"),
            Self::Path { name, .. } if name == "u8" => f.write_str("u8"),
            Self::Path { name, .. } if name == "u16" => f.write_str("u16"),
            Self::Path { name, .. } if name == "u32" => f.write_str("u32"),
            Self::Path { name, .. } if name == "u64" => f.write_str("u64"),
            Self::Path { name, .. } if name == "u128" => f.write_str("u128"),
            Self::Path { name, .. } if name == "usize" => f.write_str("usize"),
            Self::Path { name, .. } if name == "f32" => f.write_str("f32"),
            Self::Path { name, .. } if name == "f64" => f.write_str("f64"),
            Self::Path { name, .. } if name == "bool" => f.write_str("bool"),
            Self::Path { name, params } => f
                .debug_struct("Path")
                .field("name", name)
                .field("params", params)
                .finish(),
            Self::Tuple(elems) => f.debug_tuple("Tuple").field(elems).finish(),
            Self::Array { elem, len } => f
                .debug_struct("Array")
                .field("elem", elem)
                .field("len", len)
                .finish(),
            Self::Ref { elem } => f.debug_struct("Ref").field("elem", elem).finish(),
            Self::Mut { elem } => f.debug_struct("Mut").field("elem", elem).finish(),
            Self::Unit => f.write_str("Unit"),
        }
    }
}

impl fmt::Display for OwnedType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Path { name, .. } => f.write_str(name),
            Self::Tuple(elems) => {
                f.write_char('(')?;
                for elem in elems.iter() {
                    fmt::Display::fmt(elem, f)?;
                }
                f.write_char(')')
            }
            Self::Array { elem, len } => match len {
                ArrayLen::Fixed(n) => f.write_fmt(format_args!("[{elem}; {n}]")),
                ArrayLen::Dynamic => f.write_fmt(format_args!("[{elem}]")),
                ArrayLen::Generic => f.write_fmt(format_args!("[{elem}; N]")),
            },
            Self::Ref { elem } => f.write_fmt(format_args!("&{elem}")),
            Self::Mut { elem } => f.write_fmt(format_args!("&mut {elem}")),
            Self::Unit => f.write_str("()"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum OwnedParam {
    Self_,
    Other { name: String, ty: OwnedType },
}

pub(crate) trait CreateOwnedType {
    fn create_owned_type(&self, tid: TypeId) -> OwnedType;
}

thread_local! {
    pub(crate) static OWNED_TYPE_CREATOR: Cell<Option<NonNull<dyn CreateOwnedType>>> = const {
        Cell::new(None)
    };
}

pub struct UniqueTypes<'gcx>(UniqueContainer<Type<'gcx>>);

impl<'gcx> UniqueTypes<'gcx> {
    pub(crate) fn new() -> Self {
        Self(UniqueContainer::new())
    }

    pub fn get(&self, tid: TypeId) -> Option<&Type<'gcx>> {
        self.0.get(tid.0)
    }

    pub fn iter(&self) -> TypeIter<'_, 'gcx> {
        TypeIter(self.0.iter())
    }

    pub(crate) fn insert(&mut self, ty: Type<'gcx>) -> TypeId {
        let index = self.0.insert(ty);
        TypeId(index)
    }
}

impl<'gcx> ops::Deref for UniqueTypes<'gcx> {
    type Target = UniqueContainer<Type<'gcx>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl ops::DerefMut for UniqueTypes<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<'gcx> ops::Index<TypeId> for UniqueTypes<'gcx> {
    type Output = Type<'gcx>;

    fn index(&self, id: TypeId) -> &Self::Output {
        &self.0[id.0]
    }
}

pub struct TypeIter<'a, 'gcx>(PairIter<'a, Type<'gcx>>);

impl<'a, 'gcx> Iterator for TypeIter<'a, 'gcx> {
    type Item = (TypeId, &'a Type<'gcx>);

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|(index, ty)| (TypeId(index), ty))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}

impl ExactSizeIterator for TypeIter<'_, '_> {
    fn len(&self) -> usize {
        self.0.len()
    }
}

impl iter::FusedIterator for TypeIter<'_, '_> {}
