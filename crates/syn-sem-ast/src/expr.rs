use crate::{Block, FromSyn, Ident, InputDesc, Lit, Path, Span, SyntaxCx, Type};
use syn_sem_macros::CheckDropless;

/// A Rust expression supported by the semantic AST.
///
/// Examples include `x`, `1`, `f(a)`, `S { x: 1 }`, `(a, b)`, and `return x`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub enum Expr<'scx> {
    Array(ExprArray<'scx>),
    Assign(ExprAssign<'scx>),
    Binary(ExprBinary<'scx>),
    Block(ExprBlock<'scx>),
    Call(ExprCall<'scx>),
    Cast(ExprCast<'scx>),
    Const(ExprConst<'scx>),
    Field(ExprField<'scx>),
    Index(ExprIndex<'scx>),
    Lit(ExprLit<'scx>),
    MethodCall(ExprMethodCall<'scx>),
    Paren(ExprParen<'scx>),
    Path(ExprPath<'scx>),
    Reference(ExprReference<'scx>),
    Repeat(ExprRepeat<'scx>),
    Return(ExprReturn<'scx>),
    Struct(ExprStruct<'scx>),
    Tuple(ExprTuple<'scx>),
    Unary(ExprUnary<'scx>),
}

impl<'scx> FromSyn<'scx, syn::Expr> for Expr<'scx> {
    fn from_syn(scx: &'scx SyntaxCx, desc: InputDesc<'_, syn::Expr>) -> Self {
        match desc.input {
            syn::Expr::Array(v) => Self::Array(ExprArray::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: v,
                },
            )),
            syn::Expr::Assign(v) => Self::Assign(ExprAssign::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: v,
                },
            )),
            syn::Expr::Binary(v) => Self::Binary(ExprBinary::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: v,
                },
            )),
            syn::Expr::Block(v) => Self::Block(ExprBlock::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: v,
                },
            )),
            syn::Expr::Call(v) => Self::Call(ExprCall::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: v,
                },
            )),
            syn::Expr::Cast(v) => Self::Cast(ExprCast::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: v,
                },
            )),
            syn::Expr::Const(v) => Self::Const(ExprConst::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: v,
                },
            )),
            syn::Expr::Field(v) => Self::Field(ExprField::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: v,
                },
            )),
            syn::Expr::Index(v) => Self::Index(ExprIndex::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: v,
                },
            )),
            syn::Expr::Lit(v) => Self::Lit(ExprLit::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: v,
                },
            )),
            syn::Expr::MethodCall(v) => Self::MethodCall(ExprMethodCall::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: v,
                },
            )),
            syn::Expr::Paren(v) => Self::Paren(ExprParen::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: v,
                },
            )),
            syn::Expr::Path(v) => Self::Path(ExprPath::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: v,
                },
            )),
            syn::Expr::Reference(v) => Self::Reference(ExprReference::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: v,
                },
            )),
            syn::Expr::Repeat(v) => Self::Repeat(ExprRepeat::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: v,
                },
            )),
            syn::Expr::Return(v) => Self::Return(ExprReturn::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: v,
                },
            )),
            syn::Expr::Struct(v) => Self::Struct(ExprStruct::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: v,
                },
            )),
            syn::Expr::Tuple(v) => Self::Tuple(ExprTuple::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: v,
                },
            )),
            syn::Expr::Unary(v) => Self::Unary(ExprUnary::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: v,
                },
            )),
            o => todo!("{o:?}"),
        }
    }
}

/// An array literal expression.
///
/// For example, `[a, b, c]`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct ExprArray<'scx> {
    pub elems: &'scx [Expr<'scx>],
    pub span: Span<'scx>,
}

impl<'scx> FromSyn<'scx, syn::ExprArray> for ExprArray<'scx> {
    fn from_syn(scx: &'scx SyntaxCx, desc: InputDesc<'_, syn::ExprArray>) -> Self {
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

/// An assignment expression.
///
/// For example, `x = y`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct ExprAssign<'scx> {
    pub left: &'scx Expr<'scx>,
    pub right: &'scx Expr<'scx>,
    pub span: Span<'scx>,
}

impl<'scx> FromSyn<'scx, syn::ExprAssign> for ExprAssign<'scx> {
    fn from_syn(scx: &'scx SyntaxCx, desc: InputDesc<'_, syn::ExprAssign>) -> Self {
        Self {
            left: scx.alloc(Expr::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.left,
                },
            )),
            right: scx.alloc(Expr::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.right,
                },
            )),
            span: Span::from_locatable(scx, desc.file_path, desc.input),
        }
    }
}

