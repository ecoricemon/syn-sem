use crate::{
    ExprLit, ExprPath, FromSyn, Ident, InputDesc, Path, Span, SyntaxCx, Type, TypePath,
    TypeReference,
};
use num_traits::ToPrimitive;
use syn_sem_macros::CheckDropless;

/// A Rust pattern supported by the semantic AST.
///
/// Examples include `x`, `ref mut x`, `..`, `S { x }`, `(a, b)`, and `x: T`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub enum Pat<'scx> {
    Ident(PatIdent<'scx>),
    Lit(PatLit<'scx>),
    Path(PatPath<'scx>),
    Reference(PatReference<'scx>),
    Rest(PatRest<'scx>),
    Slice(PatSlice<'scx>),
    Struct(PatStruct<'scx>),
    Tuple(PatTuple<'scx>),
    Type(PatType<'scx>),
}

impl<'scx> FromSyn<'scx, syn::Pat> for Pat<'scx> {
    fn from_syn(scx: &'scx SyntaxCx, desc: InputDesc<'_, syn::Pat>) -> Self {
        match desc.input {
            syn::Pat::Ident(v) => Self::Ident(PatIdent::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: v,
                },
            )),
            syn::Pat::Lit(v) => Self::Lit(PatLit::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: v,
                },
            )),
            syn::Pat::Path(v) => Self::Path(PatPath::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: v,
                },
            )),
            syn::Pat::Reference(v) => Self::Reference(PatReference::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: v,
                },
            )),
            syn::Pat::Rest(v) => Self::Rest(PatRest::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: v,
                },
            )),
            syn::Pat::Slice(v) => Self::Slice(PatSlice::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: v,
                },
            )),
            syn::Pat::Struct(v) => Self::Struct(PatStruct::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: v,
                },
            )),
            syn::Pat::Tuple(v) => Self::Tuple(PatTuple::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: v,
                },
            )),
            syn::Pat::Type(v) => Self::Type(PatType::from_syn(
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

pub type PatLit<'scx> = ExprLit<'scx>;
pub type PatPath<'scx> = ExprPath<'scx>;

/// An identifier binding pattern.
///
/// Examples include `x`, `mut x`, `ref x`, and `ref mut x`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct PatIdent<'scx> {
    pub ident: Ident<'scx>,
    pub is_ref: bool,
    pub is_mut: bool,
    pub span: Span<'scx>,
}

impl<'scx> PatIdent<'scx> {
    pub fn from_number<T: ToPrimitive>(scx: &'scx SyntaxCx, value: T, span: Span<'scx>) -> Self {
        Self {
            ident: Ident::from_number(scx, value, span),
            is_ref: false,
            is_mut: false,
            span,
        }
    }
}

impl<'scx> FromSyn<'scx, syn::PatIdent> for PatIdent<'scx> {
    fn from_syn(scx: &'scx SyntaxCx, desc: InputDesc<'_, syn::PatIdent>) -> Self {
        Self {
            ident: Ident::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.ident,
                },
            ),
            is_ref: desc.input.by_ref.is_some(),
            is_mut: desc.input.mutability.is_some(),
            span: Span::from_locatable(scx, desc.file_path, desc.input),
        }
    }
}

impl<'scx> FromSyn<'scx, syn::Token![self]> for PatIdent<'scx> {
    fn from_syn(scx: &'scx SyntaxCx, desc: InputDesc<'_, syn::Token![self]>) -> Self {
        Self {
            ident: Ident::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: desc.input,
                },
            ),
            is_ref: false,
            is_mut: false,
            span: Span::from_locatable(scx, desc.file_path, desc.input),
        }
    }
}

/// A reference pattern.
///
/// Examples include `&x` and `&mut x`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct PatReference<'scx> {
    pub pat: &'scx Pat<'scx>,
    pub is_mut: bool,
    pub span: Span<'scx>,
}

impl<'scx> FromSyn<'scx, syn::PatReference> for PatReference<'scx> {
    fn from_syn(scx: &'scx SyntaxCx, desc: InputDesc<'_, syn::PatReference>) -> Self {
        Self {
            pat: scx.alloc(Pat::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.pat,
                },
            )),
            is_mut: desc.input.mutability.is_some(),
            span: Span::from_locatable(scx, desc.file_path, desc.input),
        }
    }
}

