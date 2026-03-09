use crate::ds::vec::BoxedSlice;
use any_intern::Interned;
use logic_eval_util::unique::{PairIter, UniqueContainer};
use std::{
    hash::{Hash, Hasher},
    ops,
};

#[derive(Debug, Clone)]
pub(crate) enum Type<'gcx> {
    Scalar(TypeScalar),
    Named(TypeNamed<'gcx>),
    Tuple(TypeTuple),
    Array(TypeArray),
    Ref(TypeRef),
    Mut(TypeMut),
    Unit,
    Var(TypeId),
    Composed(TypeComposed<'gcx>),
    Unknown,
}

impl Hash for Type<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        #[repr(u8)]
        enum TypeKind {
            Scalar,
            Named,
            Tuple,
            Array,
            Ref,
            Mut,
            Unit,
            Var,
            Composed,
            Unknown,
        }

        match self {
            Self::Scalar(scalar) => {
                state.write_u8(TypeKind::Scalar as u8);
                scalar.hash(state);
            }
            Self::Named(TypeNamed { name, params: _ }) => {
                state.write_u8(TypeKind::Named as u8);
                // We don't have to hash `params` because `name` is sufficient to identify a type.
                name.hash(state);
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
            Self::Var(tid) => {
                state.write_u8(TypeKind::Var as u8);
                tid.hash(state);
            }
            Self::Composed(elems) => {
                state.write_u8(TypeKind::Composed as u8);
                elems.hash(state);
            }
            Self::Unknown => state.write_u8(TypeKind::Unknown as u8),
        }
    }
}

impl PartialEq for Type<'_> {
    fn eq(&self, other: &Self) -> bool {
        match self {
            Self::Scalar(l) => matches!(other, Self::Scalar(r) if l == r),
            Self::Named(l) => {
                // We don't have to compare `params` because `name` is sufficient to identify a
                // type.
                matches!(other, Self::Named(r) if l.name == r.name)
            }
            Self::Tuple(l) => matches!(other, Self::Tuple(r) if l == r),
            Self::Array(l) => matches!(other, Self::Array(r) if l == r),
            Self::Ref(l) => matches!(other, Self::Ref(r) if l == r),
            Self::Mut(l) => matches!(other, Self::Mut(r) if l == r),
            Self::Unit => matches!(other, Self::Unit),
            Self::Var(l) => matches!(other, Self::Var(r) if l == r),
            Self::Composed(l) => matches!(other, Self::Composed(r) if l == r),
            Self::Unknown => matches!(other, Self::Unknown),
        }
    }
}

impl Eq for Type<'_> {}