/// A binary operator expression.
///
/// For example, `a + b` or `x == y`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct ExprBinary<'scx> {
    pub left: &'scx Expr<'scx>,
    pub right: &'scx Expr<'scx>,
    pub span: Span<'scx>,
}

impl<'scx> FromSyn<'scx, syn::ExprBinary> for ExprBinary<'scx> {
    fn from_syn(scx: &'scx SyntaxCx, desc: InputDesc<'_, syn::ExprBinary>) -> Self {
        Self {
            left: scx.alloc(Expr::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.left,
                },
            )),
            right: scx.alloc(Expr::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.right,
                },
            )),
            span: Span::from_locatable(scx, desc.file_path, desc.input),
        }
    }
}

/// A block expression.
///
/// For example, `{ let x = 1; x }`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct ExprBlock<'scx> {
    pub block: Block<'scx>,
    pub span: Span<'scx>,
}

impl<'scx> FromSyn<'scx, syn::ExprBlock> for ExprBlock<'scx> {
    fn from_syn(scx: &'scx SyntaxCx, desc: InputDesc<'_, syn::ExprBlock>) -> Self {
        Self {
            block: Block::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.block,
                },
            ),
            span: Span::from_locatable(scx, desc.file_path, desc.input),
        }
    }
}

/// A function call expression.
///
/// For example, `f(a, b)`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct ExprCall<'scx> {
    pub func: &'scx Expr<'scx>,
    pub args: &'scx [Expr<'scx>],
    pub span: Span<'scx>,
}

impl<'scx> FromSyn<'scx, syn::ExprCall> for ExprCall<'scx> {
    fn from_syn(scx: &'scx SyntaxCx, desc: InputDesc<'_, syn::ExprCall>) -> Self {
        Self {
            func: scx.alloc(Expr::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.func,
                },
            )),
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

/// A cast expression.
///
/// For example, `x as i32`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct ExprCast<'scx> {
    pub expr: &'scx Expr<'scx>,
    pub ty: &'scx Type<'scx>,
    pub span: Span<'scx>,
}

impl<'scx> FromSyn<'scx, syn::ExprCast> for ExprCast<'scx> {
    fn from_syn(scx: &'scx SyntaxCx, desc: InputDesc<'_, syn::ExprCast>) -> Self {
        Self {
            expr: scx.alloc(Expr::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.expr,
                },
            )),
            ty: scx.alloc(Type::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.ty,
                },
            )),
            span: Span::from_locatable(scx, desc.file_path, desc.input),
        }
    }
}

/// A const block expression.
///
/// For example, `const { 1 + 2 }`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct ExprConst<'scx> {
    pub block: Block<'scx>,
    pub span: Span<'scx>,
}

impl<'scx> FromSyn<'scx, syn::ExprConst> for ExprConst<'scx> {
    fn from_syn(scx: &'scx SyntaxCx, desc: InputDesc<'_, syn::ExprConst>) -> Self {
        Self {
            block: Block::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.block,
                },
            ),
            span: Span::from_locatable(scx, desc.file_path, desc.input),
        }
    }
}

/// A field access expression.
///
/// For example, `value.field` or `tuple.0`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct ExprField<'scx> {
    pub base: &'scx Expr<'scx>,
    pub member: Ident<'scx>,
    pub span: Span<'scx>,
}

impl<'scx> FromSyn<'scx, syn::ExprField> for ExprField<'scx> {
    fn from_syn(scx: &'scx SyntaxCx, desc: InputDesc<'_, syn::ExprField>) -> Self {
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
            base: scx.alloc(Expr::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.base,
                },
            )),
            member,
            span: Span::from_locatable(scx, desc.file_path, desc.input),
        }
    }
}

/// An indexing expression.
///
/// For example, `items[i]`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct ExprIndex<'scx> {
    pub expr: &'scx Expr<'scx>,
    pub index: &'scx Expr<'scx>,
    pub span: Span<'scx>,
}

impl<'scx> FromSyn<'scx, syn::ExprIndex> for ExprIndex<'scx> {
    fn from_syn(scx: &'scx SyntaxCx, desc: InputDesc<'_, syn::ExprIndex>) -> Self {
        Self {
            expr: scx.alloc(Expr::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.expr,
                },
            )),
            index: scx.alloc(Expr::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.index,
                },
            )),
            span: Span::from_locatable(scx, desc.file_path, desc.input),
        }
    }
}

