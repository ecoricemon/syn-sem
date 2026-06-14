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
    fn from_syn(scx: &'cx SyntaxCx<'cx>, desc: InputDesc<'cx, '_, syn::Type>) -> Self {
        match desc.input {
            syn::Type::Array(v) => Self::Array(TypeArray::from_syn(scx, desc.with_input(v))),
            syn::Type::Infer(v) => Self::Infer(desc.span(v)),
            syn::Type::Path(v) => Self::Path(TypePath::from_syn(scx, desc.with_input(v))),
            syn::Type::Reference(v) => {
                Self::Reference(TypeReference::from_syn(scx, desc.with_input(v)))
            }
            syn::Type::Slice(v) => Self::Slice(TypeSlice::from_syn(scx, desc.with_input(v))),
            syn::Type::Tuple(v) => Self::Tuple(TypeTuple::from_syn(scx, desc.with_input(v))),
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
    ///
    /// Stored by reference to break the recursive [`Type`] shape.
    pub elem: &'cx Type<'cx>,
    /// Length expression.
    pub len: Expr<'cx>,
    /// Source span of the array type.
    pub span: Span<'cx>,
}

impl<'cx> FromSyn<'cx, syn::TypeArray> for TypeArray<'cx> {
    fn from_syn(scx: &'cx SyntaxCx<'cx>, desc: InputDesc<'cx, '_, syn::TypeArray>) -> Self {
        Self {
            elem: scx.alloc(Type::from_syn(scx, desc.with_input(&desc.input.elem))),
            len: Expr::from_syn(scx, desc.with_input(&desc.input.len)),
            span: desc.span(desc.input),
        }
    }
}

/// A path type.
///
/// Examples include `Vec<T>`, `crate::module::Type`, `Self::Assoc`, and `<T as Trait>::Assoc`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct TypePath<'cx> {
    /// Qualified self type, when the path is written as a qualified path.
    pub qself: Option<QSelf<'cx>>,
    /// Path naming the type.
    pub path: Path<'cx>,
    /// Source span of the path type.
    pub span: Span<'cx>,
}

impl<'cx> FromSyn<'cx, syn::TypePath> for TypePath<'cx> {
    fn from_syn(scx: &'cx SyntaxCx<'cx>, desc: InputDesc<'cx, '_, syn::TypePath>) -> Self {
        Self {
            qself: desc
                .input
                .qself
                .as_ref()
                .map(|qself| QSelf::from_syn(scx, desc.with_input(qself))),
            path: Path::from_syn(scx, desc.with_input(&desc.input.path)),
            span: desc.span(desc.input),
        }
    }
}

/// Qualified self type for a path type.
///
/// For `<T as Trait>::Assoc`, `ty` is `T` and `position` is the number of path segments before
/// `Assoc` that belong to the optional trait path.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct QSelf<'cx> {
    /// Self type written inside `<...>`.
    ///
    /// Stored by reference to break the recursive [`Type`] shape.
    pub ty: &'cx Type<'cx>,
    /// Split point in the path before the associated item segment.
    pub position: usize,
    /// Source span of the qualified self type.
    pub span: Span<'cx>,
}

impl<'cx> FromSyn<'cx, syn::QSelf> for QSelf<'cx> {
    fn from_syn(scx: &'cx SyntaxCx<'cx>, desc: InputDesc<'cx, '_, syn::QSelf>) -> Self {
        Self {
            ty: scx.alloc(Type::from_syn(scx, desc.with_input(&desc.input.ty))),
            position: desc.input.position,
            span: desc.span(desc.input),
        }
    }
}

/// A borrowed reference type.
///
/// Examples include `&T` and `&mut T`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct TypeReference<'cx> {
    /// Referenced type.
    ///
    /// Stored by reference to break the recursive [`Type`] shape.
    pub elem: &'cx Type<'cx>,
    /// Whether the reference is mutable.
    pub is_mut: bool,
    /// Source span of the reference type.
    pub span: Span<'cx>,
}

impl<'cx> FromSyn<'cx, syn::TypeReference> for TypeReference<'cx> {
    fn from_syn(scx: &'cx SyntaxCx<'cx>, desc: InputDesc<'cx, '_, syn::TypeReference>) -> Self {
        Self {
            elem: scx.alloc(Type::from_syn(scx, desc.with_input(&desc.input.elem))),
            is_mut: desc.input.mutability.is_some(),
            span: desc.span(desc.input),
        }
    }
}

/// A dynamically sized slice type.
///
/// For example, `[u8]`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct TypeSlice<'cx> {
    /// Element type.
    ///
    /// Stored by reference to break the recursive [`Type`] shape.
    pub elem: &'cx Type<'cx>,
    /// Source span of the slice type.
    pub span: Span<'cx>,
}

impl<'cx> FromSyn<'cx, syn::TypeSlice> for TypeSlice<'cx> {
    fn from_syn(scx: &'cx SyntaxCx<'cx>, desc: InputDesc<'cx, '_, syn::TypeSlice>) -> Self {
        Self {
            elem: scx.alloc(Type::from_syn(scx, desc.with_input(&desc.input.elem))),
            span: desc.span(desc.input),
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
    fn from_syn(scx: &'cx SyntaxCx<'cx>, desc: InputDesc<'cx, '_, syn::TypeTuple>) -> Self {
        Self {
            elems: FromSyn::from_syn(scx, desc.with_input(&desc.input.elems)),
            span: desc.span(desc.input),
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
        let ccx = syn_sem_common::CommonCx::default();
        let scx = SyntaxCx::new(&ccx);

        let ty = parse::<syn::TypeReference, TypeReference>(&scx, "&T");
        assert!(!ty.is_mut);

        let ty = parse::<syn::TypeReference, TypeReference>(&scx, "&mut T");
        assert!(ty.is_mut);
    }

    #[test]
    fn type_path() {
        // Proves type paths preserve generic arguments on the owning path segment.
        let ccx = syn_sem_common::CommonCx::default();
        let scx = SyntaxCx::new(&ccx);

        let ty = parse::<syn::TypePath, TypePath>(&scx, "Vec<T>");
        assert!(ty.qself.is_none());
        assert_eq!(&*ty.path.segments[0].ident, "Vec");
        assert!(ty.path.segments[0].has_args());

        let ty = parse::<syn::TypePath, TypePath>(&scx, "std::vec::Vec<T>");
        assert_eq!(ty.path.segments.len(), 3);
        assert!(!ty.path.segments[0].has_args());
        assert!(!ty.path.segments[1].has_args());
        assert!(ty.path.segments[2].has_args());
    }

    #[test]
    fn qualified_type_path() {
        // Proves qualified paths preserve their self type and path split point.
        let ccx = syn_sem_common::CommonCx::default();
        let scx = SyntaxCx::new(&ccx);

        let ty = parse::<syn::TypePath, TypePath>(&scx, "<T as a::b::Trait>::Item");
        let qself = ty
            .qself
            .as_ref()
            .expect("qualified path should preserve qself");

        assert!(matches!(qself.ty, Type::Path(_)));
        assert_eq!(qself.position, 3);
        assert_eq!(ty.path.segments.len(), 4);
        assert_eq!(&*ty.path.segments[0].ident, "a");
        assert_eq!(&*ty.path.segments[1].ident, "b");
        assert_eq!(&*ty.path.segments[2].ident, "Trait");
        assert_eq!(&*ty.path.segments[3].ident, "Item");
    }
}
