use crate::{Expr, FromSyn, Ident, InputDesc, Span, SyntaxCx, Type, TypeParamBound};
use syn_sem_macros::CheckDropless;

/// A Rust path made of one or more segments.
///
/// Examples include `T`, `std::vec::Vec`, and `a::B<T>::C<U>`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct Path<'cx> {
    /// Path segments in order.
    pub segments: &'cx [PathSegment<'cx>],
    /// Source span of the path.
    pub span: Span<'cx>,
}

impl<'cx> Path<'cx> {
    /// Creates a path from `::`-separated text.
    pub fn from_str(scx: &'cx SyntaxCx<'cx>, value: &str, span: Span<'cx>) -> Self {
        Self::from_iter(scx, value.split("::"), span)
    }

    /// Creates a path from segment strings.
    pub fn from_iter<'a, I>(scx: &'cx SyntaxCx<'cx>, mut iter: I, span: Span<'cx>) -> Self
    where
        I: Iterator<Item = &'a str> + Clone,
    {
        let len = iter.clone().count();
        let segments = scx.alloc_slice(len, |_| {
            PathSegment::from_str(scx, iter.next().unwrap(), Span::new_empty(scx))
        });
        Self { segments, span }
    }

    /// Returns the identifier if this path is a single plain segment.
    pub fn get_ident(&self) -> Option<&Ident<'cx>> {
        if self.segments.len() == 1 && !self.segments[0].has_args() {
            Some(&self.segments[0].ident)
        } else {
            None
        }
    }

    /// Returns the last path segment.
    pub fn last(&self) -> Option<&PathSegment<'cx>> {
        self.segments.last()
    }

    /// Returns all segments before the last segment.
    pub fn parent_segments(&self) -> &'cx [PathSegment<'cx>] {
        let len = self.segments.len().saturating_sub(1);
        &self.segments[..len]
    }
}

impl<'cx> FromSyn<'cx, syn::Path> for Path<'cx> {
    fn from_syn(scx: &'cx SyntaxCx<'cx>, desc: InputDesc<'cx, '_, syn::Path>) -> Self {
        Self {
            segments: FromSyn::from_syn(scx, desc.with_input(&desc.input.segments)),
            span: desc.span(desc.input),
        }
    }
}

/// One segment in a path, including any generic arguments on that segment.
///
/// For example, `Vec<T>` is a segment with ident `Vec` and angle-bracketed arguments.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct PathSegment<'cx> {
    /// Segment identifier.
    pub ident: Ident<'cx>,
    /// Generic arguments on this segment.
    pub args: PathArgs<'cx>,
    /// Source span of the segment.
    pub span: Span<'cx>,
}

impl<'cx> PathSegment<'cx> {
    /// Creates a plain path segment.
    pub fn from_str(scx: &'cx SyntaxCx<'cx>, value: &str, span: Span<'cx>) -> Self {
        Self {
            ident: Ident::from_str(scx, value, span),
            args: PathArgs::None,
            span,
        }
    }

    /// Returns whether this segment has any generic arguments.
    pub fn has_args(&self) -> bool {
        !self.args.is_empty()
    }

    /// Returns whether this segment is a bare identifier.
    pub fn is_plain_ident(&self) -> bool {
        !self.has_args()
    }
}

impl<'cx> FromSyn<'cx, syn::PathSegment> for PathSegment<'cx> {
    fn from_syn(scx: &'cx SyntaxCx<'cx>, desc: InputDesc<'cx, '_, syn::PathSegment>) -> Self {
        Self {
            ident: Ident::from_syn(scx, desc.with_input(&desc.input.ident)),
            args: PathArgs::from_syn(scx, desc.with_input(&desc.input.arguments)),
            span: desc.span(desc.input),
        }
    }
}

