use crate::{Expr, FromSyn, Ident, InputDesc, Span, SyntaxCx, Type, TypeParamBound};
use syn_sem_macros::CheckDropless;

/// A Rust path made of one or more segments.
///
/// Examples include `T`, `std::vec::Vec`, and `a::B<T>::C<U>`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct Path<'scx> {
    pub segments: &'scx [PathSegment<'scx>],
    pub span: Span<'scx>,
}

impl<'scx> Path<'scx> {
    pub fn from_str(scx: &'scx SyntaxCx, value: &str, span: Span<'scx>) -> Self {
        Self::from_iter(scx, value.split("::"), span)
    }

    pub fn from_iter<'a, I>(scx: &'scx SyntaxCx, mut iter: I, span: Span<'scx>) -> Self
    where
        I: Iterator<Item = &'a str> + Clone,
    {
        let len = iter.clone().count();
        let segments = scx.alloc_slice(len, |_| {
            PathSegment::from_str(scx, iter.next().unwrap(), Span::empty())
        });
        Self { segments, span }
    }

    pub fn get_ident(&self) -> Option<&Ident<'scx>> {
        if self.segments.len() == 1 && !self.segments[0].has_args() {
            Some(&self.segments[0].ident)
        } else {
            None
        }
    }

    pub fn last(&self) -> Option<&PathSegment<'scx>> {
        self.segments.last()
    }

    pub fn parent_segments(&self) -> &'scx [PathSegment<'scx>] {
        let len = self.segments.len().saturating_sub(1);
        &self.segments[..len]
    }
}

impl<'scx> FromSyn<'scx, syn::Path> for Path<'scx> {
    fn from_syn(scx: &'scx SyntaxCx, desc: InputDesc<'_, syn::Path>) -> Self {
        Self {
            segments: FromSyn::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.segments,
                },
            ),
            span: Span::from_locatable(scx, desc.file_path, desc.input),
        }
    }
}

/// One segment in a path, including any generic arguments on that segment.
///
/// For example, `Vec<T>` is a segment with ident `Vec` and angle-bracketed arguments.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct PathSegment<'scx> {
    pub ident: Ident<'scx>,
    pub args: PathArguments<'scx>,
    pub span: Span<'scx>,
}

impl<'scx> PathSegment<'scx> {
    pub fn from_str(scx: &'scx SyntaxCx, value: &str, span: Span<'scx>) -> Self {
        Self {
            ident: Ident::from_str(scx, value, span),
            args: PathArguments::None,
            span,
        }
    }

    pub fn has_args(&self) -> bool {
        !self.args.is_empty()
    }

    pub fn is_plain_ident(&self) -> bool {
        !self.has_args()
    }
}

impl<'scx> FromSyn<'scx, syn::PathSegment> for PathSegment<'scx> {
    fn from_syn(scx: &'scx SyntaxCx, desc: InputDesc<'_, syn::PathSegment>) -> Self {
        Self {
            ident: Ident::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.ident,
                },
            ),
            args: PathArguments::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.arguments,
                },
            ),
            span: Span::from_locatable(scx, desc.file_path, desc.input),
        }
    }
}

/// Generic arguments attached to a path segment.
///
/// Examples include no arguments in `T` and angle-bracketed arguments in `Vec<T>`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub enum PathArguments<'scx> {
    None,
    AngleBracketed(AngleBracketedGenericArguments<'scx>),
    Unsupported(Span<'scx>),
}

impl PathArguments<'_> {
    pub fn args(&self) -> &[GenericArgument<'_>] {
        match self {
            Self::None => &[],
            Self::AngleBracketed(v) => v.args,
            Self::Unsupported(_) => &[],
        }
    }

    pub fn is_empty(&self) -> bool {
        self.args().is_empty()
    }

    pub fn len(&self) -> usize {
        self.args().len()
    }
}

impl<'scx> FromSyn<'scx, syn::PathArguments> for PathArguments<'scx> {
    fn from_syn(scx: &'scx SyntaxCx, desc: InputDesc<'_, syn::PathArguments>) -> Self {
        match desc.input {
            syn::PathArguments::None => Self::None,
            syn::PathArguments::AngleBracketed(v) => {
                Self::AngleBracketed(AngleBracketedGenericArguments::from_syn(
                    scx,
                    InputDesc {
                        file_path: desc.file_path,
                        input: v,
                    },
                ))
            }
            syn::PathArguments::Parenthesized(v) => {
                Self::Unsupported(Span::from_locatable(scx, desc.file_path, v))
            }
        }
    }
}

