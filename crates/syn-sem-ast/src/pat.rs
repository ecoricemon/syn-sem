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
pub enum Pat<'cx> {
    /// Identifier binding pattern.
    Ident(PatIdent<'cx>),
    /// Literal pattern.
    Lit(PatLit<'cx>),
    /// Path pattern.
    Path(PatPath<'cx>),
    /// Reference pattern.
    Reference(PatReference<'cx>),
    /// Rest pattern.
    Rest(PatRest<'cx>),
    /// Slice pattern.
    Slice(PatSlice<'cx>),
    /// Struct pattern.
    Struct(PatStruct<'cx>),
    /// Tuple pattern.
    Tuple(PatTuple<'cx>),
    /// Type-annotated pattern.
    Type(PatType<'cx>),
}

impl<'cx> FromSyn<'cx, syn::Pat> for Pat<'cx> {
    fn from_syn(scx: &'cx SyntaxCx<'cx>, desc: InputDesc<'cx, '_, syn::Pat>) -> Self {
        match desc.input {
            syn::Pat::Ident(v) => Self::Ident(PatIdent::from_syn(scx, desc.with_input(v))),
            syn::Pat::Lit(v) => Self::Lit(PatLit::from_syn(scx, desc.with_input(v))),
            syn::Pat::Path(v) => Self::Path(PatPath::from_syn(scx, desc.with_input(v))),
            syn::Pat::Reference(v) => {
                Self::Reference(PatReference::from_syn(scx, desc.with_input(v)))
            }
            syn::Pat::Rest(v) => Self::Rest(PatRest::from_syn(scx, desc.with_input(v))),
            syn::Pat::Slice(v) => Self::Slice(PatSlice::from_syn(scx, desc.with_input(v))),
            syn::Pat::Struct(v) => Self::Struct(PatStruct::from_syn(scx, desc.with_input(v))),
            syn::Pat::Tuple(v) => Self::Tuple(PatTuple::from_syn(scx, desc.with_input(v))),
            syn::Pat::Type(v) => Self::Type(PatType::from_syn(scx, desc.with_input(v))),
            _ => todo!(),
        }
    }
}

/// Literal pattern.
pub type PatLit<'cx> = ExprLit<'cx>;
/// Path pattern.
pub type PatPath<'cx> = ExprPath<'cx>;

/// An identifier binding pattern.
///
/// Examples include `x`, `mut x`, `ref x`, and `ref mut x`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct PatIdent<'cx> {
    /// Bound identifier.
    pub ident: Ident<'cx>,
    /// Whether the binding uses `ref`.
    pub is_ref: bool,
    /// Whether the binding uses `mut`.
    pub is_mut: bool,
    /// Source span of the pattern.
    pub span: Span<'cx>,
}

impl<'cx> PatIdent<'cx> {
    /// Creates a synthesized numeric identifier pattern.
    pub fn from_number<T: ToPrimitive>(scx: &'cx SyntaxCx<'cx>, value: T, span: Span<'cx>) -> Self {
        Self {
            ident: Ident::from_number(scx, value, span),
            is_ref: false,
            is_mut: false,
            span,
        }
    }
}

impl<'cx> FromSyn<'cx, syn::PatIdent> for PatIdent<'cx> {
    fn from_syn(scx: &'cx SyntaxCx<'cx>, desc: InputDesc<'cx, '_, syn::PatIdent>) -> Self {
        Self {
            ident: Ident::from_syn(scx, desc.with_input(&desc.input.ident)),
            is_ref: desc.input.by_ref.is_some(),
            is_mut: desc.input.mutability.is_some(),
            span: desc.span(desc.input),
        }
    }
}

impl<'cx> FromSyn<'cx, syn::Token![self]> for PatIdent<'cx> {
    fn from_syn(scx: &'cx SyntaxCx<'cx>, desc: InputDesc<'cx, '_, syn::Token![self]>) -> Self {
        Self {
            ident: Ident::from_syn(scx, desc.with_input(desc.input)),
            is_ref: false,
            is_mut: false,
            span: desc.span(desc.input),
        }
    }
}

/// A reference pattern.
///
/// Examples include `&x` and `&mut x`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct PatReference<'cx> {
    /// Referenced pattern.
    ///
    /// Stored by reference to break the recursive [`Pat`] shape.
    pub pat: &'cx Pat<'cx>,
    /// Whether the reference pattern is mutable.
    pub is_mut: bool,
    /// Source span of the pattern.
    pub span: Span<'cx>,
}

impl<'cx> FromSyn<'cx, syn::PatReference> for PatReference<'cx> {
    fn from_syn(scx: &'cx SyntaxCx<'cx>, desc: InputDesc<'cx, '_, syn::PatReference>) -> Self {
        Self {
            pat: scx.alloc(Pat::from_syn(scx, desc.with_input(&desc.input.pat))),
            is_mut: desc.input.mutability.is_some(),
            span: desc.span(desc.input),
        }
    }
}

/// A rest pattern.
///
/// For example, `..` in `[head, ..]` or `S { x, .. }`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct PatRest<'cx> {
    /// Source span of the rest marker.
    pub span: Span<'cx>,
}

impl<'cx> FromSyn<'cx, syn::PatRest> for PatRest<'cx> {
    fn from_syn(_: &'cx SyntaxCx<'cx>, desc: InputDesc<'cx, '_, syn::PatRest>) -> Self {
        Self {
            span: desc.span(desc.input),
        }
    }
}

