use crate::{Expr, FromSyn, InputDesc, Path, Span, SyntaxCx};
use syn_sem_macros::CheckDropless;

/// A Rust type supported by the semantic AST.
///
/// Examples include `T`, `&mut T`, `[T; N]`, `[T]`, and `(A, B)`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub enum Type<'scx> {
    Array(TypeArray<'scx>),
    Infer(Span<'scx>),
    Path(TypePath<'scx>),
    Reference(TypeReference<'scx>),
    Slice(TypeSlice<'scx>),
    Tuple(TypeTuple<'scx>),
}

impl<'scx> Type<'scx> {
    pub fn unit(span: Span<'scx>) -> Self {
        Self::Tuple(TypeTuple::unit(span))
    }
}

impl<'scx> FromSyn<'scx, syn::Type> for Type<'scx> {
    fn from_syn(scx: &'scx SyntaxCx, desc: InputDesc<'_, syn::Type>) -> Self {
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
pub struct TypeArray<'scx> {
    pub elem: &'scx Type<'scx>,
    pub len: Expr<'scx>,
    pub span: Span<'scx>,
}

impl<'scx> FromSyn<'scx, syn::TypeArray> for TypeArray<'scx> {
    fn from_syn(scx: &'scx SyntaxCx, desc: InputDesc<'_, syn::TypeArray>) -> Self {
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
pub struct TypePath<'scx> {
    pub path: Path<'scx>,
    pub span: Span<'scx>,
}

impl<'scx> FromSyn<'scx, syn::TypePath> for TypePath<'scx> {
    fn from_syn(scx: &'scx SyntaxCx, desc: InputDesc<'_, syn::TypePath>) -> Self {
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
pub struct TypeReference<'scx> {
    pub elem: &'scx Type<'scx>,
    pub is_mut: bool,
    pub span: Span<'scx>,
}

impl<'scx> FromSyn<'scx, syn::TypeReference> for TypeReference<'scx> {
    fn from_syn(scx: &'scx SyntaxCx, desc: InputDesc<'_, syn::TypeReference>) -> Self {
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
pub struct TypeSlice<'scx> {
    pub elem: &'scx Type<'scx>,
    pub span: Span<'scx>,
}

impl<'scx> FromSyn<'scx, syn::TypeSlice> for TypeSlice<'scx> {
    fn from_syn(scx: &'scx SyntaxCx, desc: InputDesc<'_, syn::TypeSlice>) -> Self {
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
pub struct TypeTuple<'scx> {
    pub elems: &'scx [Type<'scx>],
    pub span: Span<'scx>,
}

impl<'scx> TypeTuple<'scx> {
    pub fn unit(span: Span<'scx>) -> Self {
        Self { elems: &[], span }
    }
}

impl<'scx> FromSyn<'scx, syn::TypeTuple> for TypeTuple<'scx> {
    fn from_syn(scx: &'scx SyntaxCx, desc: InputDesc<'_, syn::TypeTuple>) -> Self {
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
    fn test_type_reference() {
        // Proves reference types preserve whether the reference is mutable.
        let scx = SyntaxCx::default();

        let ty = parse::<syn::TypeReference, TypeReference>(&scx, "&T");
        assert!(!ty.is_mut);

        let ty = parse::<syn::TypeReference, TypeReference>(&scx, "&mut T");
        assert!(ty.is_mut);
    }

    #[test]
    fn test_type_path() {
        // Proves type paths preserve generic arguments on the owning path segment.
        let scx = SyntaxCx::default();

        let ty = parse::<syn::TypePath, TypePath>(&scx, "Vec<T>");
        assert_eq!(&*ty.path.segments[0].ident, "Vec");
        assert!(ty.path.segments[0].has_args());

        let ty = parse::<syn::TypePath, TypePath>(&scx, "std::vec::Vec<T>");
        assert_eq!(ty.path.segments.len(), 3);
        assert!(!ty.path.segments[0].has_args());
        assert!(!ty.path.segments[1].has_args());
        assert!(ty.path.segments[2].has_args());
    }
}