/// Angle-bracketed generic arguments on a path segment.
///
/// For example, `<K, V>` in `HashMap<K, V>`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct AngleBracketedGenericArguments<'scx> {
    pub args: &'scx [GenericArgument<'scx>],
    pub span: Span<'scx>,
}

impl<'scx> FromSyn<'scx, syn::AngleBracketedGenericArguments>
    for AngleBracketedGenericArguments<'scx>
{
    fn from_syn(
        scx: &'scx SyntaxCx,
        desc: InputDesc<'_, syn::AngleBracketedGenericArguments>,
    ) -> Self {
        Self {
            args: FromSyn::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.args,
                },
            ),
            span: Span::from_locatable(scx, desc.file_path, desc.input),
        }
    }
}

/// One generic argument inside a path segment.
///
/// Examples include `T`, `N`, `Item = T`, `PANIC = false`, and `Item: Display`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub enum GenericArgument<'scx> {
    Type(Type<'scx>),
    Const(Expr<'scx>),
    AssocType(AssocTypeArg<'scx>),
    AssocConst(AssocConstArg<'scx>),
    Constraint(ConstraintArg<'scx>),
    Unsupported(Span<'scx>),
}

impl<'scx> FromSyn<'scx, syn::GenericArgument> for GenericArgument<'scx> {
    fn from_syn(scx: &'scx SyntaxCx, desc: InputDesc<'_, syn::GenericArgument>) -> Self {
        match desc.input {
            syn::GenericArgument::Type(v) => Self::Type(Type::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: v,
                },
            )),
            syn::GenericArgument::Const(v) => Self::Const(Expr::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: v,
                },
            )),
            syn::GenericArgument::AssocType(v) => {
                if v.generics.is_some() {
                    Self::Unsupported(Span::from_locatable(scx, desc.file_path, v))
                } else {
                    Self::AssocType(AssocTypeArg::from_syn(
                        scx,
                        InputDesc {
                            file_path: desc.file_path,
                            input: v,
                        },
                    ))
                }
            }
            syn::GenericArgument::AssocConst(v) => {
                if v.generics.is_some() {
                    Self::Unsupported(Span::from_locatable(scx, desc.file_path, v))
                } else {
                    Self::AssocConst(AssocConstArg::from_syn(
                        scx,
                        InputDesc {
                            file_path: desc.file_path,
                            input: v,
                        },
                    ))
                }
            }
            syn::GenericArgument::Constraint(v) => {
                if v.generics.is_some() {
                    Self::Unsupported(Span::from_locatable(scx, desc.file_path, v))
                } else {
                    Self::Constraint(ConstraintArg::from_syn(
                        scx,
                        InputDesc {
                            file_path: desc.file_path,
                            input: v,
                        },
                    ))
                }
            }
            syn::GenericArgument::Lifetime(v) => {
                Self::Unsupported(Span::from_locatable(scx, desc.file_path, v))
            }
            _ => Self::Unsupported(Span::from_locatable(scx, desc.file_path, desc.input)),
        }
    }
}

/// An associated type equality argument in a path.
///
/// For example, `Item = T` in `Iterator<Item = T>`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct AssocTypeArg<'scx> {
    pub ident: Ident<'scx>,
    pub ty: Type<'scx>,
    pub span: Span<'scx>,
}

impl<'scx> FromSyn<'scx, syn::AssocType> for AssocTypeArg<'scx> {
    fn from_syn(scx: &'scx SyntaxCx, desc: InputDesc<'_, syn::AssocType>) -> Self {
        Self {
            ident: Ident::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.ident,
                },
            ),
            ty: Type::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.ty,
                },
            ),
            span: Span::from_locatable(scx, desc.file_path, desc.input),
        }
    }
}

/// An associated const equality argument in a path.
///
/// For example, `PANIC = false` in `Trait<PANIC = false>`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct AssocConstArg<'scx> {
    pub ident: Ident<'scx>,
    pub value: Expr<'scx>,
    pub span: Span<'scx>,
}

impl<'scx> FromSyn<'scx, syn::AssocConst> for AssocConstArg<'scx> {
    fn from_syn(scx: &'scx SyntaxCx, desc: InputDesc<'_, syn::AssocConst>) -> Self {
        Self {
            ident: Ident::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.ident,
                },
            ),
            value: Expr::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.value,
                },
            ),
            span: Span::from_locatable(scx, desc.file_path, desc.input),
        }
    }
}

/// An associated type constraint argument in a path.
///
/// For example, `Item: Display` in `Iterator<Item: Display>`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct ConstraintArg<'scx> {
    pub ident: Ident<'scx>,
    pub bounds: &'scx [TypeParamBound<'scx>],
    pub span: Span<'scx>,
}

