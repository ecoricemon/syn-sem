use crate::{
    Block, FromSyn, Ident, InputDesc, Lit, Parameter, ParameterCx, Path, Span, SyntaxCx, Type,
};
use std::iter;
use syn_sem_macros::CheckDropless;

/// A Rust expression supported by the semantic AST.
///
/// Examples include `x`, `1`, `f(a)`, `S { x: 1 }`, `(a, b)`, and `return x`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub enum Expr<'cx> {
    /// Array literal expression.
    Array(ExprArray<'cx>),
    /// Assignment expression.
    Assign(ExprAssign<'cx>),
    /// Binary operator expression.
    Binary(ExprBinary<'cx>),
    /// Block expression.
    Block(ExprBlock<'cx>),
    /// Function call expression.
    Call(ExprCall<'cx>),
    /// Cast expression.
    Cast(ExprCast<'cx>),
    /// Closure expression.
    Closure(ExprClosure<'cx>),
    /// Const block expression.
    Const(ExprConst<'cx>),
    /// Field access expression.
    Field(ExprField<'cx>),
    /// Indexing expression.
    Index(ExprIndex<'cx>),
    /// Literal expression.
    Lit(ExprLit<'cx>),
    /// Method call expression.
    MethodCall(ExprMethodCall<'cx>),
    /// Parenthesized expression.
    Paren(ExprParen<'cx>),
    /// Path expression.
    Path(ExprPath<'cx>),
    /// Reference expression.
    Reference(ExprReference<'cx>),
    /// Repeated array expression.
    Repeat(ExprRepeat<'cx>),
    /// Return expression.
    Return(ExprReturn<'cx>),
    /// Struct literal expression.
    Struct(ExprStruct<'cx>),
    /// Tuple expression.
    Tuple(ExprTuple<'cx>),
    /// Unary operator expression.
    Unary(ExprUnary<'cx>),
}

impl<'cx> FromSyn<'cx, syn::Expr> for Expr<'cx> {
    fn from_syn(scx: &'cx SyntaxCx<'cx>, desc: InputDesc<'cx, '_, syn::Expr>) -> Self {
        match desc.input {
            syn::Expr::Array(v) => Self::Array(ExprArray::from_syn(scx, desc.with_input(v))),
            syn::Expr::Assign(v) => Self::Assign(ExprAssign::from_syn(scx, desc.with_input(v))),
            syn::Expr::Binary(v) => Self::Binary(ExprBinary::from_syn(scx, desc.with_input(v))),
            syn::Expr::Block(v) => Self::Block(ExprBlock::from_syn(scx, desc.with_input(v))),
            syn::Expr::Call(v) => Self::Call(ExprCall::from_syn(scx, desc.with_input(v))),
            syn::Expr::Cast(v) => Self::Cast(ExprCast::from_syn(scx, desc.with_input(v))),
            syn::Expr::Closure(v) => Self::Closure(ExprClosure::from_syn(scx, desc.with_input(v))),
            syn::Expr::Const(v) => Self::Const(ExprConst::from_syn(scx, desc.with_input(v))),
            syn::Expr::Field(v) => Self::Field(ExprField::from_syn(scx, desc.with_input(v))),
            syn::Expr::Index(v) => Self::Index(ExprIndex::from_syn(scx, desc.with_input(v))),
            syn::Expr::Lit(v) => Self::Lit(ExprLit::from_syn(scx, desc.with_input(v))),
            syn::Expr::MethodCall(v) => {
                Self::MethodCall(ExprMethodCall::from_syn(scx, desc.with_input(v)))
            }
            syn::Expr::Paren(v) => Self::Paren(ExprParen::from_syn(scx, desc.with_input(v))),
            syn::Expr::Path(v) => Self::Path(ExprPath::from_syn(scx, desc.with_input(v))),
            syn::Expr::Reference(v) => {
                Self::Reference(ExprReference::from_syn(scx, desc.with_input(v)))
            }
            syn::Expr::Repeat(v) => Self::Repeat(ExprRepeat::from_syn(scx, desc.with_input(v))),
            syn::Expr::Return(v) => Self::Return(ExprReturn::from_syn(scx, desc.with_input(v))),
            syn::Expr::Struct(v) => Self::Struct(ExprStruct::from_syn(scx, desc.with_input(v))),
            syn::Expr::Tuple(v) => Self::Tuple(ExprTuple::from_syn(scx, desc.with_input(v))),
            syn::Expr::Unary(v) => Self::Unary(ExprUnary::from_syn(scx, desc.with_input(v))),
            o => todo!("{o:?}"),
        }
    }
}

/// A closure expression.
///
/// For example, `|x| x + 1`, `move |x: i32| x`, or `|| -> i32 { 1 }`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct ExprClosure<'cx> {
    /// Whether the closure has the `async` marker.
    pub is_async: bool,
    /// Whether the closure has the `static` marker.
    pub is_static: bool,
    /// Whether the closure captures by `move`.
    pub is_move: bool,
    /// Return parameter followed by input parameters.
    pub params: &'cx [Parameter<'cx>],
    /// Closure body expression.
    ///
    /// Stored by reference to break the recursive [`Expr`] shape.
    pub body: &'cx Expr<'cx>,
    /// Source span of the expression.
    pub span: Span<'cx>,
}

impl<'cx> FromSyn<'cx, syn::ExprClosure> for ExprClosure<'cx> {
    fn from_syn(scx: &'cx SyntaxCx<'cx>, desc: InputDesc<'cx, '_, syn::ExprClosure>) -> Self {
        let output = Parameter::from_return_type(
            scx,
            desc.with_input(&desc.input.output),
            ParameterCx::Closure,
        );
        let inputs = desc
            .input
            .inputs
            .iter()
            .map(|pat| Parameter::from_closure_input(scx, desc.with_input(pat)));
        let mut params = iter::once(output).chain(inputs);
        let len = desc.input.inputs.len() + 1;

        Self {
            is_async: desc.input.asyncness.is_some(),
            is_static: desc.input.movability.is_some(),
            is_move: desc.input.capture.is_some(),
            params: scx.alloc_slice(len, |_| params.next().unwrap()),
            body: scx.alloc(Expr::from_syn(scx, desc.with_input(&desc.input.body))),
            span: desc.span(desc.input),
        }
    }
}

/// An array literal expression.
///
/// For example, `[a, b, c]`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct ExprArray<'cx> {
    /// Element expressions.
    pub elems: &'cx [Expr<'cx>],
    /// Source span of the expression.
    pub span: Span<'cx>,
}

impl<'cx> FromSyn<'cx, syn::ExprArray> for ExprArray<'cx> {
    fn from_syn(scx: &'cx SyntaxCx<'cx>, desc: InputDesc<'cx, '_, syn::ExprArray>) -> Self {
        Self {
            elems: FromSyn::from_syn(scx, desc.with_input(&desc.input.elems)),
            span: desc.span(desc.input),
        }
    }
}

/// An assignment expression.
///
/// For example, `x = y`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct ExprAssign<'cx> {
    /// Left-hand side expression.
    ///
    /// Stored by reference to break the recursive [`Expr`] shape.
    pub left: &'cx Expr<'cx>,
    /// Right-hand side expression.
    ///
    /// Stored by reference to break the recursive [`Expr`] shape.
    pub right: &'cx Expr<'cx>,
    /// Source span of the expression.
    pub span: Span<'cx>,
}

impl<'cx> FromSyn<'cx, syn::ExprAssign> for ExprAssign<'cx> {
    fn from_syn(scx: &'cx SyntaxCx<'cx>, desc: InputDesc<'cx, '_, syn::ExprAssign>) -> Self {
        Self {
            left: scx.alloc(Expr::from_syn(scx, desc.with_input(&desc.input.left))),
            right: scx.alloc(Expr::from_syn(scx, desc.with_input(&desc.input.right))),
            span: desc.span(desc.input),
        }
    }
}

/// A binary operator expression.
///
/// For example, `a + b` or `x == y`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct ExprBinary<'cx> {
    /// Left-hand side expression.
    ///
    /// Stored by reference to break the recursive [`Expr`] shape.
    pub left: &'cx Expr<'cx>,
    /// Right-hand side expression.
    ///
    /// Stored by reference to break the recursive [`Expr`] shape.
    pub right: &'cx Expr<'cx>,
    /// Source span of the expression.
    pub span: Span<'cx>,
}

impl<'cx> FromSyn<'cx, syn::ExprBinary> for ExprBinary<'cx> {
    fn from_syn(scx: &'cx SyntaxCx<'cx>, desc: InputDesc<'cx, '_, syn::ExprBinary>) -> Self {
        Self {
            left: scx.alloc(Expr::from_syn(scx, desc.with_input(&desc.input.left))),
            right: scx.alloc(Expr::from_syn(scx, desc.with_input(&desc.input.right))),
            span: desc.span(desc.input),
        }
    }
}

/// A block expression.
///
/// For example, `{ let x = 1; x }`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct ExprBlock<'cx> {
    /// Block body.
    pub block: Block<'cx>,
    /// Source span of the expression.
    pub span: Span<'cx>,
}

impl<'cx> FromSyn<'cx, syn::ExprBlock> for ExprBlock<'cx> {
    fn from_syn(scx: &'cx SyntaxCx<'cx>, desc: InputDesc<'cx, '_, syn::ExprBlock>) -> Self {
        Self {
            block: Block::from_syn(scx, desc.with_input(&desc.input.block)),
            span: desc.span(desc.input),
        }
    }
}

/// A function call expression.
///
/// For example, `f(a, b)`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct ExprCall<'cx> {
    /// Callee expression.
    ///
    /// Stored by reference to break the recursive [`Expr`] shape.
    pub func: &'cx Expr<'cx>,
    /// Call arguments.
    pub args: &'cx [Expr<'cx>],
    /// Source span of the expression.
    pub span: Span<'cx>,
}

impl<'cx> FromSyn<'cx, syn::ExprCall> for ExprCall<'cx> {
    fn from_syn(scx: &'cx SyntaxCx<'cx>, desc: InputDesc<'cx, '_, syn::ExprCall>) -> Self {
        Self {
            func: scx.alloc(Expr::from_syn(scx, desc.with_input(&desc.input.func))),
            args: FromSyn::from_syn(scx, desc.with_input(&desc.input.args)),
            span: desc.span(desc.input),
        }
    }
}

/// A cast expression.
///
/// For example, `x as i32`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct ExprCast<'cx> {
    /// Expression being cast.
    ///
    /// Stored by reference to break the recursive [`Expr`] shape.
    pub expr: &'cx Expr<'cx>,
    /// Target type.
    ///
    /// Stored by reference to break the recursive expression/type shape.
    pub ty: &'cx Type<'cx>,
    /// Source span of the expression.
    pub span: Span<'cx>,
}

impl<'cx> FromSyn<'cx, syn::ExprCast> for ExprCast<'cx> {
    fn from_syn(scx: &'cx SyntaxCx<'cx>, desc: InputDesc<'cx, '_, syn::ExprCast>) -> Self {
        Self {
            expr: scx.alloc(Expr::from_syn(scx, desc.with_input(&desc.input.expr))),
            ty: scx.alloc(Type::from_syn(scx, desc.with_input(&desc.input.ty))),
            span: desc.span(desc.input),
        }
    }
}

/// A const block expression.
///
/// For example, `const { 1 + 2 }`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct ExprConst<'cx> {
    /// Const block body.
    pub block: Block<'cx>,
    /// Source span of the expression.
    pub span: Span<'cx>,
}

