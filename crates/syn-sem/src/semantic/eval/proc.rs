use super::value::{Field, Fn, FnBody, FnInputs, Scalar, Value};
use crate::{
    etc::{
        known,
        syn::{SynPath, SynPathKind},
    },
    semantic::{
        basic_traits::{RawScope, Scope, Scoping},
        entry::GlobalCx,
        infer,
        tree::PathId,
    },
    Intern, Map, Result, TriResult,
};
use any_intern::Interned;
use logic_eval_util::{str::StrPath, symbol::SymbolTable};
use std::{collections::hash_map::Entry, mem};

struct ValueWithCtrl<'gcx> {
    value: Value<'gcx>,
    is_return: bool,
}

impl<'gcx> From<Value<'gcx>> for ValueWithCtrl<'gcx> {
    fn from(value: Value<'gcx>) -> Self {
        ValueWithCtrl {
            value,
            is_return: false,
        }
    }
}

// === Host ===

#[allow(unused_variables)]
pub(crate) trait Host<'gcx>: Scoping {
    fn find_type(&mut self, expr: &syn::Expr) -> TriResult<infer::Type<'gcx>, ()>;
    fn find_fn(&mut self, name: StrPath, types: &[infer::Type<'gcx>]) -> Fn;
    fn syn_path_to_value(&mut self, syn_path: SynPath) -> TriResult<Value<'gcx>, ()>;
}

struct HostWrapper<'a, H> {
    inner: &'a mut H,
    scope_stack: Vec<RawScope>,
}

impl<'a, 'gcx, H: Host<'gcx>> HostWrapper<'a, H> {
    fn new(host: &'a mut H) -> Self {
        Self {
            inner: host,
            scope_stack: Vec::new(),
        }
    }

    fn eval_known_fn(&mut self, abs_path: &str, values: &[Value<'gcx>]) -> Option<Value<'gcx>> {
        use known::apply;
        use once_cell::sync::OnceCell;

        type F = for<'a> fn(&[Value<'a>]) -> Result<Value<'a>>;

        static FMAP: OnceCell<Map<&'static str, F>> = OnceCell::new();

        let fmap = FMAP.get_or_init(|| {
            let mut map: Map<&'static str, F> = Map::default();

            map.insert(apply::NAME_ADD, |values: &[Value<'_>]| {
                debug_assert_eq!(values.len(), 2);
                values[0].try_add(&values[1])
            });
            map.insert(apply::NAME_SUB, |values: &[Value<'_>]| {
                debug_assert_eq!(values.len(), 2);
                values[0].try_sub(&values[1])
            });
            map.insert(apply::NAME_MUL, |values: &[Value<'_>]| {
                debug_assert_eq!(values.len(), 2);
                values[0].try_mul(&values[1])
            });
            map.insert(apply::NAME_DIV, |values: &[Value<'_>]| {
                debug_assert_eq!(values.len(), 2);
                values[0].try_div(&values[1])
            });
            map.insert(apply::NAME_REM, |values: &[Value<'_>]| {
                debug_assert_eq!(values.len(), 2);
                values[0].try_rem(&values[1])
            });
            map.insert(apply::NAME_BIT_XOR, |values: &[Value<'_>]| {
                debug_assert_eq!(values.len(), 2);
                values[0].try_bit_xor(&values[1])
            });
            map.insert(apply::NAME_BIT_AND, |values: &[Value<'_>]| {
                debug_assert_eq!(values.len(), 2);
                values[0].try_bit_and(&values[1])
            });
            map.insert(apply::NAME_BIT_OR, |values: &[Value<'_>]| {
                debug_assert_eq!(values.len(), 2);
                values[0].try_bit_or(&values[1])
            });
            map.insert(apply::NAME_SHL, |values: &[Value<'_>]| {
                debug_assert_eq!(values.len(), 2);
                values[0].try_shl(&values[1])
            });
            map.insert(apply::NAME_SHR, |values: &[Value<'_>]| {
                debug_assert_eq!(values.len(), 2);
                values[0].try_shr(&values[1])
            });
            map.insert(apply::NAME_ADD_ASSIGN, |values: &[Value<'_>]| {
                debug_assert_eq!(values.len(), 2);
                values[0].try_add(&values[1]) // Add instead of AddAssign
            });
            map.insert(apply::NAME_SUB_ASSIGN, |values: &[Value<'_>]| {
                debug_assert_eq!(values.len(), 2);
                values[0].try_sub(&values[1]) // Sub instead of SubAssign
            });
            map.insert(apply::NAME_MUL_ASSIGN, |values: &[Value<'_>]| {
                debug_assert_eq!(values.len(), 2);
                values[0].try_mul(&values[1]) // Mul instead of MulAssign
            });
            map.insert(apply::NAME_DIV_ASSIGN, |values: &[Value<'_>]| {
                debug_assert_eq!(values.len(), 2);
                values[0].try_div(&values[1]) // Div instead of DivAssign
            });
            map.insert(apply::NAME_REM_ASSIGN, |values: &[Value<'_>]| {
                debug_assert_eq!(values.len(), 2);
                values[0].try_rem(&values[1]) // Rem instead of RemAssign
            });
            map.insert(apply::NAME_BIT_XOR_ASSIGN, |values: &[Value<'_>]| {
                debug_assert_eq!(values.len(), 2);
                values[0].try_bit_xor(&values[1]) // BitXor instead of BitXorAssign
            });
            map.insert(apply::NAME_BIT_AND_ASSIGN, |values: &[Value<'_>]| {
                debug_assert_eq!(values.len(), 2);
                values[0].try_bit_and(&values[1]) // BitAnd instead of BitAndAssign
            });
            map.insert(apply::NAME_BIT_OR_ASSIGN, |values: &[Value<'_>]| {
                debug_assert_eq!(values.len(), 2);
                values[0].try_bit_or(&values[1]) // BitOr instead of BitOrAssign
            });
            map.insert(apply::NAME_SHL_ASSIGN, |values: &[Value<'_>]| {
                debug_assert_eq!(values.len(), 2);
                values[0].try_shl(&values[1]) // Shl instead of ShlAssign
            });
            map.insert(apply::NAME_SHR_ASSIGN, |values: &[Value<'_>]| {
                debug_assert_eq!(values.len(), 2);
                values[0].try_shr(&values[1]) // Shr instead of ShrAssign
            });
            map.insert(apply::NAME_NOT, |values: &[Value<'_>]| {
                debug_assert_eq!(values.len(), 1);
                values[0].try_not()
            });
            map.insert(apply::NAME_NEG, |values: &[Value<'_>]| {
                debug_assert_eq!(values.len(), 1);
                values[0].try_neg()
            });

            // TODO: Deref

            map
        });

        let f = fmap.get(&abs_path).cloned()?;
        f(values).ok()
    }