/// Generic arguments attached to a path segment.
///
/// Examples include no arguments in `T` and angle-bracketed arguments in `Vec<T>`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub enum PathArgs<'cx> {
    /// No generic arguments.
    None,
    /// Angle-bracketed generic arguments.
    AngleBracketed(AngleBracketedGenericArgs<'cx>),
    /// Unsupported argument form.
    Unsupported(Span<'cx>),
}

impl PathArgs<'_> {
    /// Returns supported generic arguments.
    pub fn args(&self) -> &[GenericArg<'_>] {
        match self {
            Self::None => &[],
            Self::AngleBracketed(v) => v.args,
            Self::Unsupported(_) => &[],
        }
    }

    /// Returns whether there are no supported generic arguments.
    pub fn is_empty(&self) -> bool {
        self.args().is_empty()
    }

    /// Returns the number of supported generic arguments.
    pub fn len(&self) -> usize {
        self.args().len()
    }
}

impl<'cx> FromSyn<'cx, syn::PathArguments> for PathArgs<'cx> {
    fn from_syn(scx: &'cx SyntaxCx<'cx>, desc: InputDesc<'cx, '_, syn::PathArguments>) -> Self {
        match desc.input {
            syn::PathArguments::None => Self::None,
            syn::PathArguments::AngleBracketed(v) => {
                Self::AngleBracketed(AngleBracketedGenericArgs::from_syn(scx, desc.with_input(v)))
            }
            syn::PathArguments::Parenthesized(v) => Self::Unsupported(desc.span(v)),
        }
    }
}

/// Angle-bracketed generic arguments on a path segment.
///
/// For example, `<K, V>` in `HashMap<K, V>`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct AngleBracketedGenericArgs<'cx> {
    /// Generic arguments.
    pub args: &'cx [GenericArg<'cx>],
    /// Source span of the argument list.
    pub span: Span<'cx>,
}

impl<'cx> FromSyn<'cx, syn::AngleBracketedGenericArguments> for AngleBracketedGenericArgs<'cx> {
    fn from_syn(
        scx: &'cx SyntaxCx<'cx>,
        desc: InputDesc<'cx, '_, syn::AngleBracketedGenericArguments>,
    ) -> Self {
        Self {
            args: FromSyn::from_syn(scx, desc.with_input(&desc.input.args)),
            span: desc.span(desc.input),
        }
    }
}