impl<'cx> FromSyn<'cx, syn::ExprConst> for ExprConst<'cx> {
    fn from_syn(scx: &'cx SyntaxCx<'cx>, desc: InputDesc<'cx, '_, syn::ExprConst>) -> Self {
        Self {
            block: Block::from_syn(scx, desc.with_input(&desc.input.block)),
            span: desc.span(desc.input),
        }
    }
}

/// A field access expression.
///
/// For example, `value.field` or `tuple.0`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct ExprField<'cx> {
    /// Base expression.
    ///
    /// Stored by reference to break the recursive [`Expr`] shape.
    pub base: &'cx Expr<'cx>,
    /// Field member name or tuple index.
    pub member: Ident<'cx>,
    /// Source span of the expression.
    pub span: Span<'cx>,
}

impl<'cx> FromSyn<'cx, syn::ExprField> for ExprField<'cx> {
    fn from_syn(scx: &'cx SyntaxCx<'cx>, desc: InputDesc<'cx, '_, syn::ExprField>) -> Self {
        let member = match &desc.input.member {
            syn::Member::Named(ident) => Ident::from_syn(scx, desc.with_input(ident)),
            syn::Member::Unnamed(idx) => Ident::from_number(scx, idx.index, desc.span(idx)),
        };
        Self {
            base: scx.alloc(Expr::from_syn(scx, desc.with_input(&desc.input.base))),
            member,
            span: desc.span(desc.input),
        }
    }
}