    fn on_enter_scope(&mut self, scope: Scope) {
        self.inner.on_enter_scope(scope);
        self.scope_stack.push(scope.into_raw());
    }

    fn on_exit_scope(&mut self) {
        let raw_scope = self.scope_stack.pop().unwrap();
        let exit_scope = Scope::from_raw(raw_scope);
        self.inner.on_exit_scope(exit_scope);

        if let Some(raw_scope) = self.scope_stack.last() {
            let reenter_scope = Scope::from_raw(*raw_scope);
            self.inner.on_enter_scope(reenter_scope);
        }
    }
}

impl<'gcx, H: Host<'gcx>> Host<'gcx> for HostWrapper<'_, H> {
    fn find_type(&mut self, expr: &syn::Expr) -> TriResult<infer::Type<'gcx>, ()> {
        self.inner.find_type(expr)
    }

    fn find_fn(&mut self, name: StrPath, types: &[infer::Type<'gcx>]) -> Fn {
        self.inner.find_fn(name, types)
    }

    fn syn_path_to_value(&mut self, syn_path: SynPath) -> TriResult<Value<'gcx>, ()> {
        self.inner.syn_path_to_value(syn_path)
    }
}

impl<'gcx, H: Host<'gcx>> Scoping for HostWrapper<'_, H> {
    fn on_enter_scope(&mut self, scope: Scope) {
        <Self>::on_enter_scope(self, scope)
    }

    fn on_exit_scope(&mut self, _: Scope) {
        <Self>::on_exit_scope(self)
    }
}

// === Evaluator ===

#[derive(Debug)]
pub(crate) struct Evaluator<'gcx> {
    gcx: &'gcx GlobalCx<'gcx>,
    symbols: SymbolTable<Interned<'gcx, str>, Value<'gcx>>,
}

impl<'gcx> Evaluator<'gcx> {
    pub(crate) fn new(gcx: &'gcx GlobalCx<'gcx>) -> Self {
        Self {
            gcx,
            symbols: SymbolTable::default(),
        }
    }

    pub(crate) fn eval_expr<H: Host<'gcx>>(
        &mut self,
        host: &mut H,
        expr: &syn::Expr,
    ) -> TriResult<Value<'gcx>, ()> {
        self.symbols.clear();

        let mut cx = EvalCx {
            gcx: self.gcx,
            symbols: &mut self.symbols,
            host: HostWrapper::new(host),
        };

        cx.eval_expr(expr).map(|ex| ex.value)
    }
}

struct EvalCx<'a, 'gcx, H> {
    gcx: &'gcx GlobalCx<'gcx>,
    symbols: &'a mut SymbolTable<Interned<'gcx, str>, Value<'gcx>>,
    host: HostWrapper<'a, H>,
}