#[derive(PartialEq, Eq, Hash, Debug, Clone, Copy)]
pub(crate) enum TypeScalar {
    Int { reserved: Option<TypeId> },
    Float { reserved: Option<TypeId> },
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
    pub(crate) fn from_str(s: &str) -> Option<Self> {
        match s {
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

    pub(crate) fn is_abstract_of(&self, rhs: &Self) -> bool {
        matches!(
            (self, rhs),
            (
                Self::Int { .. },
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
            ) | (Self::Float { .. }, Self::F32 | Self::F64)
        )
    }
}

#[derive(Debug, Clone)]
pub(crate) struct TypeNamed<'gcx> {
    pub(crate) name: Interned<'gcx, str>,
    /// e.g. ("0", Y) and ("x", X) in `struct Y { x: X }`. (0: output of the constructor)
    /// e.g. ("0", Y) and ("1", X) in `struct Y(X)`. (0: output of the constructor)
    /// e.g. ("0", Y) and ("1", X) in `f(X) -> Y`.
    pub(crate) params: BoxedSlice<Param<'gcx>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct TypeTuple {
    pub(crate) elems: BoxedSlice<TypeId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct TypeArray {
    pub(crate) elem: TypeId,
    pub(crate) len: InferArrayLen,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct TypeRef {
    pub(crate) elem: TypeId,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct TypeMut {
    pub(crate) elem: TypeId,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct TypeComposed<'gcx> {
    pub(crate) elems: BoxedSlice<(Interned<'gcx, str>, TypeId)>,
}

#[derive(Debug, Clone)]
pub(crate) enum Param<'gcx> {
    Self_,
    Other {
        name: Interned<'gcx, str>,
        tid: TypeId,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum InferArrayLen {
    Fixed(usize),
    Dynamic,
    Generic,
    Unknown,
}

#[derive(PartialEq, Eq, Hash, Debug, Clone, Copy)]
pub(crate) struct TypeId(pub(crate) usize);

#[cfg(test)]
use std::fmt;

/// Unlike [`Type`], this type owns the whole type information, which means you don't need to
/// refer to other data structures to investigate the type.
#[cfg(test)]
#[derive(Clone, PartialEq, Eq, Hash)]
pub(crate) enum OwnedType {
    /// Corresponds to both [`Type::Scalar`] and [`Type::Named`].
    Named {
        name: String,
        params: BoxedSlice<OwnedParam>,
    },
    /// Corresponds to [`Type::Tuple`].
    Tuple(BoxedSlice<OwnedType>),
    /// Corresponds to [`Type::Array`].
    Array {
        elem: Box<OwnedType>,
        len: InferArrayLen,
    },
    /// Corresponds to [`Type::Ref`].
    Ref { elem: Box<OwnedType> },
    /// Corresponds to [`Type::Mut`].
    Mut { elem: Box<OwnedType> },
    /// Corresponds to [`Type::Unit`].
    Unit,
    /// Corresponds to [`Type::Var`].
    Var,
    /// Corresponds to [`Type::Composed`].
    Composed(BoxedSlice<(String, OwnedType)>),
    /// Corresponds to [`Type::Unknown`].
    Unknown,
}

#[cfg(test)]
impl fmt::Debug for OwnedType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Named { name, .. } if name == "int" => f.write_str("int"),
            Self::Named { name, .. } if name == "float" => f.write_str("float"),
            Self::Named { name, .. } if name == "i8" => f.write_str("i8"),
            Self::Named { name, .. } if name == "i16" => f.write_str("i16"),
            Self::Named { name, .. } if name == "i32" => f.write_str("i32"),
            Self::Named { name, .. } if name == "i64" => f.write_str("i64"),
            Self::Named { name, .. } if name == "i128" => f.write_str("i128"),
            Self::Named { name, .. } if name == "isize" => f.write_str("isize"),
            Self::Named { name, .. } if name == "u8" => f.write_str("u8"),
            Self::Named { name, .. } if name == "u16" => f.write_str("u16"),
            Self::Named { name, .. } if name == "u32" => f.write_str("u32"),
            Self::Named { name, .. } if name == "u64" => f.write_str("u64"),
            Self::Named { name, .. } if name == "u128" => f.write_str("u128"),
            Self::Named { name, .. } if name == "usize" => f.write_str("usize"),
            Self::Named { name, .. } if name == "f32" => f.write_str("f32"),
            Self::Named { name, .. } if name == "f64" => f.write_str("f64"),
            Self::Named { name, .. } if name == "bool" => f.write_str("bool"),
            Self::Named { name, params } => f
                .debug_struct("Named")
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
            Self::Var => f.write_str("Var"),
            Self::Composed(elems) => f.debug_tuple("Composed").field(elems).finish(),
            Self::Unknown => f.write_str("Unknown"),
        }
    }
}

#[cfg(test)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum OwnedParam {
    Self_,
    Other { name: String, ty: OwnedType },
}

#[derive(Debug)]
pub(crate) struct UniqueTypes<'gcx>(UniqueContainer<Type<'gcx>>);

impl<'gcx> UniqueTypes<'gcx> {
    pub(crate) fn new() -> Self {
        Self(UniqueContainer::new())
    }

    pub(crate) fn iter(&self) -> PairIter<'_, Type<'gcx>> {
        self.0.iter()
    }

    pub(crate) fn insert_type(&mut self, ty: Type<'gcx>) -> TypeId {
        let idx = self.0.insert(ty);
        TypeId(idx)
    }

    pub(crate) fn find_type(&self, tid: TypeId) -> &Type<'gcx> {
        match &self[tid] {
            Type::Var(dst) if tid != *dst => self.find_type(*dst),
            ty => ty,
        }
    }

    pub(crate) fn len(&self) -> usize {
        self.0.len()
    }