/// An indexing expression.
///
/// For example, `items[i]`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct ExprIndex<'cx> {
    /// Indexed expression.
    ///
    /// Stored by reference to break the recursive [`Expr`] shape.
    pub expr: &'cx Expr<'cx>,
    /// Index expression.
    ///
    /// Stored by reference to break the recursive [`Expr`] shape.
    pub index: &'cx Expr<'cx>,
    /// Source span of the expression.
    pub span: Span<'cx>,
}

impl<'cx> FromSyn<'cx, syn::ExprIndex> for ExprIndex<'cx> {
    fn from_syn(scx: &'cx SyntaxCx<'cx>, desc: InputDesc<'cx, '_, syn::ExprIndex>) -> Self {
        Self {
            expr: scx.alloc(Expr::from_syn(scx, desc.with_input(&desc.input.expr))),
            index: scx.alloc(Expr::from_syn(scx, desc.with_input(&desc.input.index))),
            span: desc.span(desc.input),
        }
    }
}

/// A literal expression.
///
/// Examples include `1`, `1.0`, and `false`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct ExprLit<'cx> {
    /// Literal value.
    pub lit: Lit<'cx>,
    /// Source span of the expression.
    pub span: Span<'cx>,
}

impl<'cx> FromSyn<'cx, syn::ExprLit> for ExprLit<'cx> {
    fn from_syn(scx: &'cx SyntaxCx<'cx>, desc: InputDesc<'cx, '_, syn::ExprLit>) -> Self {
        Self {
            lit: Lit::from_syn(scx, desc.with_input(&desc.input.lit)),
            span: desc.span(desc.input),
        }
    }
}