impl<'a, 'gcx, H: Host<'gcx>> EvalCx<'a, 'gcx, H> {
    fn eval_expr(&mut self, expr: &syn::Expr) -> TriResult<ValueWithCtrl<'gcx>, ()> {
        match expr {
            syn::Expr::Array(v) => self.eval_expr_array(v).map(ValueWithCtrl::from),
            syn::Expr::Assign(v) => self.eval_expr_assign(v).map(ValueWithCtrl::from),
            syn::Expr::Async(_) => panic!("`async` is not supported"),
            syn::Expr::Await(_) => panic!("`await` is not supported"),
            syn::Expr::Binary(v) => self.eval_expr_binary(v).map(ValueWithCtrl::from),
            syn::Expr::Block(v) => self.eval_block(&v.block).map(ValueWithCtrl::from),
            syn::Expr::Break(v) => todo!("{v:#?}"),
            syn::Expr::Call(v) => self.eval_expr_call(v).map(ValueWithCtrl::from),
            syn::Expr::Cast(v) => self.eval_expr(&v.expr),
            syn::Expr::Closure(_) => panic!("`closure` is not supported"),
            syn::Expr::Const(v) => self.eval_block(&v.block).map(ValueWithCtrl::from),
            syn::Expr::Let(v) => todo!("{v:#?}"),
            syn::Expr::Lit(v) => self.eval_lit(&v.lit, expr).map(ValueWithCtrl::from),
            syn::Expr::Loop(v) => todo!("{v:#?}"),
            syn::Expr::Macro(_) => Ok(Value::Unit.into()),
            syn::Expr::Match(v) => todo!("{v:#?}"),
            syn::Expr::MethodCall(v) => todo!("{v:#?}"),
            syn::Expr::Paren(v) => self.eval_expr_paren(v),
            syn::Expr::Path(v) => self.eval_expr_path(v).map(ValueWithCtrl::from),
            syn::Expr::Range(v) => todo!("{v:#?}"),
            syn::Expr::RawAddr(v) => todo!("{v:#?}"),
            syn::Expr::Reference(v) => todo!("{v:#?}"),
            syn::Expr::Repeat(v) => todo!("{v:#?}"),
            syn::Expr::Return(v) => todo!("{v:#?}"),
            syn::Expr::Struct(v) => self.eval_expr_struct(v).map(ValueWithCtrl::from),
            syn::Expr::Try(v) => todo!("{v:#?}"),
            syn::Expr::TryBlock(v) => todo!("{v:#?}"),
            syn::Expr::Tuple(v) => todo!("{v:#?}"),
            syn::Expr::Unary(v) => self.eval_expr_unary(v).map(ValueWithCtrl::from),
            syn::Expr::Unsafe(v) => todo!("{v:#?}"),
            syn::Expr::Verbatim(v) => todo!("{v:#?}"),
            syn::Expr::While(v) => todo!("{v:#?}"),
            syn::Expr::Yield(v) => todo!("{v:#?}"),
            _ => todo!(),
        }
    }

    fn eval_expr_array(&mut self, expr_arr: &syn::ExprArray) -> TriResult<Value<'gcx>, ()> {
        let fields = expr_arr
            .elems
            .iter()
            .enumerate()
            .map(|(i, elem)| {
                self.eval_expr(elem).map(|ex| Field {
                    name: self.gcx.intern_str(&i.to_string()),
                    value: ex.value,
                })
            })
            .collect::<TriResult<Vec<Field<'gcx>>, ()>>()?;
        Ok(Value::Composed(fields))
    }

    fn eval_expr_assign(&mut self, expr_assign: &syn::ExprAssign) -> TriResult<Value<'gcx>, ()> {
        let rv = self.eval_expr(&expr_assign.right)?.value;
        self.update_symbol_by_expr(&expr_assign.left, rv);
        Ok(Value::Unit)
    }

    fn eval_expr_binary(&mut self, expr_bin: &syn::ExprBinary) -> TriResult<Value<'gcx>, ()> {
        use known::apply::*;

        return match expr_bin.op {
            syn::BinOp::Add(_) => bin(self, expr_bin, NAME_ADD),
            syn::BinOp::Sub(_) => bin(self, expr_bin, NAME_SUB),
            syn::BinOp::Mul(_) => bin(self, expr_bin, NAME_MUL),
            syn::BinOp::Div(_) => bin(self, expr_bin, NAME_DIV),
            syn::BinOp::Rem(_) => bin(self, expr_bin, NAME_REM),
            syn::BinOp::BitXor(_) => bin(self, expr_bin, NAME_BIT_XOR),
            syn::BinOp::BitAnd(_) => bin(self, expr_bin, NAME_BIT_AND),
            syn::BinOp::BitOr(_) => bin(self, expr_bin, NAME_BIT_OR),
            syn::BinOp::Shl(_) => bin(self, expr_bin, NAME_SHL),
            syn::BinOp::Shr(_) => bin(self, expr_bin, NAME_SHR),
            syn::BinOp::AddAssign(_) => bin_assign(self, expr_bin, NAME_ADD_ASSIGN),
            syn::BinOp::SubAssign(_) => bin_assign(self, expr_bin, NAME_SUB_ASSIGN),
            syn::BinOp::MulAssign(_) => bin_assign(self, expr_bin, NAME_MUL_ASSIGN),
            syn::BinOp::DivAssign(_) => bin_assign(self, expr_bin, NAME_DIV_ASSIGN),
            syn::BinOp::RemAssign(_) => bin_assign(self, expr_bin, NAME_REM_ASSIGN),
            syn::BinOp::BitXorAssign(_) => bin_assign(self, expr_bin, NAME_BIT_XOR_ASSIGN),
            syn::BinOp::BitAndAssign(_) => bin_assign(self, expr_bin, NAME_BIT_AND_ASSIGN),
            syn::BinOp::BitOrAssign(_) => bin_assign(self, expr_bin, NAME_BIT_OR_ASSIGN),
            syn::BinOp::ShlAssign(_) => bin_assign(self, expr_bin, NAME_SHL_ASSIGN),
            syn::BinOp::ShrAssign(_) => bin_assign(self, expr_bin, NAME_SHR_ASSIGN),
            _ => unreachable!(),
        };

        // === Internal helper functions ===

        fn bin<'gcx, H: Host<'gcx>>(
            this: &mut EvalCx<'_, 'gcx, H>,
            expr_bin: &syn::ExprBinary,
            name: &str,
        ) -> TriResult<Value<'gcx>, ()> {
            let lv = this.eval_expr(&expr_bin.left)?.value;
            let rv = this.eval_expr(&expr_bin.right)?.value;
            let values = [lv, rv];
            if let Some(res) = this.host.eval_known_fn(name, &values) {
                return Ok(res);
            }

            let lty = this.host.find_type(&expr_bin.left)?;
            let rty = this.host.find_type(&expr_bin.right)?;
            let f = this.host.find_fn(StrPath::absolute(name), &[lty, rty]);
            this.apply_to_fn(f, &values)
        }

        fn bin_assign<'gcx, H: Host<'gcx>>(
            this: &mut EvalCx<'_, 'gcx, H>,
            expr_bin: &syn::ExprBinary,
            name: &str,
        ) -> TriResult<Value<'gcx>, ()> {
            let value = bin(this, expr_bin, name)?;
            this.update_symbol_by_expr(&expr_bin.left, value);
            Ok(Value::Unit)
        }
    }

    fn eval_expr_call(&mut self, expr_call: &syn::ExprCall) -> TriResult<Value<'gcx>, ()> {
        let args = expr_call
            .args
            .iter()
            .map(|arg| self.eval_expr(arg).map(|ex| ex.value))
            .collect::<TriResult<Vec<_>, ()>>()?;

        match self.eval_expr(&expr_call.func)?.value {
            // Ordinary function call
            Value::Fn(f) => self.apply_to_fn(f, &args),
            // Constructor
            Value::Composed(fields) => {
                let field_names = fields.into_iter().map(|field| field.name);
                let value = self.apply_to_constructor(field_names, &args);
                Ok(value)
            }
            _ => unreachable!(),
        }
    }

    fn eval_expr_paren(
        &mut self,
        expr_paren: &syn::ExprParen,
    ) -> TriResult<ValueWithCtrl<'gcx>, ()> {
        self.eval_expr(&expr_paren.expr)
    }

    fn eval_expr_path(&mut self, expr_path: &syn::ExprPath) -> TriResult<Value<'gcx>, ()> {
        if expr_path.qself.is_none() {
            if let Some(ident) = expr_path.path.get_ident() {
                if let Some(value) = self.symbols.get(&*ident.to_string()) {
                    return Ok(value.clone());
                }
            }
        }

        let syn_path = SynPath {
            kind: SynPathKind::Expr,
            qself: expr_path.qself.as_ref(),
            path: &expr_path.path,
        };
        self.host.syn_path_to_value(syn_path)
    }

    fn eval_expr_struct(&mut self, expr_struct: &syn::ExprStruct) -> TriResult<Value<'gcx>, ()> {
        let fields = expr_struct
            .fields
            .iter()
            .map(|field| self.eval_field_value(field))
            .collect::<TriResult<Vec<Field>, ()>>()?;
        Ok(Value::Composed(fields))
    }

    fn eval_expr_unary(&mut self, expr_unary: &syn::ExprUnary) -> TriResult<Value<'gcx>, ()> {
        use known::apply::*;

        let name = match expr_unary.op {
            syn::UnOp::Deref(_) => todo!(),
            syn::UnOp::Not(_) => NAME_NOT,
            syn::UnOp::Neg(_) => NAME_NEG,
            _ => unreachable!(),
        };

        let v = self.eval_expr(&expr_unary.expr)?.value;
        let values = [v];
        if let Some(res) = self.host.eval_known_fn(name, &values) {
            return Ok(res);
        }

        let ty = self.host.find_type(&expr_unary.expr)?;
        let f = self.host.find_fn(StrPath::absolute(name), &[ty]);
        self.apply_to_fn(f, &values)
    }

    fn eval_block(&mut self, block: &syn::Block) -> TriResult<Value<'gcx>, ()> {
        self.host.on_enter_scope(Scope::Block(block));
        self.symbols.push_transparent_block();

        let mut last_value = Value::Unit;
        for stmt in &block.stmts {
            let ValueWithCtrl {
                value, is_return, ..
            } = self.eval_stmt(stmt)?;
            last_value = value;
            if is_return {
                break;
            }
        }

        self.symbols.pop_block();
        self.host.on_exit_scope();
        Ok(last_value)
    }

    fn eval_stmt(&mut self, stmt: &syn::Stmt) -> TriResult<ValueWithCtrl<'gcx>, ()> {
        let value = match stmt {
            syn::Stmt::Local(v) => {
                self.eval_local(v)?;
                Value::Unit.into()
            }
            syn::Stmt::Item(_) => Value::Unit.into(),
            syn::Stmt::Expr(v, _) => self.eval_expr(v)?,
            syn::Stmt::Macro(_) => Value::Unit.into(),
        };
        Ok(value)
    }

    fn eval_local(&mut self, local: &syn::Local) -> TriResult<(), ()> {
        // Evaluates rhs first due to the shadowing.
        let rhs = local
            .init
            .as_ref()
            .map(|init| self.eval_expr(&init.expr).map(|ex| ex.value))
            .unwrap_or(Ok(Value::Unit))?;
        self.push_symbol_by_pat(&local.pat, rhs);
        Ok(())
    }

    fn eval_lit(&mut self, lit: &syn::Lit, expr: &syn::Expr) -> TriResult<Value<'gcx>, ()> {
        use infer::{Type, TypeScalar::*};

        let ty = self.host.find_type(expr)?;

        let value = match lit {
            syn::Lit::Int(v) => match ty {
                Type::Scalar(Int { .. }) => {
                    let v = v.base10_parse().unwrap();
                    Value::Scalar(Scalar::Int(v))
                }
                Type::Scalar(I8) => {
                    let v = v.base10_parse().unwrap();
                    Value::Scalar(Scalar::I8(v))
                }
                Type::Scalar(I16) => {
                    let v = v.base10_parse().unwrap();
                    Value::Scalar(Scalar::I16(v))
                }
                Type::Scalar(I32) => {
                    let v = v.base10_parse().unwrap();
                    Value::Scalar(Scalar::I32(v))
                }
                Type::Scalar(I64) => {
                    let v = v.base10_parse().unwrap();
                    Value::Scalar(Scalar::I64(v))
                }
                Type::Scalar(I128) => {
                    let v = v.base10_parse().unwrap();
                    Value::Scalar(Scalar::I128(v))
                }
                Type::Scalar(Isize) => {
                    let v = v.base10_parse().unwrap();
                    Value::Scalar(Scalar::Isize(v))
                }
                Type::Scalar(U8) => {
                    let v = v.base10_parse().unwrap();
                    Value::Scalar(Scalar::U8(v))
                }
                Type::Scalar(U16) => {
                    let v = v.base10_parse().unwrap();
                    Value::Scalar(Scalar::U16(v))
                }
                Type::Scalar(U32) => {
                    let v = v.base10_parse().unwrap();
                    Value::Scalar(Scalar::U32(v))
                }
                Type::Scalar(U64) => {
                    let v = v.base10_parse().unwrap();
                    Value::Scalar(Scalar::U64(v))
                }
                Type::Scalar(U128) => {
                    let v = v.base10_parse().unwrap();
                    Value::Scalar(Scalar::U128(v))
                }
                Type::Scalar(Usize) => {
                    let v = v.base10_parse().unwrap();
                    Value::Scalar(Scalar::Usize(v))
                }
                _ => panic!("An integer does not match with the given type: {ty:?}"),
            },
            syn::Lit::Float(v) => match ty {
                Type::Scalar(Float { .. }) => {
                    let v = v.base10_parse().unwrap();
                    Value::Scalar(Scalar::Float(v))
                }
                Type::Scalar(F32) => {
                    let v = v.base10_parse().unwrap();
                    Value::Scalar(Scalar::F32(v))
                }
                Type::Scalar(F64) => {
                    let v = v.base10_parse().unwrap();
                    Value::Scalar(Scalar::F64(v))
                }
                _ => panic!("A floating point does not match with the given type: {ty:?}"),
            },
            syn::Lit::Bool(v) => match ty {
                Type::Scalar(Bool) => {
                    let v = v.value();
                    Value::Scalar(Scalar::Bool(v))
                }
                _ => panic!("A boolean does not match with the given type: {ty:?}"),
            },
            _ => panic!("not supported yet"),
        };
        Ok(value)
    }

    fn eval_field_value(&mut self, field_value: &syn::FieldValue) -> TriResult<Field<'gcx>, ()> {
        let name = match &field_value.member {
            syn::Member::Named(ident) => ident.to_string(),
            syn::Member::Unnamed(i) => i.index.to_string(),
        };
        let value = self.eval_expr(&field_value.expr)?.value;
        Ok(Field {
            name: self.gcx.intern_str(&name),
            value,
        })
    }

    fn push_symbol_by_pat(&mut self, pat: &syn::Pat, value: Value<'gcx>) {
        match pat {
            syn::Pat::Ident(v) => {
                let name = self.gcx.intern_str(&v.ident.to_string());
                self.symbols.push(name, value);
            }
            syn::Pat::Type(v) => self.push_symbol_by_pat(&v.pat, value),
            o => todo!("{o:#?}"),
        }
    }

    fn update_symbol_by_expr(&mut self, lhs: &syn::Expr, rhs: Value<'gcx>) {
        match lhs {
            syn::Expr::Path(v) => self.update_symbol_by_expr_path(v, rhs),
            o => todo!("{o:?}"),
        }
    }

    fn update_symbol_by_expr_path(&mut self, lhs: &syn::ExprPath, rhs: Value<'gcx>) {
        assert!(lhs.qself.is_none());

        let lhs = lhs.path.get_ident().unwrap();
        let name = lhs.to_string();
        let value = self.symbols.get_mut(&*name).unwrap();
        *value = rhs;
    }

    /// Applies the given values to the function.
    fn apply_to_fn(&mut self, f: Fn, args: &[Value<'gcx>]) -> TriResult<Value<'gcx>, ()> {
        self.symbols.push_opaque_block();

        match f.inputs {
            FnInputs::Params(inputs) => {
                debug_assert_eq!(inputs.len(), args.len());
                for (arg, value) in inputs.iter().cloned().zip(args) {
                    let arg = unsafe { arg.as_ref().unwrap() };
                    match arg {
                        syn::FnArg::Receiver(_) => todo!(),
                        syn::FnArg::Typed(v) => self.push_symbol_by_pat(&v.pat, value.clone()),
                    }
                }
            }
        }

        let value = match f.body {
            FnBody::Block(block) => {
                let block = unsafe { block.as_ref().unwrap() };
                self.eval_block(block)
            }
        };

        self.symbols.pop_block();
        value
    }

    fn apply_to_constructor<I>(&mut self, mut field_names: I, args: &[Value<'gcx>]) -> Value<'gcx>
    where
        I: Iterator<Item = Interned<'gcx, str>>,
    {
        let mut fields = Vec::new();
        let mut args = args.iter();

        while let (Some(field_name), Some(arg)) = (field_names.next(), args.next()) {
            fields.push(Field {
                name: field_name,
                value: arg.clone(),
            });
        }

        assert!(field_names.next().is_none());
        assert!(args.next().is_none());

        Value::Composed(fields)
    }
}

#[derive(Debug, Default)]
pub struct Evaluated<'gcx> {
    /// Evaluated values that mapped to an expression or a path item.
    mapped_values: Vec<Value<'gcx>>,

    /// Mapping between an expression and an index to [`Self::mapped_values`].
    ptr_map: Map<*const syn::Expr, usize>,

    /// Mapping between an path item and an index to [`Self::mapped_values`].
    pid_map: Map<PathId, usize>,
}

impl<'gcx> Evaluated<'gcx> {
    pub(crate) fn new() -> Self {
        Self {
            mapped_values: Vec::new(),
            ptr_map: Map::default(),
            pid_map: Map::default(),
        }
    }

    pub fn get_mapped_value_by_expr_ptr(&self, ptr: *const syn::Expr) -> Option<&Value<'gcx>> {
        self.ptr_map
            .get(&ptr)
            .map(|index| &self.mapped_values[*index])
    }

    pub fn get_mapped_value_by_path_id(&self, pid: PathId) -> Option<&Value<'gcx>> {
        self.pid_map
            .get(&pid)
            .map(|index| &self.mapped_values[*index])
    }

    pub(crate) fn get_value_by_expr(&self, expr: &syn::Expr) -> Option<&Value<'gcx>> {
        self.get_mapped_value_by_expr_ptr(expr)
    }

    /// Inserts an expression pointer and its evaluated value.
    ///
    /// You can find the value using the expression pointer later.
    pub(crate) fn insert_mapped_value(
        &mut self,
        ptr: *const syn::Expr,
        value: Value<'gcx>,
    ) -> Option<Value<'gcx>> {
        match self.ptr_map.entry(ptr) {
            Entry::Occupied(entry) => {
                let index = *entry.get();
                let old_value = mem::replace(&mut self.mapped_values[index], value);
                Some(old_value)
            }
            Entry::Vacant(entry) => {
                self.mapped_values.push(value);
                entry.insert(self.mapped_values.len() - 1);
                None
            }
        }
    }

    /// Inserts an expression pointer with path id and its evaluated value.
    ///
    /// You can find the value using the expression pointer or path id later.
    pub(crate) fn insert_mapped_value2(
        &mut self,
        ptr: *const syn::Expr,
        pid: PathId,
        value: Value<'gcx>,
    ) -> Option<Value<'gcx>> {
        match (self.ptr_map.entry(ptr), self.pid_map.entry(pid)) {
            (Entry::Occupied(ptr_entry), Entry::Occupied(pid_entry)) => {
                debug_assert_eq!(ptr_entry.get(), pid_entry.get());
                let index = *ptr_entry.get();
                let old_value = mem::replace(&mut self.mapped_values[index], value);
                Some(old_value)
            }
            (Entry::Occupied(ptr_entry), Entry::Vacant(pid_entry)) => {
                let index = *ptr_entry.get();
                pid_entry.insert(index);
                let old_value = mem::replace(&mut self.mapped_values[index], value);
                Some(old_value)
            }
            (Entry::Vacant(ptr_entry), Entry::Occupied(pid_entry)) => {
                let index = *pid_entry.get();
                ptr_entry.insert(index);
                let old_value = mem::replace(&mut self.mapped_values[index], value);
                Some(old_value)
            }
            (Entry::Vacant(ptr_entry), Entry::Vacant(pid_entry)) => {
                self.mapped_values.push(value);
                ptr_entry.insert(self.mapped_values.len() - 1);
                pid_entry.insert(self.mapped_values.len() - 1);
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Evaluator, Host};
    use crate::{
        etc::syn::SynPath,
        semantic::{
            basic_traits::EvaluateArrayLength,
            entry::GlobalCx,
            eval::{
                test_help::TestEvalHost,
                value::{Fn, Scalar, Value},
            },
            infer::{
                self,
                test_help::{test_inferer, TestInferLogicHost},
                Inferer,
            },
            logic::{self, test_help::test_logic, Logic},
        },
        Intern, Result, TriResult, TriResultHelper,
    };
    use logic_eval_util::str::StrPath;
    use syn_locator::{Find, LocateEntry};

    fn parse(code: &str) -> syn::Expr {
        syn_locator::enable_thread_local(true);
        syn_locator::clear();

        let expr: syn::Expr = syn::parse_str(code).unwrap();
        let pinned = std::pin::Pin::new(&expr);
        pinned.locate_as_entry("mod.rs", code).unwrap();
        expr
    }

    #[test]
    fn test_eval_operators() {
        fn eval<'gcx, H: infer::Host<'gcx> + logic::Host<'gcx>>(
            inferer: &mut Inferer<'gcx>,
            evaluator: &mut Evaluator<'gcx>,
            logic: &mut Logic<'gcx>,
            infer_logic_host: &mut H,
            expr: &syn::Expr,
        ) -> Result<Value<'gcx>> {
            inferer
                .infer_expr(logic, infer_logic_host, expr, None)
                .elevate_err()?;
            let mut eval_host = TestEvalHost::new(inferer);
            evaluator.eval_expr(&mut eval_host, expr).elevate_err()
        }

        let gcx = GlobalCx::default();
        let mut inferer = test_inferer(&gcx);
        let mut evaluator = Evaluator::new(&gcx);
        let mut logic = test_logic(&gcx);
        let mut infer_logic_host = TestInferLogicHost::new(&gcx);

        // Add
        let expr = parse("{ 1 + 2 }");
        let value = eval(
            &mut inferer,
            &mut evaluator,
            &mut logic,
            &mut infer_logic_host,
            &expr,
        )
        .unwrap();
        assert_eq!(value, Value::Scalar(Scalar::Int(1 + 2)));

        // Sub
        let expr = parse("{ 3 - 2 }");
        let value = eval(
            &mut inferer,
            &mut evaluator,
            &mut logic,
            &mut infer_logic_host,
            &expr,
        )
        .unwrap();
        assert_eq!(value, Value::Scalar(Scalar::Int(3 - 2)));

        // Mul
        let expr = parse("{ 2 * 3 }");
        let value = eval(
            &mut inferer,
            &mut evaluator,
            &mut logic,
            &mut infer_logic_host,
            &expr,
        )
        .unwrap();
        assert_eq!(value, Value::Scalar(Scalar::Int(2 * 3)));

        // Div
        let expr = parse("{ 6 / 3 }");
        let value = eval(
            &mut inferer,
            &mut evaluator,
            &mut logic,
            &mut infer_logic_host,
            &expr,
        )
        .unwrap();
        assert_eq!(value, Value::Scalar(Scalar::Int(6 / 3)));

        // Rem
        let expr = parse("{ 3 % 2 }");
        let value = eval(
            &mut inferer,
            &mut evaluator,
            &mut logic,
            &mut infer_logic_host,
            &expr,
        )
        .unwrap();
        assert_eq!(value, Value::Scalar(Scalar::Int(3 % 2)));

        // BitXor
        let expr = parse("{ 1 ^ 2 }");
        let value = eval(
            &mut inferer,
            &mut evaluator,
            &mut logic,
            &mut infer_logic_host,
            &expr,
        )
        .unwrap();
        assert_eq!(value, Value::Scalar(Scalar::Int(1 ^ 2)));

        // BitAnd
        let expr = parse("{ 1 & 2 }");
        let value = eval(
            &mut inferer,
            &mut evaluator,
            &mut logic,
            &mut infer_logic_host,
            &expr,
        )
        .unwrap();
        assert_eq!(value, Value::Scalar(Scalar::Int(1 & 2)));

        // BitOr
        let expr = parse("{ 1 | 2 }");
        let value = eval(
            &mut inferer,
            &mut evaluator,
            &mut logic,
            &mut infer_logic_host,
            &expr,
        )
        .unwrap();
        assert_eq!(value, Value::Scalar(Scalar::Int(1 | 2)));

        // Shl
        let expr = parse("{ 1 << 2 }");
        let value = eval(
            &mut inferer,
            &mut evaluator,
            &mut logic,
            &mut infer_logic_host,
            &expr,
        )
        .unwrap();
        assert_eq!(value, Value::Scalar(Scalar::Int(1 << 2)));

        // Shr
        let expr = parse("{ 4 >> 2 }");
        let value = eval(
            &mut inferer,
            &mut evaluator,
            &mut logic,
            &mut infer_logic_host,
            &expr,
        )
        .unwrap();
        assert_eq!(value, Value::Scalar(Scalar::Int(4 >> 2)));

        // AddAssign
        let expr = parse("{ let mut a = 1; a += 2; a }");
        let value = eval(
            &mut inferer,
            &mut evaluator,
            &mut logic,
            &mut infer_logic_host,
            &expr,
        )
        .unwrap();
        assert_eq!(value, Value::Scalar(Scalar::Int(1 + 2)));

        // SubAssign
        let expr = parse("{ let mut a = 3; a -= 2; a }");
        let value = eval(
            &mut inferer,
            &mut evaluator,
            &mut logic,
            &mut infer_logic_host,
            &expr,
        )
        .unwrap();
        assert_eq!(value, Value::Scalar(Scalar::Int(3 - 2)));

        // MulAssign
        let expr = parse("{ let mut a = 2; a *= 3; a }");
        let value = eval(
            &mut inferer,
            &mut evaluator,
            &mut logic,
            &mut infer_logic_host,
            &expr,
        )
        .unwrap();
        assert_eq!(value, Value::Scalar(Scalar::Int(2 * 3)));

        // DivAssign
        let expr = parse("{ let mut a = 6; a /= 3; a }");
        let value = eval(
            &mut inferer,
            &mut evaluator,
            &mut logic,
            &mut infer_logic_host,
            &expr,
        )
        .unwrap();
        assert_eq!(value, Value::Scalar(Scalar::Int(6 / 3)));

        // RemAssign
        let expr = parse("{ let mut a = 3; a %= 2; a }");
        let value = eval(
            &mut inferer,
            &mut evaluator,
            &mut logic,
            &mut infer_logic_host,
            &expr,
        )
        .unwrap();
        assert_eq!(value, Value::Scalar(Scalar::Int(3 % 2)));

        // BitXorAssign
        let expr = parse("{ let mut a = 1; a ^= 2; a }");
        let value = eval(
            &mut inferer,
            &mut evaluator,
            &mut logic,
            &mut infer_logic_host,
            &expr,
        )
        .unwrap();
        assert_eq!(value, Value::Scalar(Scalar::Int(1 ^ 2)));

        // BitAndAssign
        let expr = parse("{ let mut a = 1; a &= 2; a }");
        let value = eval(
            &mut inferer,
            &mut evaluator,
            &mut logic,
            &mut infer_logic_host,
            &expr,
        )
        .unwrap();
        assert_eq!(value, Value::Scalar(Scalar::Int(1 & 2)));

        // BitOrAssign
        let expr = parse("{ let mut a = 1; a |= 2; a }");
        let value = eval(
            &mut inferer,
            &mut evaluator,
            &mut logic,
            &mut infer_logic_host,
            &expr,
        )
        .unwrap();
        assert_eq!(value, Value::Scalar(Scalar::Int(1 | 2)));

        // ShlAssign
        let expr = parse("{ let mut a = 1; a <<= 2; a }");
        let value = eval(
            &mut inferer,
            &mut evaluator,
            &mut logic,
            &mut infer_logic_host,
            &expr,
        )
        .unwrap();
        assert_eq!(value, Value::Scalar(Scalar::Int(1 << 2)));

        // ShrAssign
        let expr = parse("{ let mut a = 4; a >>= 2; a }");
        let value = eval(
            &mut inferer,
            &mut evaluator,
            &mut logic,
            &mut infer_logic_host,
            &expr,
        )
        .unwrap();
        assert_eq!(value, Value::Scalar(Scalar::Int(4 >> 2)));

        // Not
        let expr = parse("{ !false }");
        let value = eval(
            &mut inferer,
            &mut evaluator,
            &mut logic,
            &mut infer_logic_host,
            &expr,
        )
        .unwrap();
        assert_eq!(value, Value::Scalar(Scalar::Bool(true)));

        // Neg
        let expr = parse("{ let mut a = 1; a = -a; a }");
        let value = eval(
            &mut inferer,
            &mut evaluator,
            &mut logic,
            &mut infer_logic_host,
            &expr,
        )
        .unwrap();
        assert_eq!(value, Value::Scalar(Scalar::Int(-1)));

        // Operator priority (syn crate is already giving us syntax tree
        // concerning this priority)
        let expr = parse("{ 1 + 2 * 3 + 4 * 5 }");
        let value = eval(
            &mut inferer,
            &mut evaluator,
            &mut logic,
            &mut infer_logic_host,
            &expr,
        )
        .unwrap();
        assert_eq!(value, Value::Scalar(Scalar::Int(1 + 2 * 3 + 4 * 5)));
        let expr = parse("{ (1 + 2) * 3 + 4 * 5 }");
        let value = eval(
            &mut inferer,
            &mut evaluator,
            &mut logic,
            &mut infer_logic_host,
            &expr,
        )
        .unwrap();
        assert_eq!(value, Value::Scalar(Scalar::Int((1 + 2) * 3 + 4 * 5)));
    }

    #[test]
    fn test_eval_function_call() {
        let code = r#"{
            fn f(x: i32) -> i32 { x * 2 }
            f(3)
        }"#;

        struct TestEvalHost<'a, 'gcx> {
            inferer: &'a mut Inferer<'gcx>,
            expr: &'a syn::Expr,
        }

        impl<'gcx> Host<'gcx> for TestEvalHost<'_, 'gcx> {
            fn find_type(&mut self, expr: &syn::Expr) -> TriResult<infer::Type<'gcx>, ()> {
                let ty = self.inferer.get_type(expr).unwrap().clone();
                Ok(ty)
            }

            fn find_fn(&mut self, _: StrPath, _: &[infer::Type<'gcx>]) -> Fn {
                panic!()
            }

            fn syn_path_to_value(&mut self, path: SynPath) -> TriResult<Value<'gcx>, ()> {
                let ident = path.path.get_ident().unwrap().to_string();
                if ident == "f" {
                    let code = "fn f(x: i32) -> i32 { x * 2 }";
                    let f: &syn::ItemFn = self.expr.find(code).unwrap();
                    let f = Fn::from_signature_and_block(&f.sig, &f.block);
                    Ok(Value::Fn(f))
                } else {
                    unreachable!()
                }
            }
        }

        crate::impl_empty_scoping!(TestEvalHost<'_, '_>);

        struct TestInferHost<'gcx> {
            gcx: &'gcx GlobalCx<'gcx>,
        }

        impl<'gcx> infer::Host<'gcx> for TestInferHost<'gcx> {
            fn syn_path_to_type(
                &mut self,
                _: SynPath,
                types: &mut infer::UniqueTypes,
            ) -> TriResult<infer::Type<'gcx>, ()> {
                use infer::{Param, Type, TypeScalar};

                let tid_i32 = types.insert_type(Type::Scalar(TypeScalar::I32));

                let res = infer::Type::Named(infer::TypeNamed {
                    name: self.gcx.intern_str("f"),
                    params: [
                        Param::Other {
                            name: self.gcx.intern_str("0"),
                            tid: tid_i32,
                        },
                        Param::Other {
                            name: self.gcx.intern_str("1"),
                            tid: tid_i32,
                        },
                    ]
                    .into(),
                });
                Ok(res)
            }
        }

        impl<'gcx> EvaluateArrayLength<'gcx> for TestInferHost<'gcx> {
            fn eval_array_len(&mut self, _: &syn::Expr) -> TriResult<crate::ArrayLen, ()> {
                unreachable!()
            }
        }

        crate::impl_empty_scoping!(TestInferHost<'_>);
        crate::impl_empty_method_host!(TestInferHost<'_>);

        let gcx = GlobalCx::default();
        let mut inferer = test_inferer(&gcx);
        let mut evaluator = Evaluator::new(&gcx);
        let mut logic = test_logic(&gcx);
        let mut infer_logic_host = TestInferLogicHost::new(&gcx);
        infer_logic_host.override_infer_host(TestInferHost { gcx: &gcx });

        let expr = parse(code);
        inferer
            .infer_expr(&mut logic, &mut infer_logic_host, &expr, None)
            .unwrap();
        let mut eval_host = TestEvalHost {
            inferer: &mut inferer,
            expr: &expr,
        };
        let value = evaluator.eval_expr(&mut eval_host, &expr).unwrap();

        assert_eq!(value, Value::Scalar(Scalar::I32(3 * 2)));
    }
}