/// A literal expression.
///
/// Examples include `1`, `1.0`, and `false`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct ExprLit<'scx> {
    pub lit: Lit<'scx>,
    pub span: Span<'scx>,
}

impl<'scx> FromSyn<'scx, syn::ExprLit> for ExprLit<'scx> {
    fn from_syn(scx: &'scx SyntaxCx, desc: InputDesc<'_, syn::ExprLit>) -> Self {
        Self {
            lit: Lit::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.lit,
                },
            ),
            span: Span::from_locatable(scx, desc.file_path, desc.input),
        }
    }
}

/// A method call expression.
///
/// For example, `value.method(arg)`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct ExprMethodCall<'scx> {
    pub receiver: &'scx Expr<'scx>,
    pub method: Ident<'scx>,
    pub args: &'scx [Expr<'scx>],
    pub span: Span<'scx>,
}

impl<'scx> FromSyn<'scx, syn::ExprMethodCall> for ExprMethodCall<'scx> {
    fn from_syn(scx: &'scx SyntaxCx, desc: InputDesc<'_, syn::ExprMethodCall>) -> Self {
        Self {
            receiver: scx.alloc(Expr::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.receiver,
                },
            )),
            method: Ident::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.method,
                },
            ),
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

/// A parenthesized expression.
///
/// For example, `(x + y)`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct ExprParen<'scx> {
    pub expr: &'scx Expr<'scx>,
    pub span: Span<'scx>,
}

impl<'scx> FromSyn<'scx, syn::ExprParen> for ExprParen<'scx> {
    fn from_syn(scx: &'scx SyntaxCx, desc: InputDesc<'_, syn::ExprParen>) -> Self {
        Self {
            expr: scx.alloc(Expr::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.expr,
                },
            )),
            span: Span::from_locatable(scx, desc.file_path, desc.input),
        }
    }
}

/// A path expression.
///
/// Examples include `x`, `Self::new`, and `module::CONST`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct ExprPath<'scx> {
    pub path: Path<'scx>,
    pub span: Span<'scx>,
}

impl<'scx> FromSyn<'scx, syn::ExprPath> for ExprPath<'scx> {
    fn from_syn(scx: &'scx SyntaxCx, desc: InputDesc<'_, syn::ExprPath>) -> Self {
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

/// A reference expression.
///
/// Examples include `&x` and `&mut x`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct ExprReference<'scx> {
    pub expr: &'scx Expr<'scx>,
    pub is_mut: bool,
    pub span: Span<'scx>,
}

impl<'scx> FromSyn<'scx, syn::ExprReference> for ExprReference<'scx> {
    fn from_syn(scx: &'scx SyntaxCx, desc: InputDesc<'_, syn::ExprReference>) -> Self {
        Self {
            expr: scx.alloc(Expr::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.expr,
                },
            )),
            is_mut: desc.input.mutability.is_some(),
            span: Span::from_locatable(scx, desc.file_path, desc.input),
        }
    }
}

/// A repeated array expression.
///
/// For example, `[value; N]`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct ExprRepeat<'scx> {
    pub expr: &'scx Expr<'scx>,
    pub len: &'scx Expr<'scx>,
    pub span: Span<'scx>,
}

impl<'scx> FromSyn<'scx, syn::ExprRepeat> for ExprRepeat<'scx> {
    fn from_syn(scx: &'scx SyntaxCx, desc: InputDesc<'_, syn::ExprRepeat>) -> Self {
        Self {
            expr: scx.alloc(Expr::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.expr,
                },
            )),
            len: scx.alloc(Expr::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.len,
                },
            )),
            span: Span::from_locatable(scx, desc.file_path, desc.input),
        }
    }
}

/// A return expression.
///
/// Examples include `return` and `return value`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct ExprReturn<'scx> {
    pub expr: Option<&'scx Expr<'scx>>,
    pub span: Span<'scx>,
}

impl<'scx> FromSyn<'scx, syn::ExprReturn> for ExprReturn<'scx> {
    fn from_syn(scx: &'scx SyntaxCx, desc: InputDesc<'_, syn::ExprReturn>) -> Self {
        Self {
            expr: desc.input.expr.as_ref().map(|expr| {
                scx.alloc(Expr::from_syn(
                    scx,
                    InputDesc {
                        file_path: desc.file_path,
                        input: expr,
                    },
                ))
            }),
            span: Span::from_locatable(scx, desc.file_path, desc.input),
        }
    }
}