/// A method call expression.
///
/// For example, `value.method(arg)`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct ExprMethodCall<'cx> {
    /// Receiver expression.
    ///
    /// Stored by reference to break the recursive [`Expr`] shape.
    pub receiver: &'cx Expr<'cx>,
    /// Method name.
    pub method: Ident<'cx>,
    /// Method arguments.
    pub args: &'cx [Expr<'cx>],
    /// Source span of the expression.
    pub span: Span<'cx>,
}

impl<'cx> FromSyn<'cx, syn::ExprMethodCall> for ExprMethodCall<'cx> {
    fn from_syn(scx: &'cx SyntaxCx<'cx>, desc: InputDesc<'cx, '_, syn::ExprMethodCall>) -> Self {
        Self {
            receiver: scx.alloc(Expr::from_syn(scx, desc.with_input(&desc.input.receiver))),
            method: Ident::from_syn(scx, desc.with_input(&desc.input.method)),
            args: FromSyn::from_syn(scx, desc.with_input(&desc.input.args)),
            span: desc.span(desc.input),
        }
    }
}

/// A parenthesized expression.
///
/// For example, `(x + y)`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct ExprParen<'cx> {
    /// Inner expression.
    ///
    /// Stored by reference to break the recursive [`Expr`] shape.
    pub expr: &'cx Expr<'cx>,
    /// Source span of the expression.
    pub span: Span<'cx>,
}

impl<'cx> FromSyn<'cx, syn::ExprParen> for ExprParen<'cx> {
    fn from_syn(scx: &'cx SyntaxCx<'cx>, desc: InputDesc<'cx, '_, syn::ExprParen>) -> Self {
        Self {
            expr: scx.alloc(Expr::from_syn(scx, desc.with_input(&desc.input.expr))),
            span: desc.span(desc.input),
        }
    }
}

/// A path expression.
///
/// Examples include `x`, `Self::new`, and `module::CONST`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct ExprPath<'cx> {
    /// Path being referenced.
    pub path: Path<'cx>,
    /// Source span of the expression.
    pub span: Span<'cx>,
}