/// A slice pattern.
///
/// For example, `[first, rest @ ..]`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct PatSlice<'cx> {
    /// Element patterns.
    pub elems: &'cx [Pat<'cx>],
    /// Source span of the slice pattern.
    pub span: Span<'cx>,
}

impl<'cx> FromSyn<'cx, syn::PatSlice> for PatSlice<'cx> {
    fn from_syn(scx: &'cx SyntaxCx<'cx>, desc: InputDesc<'cx, '_, syn::PatSlice>) -> Self {
        Self {
            elems: FromSyn::from_syn(scx, desc.with_input(&desc.input.elems)),
            span: desc.span(desc.input),
        }
    }
}

/// A struct pattern.
///
/// For example, `Point { x, y }`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct PatStruct<'cx> {
    /// Path naming the matched struct.
    pub path: Path<'cx>,
    /// Field patterns.
    pub fields: &'cx [FieldPat<'cx>],
    /// Optional rest pattern.
    pub rest: Option<PatRest<'cx>>,
    /// Source span of the struct pattern.
    pub span: Span<'cx>,
}

impl<'cx> FromSyn<'cx, syn::PatStruct> for PatStruct<'cx> {
    fn from_syn(scx: &'cx SyntaxCx<'cx>, desc: InputDesc<'cx, '_, syn::PatStruct>) -> Self {
        Self {
            path: Path::from_syn(scx, desc.with_input(&desc.input.path)),
            fields: FromSyn::from_syn(scx, desc.with_input(&desc.input.fields)),
            rest: desc
                .input
                .rest
                .as_ref()
                .map(|rest| PatRest::from_syn(scx, desc.with_input(rest))),
            span: desc.span(desc.input),
        }
    }
}

/// One field pattern inside a struct pattern.
///
/// For example, `x: value` in `Point { x: value }`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct FieldPat<'cx> {
    /// Field member being matched.
    pub member: Ident<'cx>,
    /// Pattern for the field value.
    ///
    /// Stored by reference to break the recursive [`Pat`] shape.
    pub pat: &'cx Pat<'cx>,
    /// Source span of the field pattern.
    pub span: Span<'cx>,
}

impl<'cx> FromSyn<'cx, syn::FieldPat> for FieldPat<'cx> {
    fn from_syn(scx: &'cx SyntaxCx<'cx>, desc: InputDesc<'cx, '_, syn::FieldPat>) -> Self {
        let member = match &desc.input.member {
            syn::Member::Named(ident) => Ident::from_syn(scx, desc.with_input(ident)),
            syn::Member::Unnamed(idx) => Ident::from_number(scx, idx.index, desc.span(idx)),
        };

        Self {
            member,
            pat: scx.alloc(Pat::from_syn(scx, desc.with_input(&desc.input.pat))),
            span: desc.span(desc.input),
        }
    }
}

/// A tuple pattern.
///
/// Examples include `()`, `(x,)`, and `(x, y)`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct PatTuple<'cx> {
    /// Element patterns.
    pub elems: &'cx [Pat<'cx>],
    /// Source span of the tuple pattern.
    pub span: Span<'cx>,
}

impl<'cx> FromSyn<'cx, syn::PatTuple> for PatTuple<'cx> {
    fn from_syn(scx: &'cx SyntaxCx<'cx>, desc: InputDesc<'cx, '_, syn::PatTuple>) -> Self {
        Self {
            elems: FromSyn::from_syn(scx, desc.with_input(&desc.input.elems)),
            span: desc.span(desc.input),
        }
    }
}