    /// Shortens the container to the given length.
    ///
    /// All type ids beyond the length will be invalidated.
    pub(crate) fn truncate(&mut self, len: usize) {
        self.0.truncate(len);
    }

    pub(crate) fn new_type_var(&mut self) -> TypeId {
        let next_idx = self.len();
        let tid = TypeId(next_idx);
        let out = self.insert_type(Type::Var(tid));
        assert_eq!(out, tid);
        tid
    }

    pub(crate) fn replace<Q>(&mut self, old: &Q, new: Type<'gcx>)
    where
        Q: Hash + PartialEq<Type<'gcx>> + ?Sized,
    {
        self.0.replace(old, new);
    }
}

#[cfg(test)]
impl crate::GetOwned<TypeId> for UniqueTypes<'_> {
    type Owned = OwnedType;

    fn get_owned(&self, tid: TypeId) -> Self::Owned {
        fn simple_owned_type(name: &str) -> OwnedType {
            OwnedType::Named {
                name: name.into(),
                params: [].into(),
            }
        }

        match self.find_type(tid) {
            Type::Scalar(TypeScalar::Int { .. }) => simple_owned_type("int"),
            Type::Scalar(TypeScalar::Float { .. }) => simple_owned_type("float"),
            Type::Scalar(TypeScalar::I8) => simple_owned_type("i8"),
            Type::Scalar(TypeScalar::I16) => simple_owned_type("i16"),
            Type::Scalar(TypeScalar::I32) => simple_owned_type("i32"),
            Type::Scalar(TypeScalar::I64) => simple_owned_type("i64"),
            Type::Scalar(TypeScalar::I128) => simple_owned_type("i128"),
            Type::Scalar(TypeScalar::Isize) => simple_owned_type("isize"),
            Type::Scalar(TypeScalar::U8) => simple_owned_type("u8"),
            Type::Scalar(TypeScalar::U16) => simple_owned_type("u16"),
            Type::Scalar(TypeScalar::U32) => simple_owned_type("u32"),
            Type::Scalar(TypeScalar::U64) => simple_owned_type("u64"),
            Type::Scalar(TypeScalar::U128) => simple_owned_type("u128"),
            Type::Scalar(TypeScalar::Usize) => simple_owned_type("usize"),
            Type::Scalar(TypeScalar::F32) => simple_owned_type("f32"),
            Type::Scalar(TypeScalar::F64) => simple_owned_type("f64"),
            Type::Scalar(TypeScalar::Bool) => simple_owned_type("bool"),
            Type::Named(TypeNamed { name, params }) => OwnedType::Named {
                name: (**name).to_owned(),
                params: params
                    .iter()
                    .map(|param| match param {
                        Param::Self_ => OwnedParam::Self_,
                        Param::Other { name, tid } => OwnedParam::Other {
                            name: (**name).to_owned(),
                            ty: self.get_owned(*tid),
                        },
                    })
                    .collect(),
            },
            Type::Tuple(TypeTuple { elems }) => {
                OwnedType::Tuple(elems.iter().map(|elem| self.get_owned(*elem)).collect())
            }
            Type::Array(TypeArray { elem, len }) => OwnedType::Array {
                elem: Box::new(self.get_owned(*elem)),
                len: *len,
            },
            Type::Ref(TypeRef { elem }) => OwnedType::Ref {
                elem: Box::new(self.get_owned(*elem)),
            },
            Type::Mut(TypeMut { elem }) => OwnedType::Mut {
                elem: Box::new(self.get_owned(*elem)),
            },
            Type::Unit => OwnedType::Unit,
            Type::Var(_) => OwnedType::Var,
            Type::Composed(TypeComposed { elems }) => {
                let elems = elems
                    .iter()
                    .map(|(name, tid)| ((**name).to_owned(), self.get_owned(*tid)))
                    .collect();
                OwnedType::Composed(elems)
            }
            Type::Unknown => OwnedType::Unknown,
        }
    }
}

impl Default for UniqueTypes<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'gcx> ops::Index<usize> for UniqueTypes<'gcx> {
    type Output = Type<'gcx>;

    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

impl<'gcx> ops::Index<TypeId> for UniqueTypes<'gcx> {
    type Output = Type<'gcx>;

    fn index(&self, index: TypeId) -> &Self::Output {
        &self[index.0]
    }
}