impl<'cx> FromSyn<'cx, syn::ExprPath> for ExprPath<'cx> {
    fn from_syn(scx: &'cx SyntaxCx<'cx>, desc: InputDesc<'cx, '_, syn::ExprPath>) -> Self {
        Self {
            path: Path::from_syn(scx, desc.with_input(&desc.input.path)),
            span: desc.span(desc.input),
        }
    }
}

/// A reference expression.
///
/// Examples include `&x` and `&mut x`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct ExprReference<'cx> {
    /// Referenced expression.
    ///
    /// Stored by reference to break the recursive [`Expr`] shape.
    pub expr: &'cx Expr<'cx>,
    /// Whether the reference is mutable.
    pub is_mut: bool,
    /// Source span of the expression.
    pub span: Span<'cx>,
}

impl<'cx> FromSyn<'cx, syn::ExprReference> for ExprReference<'cx> {
    fn from_syn(scx: &'cx SyntaxCx<'cx>, desc: InputDesc<'cx, '_, syn::ExprReference>) -> Self {
        Self {
            expr: scx.alloc(Expr::from_syn(scx, desc.with_input(&desc.input.expr))),
            is_mut: desc.input.mutability.is_some(),
            span: desc.span(desc.input),
        }
    }
}

/// A repeated array expression.
///
/// For example, `[value; N]`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct ExprRepeat<'cx> {
    /// Repeated value expression.
    ///
    /// Stored by reference to break the recursive [`Expr`] shape.
    pub expr: &'cx Expr<'cx>,
    /// Length expression.
    ///
    /// Stored by reference to break the recursive [`Expr`] shape.
    pub len: &'cx Expr<'cx>,
    /// Source span of the expression.
    pub span: Span<'cx>,
}

impl<'cx> FromSyn<'cx, syn::ExprRepeat> for ExprRepeat<'cx> {
    fn from_syn(scx: &'cx SyntaxCx<'cx>, desc: InputDesc<'cx, '_, syn::ExprRepeat>) -> Self {
        Self {
            expr: scx.alloc(Expr::from_syn(scx, desc.with_input(&desc.input.expr))),
            len: scx.alloc(Expr::from_syn(scx, desc.with_input(&desc.input.len))),
            span: desc.span(desc.input),
        }
    }
}

/// A return expression.
///
/// Examples include `return` and `return value`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct ExprReturn<'cx> {
    /// Optional returned expression.
    ///
    /// Stored by reference to break the recursive [`Expr`] shape.
    pub expr: Option<&'cx Expr<'cx>>,
    /// Source span of the expression.
    pub span: Span<'cx>,
}

impl<'cx> FromSyn<'cx, syn::ExprReturn> for ExprReturn<'cx> {
    fn from_syn(scx: &'cx SyntaxCx<'cx>, desc: InputDesc<'cx, '_, syn::ExprReturn>) -> Self {
        Self {
            expr: desc
                .input
                .expr
                .as_ref()
                .map(|expr| scx.alloc(Expr::from_syn(scx, desc.with_input(expr)))),
            span: desc.span(desc.input),
        }
    }
}

/// A struct literal expression.
///
/// For example, `Point { x: 1, y: 2 }`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct ExprStruct<'cx> {
    /// Path naming the constructed struct.
    pub path: Path<'cx>,
    /// Field initializers.
    pub fields: &'cx [FieldValue<'cx>],
    /// Optional rest expression.
    ///
    /// Stored by reference to break the recursive [`Expr`] shape.
    pub rest: Option<&'cx Expr<'cx>>,
    /// Source span of the expression.
    pub span: Span<'cx>,
}

impl<'cx> FromSyn<'cx, syn::ExprStruct> for ExprStruct<'cx> {
    fn from_syn(scx: &'cx SyntaxCx<'cx>, desc: InputDesc<'cx, '_, syn::ExprStruct>) -> Self {
        Self {
            path: Path::from_syn(scx, desc.with_input(&desc.input.path)),
            fields: FromSyn::from_syn(scx, desc.with_input(&desc.input.fields)),
            rest: desc
                .input
                .rest
                .as_ref()
                .map(|rest| scx.alloc(Expr::from_syn(scx, desc.with_input(rest)))),
            span: desc.span(desc.input),
        }
    }
}

