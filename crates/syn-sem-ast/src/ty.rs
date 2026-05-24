use crate::{Expr, FromSyn, InputDesc, Path, Span, SyntaxCx};
use syn_sem_macros::CheckDropless;

/// A Rust type supported by the semantic AST.
///
/// Examples include `T`, `&mut T`, `[T; N]`, `[T]`, and `(A, B)`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub enum Type<'cx> {
    /// Fixed-length array type.
    Array(TypeArray<'cx>),
    /// Inferred type placeholder.
    Infer(Span<'cx>),
    /// Path type.
    Path(TypePath<'cx>),
    /// Borrowed reference type.
    Reference(TypeReference<'cx>),
    /// Dynamically sized slice type.
    Slice(TypeSlice<'cx>),
    /// Tuple type.
    Tuple(TypeTuple<'cx>),
}

impl<'cx> Type<'cx> {
    /// Creates the unit type `()`.
    pub fn unit(span: Span<'cx>) -> Self {
        Self::Tuple(TypeTuple::unit(span))
    }
}

impl<'cx> FromSyn<'cx, syn::Type> for Type<'cx> {
    fn from_syn(scx: &'cx SyntaxCx<'cx>, desc: InputDesc<'cx, syn::Type>) -> Self {
        match desc.input {
            syn::Type::Array(v) => Self::Array(TypeArray::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: v,
                },
            )),
            syn::Type::Infer(v) => Self::Infer(Span::from_locatable(scx, desc.file_path, v)),
            syn::Type::Path(v) => Self::Path(TypePath::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: v,
                },
            )),
            syn::Type::Reference(v) => Self::Reference(TypeReference::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: v,
                },
            )),
            syn::Type::Slice(v) => Self::Slice(TypeSlice::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: v,
                },
            )),
            syn::Type::Tuple(v) => Self::Tuple(TypeTuple::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: v,
                },
            )),
            _ => todo!(),
        }
    }
}

/// An array type with a fixed length expression.
///
/// For example, `[u8; 4]`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct TypeArray<'cx> {
    /// Element type.
    pub elem: &'cx Type<'cx>,
    /// Length expression.
    pub len: Expr<'cx>,
    /// Source span of the array type.
    pub span: Span<'cx>,
}

impl<'cx> FromSyn<'cx, syn::TypeArray> for TypeArray<'cx> {
    fn from_syn(scx: &'cx SyntaxCx<'cx>, desc: InputDesc<'cx, syn::TypeArray>) -> Self {
        Self {
            elem: scx.alloc(Type::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.elem,
                },
            )),
            len: Expr::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.len,
                },
            ),
            span: Span::from_locatable(scx, desc.file_path, desc.input),
        }
    }
}

/// A path type.
///
/// Examples include `Vec<T>`, `crate::module::Type`, and `Self::Assoc`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct TypePath<'cx> {
    /// Path naming the type.
    pub path: Path<'cx>,
    /// Source span of the path type.
    pub span: Span<'cx>,
}

impl<'cx> FromSyn<'cx, syn::TypePath> for TypePath<'cx> {
    fn from_syn(scx: &'cx SyntaxCx<'cx>, desc: InputDesc<'cx, syn::TypePath>) -> Self {
        Self {
            path: Path::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.path,
                },
            ),
            span: Span::from_locatable(scx, desc.file_path, desc.input),
        }
    }
}

/// A borrowed reference type.
///
/// Examples include `&T` and `&mut T`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct TypeReference<'cx> {
    /// Referenced type.
    pub elem: &'cx Type<'cx>,
    /// Whether the reference is mutable.
    pub is_mut: bool,
    /// Source span of the reference type.
    pub span: Span<'cx>,
}

impl<'cx> FromSyn<'cx, syn::TypeReference> for TypeReference<'cx> {
    fn from_syn(scx: &'cx SyntaxCx<'cx>, desc: InputDesc<'cx, syn::TypeReference>) -> Self {
        Self {
            elem: scx.alloc(Type::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.elem,
                },
            )),
            is_mut: desc.input.mutability.is_some(),
            span: Span::from_locatable(scx, desc.file_path, desc.input),
        }
    }
}

/// A dynamically sized slice type.
///
/// For example, `[u8]`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct TypeSlice<'cx> {
    /// Element type.
    pub elem: &'cx Type<'cx>,
    /// Source span of the slice type.
    pub span: Span<'cx>,
}

impl<'cx> FromSyn<'cx, syn::TypeSlice> for TypeSlice<'cx> {
    fn from_syn(scx: &'cx SyntaxCx<'cx>, desc: InputDesc<'cx, syn::TypeSlice>) -> Self {
        Self {
            elem: scx.alloc(Type::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.elem,
                },
            )),
            span: Span::from_locatable(scx, desc.file_path, desc.input),
        }
    }
}

/// A tuple type.
///
/// Examples include `()`, `(A,)`, and `(A, B)`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct TypeTuple<'cx> {
    /// Tuple element types.
    pub elems: &'cx [Type<'cx>],
    /// Source span of the tuple type.
    pub span: Span<'cx>,
}

impl<'cx> TypeTuple<'cx> {
    /// Creates the unit tuple type.
    pub fn unit(span: Span<'cx>) -> Self {
        Self { elems: &[], span }
    }
}

impl<'cx> FromSyn<'cx, syn::TypeTuple> for TypeTuple<'cx> {
    fn from_syn(scx: &'cx SyntaxCx<'cx>, desc: InputDesc<'cx, syn::TypeTuple>) -> Self {
        Self {
            elems: FromSyn::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.elems,
                },
            ),
            span: Span::from_locatable(scx, desc.file_path, desc.input),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::*;

    #[test]
    fn type_reference() {
        // Proves reference types preserve whether the reference is mutable.
        let ccx = syn_sem_common::CommonCx::new();
        let cx = SyntaxCx::new(&ccx);

        let ty = parse::<syn::TypeReference, TypeReference>(&cx, "&T");
        assert!(!ty.is_mut);

        let ty = parse::<syn::TypeReference, TypeReference>(&cx, "&mut T");
        assert!(ty.is_mut);
    }

    #[test]
    fn type_path() {
        // Proves type paths preserve generic arguments on the owning path segment.
        let ccx = syn_sem_common::CommonCx::new();
        let cx = SyntaxCx::new(&ccx);

        let ty = parse::<syn::TypePath, TypePath>(&cx, "Vec<T>");
        assert_eq!(&*ty.path.segments[0].ident, "Vec");
        assert!(ty.path.segments[0].has_args());

        let ty = parse::<syn::TypePath, TypePath>(&cx, "std::vec::Vec<T>");
        assert_eq!(ty.path.segments.len(), 3);
        assert!(!ty.path.segments[0].has_args());
        assert!(!ty.path.segments[1].has_args());
        assert!(ty.path.segments[2].has_args());
    }
}