/// A pattern with an explicit type annotation.
///
/// For example, `x: i32`; receivers like `&self` are normalized into this shape.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct PatType<'cx> {
    /// Inner pattern.
    ///
    /// Stored by reference to break the recursive [`Pat`] shape.
    pub pat: &'cx Pat<'cx>,
    /// Annotated type.
    pub ty: Type<'cx>,
    /// Source span of the typed pattern.
    pub span: Span<'cx>,
}

impl<'cx> FromSyn<'cx, syn::PatType> for PatType<'cx> {
    fn from_syn(scx: &'cx SyntaxCx<'cx>, desc: InputDesc<'cx, '_, syn::PatType>) -> Self {
        Self {
            pat: scx.alloc(Pat::from_syn(scx, desc.with_input(&desc.input.pat))),
            ty: Type::from_syn(scx, desc.with_input(&desc.input.ty)),
            span: desc.span(desc.input),
        }
    }
}

impl<'cx> FromSyn<'cx, syn::Receiver> for PatType<'cx> {
    fn from_syn(scx: &'cx SyntaxCx<'cx>, desc: InputDesc<'cx, '_, syn::Receiver>) -> Self {
        let span = desc.span(desc.input);
        let self_ty = Type::Path(TypePath {
            qself: None,
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
                desc.with_input(&desc.input.self_token),
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
    fn pat_rest() {
        // Proves rest patterns are preserved inside slice patterns.
        let ccx = syn_sem_common::CommonCx::default();
        let scx = SyntaxCx::new(&ccx);

        let stmt = parse::<syn::Stmt, crate::Stmt>(&scx, "let [head, .., tail] = value;");
        let crate::Stmt::Local(local) = stmt.value else {
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
    fn pat_ident() {
        // Proves identifier patterns preserve `ref` and `mut` modifiers.
        let ccx = syn_sem_common::CommonCx::default();
        let scx = SyntaxCx::new(&ccx);

        let stmt = parse::<syn::Stmt, crate::Stmt>(&scx, "let ref mut value = input;");
        let crate::Stmt::Local(local) = stmt.value else {
            panic!()
        };
        let Pat::Ident(pat) = local.pat else { panic!() };
        assert_eq!(&*pat.ident, "value");
        assert!(pat.is_ref);
        assert!(pat.is_mut);
    }

    #[test]
    fn pat_reference() {
        // Proves reference patterns preserve whether the pattern reference is mutable.
        let ccx = syn_sem_common::CommonCx::default();
        let scx = SyntaxCx::new(&ccx);

        let stmt = parse::<syn::Stmt, crate::Stmt>(&scx, "let &value = input;");
        let crate::Stmt::Local(local) = stmt.value else {
            panic!()
        };
        let Pat::Reference(pat) = local.pat else {
            panic!()
        };
        assert!(!pat.is_mut);

        let stmt = parse::<syn::Stmt, crate::Stmt>(&scx, "let &mut value = input;");
        let crate::Stmt::Local(local) = stmt.value else {
            panic!()
        };
        let Pat::Reference(pat) = local.pat else {
            panic!()
        };
        assert!(pat.is_mut);
    }

    #[test]
    fn pat_struct() {
        // Proves struct patterns preserve path, fields, rest, and path arguments.
        let ccx = syn_sem_common::CommonCx::default();
        let scx = SyntaxCx::new(&ccx);

        let stmt = parse::<syn::Stmt, crate::Stmt>(&scx, "let S { a, b: c, .. } = value;");
        let crate::Stmt::Local(local) = stmt.value else {
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
        let crate::Stmt::Local(local) = stmt.value else {
            panic!()
        };
        let Pat::Struct(pat) = local.pat else {
            panic!()
        };
        assert_eq!(&*pat.path.segments[0].ident, "S");
        assert!(pat.path.segments[0].has_args());
    }

    #[test]
    fn pat_type_receiver() {
        // Proves method receivers synthesize the expected typed `self` pattern.
        let ccx = syn_sem_common::CommonCx::default();
        let scx = SyntaxCx::new(&ccx);

        let receiver = parse::<syn::Receiver, PatType>(&scx, "&self");
        let Type::Reference(ty) = &receiver.ty else {
            panic!()
        };
        assert!(!ty.is_mut);

        let receiver = parse::<syn::Receiver, PatType>(&scx, "&mut self");
        let Type::Reference(ty) = &receiver.ty else {
            panic!()
        };
        assert!(ty.is_mut);
    }
}