impl<'scx> FromSyn<'scx, syn::Constraint> for ConstraintArg<'scx> {
    fn from_syn(scx: &'scx SyntaxCx, desc: InputDesc<'_, syn::Constraint>) -> Self {
        Self {
            ident: Ident::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.ident,
                },
            ),
            bounds: FromSyn::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.bounds,
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
    fn test_path() {
        // Proves paths preserve segments and expose helper accessors.
        let scx = SyntaxCx::default();

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
    fn test_path_arguments() {
        // Proves path arguments preserve single and multiple type arguments.
        let scx = SyntaxCx::default();

        let path = parse::<syn::Path, Path>(&scx, "A<T>");
        assert!(path.get_ident().is_none());
        assert_eq!(&*path.segments[0].ident, "A");

        let PathArguments::AngleBracketed(args) = &path.segments[0].args else {
            panic!()
        };
        assert_eq!(args.args.len(), 1);
        assert_eq!(path.segments[0].args.len(), 1);
        assert_eq!(path.segments[0].args.args().len(), 1);
        assert!(matches!(args.args[0], GenericArgument::Type(_)));

        let path = parse::<syn::Path, Path>(&scx, "HashMap<K, V>");
        let PathArguments::AngleBracketed(args) = &path.segments[0].args else {
            panic!()
        };
        assert_eq!(args.args.len(), 2);
        assert!(matches!(args.args[0], GenericArgument::Type(_)));
        assert!(matches!(args.args[1], GenericArgument::Type(_)));
    }

    #[test]
    fn test_generic_argument() {
        // Proves generic arguments preserve const, associated type, associated const, and constraints.
        let scx = SyntaxCx::default();

        let path = parse::<syn::Path, Path>(&scx, "Array<3>");
        let PathArguments::AngleBracketed(args) = &path.segments[0].args else {
            panic!()
        };
        assert_eq!(args.args.len(), 1);
        assert!(matches!(args.args[0], GenericArgument::Const(_)));

        let path = parse::<syn::Path, Path>(&scx, "Iterator<Item = T>");
        let PathArguments::AngleBracketed(args) = &path.segments[0].args else {
            panic!()
        };
        let GenericArgument::AssocType(arg) = &args.args[0] else {
            panic!()
        };
        assert_eq!(&*arg.ident, "Item");

        let path = parse::<syn::Path, Path>(&scx, "Trait<PANIC = false>");
        let PathArguments::AngleBracketed(args) = &path.segments[0].args else {
            panic!()
        };
        let GenericArgument::AssocConst(arg) = &args.args[0] else {
            panic!()
        };
        assert_eq!(&*arg.ident, "PANIC");

        let path = parse::<syn::Path, Path>(&scx, "Iterator<Item: Display>");
        let PathArguments::AngleBracketed(args) = &path.segments[0].args else {
            panic!()
        };
        let GenericArgument::Constraint(arg) = &args.args[0] else {
            panic!()
        };
        assert_eq!(&*arg.ident, "Item");
        assert_eq!(arg.bounds.len(), 1);
    }

    #[test]
    fn test_unsupported() {
        // Proves unsupported path argument forms recover as `Unsupported` instead of panicking.
        let scx = SyntaxCx::default();

        let bound = parse::<syn::TypeParamBound, crate::TypeParamBound>(&scx, "Fn(A)");
        let crate::TypeParamBound::Trait(bound) = bound else {
            panic!()
        };
        assert!(matches!(
            bound.path.segments[0].args,
            PathArguments::Unsupported(_)
        ));
        assert!(bound.path.segments[0].args.is_empty());

        let path = parse::<syn::Path, Path>(&scx, "Borrowed<'a>");
        let PathArguments::AngleBracketed(args) = &path.segments[0].args else {
            panic!()
        };
        assert!(matches!(args.args[0], GenericArgument::Unsupported(_)));

        let path = parse::<syn::Path, Path>(&scx, "Trait<Assoc<T> = U>");
        let PathArguments::AngleBracketed(args) = &path.segments[0].args else {
            panic!()
        };
        assert!(matches!(args.args[0], GenericArgument::Unsupported(_)));

        let path = parse::<syn::Path, Path>(&scx, "Trait<Assoc<T>: Display>");
        let PathArguments::AngleBracketed(args) = &path.segments[0].args else {
            panic!()
        };
        assert!(matches!(args.args[0], GenericArgument::Unsupported(_)));
    }
}