/// One generic argument inside a path segment.
///
/// Examples include `T`, `N`, `Item = T`, `PANIC = false`, and `Item: Display`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub enum GenericArg<'cx> {
    /// Type argument.
    Type(Type<'cx>),
    /// Const expression argument.
    Const(Expr<'cx>),
    /// Associated type equality.
    AssocType(AssocTypeArg<'cx>),
    /// Associated const equality.
    AssocConst(AssocConstArg<'cx>),
    /// Associated type constraint.
    Constraint(ConstraintArg<'cx>),
    /// Unsupported argument form.
    Unsupported(Span<'cx>),
}

impl<'cx> FromSyn<'cx, syn::GenericArgument> for GenericArg<'cx> {
    fn from_syn(scx: &'cx SyntaxCx<'cx>, desc: InputDesc<'cx, '_, syn::GenericArgument>) -> Self {
        match desc.input {
            syn::GenericArgument::Type(v) => Self::Type(Type::from_syn(scx, desc.with_input(v))),
            syn::GenericArgument::Const(v) => Self::Const(Expr::from_syn(scx, desc.with_input(v))),
            syn::GenericArgument::AssocType(v) => {
                if v.generics.is_some() {
                    Self::Unsupported(desc.span(v))
                } else {
                    Self::AssocType(AssocTypeArg::from_syn(scx, desc.with_input(v)))
                }
            }
            syn::GenericArgument::AssocConst(v) => {
                if v.generics.is_some() {
                    Self::Unsupported(desc.span(v))
                } else {
                    Self::AssocConst(AssocConstArg::from_syn(scx, desc.with_input(v)))
                }
            }
            syn::GenericArgument::Constraint(v) => {
                if v.generics.is_some() {
                    Self::Unsupported(desc.span(v))
                } else {
                    Self::Constraint(ConstraintArg::from_syn(scx, desc.with_input(v)))
                }
            }
            syn::GenericArgument::Lifetime(v) => Self::Unsupported(desc.span(v)),
            _ => Self::Unsupported(desc.span(desc.input)),
        }
    }
}

/// An associated type equality argument in a path.
///
/// For example, `Item = T` in `Iterator<Item = T>`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct AssocTypeArg<'cx> {
    /// Associated type name.
    pub ident: Ident<'cx>,
    /// Assigned type.
    pub ty: Type<'cx>,
    /// Source span of the argument.
    pub span: Span<'cx>,
}

impl<'cx> FromSyn<'cx, syn::AssocType> for AssocTypeArg<'cx> {
    fn from_syn(scx: &'cx SyntaxCx<'cx>, desc: InputDesc<'cx, '_, syn::AssocType>) -> Self {
        Self {
            ident: Ident::from_syn(scx, desc.with_input(&desc.input.ident)),
            ty: Type::from_syn(scx, desc.with_input(&desc.input.ty)),
            span: desc.span(desc.input),
        }
    }
}

/// An associated const equality argument in a path.
///
/// For example, `PANIC = false` in `Trait<PANIC = false>`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct AssocConstArg<'cx> {
    /// Associated const name.
    pub ident: Ident<'cx>,
    /// Assigned value.
    pub value: Expr<'cx>,
    /// Source span of the argument.
    pub span: Span<'cx>,
}

impl<'cx> FromSyn<'cx, syn::AssocConst> for AssocConstArg<'cx> {
    fn from_syn(scx: &'cx SyntaxCx<'cx>, desc: InputDesc<'cx, '_, syn::AssocConst>) -> Self {
        Self {
            ident: Ident::from_syn(scx, desc.with_input(&desc.input.ident)),
            value: Expr::from_syn(scx, desc.with_input(&desc.input.value)),
            span: desc.span(desc.input),
        }
    }
}

/// An associated type constraint argument in a path.
///
/// For example, `Item: Display` in `Iterator<Item: Display>`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct ConstraintArg<'cx> {
    /// Associated type name.
    pub ident: Ident<'cx>,
    /// Required bounds.
    pub bounds: &'cx [TypeParamBound<'cx>],
    /// Source span of the constraint.
    pub span: Span<'cx>,
}