/// One field assignment inside a struct literal.
///
/// For example, `x: 1` in `Point { x: 1 }`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct FieldValue<'cx> {
    /// Field member being initialized.
    pub member: Ident<'cx>,
    /// Initializer expression.
    ///
    /// Stored by reference to break the recursive [`Expr`] shape.
    pub expr: &'cx Expr<'cx>,
    /// Source span of the field value.
    pub span: Span<'cx>,
}

impl<'cx> FromSyn<'cx, syn::FieldValue> for FieldValue<'cx> {
    fn from_syn(scx: &'cx SyntaxCx<'cx>, desc: InputDesc<'cx, '_, syn::FieldValue>) -> Self {
        let member = match &desc.input.member {
            syn::Member::Named(ident) => Ident::from_syn(scx, desc.with_input(ident)),
            syn::Member::Unnamed(idx) => Ident::from_number(scx, idx.index, desc.span(idx)),
        };

        Self {
            member,
            expr: scx.alloc(Expr::from_syn(scx, desc.with_input(&desc.input.expr))),
            span: desc.span(desc.input),
        }
    }
}

/// A tuple expression.
///
/// Examples include `()`, `(x,)`, and `(x, y)`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct ExprTuple<'cx> {
    /// Element expressions.
    pub elems: &'cx [Expr<'cx>],
    /// Source span of the expression.
    pub span: Span<'cx>,
}

impl<'cx> FromSyn<'cx, syn::ExprTuple> for ExprTuple<'cx> {
    fn from_syn(scx: &'cx SyntaxCx<'cx>, desc: InputDesc<'cx, '_, syn::ExprTuple>) -> Self {
        Self {
            elems: FromSyn::from_syn(scx, desc.with_input(&desc.input.elems)),
            span: desc.span(desc.input),
        }
    }
}

/// A unary operator.
///
/// Examples include dereference `*x`, not `!x`, and negation `-x`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub enum UnOp {
    /// Dereference operator `*`.
    Deref,
    /// Logical or bitwise not operator `!`.
    Not,
    /// Negation operator `-`.
    Neg,
}