/// A struct literal expression.
///
/// For example, `Point { x: 1, y: 2 }`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct ExprStruct<'scx> {
    pub path: Path<'scx>,
    pub fields: &'scx [FieldValue<'scx>],
    pub rest: Option<&'scx Expr<'scx>>,
    pub span: Span<'scx>,
}

impl<'scx> FromSyn<'scx, syn::ExprStruct> for ExprStruct<'scx> {
    fn from_syn(scx: &'scx SyntaxCx, desc: InputDesc<'_, syn::ExprStruct>) -> Self {
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
                scx.alloc(Expr::from_syn(
                    scx,
                    InputDesc {
                        file_path: desc.file_path,
                        input: rest,
                    },
                ))
            }),
            span: Span::from_locatable(scx, desc.file_path, desc.input),
        }
    }
}

/// One field assignment inside a struct literal.
///
/// For example, `x: 1` in `Point { x: 1 }`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct FieldValue<'scx> {
    pub member: Ident<'scx>,
    pub expr: &'scx Expr<'scx>,
    pub span: Span<'scx>,
}

impl<'scx> FromSyn<'scx, syn::FieldValue> for FieldValue<'scx> {
    fn from_syn(scx: &'scx SyntaxCx, desc: InputDesc<'_, syn::FieldValue>) -> Self {
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
            expr: scx.alloc(Expr::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.expr,
                },
            )),
            span: Span::from_locatable(scx, desc.file_path, desc.input),
        }
    }
}

/// A tuple expression.
///
/// Examples include `()`, `(x,)`, and `(x, y)`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct ExprTuple<'scx> {
    pub elems: &'scx [Expr<'scx>],
    pub span: Span<'scx>,
}

impl<'scx> FromSyn<'scx, syn::ExprTuple> for ExprTuple<'scx> {
    fn from_syn(scx: &'scx SyntaxCx, desc: InputDesc<'_, syn::ExprTuple>) -> Self {
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

/// A unary operator.
///
/// Examples include dereference `*x`, not `!x`, and negation `-x`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub enum UnOp {
    Deref,
    Not,
    Neg,
}

impl<'scx> FromSyn<'scx, syn::UnOp> for UnOp {
    fn from_syn(_: &'scx SyntaxCx, desc: InputDesc<'_, syn::UnOp>) -> Self {
        match desc.input {
            syn::UnOp::Deref(_) => Self::Deref,
            syn::UnOp::Not(_) => Self::Not,
            syn::UnOp::Neg(_) => Self::Neg,
            _ => todo!(),
        }
    }
}

/// A unary operator expression.
///
/// For example, `!flag` or `-value`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct ExprUnary<'scx> {
    pub op: UnOp,
    pub expr: &'scx Expr<'scx>,
    pub span: Span<'scx>,
}