/// A rest pattern.
///
/// For example, `..` in `[head, ..]` or `S { x, .. }`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct PatRest<'scx> {
    pub span: Span<'scx>,
}

impl<'scx> FromSyn<'scx, syn::PatRest> for PatRest<'scx> {
    fn from_syn(scx: &'scx SyntaxCx, desc: InputDesc<'_, syn::PatRest>) -> Self {
        Self {
            span: Span::from_locatable(scx, desc.file_path, desc.input),
        }
    }
}

/// A slice pattern.
///
/// For example, `[first, rest @ ..]`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct PatSlice<'scx> {
    pub elems: &'scx [Pat<'scx>],
    pub span: Span<'scx>,
}

impl<'scx> FromSyn<'scx, syn::PatSlice> for PatSlice<'scx> {
    fn from_syn(scx: &'scx SyntaxCx, desc: InputDesc<'_, syn::PatSlice>) -> Self {
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

/// A struct pattern.
///
/// For example, `Point { x, y }`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct PatStruct<'scx> {
    pub path: Path<'scx>,
    pub fields: &'scx [FieldPat<'scx>],
    pub rest: Option<PatRest<'scx>>,
    pub span: Span<'scx>,
}

impl<'scx> FromSyn<'scx, syn::PatStruct> for PatStruct<'scx> {
    fn from_syn(scx: &'scx SyntaxCx, desc: InputDesc<'_, syn::PatStruct>) -> Self {
        Self {
            path: Path::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.path,
                },
            ),
            fields: FromSyn::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.fields,
                },
            ),
            rest: desc.input.rest.as_ref().map(|rest| {
                PatRest::from_syn(
                    scx,
                    InputDesc {
                        file_path: desc.file_path,
                        input: rest,
                    },
                )
            }),
            span: Span::from_locatable(scx, desc.file_path, desc.input),
        }
    }
}

/// One field pattern inside a struct pattern.
///
/// For example, `x: value` in `Point { x: value }`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct FieldPat<'scx> {
    pub member: Ident<'scx>,
    pub pat: &'scx Pat<'scx>,
    pub span: Span<'scx>,
}

impl<'scx> FromSyn<'scx, syn::FieldPat> for FieldPat<'scx> {
    fn from_syn(scx: &'scx SyntaxCx, desc: InputDesc<'_, syn::FieldPat>) -> Self {
        let member = match &desc.input.member {
            syn::Member::Named(ident) => Ident::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: ident,
                },
            ),
            syn::Member::Unnamed(idx) => Ident::from_number(
                scx,
                idx.index,
                Span::from_locatable(scx, desc.file_path, idx),
            ),
        };

        Self {
            member,
            pat: scx.alloc(Pat::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.pat,
                },
            )),
            span: Span::from_locatable(scx, desc.file_path, desc.input),
        }
    }
}

/// A tuple pattern.
///
/// Examples include `()`, `(x,)`, and `(x, y)`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct PatTuple<'scx> {
    pub elems: &'scx [Pat<'scx>],
    pub span: Span<'scx>,
}

impl<'scx> FromSyn<'scx, syn::PatTuple> for PatTuple<'scx> {
    fn from_syn(scx: &'scx SyntaxCx, desc: InputDesc<'_, syn::PatTuple>) -> Self {
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

/// A pattern with an explicit type annotation.
///
/// For example, `x: i32`; receivers like `&self` are normalized into this shape.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct PatType<'scx> {
    pub pat: &'scx Pat<'scx>,
    pub ty: Type<'scx>,
    pub span: Span<'scx>,
}

impl<'scx> FromSyn<'scx, syn::PatType> for PatType<'scx> {
    fn from_syn(scx: &'scx SyntaxCx, desc: InputDesc<'_, syn::PatType>) -> Self {
        Self {
            pat: scx.alloc(Pat::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.pat,
                },
            )),
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