impl<'cx> FromSyn<'cx, syn::Constraint> for ConstraintArg<'cx> {
    fn from_syn(scx: &'cx SyntaxCx<'cx>, desc: InputDesc<'cx, '_, syn::Constraint>) -> Self {
        Self {
            ident: Ident::from_syn(scx, desc.with_input(&desc.input.ident)),
            bounds: FromSyn::from_syn(scx, desc.with_input(&desc.input.bounds)),
            span: desc.span(desc.input),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::*;

    #[test]
    fn path() {
        // Proves paths preserve segments and expose helper accessors.
        let ccx = syn_sem_common::CommonCx::default();
        let scx = SyntaxCx::new(&ccx);

        let path = parse::<syn::Path, Path>(&scx, "A");
        assert_eq!(&**path.get_ident().unwrap(), "A");
        assert!(!path.segments[0].has_args());
        assert!(path.segments[0].is_plain_ident());
        assert_eq!(&*path.last().unwrap().ident, "A");
        assert!(path.parent_segments().is_empty());

        let path = parse::<syn::Path, Path>(&scx, "a::B<T>::C<U>");
        assert_eq!(path.segments.len(), 3);
        assert!(!path.segments[0].has_args());
        assert!(path.segments[1].has_args());
        assert!(path.segments[2].has_args());
        assert_eq!(path.parent_segments().len(), 2);
        assert_eq!(&*path.last().unwrap().ident, "C");
    }

    #[test]
    fn path_arguments() {
        // Proves path arguments preserve single and multiple type arguments.
        let ccx = syn_sem_common::CommonCx::default();
        let scx = SyntaxCx::new(&ccx);

        let path = parse::<syn::Path, Path>(&scx, "A<T>");
        assert!(path.get_ident().is_none());
        assert_eq!(&*path.segments[0].ident, "A");

        let PathArgs::AngleBracketed(args) = &path.segments[0].args else {
            panic!()
        };
        assert_eq!(args.args.len(), 1);
        assert_eq!(path.segments[0].args.len(), 1);
        assert_eq!(path.segments[0].args.args().len(), 1);
        assert!(matches!(args.args[0], GenericArg::Type(_)));

        let path = parse::<syn::Path, Path>(&scx, "HashMap<K, V>");
        let PathArgs::AngleBracketed(args) = &path.segments[0].args else {
            panic!()
        };
        assert_eq!(args.args.len(), 2);
        assert!(matches!(args.args[0], GenericArg::Type(_)));
        assert!(matches!(args.args[1], GenericArg::Type(_)));
    }

    #[test]
    fn generic_argument() {
        // Proves generic arguments preserve const, associated type, associated const, and constraints.
        let ccx = syn_sem_common::CommonCx::default();
        let scx = SyntaxCx::new(&ccx);

        let path = parse::<syn::Path, Path>(&scx, "Array<3>");
        let PathArgs::AngleBracketed(args) = &path.segments[0].args else {
            panic!()
        };
        assert_eq!(args.args.len(), 1);
        assert!(matches!(args.args[0], GenericArg::Const(_)));

        let path = parse::<syn::Path, Path>(&scx, "Iterator<Item = T>");
        let PathArgs::AngleBracketed(args) = &path.segments[0].args else {
            panic!()
        };
        let GenericArg::AssocType(arg) = &args.args[0] else {
            panic!()
        };
        assert_eq!(&*arg.ident, "Item");

        let path = parse::<syn::Path, Path>(&scx, "Trait<PANIC = false>");
        let PathArgs::AngleBracketed(args) = &path.segments[0].args else {
            panic!()
        };
        let GenericArg::AssocConst(arg) = &args.args[0] else {
            panic!()
        };
        assert_eq!(&*arg.ident, "PANIC");

        let path = parse::<syn::Path, Path>(&scx, "Iterator<Item: Display>");
        let PathArgs::AngleBracketed(args) = &path.segments[0].args else {
            panic!()
        };
        let GenericArg::Constraint(arg) = &args.args[0] else {
            panic!()
        };
        assert_eq!(&*arg.ident, "Item");
        assert_eq!(arg.bounds.len(), 1);
    }

    #[test]
    fn unsupported() {
        // Proves unsupported path argument forms recover as `Unsupported` instead of panicking.
        let ccx = syn_sem_common::CommonCx::default();
        let scx = SyntaxCx::new(&ccx);

        let bound = parse::<syn::TypeParamBound, crate::TypeParamBound>(&scx, "Fn(A)");
        let crate::TypeParamBound::Trait(bound) = bound.value else {
            panic!()
        };
        assert!(matches!(
            bound.path.segments[0].args,
            PathArgs::Unsupported(_)
        ));
        assert!(bound.path.segments[0].args.is_empty());

        let path = parse::<syn::Path, Path>(&scx, "Borrowed<'a>");
        let PathArgs::AngleBracketed(args) = &path.segments[0].args else {
            panic!()
        };
        assert!(matches!(args.args[0], GenericArg::Unsupported(_)));

        let path = parse::<syn::Path, Path>(&scx, "Trait<Assoc<T> = U>");
        let PathArgs::AngleBracketed(args) = &path.segments[0].args else {
            panic!()
        };
        assert!(matches!(args.args[0], GenericArg::Unsupported(_)));

        let path = parse::<syn::Path, Path>(&scx, "Trait<Assoc<T>: Display>");
        let PathArgs::AngleBracketed(args) = &path.segments[0].args else {
            panic!()
        };
        assert!(matches!(args.args[0], GenericArg::Unsupported(_)));
    }
}