impl<'scx> FromSyn<'scx, syn::ExprUnary> for ExprUnary<'scx> {
    fn from_syn(scx: &'scx SyntaxCx, desc: InputDesc<'_, syn::ExprUnary>) -> Self {
        Self {
            op: UnOp::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.op,
                },
            ),
            expr: scx.alloc(Expr::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.expr,
                },
            )),
            span: Span::from_locatable(scx, desc.file_path, desc.input),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::*;

    #[test]
    fn test_expr_call() {
        // Proves call expressions preserve the callee and argument list.
        let scx = SyntaxCx::default();
        let expr = parse::<syn::ExprCall, ExprCall>(&scx, "invoke(a, b)");
        let Expr::Path(path) = expr.func else {
            panic!()
        };
        assert_eq!(&**path.path.get_ident().unwrap(), "invoke");
        assert_eq!(expr.args.len(), 2);
    }

    #[test]
    fn test_expr_lit() {
        // Proves expression literals wrap the parsed literal value.
        let scx = SyntaxCx::default();
        let expr = parse::<syn::ExprLit, ExprLit>(&scx, "1");
        let Lit::Int(v) = expr.lit else { panic!() };
        assert_eq!(v.base10_parse::<i32>().unwrap(), 1);
    }

    #[test]
    fn test_expr_const() {
        // Proves const expressions preserve their block body.
        let scx = SyntaxCx::default();

        let expr = parse::<syn::ExprConst, ExprConst>(&scx, "const { 1 }");
        assert_eq!(expr.block.stmts.len(), 1);
    }

    #[test]
    fn test_expr_repeat() {
        // Proves repeat expressions preserve both the repeated value and length.
        let scx = SyntaxCx::default();
        let expr = parse::<syn::ExprRepeat, ExprRepeat>(&scx, "[value; len]");
        assert!(matches!(expr.expr, Expr::Path(_)));
        assert!(matches!(expr.len, Expr::Path(_)));
    }

    #[test]
    fn test_expr_reference() {
        // Proves reference expressions preserve mutability and referenced expression.
        let scx = SyntaxCx::default();

        let expr = parse::<syn::ExprReference, ExprReference>(&scx, "&value");
        assert!(!expr.is_mut);
        assert!(matches!(expr.expr, Expr::Path(_)));

        let expr = parse::<syn::ExprReference, ExprReference>(&scx, "&mut value");
        assert!(expr.is_mut);
        assert!(matches!(expr.expr, Expr::Path(_)));
    }

    #[test]
    fn test_expr_return() {
        // Proves return expressions preserve optional return values.
        let scx = SyntaxCx::default();

        let expr = parse::<syn::ExprReturn, ExprReturn>(&scx, "return");
        assert!(expr.expr.is_none());

        let expr = parse::<syn::ExprReturn, ExprReturn>(&scx, "return value");
        assert!(matches!(expr.expr.unwrap(), Expr::Path(_)));
    }

    #[test]
    fn test_expr_struct() {
        // Proves struct expressions preserve path, fields, rest, and path arguments.
        let scx = SyntaxCx::default();

        let expr = parse::<syn::ExprStruct, ExprStruct>(&scx, "S { a, b: c }");
        assert_eq!(&**expr.path.get_ident().unwrap(), "S");
        assert_eq!(expr.fields.len(), 2);
        assert_eq!(&*expr.fields[0].member, "a");
        assert!(matches!(expr.fields[0].expr, Expr::Path(_)));
        assert_eq!(&*expr.fields[1].member, "b");
        assert!(matches!(expr.fields[1].expr, Expr::Path(_)));
        assert!(expr.rest.is_none());

        let expr = parse::<syn::ExprStruct, ExprStruct>(&scx, "S { a: 1, ..base }");
        assert_eq!(expr.fields.len(), 1);
        assert!(matches!(expr.rest.unwrap(), Expr::Path(_)));

        let expr = parse::<syn::ExprStruct, ExprStruct>(&scx, "S::<T> { value }");
        assert_eq!(&*expr.path.segments[0].ident, "S");
        assert!(expr.path.segments[0].has_args());
    }

    #[test]
    fn test_expr_path() {
        // Proves expression paths preserve generic arguments on path segments.
        let scx = SyntaxCx::default();

        let expr = parse::<syn::ExprPath, ExprPath>(&scx, "make::<T>");
        assert_eq!(&*expr.path.segments[0].ident, "make");
        assert!(expr.path.segments[0].has_args());
    }

    #[test]
    fn test_expr_tuple() {
        // Proves tuple expressions preserve empty and multi-element tuples.
        let scx = SyntaxCx::default();

        let expr = parse::<syn::ExprTuple, ExprTuple>(&scx, "()");
        assert!(expr.elems.is_empty());

        let expr = parse::<syn::ExprTuple, ExprTuple>(&scx, "(a, b)");
        assert_eq!(expr.elems.len(), 2);
        assert!(matches!(expr.elems[0], Expr::Path(_)));
        assert!(matches!(expr.elems[1], Expr::Path(_)));
    }

    #[test]
    fn test_expr_unary() {
        // Proves unary expressions classify deref, not, and negation operators.
        let scx = SyntaxCx::default();

        let expr = parse::<syn::ExprUnary, ExprUnary>(&scx, "*value");
        assert_eq!(expr.op, UnOp::Deref);
        assert!(matches!(expr.expr, Expr::Path(_)));

        let expr = parse::<syn::ExprUnary, ExprUnary>(&scx, "!value");
        assert_eq!(expr.op, UnOp::Not);
        assert!(matches!(expr.expr, Expr::Path(_)));

        let expr = parse::<syn::ExprUnary, ExprUnary>(&scx, "-value");
        assert_eq!(expr.op, UnOp::Neg);
        assert!(matches!(expr.expr, Expr::Path(_)));
    }
}