impl<'scx> FromSyn<'scx, syn::Receiver> for PatType<'scx> {
    fn from_syn(scx: &'scx SyntaxCx, desc: InputDesc<'_, syn::Receiver>) -> Self {
        let span = Span::from_locatable(scx, desc.file_path, desc.input);
        let self_ty = Type::Path(TypePath {
            path: Path::from_str(scx, "Self", span),
            span,
        });
        let ty = if desc.input.reference.is_some() {
            Type::Reference(TypeReference {
                elem: scx.alloc(self_ty),
                is_mut: desc.input.mutability.is_some(),
                span,
            })
        } else {
            self_ty
        };

        Self {
            pat: scx.alloc(Pat::Ident(PatIdent::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.self_token,
                },
            ))),
            ty,
            span,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::*;

    #[test]
    fn test_pat_rest() {
        // Proves rest patterns are preserved inside slice patterns.
        let scx = SyntaxCx::default();

        let stmt = parse::<syn::Stmt, crate::Stmt>(&scx, "let [head, .., tail] = value;");
        let crate::Stmt::Local(local) = stmt else {
            panic!()
        };
        let Pat::Slice(pat) = local.pat else { panic!() };
        assert_eq!(pat.elems.len(), 3);

        let Pat::Rest(rest) = &pat.elems[1] else {
            panic!()
        };
        assert_eq!(rest.span.source_text(), "..");
    }

    #[test]
    fn test_pat_ident() {
        // Proves identifier patterns preserve `ref` and `mut` modifiers.
        let scx = SyntaxCx::default();

        let stmt = parse::<syn::Stmt, crate::Stmt>(&scx, "let ref mut value = input;");
        let crate::Stmt::Local(local) = stmt else {
            panic!()
        };
        let Pat::Ident(pat) = local.pat else { panic!() };
        assert_eq!(&*pat.ident, "value");
        assert!(pat.is_ref);
        assert!(pat.is_mut);
    }

    #[test]
    fn test_pat_reference() {
        // Proves reference patterns preserve whether the pattern reference is mutable.
        let scx = SyntaxCx::default();

        let stmt = parse::<syn::Stmt, crate::Stmt>(&scx, "let &value = input;");
        let crate::Stmt::Local(local) = stmt else {
            panic!()
        };
        let Pat::Reference(pat) = local.pat else {
            panic!()
        };
        assert!(!pat.is_mut);

        let stmt = parse::<syn::Stmt, crate::Stmt>(&scx, "let &mut value = input;");
        let crate::Stmt::Local(local) = stmt else {
            panic!()
        };
        let Pat::Reference(pat) = local.pat else {
            panic!()
        };
        assert!(pat.is_mut);
    }

    #[test]
    fn test_pat_struct() {
        // Proves struct patterns preserve path, fields, rest, and path arguments.
        let scx = SyntaxCx::default();

        let stmt = parse::<syn::Stmt, crate::Stmt>(&scx, "let S { a, b: c, .. } = value;");
        let crate::Stmt::Local(local) = stmt else {
            panic!()
        };
        let Pat::Struct(pat) = local.pat else {
            panic!()
        };

        assert_eq!(&**pat.path.get_ident().unwrap(), "S");
        assert_eq!(pat.fields.len(), 2);
        assert_eq!(&*pat.fields[0].member, "a");
        assert!(matches!(pat.fields[0].pat, Pat::Ident(_)));
        assert_eq!(&*pat.fields[1].member, "b");
        assert!(matches!(pat.fields[1].pat, Pat::Ident(_)));
        assert!(pat.rest.is_some());

        let stmt = parse::<syn::Stmt, crate::Stmt>(&scx, "let S::<T> { value } = input;");
        let crate::Stmt::Local(local) = stmt else {
            panic!()
        };
        let Pat::Struct(pat) = local.pat else {
            panic!()
        };
        assert_eq!(&*pat.path.segments[0].ident, "S");
        assert!(pat.path.segments[0].has_args());
    }

    #[test]
    fn test_pat_type_receiver() {
        // Proves method receivers synthesize the expected typed `self` pattern.
        let scx = SyntaxCx::default();

        let receiver = parse::<syn::Receiver, PatType>(&scx, "&self");
        let Type::Reference(ty) = receiver.ty else {
            panic!()
        };
        assert!(!ty.is_mut);

        let receiver = parse::<syn::Receiver, PatType>(&scx, "&mut self");
        let Type::Reference(ty) = receiver.ty else {
            panic!()
        };
        assert!(ty.is_mut);
    }
}