impl<'cx> FromSyn<'cx, syn::UnOp> for UnOp {
    fn from_syn(_: &'cx SyntaxCx<'cx>, desc: InputDesc<'cx, '_, syn::UnOp>) -> Self {
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
pub struct ExprUnary<'cx> {
    /// Unary operator.
    pub op: UnOp,
    /// Operand expression.
    ///
    /// Stored by reference to break the recursive [`Expr`] shape.
    pub expr: &'cx Expr<'cx>,
    /// Source span of the expression.
    pub span: Span<'cx>,
}

impl<'cx> FromSyn<'cx, syn::ExprUnary> for ExprUnary<'cx> {
    fn from_syn(scx: &'cx SyntaxCx<'cx>, desc: InputDesc<'cx, '_, syn::ExprUnary>) -> Self {
        Self {
            op: UnOp::from_syn(scx, desc.with_input(&desc.input.op)),
            expr: scx.alloc(Expr::from_syn(scx, desc.with_input(&desc.input.expr))),
            span: desc.span(desc.input),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::*;

    #[test]
    fn expr_call() {
        // Proves call expressions preserve the callee and argument list.
        let ccx = syn_sem_common::CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let expr = parse::<syn::ExprCall, ExprCall>(&scx, "invoke(a, b)");
        let Expr::Path(path) = expr.func else {
            panic!()
        };
        assert_eq!(&**path.path.get_ident().unwrap(), "invoke");
        assert_eq!(expr.args.len(), 2);
    }

    #[test]
    fn expr_lit() {
        // Proves expression literals wrap the parsed literal value.
        let ccx = syn_sem_common::CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let expr = parse::<syn::ExprLit, ExprLit>(&scx, "1");
        let Lit::Int(v) = &expr.lit else { panic!() };
        assert_eq!(v.base10_parse::<i32>().unwrap(), 1);
    }

    #[test]
    fn expr_const() {
        // Proves const expressions preserve their block body.
        let ccx = syn_sem_common::CommonCx::default();
        let scx = SyntaxCx::new(&ccx);

        let expr = parse::<syn::ExprConst, ExprConst>(&scx, "const { 1 }");
        assert_eq!(expr.block.stmts.len(), 1);
    }

    #[test]
    fn expr_closure() {
        // Proves closure expressions preserve markers, params, inferred types, and body.
        let ccx = syn_sem_common::CommonCx::default();
        let scx = SyntaxCx::new(&ccx);

        let expr = parse::<syn::ExprClosure, ExprClosure>(&scx, "move |x| x");
        assert!(expr.is_move);
        assert_eq!(expr.params.len(), 2);
        assert!(matches!(expr.params[0].pat.ty, Type::Infer(_)));
        assert!(matches!(expr.params[1].pat.ty, Type::Infer(_)));
        assert!(matches!(expr.body, Expr::Path(_)));

        let expr = parse::<syn::ExprClosure, ExprClosure>(&scx, "|x: i32| -> i32 { x }");
        assert!(matches!(expr.params[0].pat.ty, Type::Path(_)));
        assert!(matches!(expr.params[1].pat.ty, Type::Path(_)));
        assert!(matches!(expr.body, Expr::Block(_)));
    }

    #[test]
    fn expr_repeat() {
        // Proves repeat expressions preserve both the repeated value and length.
        let ccx = syn_sem_common::CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let expr = parse::<syn::ExprRepeat, ExprRepeat>(&scx, "[value; len]");
        assert!(matches!(expr.expr, Expr::Path(_)));
        assert!(matches!(expr.len, Expr::Path(_)));
    }

    #[test]
    fn expr_reference() {
        // Proves reference expressions preserve mutability and referenced expression.
        let ccx = syn_sem_common::CommonCx::default();
        let scx = SyntaxCx::new(&ccx);

        let expr = parse::<syn::ExprReference, ExprReference>(&scx, "&value");
        assert!(!expr.is_mut);
        assert!(matches!(expr.expr, Expr::Path(_)));

        let expr = parse::<syn::ExprReference, ExprReference>(&scx, "&mut value");
        assert!(expr.is_mut);
        assert!(matches!(expr.expr, Expr::Path(_)));
    }

    #[test]
    fn expr_return() {
        // Proves return expressions preserve optional return values.
        let ccx = syn_sem_common::CommonCx::default();
        let scx = SyntaxCx::new(&ccx);

        let expr = parse::<syn::ExprReturn, ExprReturn>(&scx, "return");
        assert!(expr.expr.is_none());

        let expr = parse::<syn::ExprReturn, ExprReturn>(&scx, "return value");
        assert!(matches!(expr.expr.unwrap(), Expr::Path(_)));
    }

    #[test]
    fn expr_struct() {
        // Proves struct expressions preserve path, fields, rest, and path arguments.
        let ccx = syn_sem_common::CommonCx::default();
        let scx = SyntaxCx::new(&ccx);

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
    fn expr_path() {
        // Proves expression paths preserve generic arguments on path segments.
        let ccx = syn_sem_common::CommonCx::default();
        let scx = SyntaxCx::new(&ccx);

        let expr = parse::<syn::ExprPath, ExprPath>(&scx, "make::<T>");
        assert_eq!(&*expr.path.segments[0].ident, "make");
        assert!(expr.path.segments[0].has_args());
    }

    #[test]
    fn expr_tuple() {
        // Proves tuple expressions preserve empty and multi-element tuples.
        let ccx = syn_sem_common::CommonCx::default();
        let scx = SyntaxCx::new(&ccx);

        let expr = parse::<syn::ExprTuple, ExprTuple>(&scx, "()");
        assert!(expr.elems.is_empty());

        let expr = parse::<syn::ExprTuple, ExprTuple>(&scx, "(a, b)");
        assert_eq!(expr.elems.len(), 2);
        assert!(matches!(expr.elems[0], Expr::Path(_)));
        assert!(matches!(expr.elems[1], Expr::Path(_)));
    }

    #[test]
    fn expr_unary() {
        // Proves unary expressions classify deref, not, and negation operators.
        let ccx = syn_sem_common::CommonCx::default();
        let scx = SyntaxCx::new(&ccx);

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
