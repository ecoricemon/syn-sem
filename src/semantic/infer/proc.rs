// TODO: Support generic types
// TODO: Deref coercions like &[T; N] -> &[T]

use super::ty::{
    InferArrayLen, Param, Type, TypeArray, TypeComposed, TypeId, TypeMut, TypeNamed, TypeRef,
    TypeScalar, TypeTuple, UniqueTypes,
};
use crate::{
    ds::vec::BoxedSlice,
    err,
    etc::{
        known,
        syn::{SynPath, SynPathKind},
        util::FiniteLoop,
    },
    semantic::{
        basic_traits::{EvaluateArrayLength, RawScope, Scope, Scoping},
        entry::GlobalCx,
        logic::{self, find_method, find_ty::TypeFinder, term, var_name, Logic},
        tree,
    },
    Intern, Map, NameIn, Set, TermIn, TriError, TriResult,
};
use any_intern::Interned;
use indexmap::IndexMap;
use logic_eval::{Name, Term, VAR_PREFIX};
use logic_eval_util::{str::StrPath, symbol::SymbolTable};
use proc_macro2::TokenStream as TokenStream2;
use std::{collections::VecDeque, fmt, hash::Hash, iter};
use syn_locator::Locate;

pub(crate) trait Host<'gcx>:
    find_method::Host<'gcx> + Scoping + EvaluateArrayLength<'gcx>
{
    fn syn_path_to_type(
        &mut self,
        syn_path: SynPath,
        types: &mut UniqueTypes<'gcx>,
    ) -> TriResult<Type<'gcx>, ()>;
}

struct HostWrapper<'a, H> {
    inner: &'a mut H,
    scope_stack: Vec<RawScope>,
}

impl<'a, 'gcx, H: Host<'gcx> + logic::Host<'gcx>> HostWrapper<'a, H> {
    fn new(host: &'a mut H) -> Self {
        Self {
            inner: host,
            scope_stack: Vec::new(),
        }
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

impl<'gcx, H: Host<'gcx> + logic::Host<'gcx>> Host<'gcx> for HostWrapper<'_, H> {
    fn syn_path_to_type(
        &mut self,
        syn_path: SynPath,
        types: &mut UniqueTypes<'gcx>,
    ) -> TriResult<Type<'gcx>, ()> {
        Host::syn_path_to_type(self.inner, syn_path, types)
    }
}

impl<'gcx, H: Host<'gcx> + logic::Host<'gcx>> logic::Host<'gcx> for HostWrapper<'_, H> {
    fn ident_to_npath(&mut self, ident: &syn::Ident) -> TriResult<String, ()> {
        logic::Host::ident_to_npath(self.inner, ident)
    }
}

impl<'gcx, H: Host<'gcx> + logic::Host<'gcx>> find_method::Host<'gcx> for HostWrapper<'_, H> {
    fn is_visible(&mut self, parent_path: &str, fn_ident: &str) -> TriResult<bool, ()> {
        find_method::Host::is_visible(self.inner, parent_path, fn_ident)
    }
}

impl<'gcx, H: Host<'gcx> + logic::Host<'gcx>> Scoping for HostWrapper<'_, H> {
    fn on_enter_scope(&mut self, scope: Scope) {
        <Self>::on_enter_scope(self, scope)
    }

    fn on_exit_scope(&mut self, _: Scope) {
        <Self>::on_exit_scope(self)
    }
}

impl<'gcx, H: Host<'gcx> + logic::Host<'gcx>> EvaluateArrayLength<'gcx> for HostWrapper<'_, H> {
    fn eval_array_len(&mut self, expr: &syn::Expr) -> TriResult<crate::ArrayLen, ()> {
        EvaluateArrayLength::eval_array_len(self.inner, expr)
    }
}

pub(crate) trait Inferable {}
impl Inferable for syn::Block {}
impl Inferable for syn::Expr {}
impl Inferable for syn::Ident {}
impl Inferable for syn::Pat {}

pub(crate) struct Inferer<'gcx> {
    gcx: &'gcx GlobalCx<'gcx>,
    pub(crate) types: UniqueTypes<'gcx>, // TODO: Remove pub
    /// Mapping from syn pointer to type id.
    ptr2tid: IndexMap<*const (), TypeId>,
    cons: VecDeque<Constraint>,
    cand_cons: Map<Type<'gcx>, Vec<Constraint>>,
    cand_id: u32,
    finish_cand: Set<u32>,
    symbols: SymbolTable<Interned<'gcx, str>, TypeId>,
}

impl<'gcx> Inferer<'gcx> {
    pub(crate) fn new(gcx: &'gcx GlobalCx<'gcx>) -> Self {
        Self {
            gcx,
            types: UniqueTypes::new(),
            ptr2tid: IndexMap::default(),
            cons: VecDeque::new(),
            cand_cons: Map::default(),
            cand_id: 0,
            finish_cand: Set::default(),
            symbols: SymbolTable::default(),
        }
    }

    /// This method may infer an incomplete type from the given expression.
    ///
    /// For example, [`TypeScalar::Int`] can be inferred instead of [`TypeScalar::Usize`] due to
    /// lack of context around the expression. Use [`Inferer::infer_block`] instead in that case.
    pub(crate) fn infer_expr<H: Host<'gcx> + logic::Host<'gcx>>(
        &mut self,
        logic: &mut Logic<'gcx>,
        infer_logic_host: &mut H,
        expr: &syn::Expr,
        type_hint: Option<Type<'gcx>>,
    ) -> TriResult<(), ()> {
        if self.get_type(expr).is_some() {
            return Ok(());
        }

        let old_state = self.state();

        let mut cx = InferCx {
            gcx: self.gcx,
            inner: self,
            logic,
            infer_logic_host: HostWrapper::new(infer_logic_host),
        };
        if let Some(type_hint) = type_hint {
            cx.solve_expr_with_type_hint(expr, type_hint)?;
        } else {
            cx.solve_expr(expr)?;
        }
        let res = cx.unify();

        if self.contains_invalid_type(old_state) {
            self.revert(old_state);
            return err!(soft, ());
        }
        res
    }

    /// Call this method on a top block.
    pub(crate) fn infer_block<H: Host<'gcx> + logic::Host<'gcx>>(
        &mut self,
        logic: &mut Logic<'gcx>,
        infer_logic_host: &mut H,
        block: &syn::Block,
        type_hint: Option<Type<'gcx>>,
    ) -> TriResult<(), ()> {
        if self.get_type(block).is_some() {
            return Ok(());
        }

        let old_state = self.state();

        let mut cx = InferCx {
            gcx: self.gcx,
            inner: self,
            logic,
            infer_logic_host: HostWrapper::new(infer_logic_host),
        };
        if let Some(type_hint) = type_hint {
            cx.solve_block_with_type_hint(block, type_hint)?;
        } else {
            cx.solve_block(block)?;
        }
        let res = cx.unify();

        if self.contains_invalid_type(old_state) {
            self.revert(old_state);
            return err!(soft, ());
        }
        res
    }

    pub(crate) fn infer_signature_and_block<H: Host<'gcx> + logic::Host<'gcx>>(
        &mut self,
        logic: &mut Logic<'gcx>,
        infer_logic_host: &mut H,
        sig: &syn::Signature,
        block: &syn::Block,
    ) -> TriResult<(), ()> {
        if self.get_type(block).is_some() {
            return Ok(());
        }

        let old_state = self.state();

        let mut cx = InferCx {
            gcx: self.gcx,
            inner: self,
            logic,
            infer_logic_host: HostWrapper::new(infer_logic_host),
        };
        cx.solve_signature_and_block(sig, block)?;
        let res = cx.unify();

        if self.contains_invalid_type(old_state) {
            self.revert(old_state);
            return err!(soft, ());
        }
        res
    }

    fn state(&self) -> (usize, usize) {
        (self.ptr2tid.len(), self.types.len())
    }

    fn revert(&mut self, state: (usize, usize)) {
        self.ptr2tid.truncate(state.0);
        // # Can we shrink type container to revert it back? evan it is a unique container?
        // - The only problem we need to consider before shrinking is the type replacement in the
        //   container because the replacement would break previously existing types. But, the
        //   previously existing types are complete so that they won't be destination of the
        //   replacement. In other words, they won't change.
        self.types.truncate(state.1);
    }

    /// # Unification fail cases
    /// - If unknown array lengths exist after unification
    fn contains_invalid_type(&self, state: (usize, usize)) -> bool {
        let old_len_types = state.1;
        self.types.iter().skip(old_len_types).any(|(_, ty)| {
            matches!(
                ty,
                Type::Array(TypeArray { len, .. }) if len == &InferArrayLen::Unknown
            )
        })
    }

    pub(crate) fn find_type(&self, tid: TypeId) -> &Type<'gcx> {
        self.types.find_type(tid)
    }

    /// Returns already inferred type about the given syntax node.
    pub(crate) fn get_type<T: Inferable + ?Sized>(&self, syn: &T) -> Option<&Type<'gcx>> {
        let ptr = syn as *const _ as *const ();
        let input_tid = self.ptr2tid.get(&ptr)?;
        let ty = self.types.find_type(*input_tid);
        Some(ty)
    }

    fn add_ptr_mapping<T: Inferable + ?Sized>(&mut self, syn: &T, tid: TypeId) {
        self.ptr2tid.insert(syn as *const _ as *const (), tid);
    }

    #[cfg(test)]
    fn get_owned_type_of_expr(&self, expr: &syn::Expr) -> Option<super::ty::OwnedType> {
        let ptr = expr as *const _ as *const ();
        self.ptr2tid
            .get(&ptr)
            .map(|tid| crate::GetOwned::get_owned(&self.types, *tid))
    }

    #[cfg(test)]
    fn get_owned_type_of_ident(&self, ident: &syn::Ident) -> Option<super::ty::OwnedType> {
        let ptr = ident as *const _ as *const ();
        self.ptr2tid
            .get(&ptr)
            .map(|tid| crate::GetOwned::get_owned(&self.types, *tid))
    }
}

impl fmt::Debug for Inferer<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Inferer")
            .field("types", &self.types)
            .field("cons", &self.cons)
            .field("cand_cons", &self.cand_cons)
            .field("symbol", &self.symbols)
            .finish_non_exhaustive()
    }
}

struct InferCx<'a, 'gcx, H> {
    gcx: &'gcx GlobalCx<'gcx>,
    inner: &'a mut Inferer<'gcx>,
    logic: &'a mut Logic<'gcx>,
    infer_logic_host: HostWrapper<'a, H>,
}

impl<'gcx, H: Host<'gcx> + logic::Host<'gcx>> InferCx<'_, 'gcx, H> {
    fn solve_block_with_type_hint(
        &mut self,
        block: &syn::Block,
        type_hint: Type<'gcx>,
    ) -> TriResult<TypeId, ()> {
        let tid = self.solve_block(block)?;

        let hint_tid = self.inner.types.insert_type(type_hint);
        self.inner.cons.push_back(Constraint::Equal {
            lhs: tid,
            rhs: hint_tid,
        });

        Ok(tid)
    }

    fn solve_block(&mut self, block: &syn::Block) -> TriResult<TypeId, ()> {
        let block_tid = self.inner.types.new_type_var();

        self.infer_logic_host.on_enter_scope(Scope::Block(block));
        self.inner.symbols.push_transparent_block();

        let mut last_tid = None;
        for stmt in &block.stmts {
            let TypeIdWithCtrl { tid, is_return } = self.solve_stmt(stmt)?;
            if is_return {
                self.inner.cons.push_back(Constraint::Equal {
                    lhs: tid,
                    rhs: block_tid,
                });
            }
            last_tid = Some(tid);
        }

        let last_tid = last_tid.unwrap_or_else(|| self.inner.types.insert_type(Type::Unit));
        self.inner.cons.push_back(Constraint::Equal {
            lhs: last_tid,
            rhs: block_tid,
        });

        self.inner.symbols.pop_block();
        self.infer_logic_host.on_exit_scope();

        self.inner.add_ptr_mapping(block, block_tid);
        Ok(block_tid)
    }

    fn solve_stmt(&mut self, stmt: &syn::Stmt) -> TriResult<TypeIdWithCtrl, ()> {
        let res = match stmt {
            syn::Stmt::Local(v) => {
                self.solve_local(v)?;
                TypeIdWithCtrl {
                    tid: self.inner.types.insert_type(Type::Unit),
                    is_return: false,
                }
            }
            syn::Stmt::Item(v) => {
                self.solve_item(v)?;
                TypeIdWithCtrl {
                    tid: self.inner.types.insert_type(Type::Unit),
                    is_return: false,
                }
            }
            syn::Stmt::Expr(v, _) => self.solve_expr(v)?,
            syn::Stmt::Macro(_) => TypeIdWithCtrl {
                tid: self.inner.types.insert_type(Type::Unit),
                is_return: false,
            },
        };
        Ok(res)
    }

    fn solve_local(&mut self, local: &syn::Local) -> TriResult<(), ()> {
        // Solves rhs first due to the shadowing.
        let rhs = local
            .init
            .as_ref()
            .map(|init| self.solve_expr(&init.expr).map(|ret| ret.tid))
            .transpose()?;
        let lhs = self.solve_pat(&local.pat)?;

        if let Some(rhs) = rhs {
            self.inner.cons.push_back(Constraint::Equal { lhs, rhs });
        }
        Ok(())
    }

    fn solve_item(&mut self, item: &syn::Item) -> TriResult<(), ()> {
        match item {
            syn::Item::Const(v) => self.solve_item_const(v)?,
            syn::Item::Fn(v) => self.solve_item_fn(v)?,
            syn::Item::Mod(v) => self.solve_item_mod(v),
            _ => {}
        };
        Ok(())
    }

    fn solve_item_const(&mut self, item_const: &syn::ItemConst) -> TriResult<(), ()> {
        self.solve_expr(&item_const.expr)?;
        Ok(())
    }

    fn solve_item_fn(&mut self, item_fn: &syn::ItemFn) -> TriResult<(), ()> {
        self.infer_logic_host.on_enter_scope(Scope::ItemFn(item_fn));
        self.inner.symbols.push_opaque_block();

        self.solve_signature_and_block(&item_fn.sig, &item_fn.block)?;

        self.inner.symbols.pop_block();
        self.infer_logic_host.on_exit_scope();
        Ok(())
    }

    fn solve_item_mod(&mut self, item_mod: &syn::ItemMod) {
        self.infer_logic_host.on_enter_scope(Scope::Mod(item_mod));
        self.inner.symbols.push_opaque_block();

        // It seems there's nothing to do.

        self.inner.symbols.pop_block();
        self.infer_logic_host.on_exit_scope();
    }

    fn solve_signature_and_block(
        &mut self,
        sig: &syn::Signature,
        block: &syn::Block,
    ) -> TriResult<(), ()> {
        self.inner.symbols.push_opaque_block();

        for input in &sig.inputs {
            match input {
                syn::FnArg::Receiver(_) => todo!(),
                syn::FnArg::Typed(v) => {
                    self.solve_pat_type(v)?; // Symbols will be added.
                }
            }
        }

        let output_tid = match &sig.output {
            syn::ReturnType::Default => None,
            syn::ReturnType::Type(_, v) => {
                let ty_ty = self.solve_type(v)?;
                let ty_tid = self.inner.types.insert_type(ty_ty);
                Some(ty_tid)
            }
        };

        let block_tid = self.solve_block(block)?;
        if let Some(output_tid) = output_tid {
            self.inner.cons.push_back(Constraint::Equal {
                lhs: block_tid,
                rhs: output_tid,
            });
        }

        self.inner.symbols.pop_block();
        Ok(())
    }

    fn solve_expr_with_type_hint(
        &mut self,
        expr: &syn::Expr,
        type_hint: Type<'gcx>,
    ) -> TriResult<TypeIdWithCtrl, ()> {
        let res = self.solve_expr(expr)?;

        let hint_tid = self.inner.types.insert_type(type_hint);
        self.inner.cons.push_back(Constraint::Equal {
            lhs: res.tid,
            rhs: hint_tid,
        });

        Ok(res)
    }

    fn solve_expr_array_length(&mut self, expr: &syn::Expr) -> TriResult<InferArrayLen, ()> {
        let expr_tid = self.solve_expr(expr)?.tid;
        let usize_tid = self
            .inner
            .types
            .insert_type(Type::Scalar(TypeScalar::Usize));
        self.inner.cons.push_back(Constraint::Equal {
            lhs: expr_tid,
            rhs: usize_tid,
        });

        match self.infer_logic_host.eval_array_len(expr) {
            Ok(tree::ArrayLen::Fixed(n)) => Ok(InferArrayLen::Fixed(n)),
            Ok(tree::ArrayLen::Dynamic) => Ok(InferArrayLen::Dynamic),
            Ok(tree::ArrayLen::Generic) => Ok(InferArrayLen::Generic),
            Err(TriError::Soft(())) => Ok(InferArrayLen::Unknown),
            Err(TriError::Hard(e)) => Err(TriError::Hard(e)),
        }
    }

    fn solve_expr(&mut self, expr: &syn::Expr) -> TriResult<TypeIdWithCtrl, ()> {
        let outer = self.inner.types.new_type_var();

        let mut is_return = false;

        let inner = match expr {
            syn::Expr::Array(v) => self.solve_expr_array(v)?,
            syn::Expr::Assign(v) => self.solve_expr_assign(v)?,
            syn::Expr::Binary(v) => self.solve_expr_binary(v)?,
            syn::Expr::Block(v) => self.solve_expr_block(v)?,
            syn::Expr::Call(v) => self.solve_expr_call(v)?,
            syn::Expr::Cast(v) => self.solve_expr_cast(v)?,
            syn::Expr::Lit(v) => self.solve_expr_lit(v, outer),
            syn::Expr::Paren(v) => {
                let TypeIdWithCtrl { tid, is_return: r } = self.solve_expr_paren(v)?;
                is_return = r;
                tid
            }
            syn::Expr::Path(v) => self.solve_expr_path(v)?,
            syn::Expr::Reference(v) => self.solve_expr_reference(v)?,
            syn::Expr::Repeat(v) => self.solve_expr_repeat(v)?,
            syn::Expr::Return(v) => {
                is_return = true;
                self.solve_expr_return(v)?
            }
            syn::Expr::Struct(v) => self.solve_expr_struct(v)?,
            syn::Expr::Tuple(v) => self.solve_expr_tuple(v)?,
            syn::Expr::Unary(v) => self.solve_expr_unary(v)?,
            syn::Expr::Verbatim(v) => self.solve_expr_verbatim(v),
            o => todo!("{o:?}"),
        };

        self.inner.cons.push_back(Constraint::Equal {
            lhs: outer,
            rhs: inner,
        });

        self.inner.add_ptr_mapping(expr, outer);
        Ok(TypeIdWithCtrl {
            tid: outer,
            is_return,
        })
    }

    fn solve_expr_array(&mut self, expr_arr: &syn::ExprArray) -> TriResult<TypeId, ()> {
        let mut first = None;

        for elem in &expr_arr.elems {
            let elem = self.solve_expr(elem)?.tid;

            if let Some(first) = first {
                self.inner.cons.push_back(Constraint::Equal {
                    lhs: first,
                    rhs: elem,
                });
            } else {
                first = Some(elem);
            }
        }

        let elem = first.unwrap_or_else(|| self.inner.types.insert_type(Type::Unknown));
        let len = InferArrayLen::Fixed(expr_arr.elems.len());
        let tid = self
            .inner
            .types
            .insert_type(Type::Array(TypeArray { elem, len }));
        Ok(tid)
    }

    fn solve_expr_assign(&mut self, expr_assign: &syn::ExprAssign) -> TriResult<TypeId, ()> {
        let lhs = self.solve_expr(&expr_assign.left)?.tid;
        let rhs = self.solve_expr(&expr_assign.right)?.tid;

        self.inner.cons.push_back(Constraint::Equal { lhs, rhs });

        let tid = self.inner.types.insert_type(Type::Unit);
        Ok(tid)
    }

    fn solve_expr_binary(&mut self, expr_bin: &syn::ExprBinary) -> TriResult<TypeId, ()> {
        use known::apply;

        let l = self.solve_expr(&expr_bin.left)?.tid;
        let r = self.solve_expr(&expr_bin.right)?.tid;

        let tid = match expr_bin.op {
            syn::BinOp::Add(_) => bin(self, apply::NAME_ADD, l, r),
            syn::BinOp::Sub(_) => bin(self, apply::NAME_SUB, l, r),
            syn::BinOp::Mul(_) => bin(self, apply::NAME_MUL, l, r),
            syn::BinOp::Div(_) => bin(self, apply::NAME_DIV, l, r),
            syn::BinOp::Rem(_) => bin(self, apply::NAME_REM, l, r),
            syn::BinOp::BitXor(_) => bin(self, apply::NAME_BIT_XOR, l, r),
            syn::BinOp::BitAnd(_) => bin(self, apply::NAME_BIT_AND, l, r),
            syn::BinOp::BitOr(_) => bin(self, apply::NAME_BIT_OR, l, r),
            syn::BinOp::Shl(_) => bin(self, apply::NAME_SHL, l, r),
            syn::BinOp::Shr(_) => bin(self, apply::NAME_SHR, l, r),
            syn::BinOp::AddAssign(_) => bin_assign(self, apply::NAME_ADD_ASSIGN, l, r),
            syn::BinOp::SubAssign(_) => bin_assign(self, apply::NAME_SUB_ASSIGN, l, r),
            syn::BinOp::MulAssign(_) => bin_assign(self, apply::NAME_MUL_ASSIGN, l, r),
            syn::BinOp::DivAssign(_) => bin_assign(self, apply::NAME_DIV_ASSIGN, l, r),
            syn::BinOp::RemAssign(_) => bin_assign(self, apply::NAME_REM_ASSIGN, l, r),
            syn::BinOp::BitXorAssign(_) => bin_assign(self, apply::NAME_BIT_XOR_ASSIGN, l, r),
            syn::BinOp::BitAndAssign(_) => bin_assign(self, apply::NAME_BIT_AND_ASSIGN, l, r),
            syn::BinOp::BitOrAssign(_) => bin_assign(self, apply::NAME_BIT_OR_ASSIGN, l, r),
            // There's no constraints b/w input and output.
            syn::BinOp::ShlAssign(_) => self.inner.types.insert_type(Type::Unit),
            syn::BinOp::ShrAssign(_) => self.inner.types.insert_type(Type::Unit),
            _ => todo!(),
        };
        return Ok(tid);

        // === Internal helper functions ===

        fn bin<'gcx, H: Host<'gcx> + logic::Host<'gcx>>(
            this: &mut InferCx<'_, 'gcx, H>,
            name: &'static str,
            l: TypeId,
            r: TypeId,
        ) -> TypeId {
            let out = this.inner.types.new_type_var();

            this.inner.cons.push_back(Constraint::ApplyMethod {
                name: StrPath::absolute(name),
                params: [out, l, r].into(),
            });

            out
        }

        fn bin_assign<'gcx, H: Host<'gcx> + logic::Host<'gcx>>(
            this: &mut InferCx<'_, 'gcx, H>,
            name: &'static str,
            l: TypeId,
            r: TypeId,
        ) -> TypeId {
            let out = this.inner.types.insert_type(Type::Unit);

            this.inner.cons.push_back(Constraint::ApplyMethod {
                name: StrPath::absolute(name),
                params: [out, l, r].into(),
            });

            out
        }
    }

    fn solve_expr_block(&mut self, expr_block: &syn::ExprBlock) -> TriResult<TypeId, ()> {
        self.solve_block(&expr_block.block)
    }

    fn solve_expr_call(&mut self, expr_call: &syn::ExprCall) -> TriResult<TypeId, ()> {
        let target = self.solve_expr(&expr_call.func)?.tid;

        let out = self.inner.types.new_type_var();
        let args = expr_call
            .args
            .iter()
            .map(|arg| self.solve_expr(arg).map(|r| r.tid));
        let params = iter::once(Ok(out))
            .chain(args)
            .collect::<TriResult<BoxedSlice<TypeId>, ()>>()?;

        self.inner
            .cons
            .push_back(Constraint::ApplyByType { target, params });

        Ok(out)
    }

    fn solve_expr_cast(&mut self, expr_cast: &syn::ExprCast) -> TriResult<TypeId, ()> {
        let ty = self.solve_type(&expr_cast.ty)?;
        let tid = self.inner.types.insert_type(ty);
        Ok(tid)
    }

    fn solve_expr_lit(&mut self, expr_lit: &syn::ExprLit, expr_tid: TypeId) -> TypeId {
        match &expr_lit.lit {
            syn::Lit::Int(v) => {
                let scalar = if let Some(scalar) = TypeScalar::from_str(v.suffix()) {
                    scalar
                } else {
                    TypeScalar::Int {
                        reserved: Some(expr_tid),
                    }
                };
                let ty = Type::Scalar(scalar);
                self.inner.types.insert_type(ty)
            }
            syn::Lit::Float(v) => {
                let scalar = if let Some(scalar) = TypeScalar::from_str(v.suffix()) {
                    scalar
                } else {
                    TypeScalar::Float {
                        reserved: Some(expr_tid),
                    }
                };
                let ty = Type::Scalar(scalar);
                self.inner.types.insert_type(ty)
            }
            syn::Lit::Bool(_) => self.inner.types.insert_type(Type::Scalar(TypeScalar::Bool)),
            syn::Lit::Str(_)
            | syn::Lit::ByteStr(_)
            | syn::Lit::CStr(_)
            | syn::Lit::Byte(_)
            | syn::Lit::Char(_)
            | syn::Lit::Verbatim(_) => todo!(),
            _ => panic!(),
        }
    }

    fn solve_expr_paren(&mut self, expr_paren: &syn::ExprParen) -> TriResult<TypeIdWithCtrl, ()> {
        self.solve_expr(&expr_paren.expr)
    }

    fn solve_expr_path(&mut self, expr_path: &syn::ExprPath) -> TriResult<TypeId, ()> {
        if expr_path.qself.is_none() {
            if let Some(ident) = expr_path.path.get_ident() {
                if let Some(tid) = self.inner.symbols.get(&*ident.to_string()).cloned() {
                    self.inner.add_ptr_mapping(ident, tid);
                    return Ok(tid);
                }
            }
        }

        let syn_path = SynPath {
            kind: SynPathKind::Expr,
            qself: expr_path.qself.as_ref(),
            path: &expr_path.path,
        };

        // First, tries to find the type from the path tree.
        let ty = match self
            .infer_logic_host
            .syn_path_to_type(syn_path.clone(), &mut self.inner.types)
        {
            Ok(ty) => ty,
            Err(TriError::Soft(())) => {
                // Failed from the path tree, but the logic could have some info. For example,
                // associated constants inside "impl" block don't belong to the path tree because
                // impl block is related to a type, not a path. But the logic has some info about
                // the impl block.
                let term =
                    TypeFinder::new(self.gcx, &mut self.logic.impl_, &mut self.infer_logic_host)
                        .find_type_by_path(syn_path.path)?;

                TermToTypeCx {
                    types: &mut self.inner.types,
                    name2tid: &Map::default(),
                }
                .term_to_type(&term)
            }
            Err(hard) => return Err(hard),
        };

        let tid = self.inner.types.insert_type(ty);
        Ok(tid)
    }

    fn solve_expr_reference(&mut self, expr_ref: &syn::ExprReference) -> TriResult<TypeId, ()> {
        let mut elem = self.solve_expr(&expr_ref.expr)?.tid;

        if expr_ref.mutability.is_some() {
            elem = self.inner.types.insert_type(Type::Mut(TypeMut { elem }));
        }

        let tid = self.inner.types.insert_type(Type::Ref(TypeRef { elem }));
        Ok(tid)
    }

    fn solve_expr_repeat(&mut self, expr_repeat: &syn::ExprRepeat) -> TriResult<TypeId, ()> {
        let elem = self.solve_expr(&expr_repeat.expr)?.tid;
        let len = match self.infer_logic_host.eval_array_len(&expr_repeat.len) {
            Ok(tree::ArrayLen::Fixed(n)) => InferArrayLen::Fixed(n),
            Ok(tree::ArrayLen::Dynamic) => InferArrayLen::Dynamic,
            Ok(tree::ArrayLen::Generic) => InferArrayLen::Generic,
            Err(TriError::Soft(())) => InferArrayLen::Unknown,
            Err(TriError::Hard(e)) => return Err(TriError::Hard(e)),
        };
        let tid = self
            .inner
            .types
            .insert_type(Type::Array(TypeArray { elem, len }));
        Ok(tid)
    }

    fn solve_expr_return(&mut self, expr_return: &syn::ExprReturn) -> TriResult<TypeId, ()> {
        let tid = if let Some(expr) = &expr_return.expr {
            self.solve_expr(expr)?.tid
        } else {
            self.inner.types.insert_type(Type::Unit)
        };
        Ok(tid)
    }

    fn solve_expr_struct(&mut self, expr_struct: &syn::ExprStruct) -> TriResult<TypeId, ()> {
        let syn_path = SynPath {
            kind: SynPathKind::Expr,
            qself: expr_struct.qself.as_ref(),
            path: &expr_struct.path,
        };
        let ty = self
            .infer_logic_host
            .syn_path_to_type(syn_path, &mut self.inner.types)?;
        let tid = self.inner.types.insert_type(ty);
        Ok(tid)
    }

    fn solve_expr_tuple(&mut self, expr_tuple: &syn::ExprTuple) -> TriResult<TypeId, ()> {
        let elems = expr_tuple
            .elems
            .iter()
            .map(|elem| self.solve_expr(elem).map(|r| r.tid))
            .collect::<TriResult<BoxedSlice<TypeId>, ()>>()?;
        let ty = Type::Tuple(TypeTuple { elems });
        let tid = self.inner.types.insert_type(ty);
        Ok(tid)
    }

    fn solve_expr_unary(&mut self, expr_unary: &syn::ExprUnary) -> TriResult<TypeId, ()> {
        let input = self.solve_expr(&expr_unary.expr)?.tid;

        let tid = match expr_unary.op {
            syn::UnOp::Deref(_) => todo!(),
            syn::UnOp::Not(_) => unary(self, known::apply::NAME_NOT, input),
            syn::UnOp::Neg(_) => unary(self, known::apply::NAME_NEG, input),
            _ => todo!(),
        };
        return Ok(tid);

        // === Internal helper functions ===

        fn unary<'gcx, H: Host<'gcx> + logic::Host<'gcx>>(
            this: &mut InferCx<'_, 'gcx, H>,
            name: &'static str,
            input: TypeId,
        ) -> TypeId {
            let out = this.inner.types.new_type_var();

            this.inner.cons.push_back(Constraint::ApplyMethod {
                name: StrPath::absolute(name),
                params: [out, input].into(),
            });

            out
        }
    }

    fn solve_expr_verbatim(&mut self, tokens: &TokenStream2) -> TypeId {
        // Redundant semicolons can make empty Verbatim expressions.
        assert!(tokens.is_empty());
        self.inner.types.insert_type(Type::Unit)
    }

    fn solve_pat(&mut self, pat: &syn::Pat) -> TriResult<TypeId, ()> {
        let tid = match pat {
            syn::Pat::Ident(v) => self.solve_pat_ident(v),
            syn::Pat::Rest(v) => self.solve_pat_rest(v),
            syn::Pat::Slice(v) => self.solve_pat_slice(v)?,
            syn::Pat::Struct(v) => self.solve_pat_struct(v)?,
            syn::Pat::Tuple(v) => self.solve_pat_tuple(v)?,
            syn::Pat::Type(v) => self.solve_pat_type(v)?,
            o => todo!("{o:#?}"),
        };

        self.inner.add_ptr_mapping(pat, tid);
        Ok(tid)
    }

    fn solve_pat_ident(&mut self, pat_ident: &syn::PatIdent) -> TypeId {
        let name = self.gcx.intern_str(&pat_ident.ident.to_string());
        let tid = self.inner.types.new_type_var();
        self.inner.symbols.push(name, tid);
        self.inner.add_ptr_mapping(&pat_ident.ident, tid);
        tid
    }

    fn solve_pat_rest(&mut self, _: &syn::PatRest) -> TypeId {
        self.inner.types.new_type_var()
    }

    fn solve_pat_slice(&mut self, pat_slice: &syn::PatSlice) -> TriResult<TypeId, ()> {
        let elems = pat_slice
            .elems
            .iter()
            .map(|elem| {
                let tid = self.solve_pat(elem)?;
                let code = self.gcx.intern_str(&elem.code());
                Ok((code, tid))
            })
            .collect::<TriResult<BoxedSlice<(Interned<'gcx, str>, TypeId)>, ()>>()?;
        let tid = self
            .inner
            .types
            .insert_type(Type::Composed(TypeComposed { elems }));
        Ok(tid)
    }

    fn solve_pat_struct(&mut self, pat_struct: &syn::PatStruct) -> TriResult<TypeId, ()> {
        // Queries type for the struct from the host.
        let syn_path = SynPath {
            kind: SynPathKind::Pat,
            qself: None,
            path: &pat_struct.path,
        };
        let ty = self
            .infer_logic_host
            .syn_path_to_type(syn_path, &mut self.inner.types)?;
        let Type::Named(TypeNamed {
            name: _,
            params: type_params,
        }) = &ty
        else {
            unreachable!()
        };

        for field in &pat_struct.fields {
            // Gets some info from the syn's field.
            let syn::Member::Named(ident) = &field.member else {
                unreachable!()
            };
            let syn_field_name = ident.to_string();
            let syn_field_tid = self.solve_pat(&field.pat)?;

            // It is fine to add a pointer mapping here because there's no early exit from here to
            // the end of this function.
            self.inner.add_ptr_mapping(ident, syn_field_tid);

            // Finds out the matching param.
            let type_param_tid = type_params
                .iter()
                .find_map(|param| {
                    let Param::Other { name, tid } = param else {
                        unreachable!()
                    };
                    (name.as_ref() == &*syn_field_name).then_some(*tid)
                })
                .unwrap();

            // Adds a constraint for (syn's field = type's param).
            self.inner.cons.push_back(Constraint::Equal {
                lhs: syn_field_tid,
                rhs: type_param_tid,
            });
        }

        let tid = self.inner.types.insert_type(ty);
        Ok(tid)
    }

    fn solve_pat_tuple(&mut self, pat_tuple: &syn::PatTuple) -> TriResult<TypeId, ()> {
        let elems = pat_tuple
            .elems
            .iter()
            .map(|elem| {
                let tid = self.solve_pat(elem)?;
                let code = self.gcx.intern_str(&elem.code());
                Ok((code, tid))
            })
            .collect::<TriResult<BoxedSlice<(Interned<'gcx, str>, TypeId)>, ()>>()?;
        let tid = self
            .inner
            .types
            .insert_type(Type::Composed(TypeComposed { elems }));
        Ok(tid)
    }

    fn solve_pat_type(&mut self, pat_type: &syn::PatType) -> TriResult<TypeId, ()> {
        let pat_tid = self.solve_pat(&pat_type.pat)?;
        let ty_ty = self.solve_type(&pat_type.ty)?;
        let ty_tid = self.inner.types.insert_type(ty_ty);

        self.inner.cons.push_back(Constraint::Equal {
            lhs: pat_tid,
            rhs: ty_tid,
        });

        Ok(pat_tid)
    }

    fn solve_type(&mut self, ty: &syn::Type) -> TriResult<Type<'gcx>, ()> {
        let ty = match ty {
            syn::Type::Array(ty_arr) => {
                let elem_ty = self.solve_type(&ty_arr.elem)?;
                let elem = self.inner.types.insert_type(elem_ty);
                let len = self.solve_expr_array_length(&ty_arr.len)?;
                Type::Array(TypeArray { elem, len })
            }
            syn::Type::Path(ty_path) => {
                if ty_path.qself.is_some() {
                    todo!("not yet allowed: {ty:?}");
                }

                if let Some(ident) = ty_path.path.get_ident() {
                    if let Some(scalar) = TypeScalar::from_str(&ident.to_string()) {
                        return Ok(Type::Scalar(scalar));
                    }
                }

                let syn_path = SynPath {
                    kind: SynPathKind::Type,
                    qself: ty_path.qself.as_ref(),
                    path: &ty_path.path,
                };
                self.infer_logic_host
                    .syn_path_to_type(syn_path, &mut self.inner.types)?
            }
            syn::Type::Reference(ty_ref) => {
                let elem_ty = self.solve_type(&ty_ref.elem)?;
                let mut elem = self.inner.types.insert_type(elem_ty);

                if ty_ref.mutability.is_some() {
                    elem = self.inner.types.insert_type(Type::Mut(TypeMut { elem }));
                }

                Type::Ref(TypeRef { elem })
            }
            syn::Type::Slice(ty_slice) => {
                let elem_ty = self.solve_type(&ty_slice.elem)?;
                let elem = self.inner.types.insert_type(elem_ty);
                Type::Array(TypeArray {
                    elem,
                    len: InferArrayLen::Dynamic,
                })
            }
            syn::Type::Tuple(ty_tuple) => {
                let elems = ty_tuple
                    .elems
                    .iter()
                    .map(|elem| {
                        let elem_ty = self.solve_type(elem)?;
                        let tid = self.inner.types.insert_type(elem_ty);
                        Ok(tid)
                    })
                    .collect::<TriResult<BoxedSlice<TypeId>, ()>>()?;
                Type::Tuple(TypeTuple { elems })
            }
            o => todo!("{o:?}"),
        };
        Ok(ty)
    }

    fn unify(&mut self) -> TriResult<(), ()> {
        const LOOP_ID: &str = "infer-loop";
        FiniteLoop::set_limit(LOOP_ID, 10);
        FiniteLoop::reset(LOOP_ID);

        while let Some(con) = self.inner.cons.pop_front() {
            let key = iter::once(&con).chain(&self.inner.cons);
            FiniteLoop::assert(LOOP_ID, key, || panic!("infinite loop detected"));

            match con {
                Constraint::Equal { lhs, rhs } => self.unify_equal(lhs, rhs)?,
                Constraint::ApplyMethod { name, params } => {
                    self.unify_apply_method(name, params)?;
                }
                Constraint::ReplicatedApplyMethod {
                    name,
                    params,
                    candidate_id,
                } => {
                    if self.inner.finish_cand.contains(&candidate_id) {
                        continue;
                    }
                    let finish = self.unify_apply_method(name, params)?;
                    if finish {
                        self.inner.finish_cand.insert(candidate_id);
                    }
                }
                Constraint::ApplyByType { target, params } => {
                    self.unify_apply_by_type(target, &params);
                }
            }
        }
        Ok(())
    }

    fn unify_equal(&mut self, lhs: TypeId, rhs: TypeId) -> TriResult<(), ()> {
        use TypeScalar::*;

        let lty = self.inner.types.find_type(lhs);
        let rty = self.inner.types.find_type(rhs);

        if lty == rty {
            return Ok(());
        }

        // Priority: Var & Unknown, Composed, ..

        match rty {
            Type::Var(_) | Type::Unknown => {
                self.replace(rhs, lhs);
                return Ok(());
            }
            Type::Composed(_) if !matches!(lty, Type::Var(_) | Type::Unknown) => {
                handle_composed(self, rhs, lhs);
                return Ok(());
            }
            _ => {}
        }

        match lty {
            Type::Var(_) | Type::Unknown => self.replace(lhs, rhs),
            Type::Composed(_) => handle_composed(self, lhs, rhs),
            Type::Scalar(l) => {
                if let Some(how) = is_equal_scalar(l, rty) {
                    match how {
                        REPLACE_NOT_NECESSARY => {}
                        REPLACE_L_TO_R => self.replace(lhs, rhs),
                        REPLACE_R_TO_L => self.replace(rhs, lhs),
                        _ => unreachable!(),
                    }
                } else {
                    return err!(soft, ());
                }
            }
            Type::Named(TypeNamed { name, params }) => {
                if !is_equal_named_then(*name, params, rty, |l_param, r_param| {
                    self.inner.cons.push_front(Constraint::Equal {
                        lhs: l_param,
                        rhs: r_param,
                    });
                }) {
                    return err!(soft, ());
                }
            }
            Type::Tuple(TypeTuple { elems }) => {
                if !is_equal_tuple_then(elems, rty, |l_elem, r_elem| {
                    self.inner.cons.push_front(Constraint::Equal {
                        lhs: l_elem,
                        rhs: r_elem,
                    });
                }) {
                    return err!(soft, ());
                }
            }
            Type::Array(TypeArray { elem, len }) => {
                if let Some(EqualArray {
                    l_elem,
                    r_elem,
                    how_to_replace,
                }) = is_equal_array(*elem, *len, rty)
                {
                    self.inner.cons.push_front(Constraint::Equal {
                        lhs: l_elem,
                        rhs: r_elem,
                    });
                    match how_to_replace {
                        REPLACE_NOT_NECESSARY => {}
                        REPLACE_L_TO_R => self.replace(lhs, rhs),
                        REPLACE_R_TO_L => self.replace(rhs, lhs),
                        _ => unreachable!(),
                    }
                } else {
                    return err!(soft, ());
                }
            }
            Type::Ref(TypeRef { elem: l_elem }) => {
                if let Type::Ref(TypeRef { elem: r_elem }) = rty {
                    self.unify_equal(*l_elem, *r_elem)?;
                } else {
                    return err!(soft, ());
                }
            }
            Type::Mut(TypeMut { elem: l_elem }) => {
                if let Type::Mut(TypeMut { elem: r_elem }) = rty {
                    self.unify_equal(*l_elem, *r_elem)?;
                } else {
                    return err!(soft, ());
                }
            }
            Type::Unit => return err!(soft, ()),
        }
        return Ok(());

        // === Internal helper functions ===

        fn handle_composed<'gcx, H: Host<'gcx> + logic::Host<'gcx>>(
            this: &mut InferCx<'_, 'gcx, H>,
            pat: TypeId,
            other: TypeId,
        ) {
            // Composed type may have rest pattern(..) in it.

            let Type::Composed(TypeComposed { elems: p_elems }) = this.inner.types.find_type(pat)
            else {
                unreachable!()
            };
            let p_elems = p_elems.iter().collect::<Vec<_>>();

            let o_ty = this.inner.types.find_type(other);
            match o_ty {
                Type::Array(TypeArray { elem: o_elem, len }) => {
                    let o_elem = *o_elem;

                    for (_, p_elem) in p_elems.iter().filter(|(name, _)| name.as_ref() != "..") {
                        this.inner.cons.push_front(Constraint::Equal {
                            lhs: *p_elem,
                            rhs: o_elem,
                        });
                    }

                    // Exits if '..' was not found.
                    let Some((_, p_rest)) = p_elems.iter().find(|(name, _)| name.as_ref() == "..")
                    else {
                        return;
                    };
                    let p_rest = *p_rest;

                    if let InferArrayLen::Fixed(n) = len {
                        let elem = (this.gcx.intern_str(""), o_elem);
                        let elems = iter::repeat(elem).take(*n).collect();
                        let ty = Type::Composed(TypeComposed { elems });
                        let tid = this.inner.types.insert_type(ty);
                        this.inner.cons.push_front(Constraint::Equal {
                            lhs: p_rest,
                            rhs: tid,
                        });
                    }
                }
                Type::Tuple(TypeTuple { elems: o_elems }) => {
                    // e.g.
                    // pat  : (a, b, ..,   c, d)
                    // other: (0, 1, 2, 3, 4, 5)

                    // ol: start of '..'
                    // or: end of '..' + 1
                    let (mut ol, mut or) = (0, o_elems.len());

                    for (i, ((name, p_elem), o_elem)) in p_elems.iter().zip(o_elems).enumerate() {
                        ol = i;
                        if name.as_ref() == ".." {
                            break;
                        }

                        this.inner.cons.push_front(Constraint::Equal {
                            lhs: *p_elem,
                            rhs: *o_elem,
                        });
                    }

                    // Exits if '..' was not found.
                    if ol == o_elems.len() - 1 {
                        return;
                    }

                    for (i, ((_, p_elem), o_elem)) in p_elems
                        .iter()
                        .rev()
                        .take_while(|(name, _)| name.as_ref() != "..")
                        .zip(o_elems.iter().rev())
                        .enumerate()
                    {
                        let i = o_elems.len() - i;
                        or = i;

                        this.inner.cons.push_front(Constraint::Equal {
                            lhs: *p_elem,
                            rhs: *o_elem,
                        });
                    }

                    let p_rest = p_elems[ol].1; // Pattern '..'

                    let elems = o_elems
                        .iter()
                        .skip(ol)
                        .take(or - ol)
                        .map(|elem| (this.gcx.intern_str(""), *elem))
                        .collect();
                    let ty = Type::Composed(TypeComposed { elems });
                    let tid = this.inner.types.insert_type(ty);

                    this.inner.cons.push_front(Constraint::Equal {
                        lhs: p_rest,
                        rhs: tid,
                    });
                }
                o => todo!("{o:?}"),
            }
        }

        const REPLACE_NOT_NECESSARY: u8 = 0;
        const REPLACE_L_TO_R: u8 = 1;
        const REPLACE_R_TO_L: u8 = 2;

        /// * return - [`REPLACE_L_TO_R`], [`REPLACE_R_TO_L`], or None if lhs
        ///   and rhs are not equal.
        fn is_equal_scalar(l: &TypeScalar, rty: &Type<'_>) -> Option<u8> {
            let Type::Scalar(r) = rty else {
                return None;
            };

            if l.is_abstract_of(r) {
                return Some(REPLACE_L_TO_R);
            }

            if r.is_abstract_of(l) {
                return Some(REPLACE_R_TO_L);
            }

            match (l, r) {
                (Int { .. }, Int { .. }) => Some(REPLACE_L_TO_R),
                (Float { .. }, Float { .. }) => Some(REPLACE_L_TO_R),
                _ => None,
            }
        }

        /// * 1st param of f - Type id to an element of the lhs [`Type::Named::params`].
        /// * 2nd param of f - Type id to an element of the rhs [`Type::Named::params`].
        fn is_equal_named_then<F>(
            l_name: Interned<'_, str>,
            l_params: &[Param<'_>],
            rty: &Type<'_>,
            mut f: F,
        ) -> bool
        where
            F: FnMut(TypeId, TypeId),
        {
            let Type::Named(TypeNamed {
                name: r_name,
                params: r_params,
            }) = rty
            else {
                return false;
            };

            if l_name != *r_name || l_params.len() != r_params.len() {
                return false;
            }

            for (l_param, r_param) in l_params.iter().zip(r_params) {
                match (l_param, r_param) {
                    (Param::Self_, Param::Self_) => {}
                    (
                        Param::Other {
                            name: _,
                            tid: l_tid,
                        },
                        Param::Other {
                            name: _,
                            tid: r_tid,
                        },
                    ) => {
                        f(*l_tid, *r_tid);
                    }
                    _ => return false,
                }
            }

            true
        }

        /// * 1st param of f - Type id to an element of the lhs tuple.
        /// * 2nd param of f - Type id to an element of the rhs tuple.
        fn is_equal_tuple_then<F>(l_elems: &[TypeId], rty: &Type<'_>, mut f: F) -> bool
        where
            F: FnMut(TypeId, TypeId),
        {
            let Type::Tuple(TypeTuple { elems: r_elems }) = rty else {
                return false;
            };

            if l_elems.len() != r_elems.len() {
                return false;
            }

            for (l_elem, r_elem) in l_elems.iter().zip(r_elems) {
                f(*l_elem, *r_elem);
            }
            true
        }

        struct EqualArray {
            l_elem: TypeId,
            r_elem: TypeId,
            how_to_replace: u8,
        }

        fn is_equal_array(
            l_elem: TypeId,
            l_len: InferArrayLen,
            rty: &Type<'_>,
        ) -> Option<EqualArray> {
            let Type::Array(TypeArray {
                elem: r_elem,
                len: r_len,
            }) = rty
            else {
                return None;
            };

            let mut res = EqualArray {
                l_elem,
                r_elem: *r_elem,
                how_to_replace: REPLACE_NOT_NECESSARY,
            };

            match (l_len, *r_len) {
                (InferArrayLen::Fixed(l), InferArrayLen::Fixed(r)) if l == r => {}
                (
                    InferArrayLen::Fixed(_) | InferArrayLen::Dynamic | InferArrayLen::Generic,
                    InferArrayLen::Unknown,
                ) => res.how_to_replace = REPLACE_R_TO_L,
                (
                    InferArrayLen::Unknown,
                    InferArrayLen::Fixed(_) | InferArrayLen::Dynamic | InferArrayLen::Generic,
                ) => res.how_to_replace = REPLACE_L_TO_R,
                _ => return None,
            }

            Some(res)
        }
    }

    fn unify_apply_method(
        &mut self,
        name: StrPath<'static>,
        params: BoxedSlice<TypeId>,
    ) -> TriResult<bool, ()> {
        let mut corrected_types = Vec::new();
        let mut corrected_tids = Vec::new();

        let mut finish = true;

        self.correct_method_call(name.clone(), &params, &mut corrected_types)?;

        if params.len() == corrected_types.len() {
            while let Some(ty) = corrected_types.pop() {
                let tid = self.inner.types.insert_type(ty);
                corrected_tids.push(tid);
            }
            corrected_tids.reverse();

            // If it contains non-concrete types, retries when something has changed.
            if corrected_tids.iter().any(|tid| {
                matches!(
                    self.inner.types[*tid],
                    Type::Scalar(TypeScalar::Int { .. } | TypeScalar::Float { .. })
                )
            }) {
                self.add_candidate_constraint(Constraint::ReplicatedApplyMethod {
                    name: name.clone(),
                    params: corrected_tids.iter().cloned().collect(),
                    candidate_id: self.inner.cand_id,
                });
                self.inner.cand_id += 1;
                finish = false;
            }

            for (old, new) in params.into_iter().zip(corrected_tids) {
                if self.inner.find_type(old) != self.inner.find_type(new) {
                    self.inner
                        .cons
                        .push_front(Constraint::Equal { lhs: old, rhs: new });
                }
            }
        } else {
            // Correction failed, retries when something has changed.
            self.add_candidate_constraint(Constraint::ReplicatedApplyMethod {
                name,
                params,
                candidate_id: self.inner.cand_id,
            });
            self.inner.cand_id += 1;
            finish = false;
        }
        TriResult::Ok(finish)
    }

    fn unify_apply_by_type(&mut self, target: TypeId, params: &[TypeId]) {
        let Type::Named(TypeNamed {
            params: known_params,
            name: _,
        }) = self.inner.types.find_type(target)
        else {
            unreachable!()
        };

        assert_eq!(params.len(), known_params.len());

        for (input, known) in params.iter().zip(known_params.iter()) {
            self.inner.cons.push_front(Constraint::Equal {
                lhs: *input,
                rhs: match known {
                    Param::Self_ => target,
                    Param::Other { name: _, tid } => *tid,
                },
            });
        }
    }

    fn add_candidate_constraint(&mut self, con: Constraint) {
        match &con {
            Constraint::ReplicatedApplyMethod { params, .. } => {
                for param in params.iter() {
                    add(self, con.clone(), *param);
                }
            }
            _ => unreachable!(),
        }

        // === Internal helper functions ===

        /// Add a candidate constraint for the given type id. When the given
        /// type changes, the candidate constraint will become a normal
        /// constraint and then be unified again.
        fn add<'gcx, H: Host<'gcx> + logic::Host<'gcx>>(
            this: &mut InferCx<'_, 'gcx, H>,
            con: Constraint,
            tid: TypeId,
        ) {
            let ty = &this.inner.types[tid];
            if let Some(cons) = this.inner.cand_cons.get_mut(ty) {
                cons.push(con);
            } else {
                this.inner.cand_cons.insert(ty.clone(), vec![con]);
            }
        }
    }

    fn replace(&mut self, from: TypeId, to: TypeId) {
        let from_ty = self.inner.types.find_type(from);
        let to_ty = self.inner.types.find_type(to);
        if from_ty == to_ty {
            return;
        }

        if let Some(cons) = self.inner.cand_cons.remove(from_ty) {
            self.inner.cons.extend(cons);
        }

        self.inner.types.replace(&from_ty.clone(), to_ty.clone());
    }

    fn correct_method_call(
        &mut self,
        name: StrPath,
        args: &[TypeId],
        output: &mut Vec<Type<'gcx>>,
    ) -> TriResult<(), ()> {
        // `parent` is either trait path or type path.
        let (parent_path, fn_ident) = name.rsplit_once("::").unwrap();

        // Conversion: Type -> Term
        let mut name2tid = Map::default();
        let mut args = args
            .iter()
            .enumerate()
            .map(|(i, arg)| {
                TypeToTermCx {
                    gcx: self.gcx,
                    types: &self.inner.types,
                    var_ident: i as u32,
                    name2tid: &mut name2tid,
                }
                .type_to_term(&self.inner.types[*arg])
            })
            .collect::<Box<_>>();

        // Solve using the logic.
        self.logic
            .find_method_sig(&mut self.infer_logic_host, parent_path, fn_ident, &mut args)?;

        // Conversion: Term -> Type
        debug_assert!(output.is_empty());
        for param in args.iter() {
            let ty = TermToTypeCx {
                types: &mut self.inner.types,
                name2tid: &name2tid,
            }
            .term_to_type(param);

            // Fills the output.
            output.push(ty);
        }
        Ok(())
    }
}

struct TypeToTermCx<'a, 'gcx> {
    gcx: &'gcx GlobalCx<'gcx>,

    types: &'a UniqueTypes<'gcx>,

    // TODO: If a single type is converted into a term that has variables more than one?
    var_ident: u32,

    /// Mapping between functor of a term and type id inside a type.
    ///
    /// When we do the conversion(type -> term or term -> type), we need to preserve [`TypeId`]s of
    /// terms to get them back from variable terms.
    /// e.g.
    /// Type::Var(tid_42) -> Term { functor: "$0" } -> Type::Var(tid_42)
    name2tid: &'a mut Map<NameIn<'gcx>, TypeId>,
}

impl<'a, 'gcx> TypeToTermCx<'a, 'gcx> {
    fn type_to_term(&mut self, ty: &'a Type) -> TermIn<'gcx> {
        match ty {
            Type::Scalar(scalar) => self.scalar_to_term(scalar),
            Type::Named(_named) => todo!(),
            Type::Tuple(TypeTuple { elems }) => {
                let elems = elems
                    .iter()
                    .map(|elem| self.type_to_term(&self.types[*elem]))
                    .collect();
                term::tuple_n(elems, self.gcx)
            }
            Type::Array(TypeArray { elem, len }) => {
                let elem = self.type_to_term(&self.types[*elem]);
                match len {
                    InferArrayLen::Fixed(len) => {
                        let len = Term {
                            functor: Name::with_intern(&len.to_string(), self.gcx),
                            args: [].into(),
                        };
                        term::array_2(elem, len, self.gcx)
                    }
                    InferArrayLen::Dynamic => term::array_1(elem, self.gcx),
                    InferArrayLen::Generic | InferArrayLen::Unknown => {
                        let var = Term {
                            functor: var_name(&self.var_ident, self.gcx),
                            args: [].into(),
                        };
                        term::array_2(elem, var, self.gcx)
                    }
                }
            }
            Type::Ref(TypeRef { elem }) => {
                let elem = self.type_to_term(&self.types[*elem]);
                term::ref_1(elem, self.gcx)
            }
            Type::Mut(TypeMut { elem }) => {
                let elem = self.type_to_term(&self.types[*elem]);
                term::mut_1(elem, self.gcx)
            }
            Type::Unit => term::unit_0(self.gcx),
            Type::Var(tid) => {
                let functor = var_name(&self.var_ident, self.gcx);
                self.name2tid.insert(functor, *tid);
                Term {
                    functor,
                    args: [].into(),
                }
            }
            Type::Composed(_) => todo!(),
            Type::Unknown => todo!(),
        }
    }

    fn scalar_to_term(&mut self, scalar: &TypeScalar) -> TermIn<'gcx> {
        let functor_only = |functor: &str| Term {
            functor: Name::with_intern(functor, self.gcx),
            args: [].into(),
        };

        match scalar {
            TypeScalar::Int { reserved } => {
                let functor = var_name(&self.var_ident, self.gcx);

                if let Some(tid) = reserved {
                    self.name2tid.insert(functor, *tid);
                }

                let int = Term {
                    functor,
                    args: [].into(),
                };
                term::int_1(int, self.gcx)
            }
            TypeScalar::Float { reserved } => {
                let functor = var_name(&self.var_ident, self.gcx);

                if let Some(tid) = reserved {
                    self.name2tid.insert(functor, *tid);
                }

                let float = Term {
                    functor,
                    args: [].into(),
                };
                term::float_1(float, self.gcx)
            }
            TypeScalar::I8 => term::int_1(functor_only("i8"), self.gcx),
            TypeScalar::I16 => term::int_1(functor_only("i16"), self.gcx),
            TypeScalar::I32 => term::int_1(functor_only("i32"), self.gcx),
            TypeScalar::I64 => term::int_1(functor_only("i64"), self.gcx),
            TypeScalar::I128 => term::int_1(functor_only("i128"), self.gcx),
            TypeScalar::Isize => term::int_1(functor_only("isize"), self.gcx),
            TypeScalar::U8 => term::int_1(functor_only("u8"), self.gcx),
            TypeScalar::U16 => term::int_1(functor_only("u16"), self.gcx),
            TypeScalar::U32 => term::int_1(functor_only("u32"), self.gcx),
            TypeScalar::U64 => term::int_1(functor_only("u64"), self.gcx),
            TypeScalar::U128 => term::int_1(functor_only("u128"), self.gcx),
            TypeScalar::Usize => term::int_1(functor_only("usize"), self.gcx),
            TypeScalar::F32 => term::float_1(functor_only("f32"), self.gcx),
            TypeScalar::F64 => term::float_1(functor_only("f64"), self.gcx),
            TypeScalar::Bool => functor_only("bool"),
        }
    }
}

struct TermToTypeCx<'a, 'gcx> {
    types: &'a mut UniqueTypes<'gcx>,

    /// Mapping between functor of a term and type id inside a type.
    ///
    /// When we do the conversion(type -> term or term -> type), we need to preserve [`TypeId`]s of
    /// terms to get them back from variable terms.
    /// e.g.
    /// Type::Var(tid_42) -> Term { functor: "$0" } -> Type::Var(tid_42)
    name2tid: &'a Map<NameIn<'gcx>, TypeId>,
}

impl<'a, 'gcx> TermToTypeCx<'a, 'gcx> {
    fn term_to_type(&mut self, term: &TermIn<'gcx>) -> Type<'gcx> {
        match term.functor.as_ref() {
            term::FUNCTOR_INT => self.int_to_type(&term.args[0]),
            term::FUNCTOR_FLOAT => self.float_to_type(&term.args[0]),
            "bool" => Type::Scalar(TypeScalar::Bool),
            term::FUNCTOR_UNIT => Type::Unit,
            term::FUNCTOR_REF => {
                let elem_ty = self.term_to_type(&term.args[0]);
                let elem_tid = self.types.insert_type(elem_ty);
                Type::Ref(TypeRef { elem: elem_tid })
            }
            term::FUNCTOR_MUT => {
                let elem_ty = self.term_to_type(&term.args[0]);
                let elem_tid = self.types.insert_type(elem_ty);
                Type::Mut(TypeMut { elem: elem_tid })
            }
            functor if functor.starts_with(VAR_PREFIX) => {
                let tid = self.name2tid.get(&term.functor).unwrap();
                Type::Var(*tid)
            }
            _ => todo!(),
        }
    }

    fn int_to_type(&self, term: &TermIn<'gcx>) -> Type<'gcx> {
        match term.functor.as_ref() {
            "i8" => Type::Scalar(TypeScalar::I8),
            "i16" => Type::Scalar(TypeScalar::I16),
            "i32" => Type::Scalar(TypeScalar::I32),
            "i64" => Type::Scalar(TypeScalar::I64),
            "i128" => Type::Scalar(TypeScalar::I128),
            "isize" => Type::Scalar(TypeScalar::Isize),
            "u8" => Type::Scalar(TypeScalar::U8),
            "u16" => Type::Scalar(TypeScalar::U16),
            "u32" => Type::Scalar(TypeScalar::U32),
            "u64" => Type::Scalar(TypeScalar::U64),
            "u128" => Type::Scalar(TypeScalar::U128),
            "usize" => Type::Scalar(TypeScalar::Usize),
            functor if functor.starts_with(VAR_PREFIX) => Type::Scalar(TypeScalar::Int {
                reserved: self.name2tid.get(functor).cloned(),
            }),
            o => unreachable!("{o:?}"),
        }
    }

    fn float_to_type(&self, term: &TermIn<'gcx>) -> Type<'gcx> {
        match term.functor.as_ref() {
            "f32" => Type::Scalar(TypeScalar::F32),
            "f64" => Type::Scalar(TypeScalar::F64),
            functor if functor.starts_with(VAR_PREFIX) => Type::Scalar(TypeScalar::Float {
                reserved: self.name2tid.get(functor).cloned(),
            }),
            _ => unreachable!(),
        }
    }
}

#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub(crate) enum Constraint {
    Equal {
        lhs: TypeId,
        rhs: TypeId,
    },
    ApplyMethod {
        /// e.g. "core::ops::Add::add"
        name: StrPath<'static>,
        params: BoxedSlice<TypeId>,
    },
    ReplicatedApplyMethod {
        /// e.g. "core::ops::Add::add"
        name: StrPath<'static>,
        params: BoxedSlice<TypeId>,
        candidate_id: u32,
    },
    // e.g. Host converts function name "foo" into a type of the function
    ApplyByType {
        target: TypeId,
        params: BoxedSlice<TypeId>,
    },
}

struct TypeIdWithCtrl {
    tid: TypeId,
    is_return: bool,
}

#[cfg(test)]
pub(crate) mod tests {
    use super::{Host, Inferer};
    use crate::{
        etc::syn::SynPath,
        semantic::{
            basic_traits::EvaluateArrayLength,
            entry::GlobalCx,
            infer::{
                test_help::{test_inferer, TestInferLogicHost},
                ty::{
                    InferArrayLen, OwnedParam, OwnedType, Param, Type, TypeNamed, TypeScalar,
                    UniqueTypes,
                },
            },
            logic::{self, test_help::test_logic},
        },
        Intern, TriResult,
    };
    use std::pin::Pin;
    use syn_locator::{Find, LocateEntry};

    fn type_of_local_stmt(inferer: &Inferer, stmt: &syn::Stmt) -> OwnedType {
        let syn::Stmt::Local(local) = stmt else {
            unreachable!()
        };
        let syn::Pat::Ident(pat_ident) = &local.pat else {
            unreachable!()
        };
        inferer
            .get_owned_type_of_ident(&pat_ident.ident)
            .unwrap()
            .clone()
    }

    fn type_of_local_stmt_with_type(inferer: &Inferer, stmt: &syn::Stmt) -> OwnedType {
        let syn::Stmt::Local(local) = stmt else {
            unreachable!()
        };
        let syn::Pat::Type(syn::PatType { pat, .. }) = &local.pat else {
            unreachable!()
        };
        let syn::Pat::Ident(pat_ident) = &**pat else {
            unreachable!()
        };
        inferer
            .get_owned_type_of_ident(&pat_ident.ident)
            .unwrap()
            .clone()
    }

    fn type_of_ident<P>(inferer: &Inferer, parent: &P, ident: &str) -> OwnedType
    where
        P: Find<syn::Ident> + ?Sized,
    {
        let ident: &syn::Ident = parent.find(ident).unwrap();
        inferer.get_owned_type_of_ident(ident).unwrap()
    }

    #[test]
    #[rustfmt::skip]
    fn test_infer_primitives() {
        let code = r#"{
            // Primitive types
            let a: i8 = 0;
            let a: i16 = 0;
            let a: i32 = 0;
            let a: i64 = 0;
            let a: i128 = 0;
            let a: isize = 0;
            let a: u8 = 0;
            let a: u16 = 0;
            let a: u32 = 0;
            let a: u64 = 0;
            let a: u128 = 0;
            let a: usize = 0;
            let a: f32 = 0.;
            let a: f64 = 0.;
            let a: bool = false;
        }"#;

        let block = syn::parse_str::<syn::Block>(code).unwrap();
        let gcx = GlobalCx::default();
        let mut inferer = test_inferer(&gcx);
        let mut logic = test_logic(&gcx);
        let mut infer_logic_host = TestInferLogicHost::new(&gcx);
        inferer.infer_block(&mut logic, &mut infer_logic_host, &block, None).unwrap();

        // Is symbol table empty?
        assert!(inferer.symbols.is_empty());

        // let a: i8 = 0;
        let mut stmt_i = 0;
        let lhs = type_of_local_stmt_with_type(&inferer, &block.stmts[stmt_i]);
        assert_eq!(lhs, OwnedType::Named { name: "i8".into(), params: [].into() });
        stmt_i += 1;

        // let a: i16 = 0;
        let lhs = type_of_local_stmt_with_type(&inferer, &block.stmts[stmt_i]);
        assert_eq!(lhs, OwnedType::Named { name: "i16".into(), params: [].into() });
        stmt_i += 1;

        // let a: i32 = 0;
        let lhs = type_of_local_stmt_with_type(&inferer, &block.stmts[stmt_i]);
        assert_eq!(lhs, OwnedType::Named { name: "i32".into(), params: [].into() });
        stmt_i += 1;

        // let a: i64 = 0;
        let lhs = type_of_local_stmt_with_type(&inferer, &block.stmts[stmt_i]);
        assert_eq!(lhs, OwnedType::Named { name: "i64".into(), params: [].into() });
        stmt_i += 1;

        // let a: i128 = 0;
        let lhs = type_of_local_stmt_with_type(&inferer, &block.stmts[stmt_i]);
        assert_eq!(lhs, OwnedType::Named { name: "i128".into(), params: [].into() });
        stmt_i += 1;

        // let a: isize = 0;
        let lhs = type_of_local_stmt_with_type(&inferer, &block.stmts[stmt_i]);
        assert_eq!(lhs, OwnedType::Named { name: "isize".into(), params: [].into() });
        stmt_i += 1;

        // let a: u8 = 0;
        let lhs = type_of_local_stmt_with_type(&inferer, &block.stmts[stmt_i]);
        assert_eq!(lhs, OwnedType::Named { name: "u8".into(), params: [].into() });
        stmt_i += 1;

        // let a: u16 = 0;
        let lhs = type_of_local_stmt_with_type(&inferer, &block.stmts[stmt_i]);
        assert_eq!(lhs, OwnedType::Named { name: "u16".into(), params: [].into() });
        stmt_i += 1;

        // let a: u32 = 0;
        let lhs = type_of_local_stmt_with_type(&inferer, &block.stmts[stmt_i]);
        assert_eq!(lhs, OwnedType::Named { name: "u32".into(), params: [].into() });
        stmt_i += 1;

        // let a: u64 = 0;
        let lhs = type_of_local_stmt_with_type(&inferer, &block.stmts[stmt_i]);
        assert_eq!(lhs, OwnedType::Named { name: "u64".into(), params: [].into() });
        stmt_i += 1;

        // let a: u128 = 0;
        let lhs = type_of_local_stmt_with_type(&inferer, &block.stmts[stmt_i]);
        assert_eq!(lhs, OwnedType::Named { name: "u128".into(), params: [].into() });
        stmt_i += 1;

        // let a: usize = 0;
        let lhs = type_of_local_stmt_with_type(&inferer, &block.stmts[stmt_i]);
        assert_eq!(lhs, OwnedType::Named { name: "usize".into(), params: [].into() });
        stmt_i += 1;

        // let a: f32 = 0.;
        let lhs = type_of_local_stmt_with_type(&inferer, &block.stmts[stmt_i]);
        assert_eq!(lhs, OwnedType::Named { name: "f32".into(), params: [].into() });
        stmt_i += 1;

        // let a: f64 = 0.;
        let lhs = type_of_local_stmt_with_type(&inferer, &block.stmts[stmt_i]);
        assert_eq!(lhs, OwnedType::Named { name: "f64".into(), params: [].into() });
        stmt_i += 1;

        // let a: bool = false;
        let lhs = type_of_local_stmt_with_type(&inferer, &block.stmts[stmt_i]);
        assert_eq!(lhs, OwnedType::Named { name: "bool".into(), params: [].into() });
    }

    #[test]
    #[rustfmt::skip]
    fn test_infer_operators() {
        let code = r#"{
            // Binary operators
            let a = 0_u32 + 0;
            let a = 0_u32 - 0;
            let a = 0_u32 * 0;
            let a = 0_u32 / 1;
            let a = 0_u32 % 1;
            let a = 0_u32 ^ 0;
            let a = 0_u32 & 0;
            let a = 0_u32 | 0;
            let a = 0_u32 << 0;
            let a = 0_u32 >> 0;
            // Binary assign operators
            let mut a = 0;
            a += 0_u32;
            let mut a = 0;
            a -= 0_u32;
            let mut a = 0;
            a *= 0_u32;
            let mut a = 0;
            a /= 1_u32;
            let mut a = 0;
            a %= 1_u32;
            let mut a = 0;
            a ^= 0_u32;
            let mut a = 0;
            a &= 0_u32;
            let mut a = 0;
            a |= 0_u32;
            // Unary operators
            let a = !false;
            let a = -0_i32;
        }"#;

        let block = syn::parse_str::<syn::Block>(code).unwrap();
        let gcx = GlobalCx::default();
        let mut inferer = test_inferer(&gcx);
        let mut logic = test_logic(&gcx);
        let mut infer_logic_host = TestInferLogicHost::new(&gcx);
        inferer.infer_block(&mut logic, &mut infer_logic_host, &block, None).unwrap();

        // Is symbol table empty?
        assert!(inferer.symbols.is_empty());

        // let a = 0_u32 + 0;
        let mut stmt_i = 0;
        let lhs = type_of_local_stmt(&inferer, &block.stmts[stmt_i]);
        assert_eq!(lhs, OwnedType::Named { name: "u32".into(), params: [].into() });
        stmt_i += 1;

        // let a = 0_u32 - 0;
        let lhs = type_of_local_stmt(&inferer, &block.stmts[stmt_i]);
        assert_eq!(lhs, OwnedType::Named { name: "u32".into(), params: [].into() });
        stmt_i += 1;

        // let a = 0_u32 * 0;
        let lhs = type_of_local_stmt(&inferer, &block.stmts[stmt_i]);
        assert_eq!(lhs, OwnedType::Named { name: "u32".into(), params: [].into() });
        stmt_i += 1;

        // let a = 0_u32 / 1;
        let lhs = type_of_local_stmt(&inferer, &block.stmts[stmt_i]);
        assert_eq!(lhs, OwnedType::Named { name: "u32".into(), params: [].into() });
        stmt_i += 1;

        // let a = 0_u32 % 1;
        let lhs = type_of_local_stmt(&inferer, &block.stmts[stmt_i]);
        assert_eq!(lhs, OwnedType::Named { name: "u32".into(), params: [].into() });
        stmt_i += 1;

        // let a = 0_u32 ^ 0;
        let lhs = type_of_local_stmt(&inferer, &block.stmts[stmt_i]);
        assert_eq!(lhs, OwnedType::Named { name: "u32".into(), params: [].into() });
        stmt_i += 1;

        // let a = 0_u32 & 0;
        let lhs = type_of_local_stmt(&inferer, &block.stmts[stmt_i]);
        assert_eq!(lhs, OwnedType::Named { name: "u32".into(), params: [].into() });
        stmt_i += 1;

        // let a = 0_u32 | 0;
        let lhs = type_of_local_stmt(&inferer, &block.stmts[stmt_i]);
        assert_eq!(lhs, OwnedType::Named { name: "u32".into(), params: [].into() });
        stmt_i += 1;

        // let a = 0_u32 << 0;
        let lhs = type_of_local_stmt(&inferer, &block.stmts[stmt_i]);
        assert_eq!(lhs, OwnedType::Named { name: "u32".into(), params: [].into() });
        stmt_i += 1;

        // let a = 0_u32 >> 0;
        let lhs = type_of_local_stmt(&inferer, &block.stmts[stmt_i]);
        assert_eq!(lhs, OwnedType::Named { name: "u32".into(), params: [].into() });
        stmt_i += 1;

        // let mut a = 0; a += 0_u32;
        let lhs = type_of_local_stmt(&inferer, &block.stmts[stmt_i]);
        assert_eq!(lhs, OwnedType::Named { name: "u32".into(), params: [].into() });
        stmt_i += 2;

        // let mut a = 0; a -= 0_u32;
        let lhs = type_of_local_stmt(&inferer, &block.stmts[stmt_i]);
        assert_eq!(lhs, OwnedType::Named { name: "u32".into(), params: [].into() });
        stmt_i += 2;

        // let mut a = 0; a *= 0_u32;
        let lhs = type_of_local_stmt(&inferer, &block.stmts[stmt_i]);
        assert_eq!(lhs, OwnedType::Named { name: "u32".into(), params: [].into() });
        stmt_i += 2;

        // let mut a = 0; a /= 1_u32;
        let lhs = type_of_local_stmt(&inferer, &block.stmts[stmt_i]);
        assert_eq!(lhs, OwnedType::Named { name: "u32".into(), params: [].into() });
        stmt_i += 2;

        // let mut a = 0; a %= 1_u32;
        let lhs = type_of_local_stmt(&inferer, &block.stmts[stmt_i]);
        assert_eq!(lhs, OwnedType::Named { name: "u32".into(), params: [].into() });
        stmt_i += 2;

        // let mut a = 0; a ^= 0_u32;
        let lhs = type_of_local_stmt(&inferer, &block.stmts[stmt_i]);
        assert_eq!(lhs, OwnedType::Named { name: "u32".into(), params: [].into() });
        stmt_i += 2;

        // let mut a = 0; a &= 0_u32;
        let lhs = type_of_local_stmt(&inferer, &block.stmts[stmt_i]);
        assert_eq!(lhs, OwnedType::Named { name: "u32".into(), params: [].into() });
        stmt_i += 2;

        // let mut a = 0; a |= 0_u32;
        let lhs = type_of_local_stmt(&inferer, &block.stmts[stmt_i]);
        assert_eq!(lhs, OwnedType::Named { name: "u32".into(), params: [].into() });
        stmt_i += 2;

        // let a = !false;
        let lhs = type_of_local_stmt(&inferer, &block.stmts[stmt_i]);
        assert_eq!(lhs, OwnedType::Named { name: "bool".into(), params: [].into() });
        stmt_i += 1;

        // let a = -0_i32;
        let lhs = type_of_local_stmt(&inferer, &block.stmts[stmt_i]);
        assert_eq!(lhs, OwnedType::Named { name: "i32".into(), params: [].into() });
    }

    #[test]
    fn test_infer_function_type() {
        let code = r#"{
            fn f(i: u32) -> u32 { i }
            let a = f;
        }"#;

        trait InferLogicHost<'gcx>: Host<'gcx> + logic::Host<'gcx> {}

        impl<'gcx, T: Host<'gcx> + logic::Host<'gcx>> InferLogicHost<'gcx> for T {}

        struct TestHost<'gcx> {
            gcx: &'gcx GlobalCx<'gcx>,
            inner: Box<dyn InferLogicHost<'gcx> + 'gcx>,
        }

        impl<'gcx> Host<'gcx> for TestHost<'gcx> {
            fn syn_path_to_type(
                &mut self,
                _: SynPath,
                types: &mut UniqueTypes<'gcx>,
            ) -> TriResult<Type<'gcx>, ()> {
                let tid_u32 = types.insert_type(Type::Scalar(TypeScalar::U32));
                let output = Param::Other {
                    name: self.gcx.intern_str("0"),
                    tid: tid_u32,
                };
                let input = Param::Other {
                    name: self.gcx.intern_str("i"),
                    tid: tid_u32,
                };
                let res = Type::Named(TypeNamed {
                    name: self.gcx.intern_str("f"),
                    params: [output, input].into(),
                });
                Ok(res)
            }
        }

        impl<'gcx> logic::Host<'gcx> for TestHost<'gcx> {
            fn ident_to_npath(&mut self, ident: &syn::Ident) -> TriResult<String, ()> {
                logic::Host::ident_to_npath(&mut *self.inner, ident)
            }
        }

        impl<'gcx> EvaluateArrayLength<'gcx> for TestHost<'gcx> {
            fn eval_array_len(&mut self, expr: &syn::Expr) -> TriResult<crate::ArrayLen, ()> {
                EvaluateArrayLength::eval_array_len(&mut *self.inner, expr)
            }
        }

        crate::impl_empty_method_host!(TestHost<'_>);
        crate::impl_empty_scoping!(TestHost<'_>);

        let block = syn::parse_str::<syn::Block>(code).unwrap();
        let gcx = GlobalCx::default();
        let mut inferer = test_inferer(&gcx);
        let mut logic = test_logic(&gcx);
        let mut infer_logic_host = TestHost {
            gcx: &gcx,
            inner: Box::new(TestInferLogicHost::new(&gcx)),
        };
        inferer
            .infer_block(&mut logic, &mut infer_logic_host, &block, None)
            .unwrap();

        // Is symbol table empty?
        assert!(inferer.symbols.is_empty());

        // let a = f;
        let stmt_i = 1;
        let lhs = type_of_local_stmt(&inferer, &block.stmts[stmt_i]);
        assert_eq!(
            lhs,
            OwnedType::Named {
                name: "f".into(),
                params: [
                    OwnedParam::Other {
                        name: "0".into(),
                        ty: OwnedType::Named {
                            name: "u32".into(),
                            params: [].into()
                        }
                    },
                    OwnedParam::Other {
                        name: "i".into(),
                        ty: OwnedType::Named {
                            name: "u32".into(),
                            params: [].into()
                        }
                    },
                ]
                .into()
            }
        );
    }

    // TODO: Test host related functionalities like function, struct
    // constructor, etc in the host implementation.
    #[test]
    fn test_infer_by_function_parameters() {
        let code = r#"{
            fn foo(a: u32) -> u32 { a }
            let a = 0;
            let b = foo(a);
        }"#;

        trait InferLogicHost<'gcx>: Host<'gcx> + logic::Host<'gcx> {}

        impl<'gcx, T> InferLogicHost<'gcx> for T where T: Host<'gcx> + logic::Host<'gcx> {}

        struct TestHost<'gcx> {
            gcx: &'gcx GlobalCx<'gcx>,
            inner: Box<dyn InferLogicHost<'gcx> + 'gcx>,
        }

        impl<'gcx> Host<'gcx> for TestHost<'gcx> {
            fn syn_path_to_type(
                &mut self,
                _: SynPath,
                types: &mut UniqueTypes<'gcx>,
            ) -> TriResult<Type<'gcx>, ()> {
                let tid_u32 = types.insert_type(Type::Scalar(TypeScalar::U32));
                let output = Param::Other {
                    name: self.gcx.intern_str("0"),
                    tid: tid_u32,
                };
                let input = Param::Other {
                    name: self.gcx.intern_str("a"),
                    tid: tid_u32,
                };
                let res = Type::Named(TypeNamed {
                    name: self.gcx.intern_str("foo"),
                    params: [output, input].into(),
                });
                Ok(res)
            }
        }

        impl<'gcx> logic::Host<'gcx> for TestHost<'gcx> {
            fn ident_to_npath(&mut self, ident: &syn::Ident) -> TriResult<String, ()> {
                logic::Host::ident_to_npath(&mut *self.inner, ident)
            }
        }

        impl<'gcx> EvaluateArrayLength<'gcx> for TestHost<'gcx> {
            fn eval_array_len(&mut self, expr: &syn::Expr) -> TriResult<crate::ArrayLen, ()> {
                EvaluateArrayLength::eval_array_len(&mut *self.inner, expr)
            }
        }

        crate::impl_empty_method_host!(TestHost<'_>);
        crate::impl_empty_scoping!(TestHost<'_>);

        let block = syn::parse_str::<syn::Block>(code).unwrap();
        let gcx = GlobalCx::default();
        let mut inferer = test_inferer(&gcx);
        let mut logic = test_logic(&gcx);
        let mut infer_logic_host = TestHost {
            gcx: &gcx,
            inner: Box::new(TestInferLogicHost::new(&gcx)),
        };
        inferer
            .infer_block(&mut logic, &mut infer_logic_host, &block, None)
            .unwrap();

        // Is symbol table empty?
        assert!(inferer.symbols.is_empty());

        // let a = 0;
        let mut stmt_i = 1;
        let lhs = type_of_local_stmt(&inferer, &block.stmts[stmt_i]);
        assert_eq!(
            lhs,
            OwnedType::Named {
                name: "u32".into(),
                params: [].into()
            }
        );
        stmt_i += 1;

        // let b = foo(a);
        let lhs = type_of_local_stmt(&inferer, &block.stmts[stmt_i]);
        assert_eq!(
            lhs,
            OwnedType::Named {
                name: "u32".into(),
                params: [].into()
            }
        );
    }

    #[test]
    #[rustfmt::skip]
    fn test_infer_function_block() {
        let code = r#"
        fn f(x: u32) -> u64 { 
            let a = x;
            let b = 0;
            return b;
            let c = 0;
            c
        }
        "#;

        let f = syn::parse_str::<syn::ItemFn>(code).unwrap();
        let gcx = GlobalCx::default();
        let mut inferer = test_inferer(&gcx);
        let mut logic = test_logic(&gcx);
        let mut infer_logic_host = TestInferLogicHost::new(&gcx);
        let sig = &f.sig;
        let block = &f.block;
        inferer
            .infer_signature_and_block(&mut logic, &mut infer_logic_host, sig, block)
            .unwrap();

        // Is symbol table empty?
        assert!(inferer.symbols.is_empty());

        // let a = x;
        let mut stmt_i = 0;
        let lhs = type_of_local_stmt(&inferer, &block.stmts[stmt_i]);
        assert_eq!(lhs, OwnedType::Named { name: "u32".into(), params: [].into() });
        stmt_i += 1;

        // let b = 0;
        let lhs = type_of_local_stmt(&inferer, &block.stmts[stmt_i]);
        assert_eq!(lhs, OwnedType::Named { name: "u64".into(), params: [].into() });
        stmt_i += 2;

        // let c = 0;
        let lhs = type_of_local_stmt(&inferer, &block.stmts[stmt_i]);
        assert_eq!(lhs, OwnedType::Named { name: "u64".into(), params: [].into() });
    }

    #[test]
    #[rustfmt::skip]
    fn test_infer_references() {
        let code = r#"{
            let a: u32 = 0;
            let b = &a;
            let c = b + 0;
        }"#;

        let block = syn::parse_str::<syn::Block>(code).unwrap();
        let gcx = GlobalCx::default();
        let mut inferer = test_inferer(&gcx);
        let mut logic = test_logic(&gcx);
        let mut infer_logic_host = TestInferLogicHost::new(&gcx);
        inferer.infer_block(&mut logic, &mut infer_logic_host, &block, None).unwrap();

        // Is symbol table empty?
        assert!(inferer.symbols.is_empty());

        // let b = &a;
        let mut stmt_i = 1;
        let lhs = type_of_local_stmt(&inferer, &block.stmts[stmt_i]);
        let OwnedType::Ref { elem, .. } = lhs else {
            panic!("{lhs:?} is not a reference");
        };
        assert_eq!(*elem, OwnedType::Named { name: "u32".into(), params: [].into() });
        stmt_i += 1;

        // let c = b + 0;
        let lhs = type_of_local_stmt(&inferer, &block.stmts[stmt_i]);
        assert_eq!(lhs, OwnedType::Named { name: "u32".into(), params: [].into() });
    }

    #[test]
    #[rustfmt::skip]
    fn test_infer_etc() {
        let code = r#"{
            // Literal only
            let a = 0;
            // Suffix
            let a = 0_u32;
            // Explicit type 
            let a: u32 = 0;
            // Array
            let a = [0; 1];
            // Empty array
            let a = [];
            let b: &[u32; 0] = &a;
            // Tuple
            let a = (0_u32, 1);
            // Determined by the following statement
            let mut a = 0;
            a = 0_u32;
            // Determined by nested following statements
            let a = 0;
            let b = a + 0;
            let c = b + 0_u32;
            // Still determined as a abstract number.
            let mut a = 0;
            a = 0;
            // 'a' and 'b' are determined at the same time
            let mut a = 0;
            let b = a + 0;
            a = 0_u32 + b;
            // Inference inside a block
            { let a = 0_u32; }
        }"#;

        let block = syn::parse_str::<syn::Block>(code).unwrap();
        let gcx = GlobalCx::default();
        let mut inferer = test_inferer(&gcx);
        let mut logic = test_logic(&gcx);
        let mut infer_logic_host = TestInferLogicHost::new(&gcx);
        inferer.infer_block(&mut logic, &mut infer_logic_host, &block, None).unwrap();

        // Is symbol table empty?
        assert!(inferer.symbols.is_empty());

        // let a = 0;
        let mut stmt_i = 0;
        let lhs = type_of_local_stmt(&inferer, &block.stmts[stmt_i]);
        assert_eq!(lhs, OwnedType::Named { name: "int".into(), params: [].into() });
        stmt_i += 1;

        // let a = 0_u32;
        let lhs = type_of_local_stmt(&inferer, &block.stmts[stmt_i]);
        assert_eq!(lhs, OwnedType::Named { name: "u32".into(), params: [].into() });
        stmt_i += 1;

        // let a: u32 = 0;
        let lhs = type_of_local_stmt_with_type(&inferer, &block.stmts[stmt_i]);
        assert_eq!(lhs, OwnedType::Named { name: "u32".into(), params: [].into() });
        stmt_i += 1;

        // let a = [0; 1];
        let lhs = type_of_local_stmt(&inferer, &block.stmts[stmt_i]);
        let OwnedType::Array { elem, len } = lhs else {
            panic!("{lhs:?} is not an array");
        };
        assert_eq!(*elem, OwnedType::Named { name: "int".into(), params: [].into() });
        assert_eq!(len, InferArrayLen::Fixed(1));
        stmt_i += 1;

        // let a = [];
        // let b: &[u32; 0] = &a;
        let lhs = type_of_local_stmt(&inferer, &block.stmts[stmt_i]);
        let OwnedType::Array { elem, len } = lhs else {
            panic!("{lhs:?} is not an array");
        };
        assert_eq!(*elem, OwnedType::Named { name: "u32".into(), params: [].into() });
        assert_eq!(len, InferArrayLen::Fixed(0));
        stmt_i += 2;

        // let a = (0_u32, 1);
        let lhs = type_of_local_stmt(&inferer, &block.stmts[stmt_i]);
        let OwnedType::Tuple(elems) = lhs else {
            panic!("{lhs:?} is not a tuple");
        };
        assert_eq!(
            *elems,
            [
                OwnedType::Named { name: "u32".into(), params: [].into() },
                OwnedType::Named { name: "int".into(), params: [].into() },
            ]
        );
        stmt_i += 1;

        // let mut a = 0; a = 0_u32;
        let lhs = type_of_local_stmt(&inferer, &block.stmts[stmt_i]);
        assert_eq!(lhs, OwnedType::Named { name: "u32".into(), params: [].into() });
        stmt_i += 2;

        // let a = 0; let b = a + 0; let c = b + 0_u32;
        let a = type_of_local_stmt(&inferer, &block.stmts[stmt_i]);
        assert_eq!(a, OwnedType::Named { name: "u32".into(), params: [].into() });
        stmt_i += 3;

        // let mut a = 0; a = 0;
        let a = type_of_local_stmt(&inferer, &block.stmts[stmt_i]);
        assert_eq!(a, OwnedType::Named { name: "int".into(), params: [].into() });
        stmt_i += 2;

        // let mut a = 0; let b = a + 0; a = 0_u32 + b;
        let a = type_of_local_stmt(&inferer, &block.stmts[stmt_i]);
        let b = type_of_local_stmt(&inferer, &block.stmts[stmt_i + 1]);
        assert_eq!(a, OwnedType::Named { name: "u32".into(), params: [].into() });
        assert_eq!(b, OwnedType::Named { name: "u32".into(), params: [].into() });
        stmt_i += 3;

        // { let a = 0_u32; }
        let syn::Stmt::Expr(syn::Expr::Block(syn::ExprBlock { block: inner_block, .. }), _) = 
            &block.stmts[stmt_i] else { unreachable!() };
        let lhs = type_of_local_stmt(&inferer, &inner_block.stmts[0]);
        assert_eq!(lhs, OwnedType::Named { name: "u32".into(), params: [].into() });
    }

    #[test]
    fn test_infer_various_pat() {
        let code = r#"{
            // Struct pattern
            struct T { i: i32, u: u32 }
            fn f1(T { i, u }: T) {}
            let T { i, .. } = T { i: 0, u: 0 };
            let T { u, .. } = T { i: 0, u: 0 };
            // Tuple pattern
            let (i, u) = (0_i16, 0_u16);
            let (i, u, .., b, f) = (0_i8, 0_u8, 0, 0, true, 0_f32);
            // Slice pattern
            let [a, b] = [0_i8, 1];
            let [a, ..] = [0_u8, 1, 2];
        }"#;

        trait InferLogicHost<'gcx>: Host<'gcx> + logic::Host<'gcx> {}

        impl<'gcx, T> InferLogicHost<'gcx> for T where T: Host<'gcx> + logic::Host<'gcx> {}

        struct TestHost<'gcx> {
            gcx: &'gcx GlobalCx<'gcx>,
            inner: Box<dyn InferLogicHost<'gcx> + 'gcx>,
        }

        impl<'gcx> Host<'gcx> for TestHost<'gcx> {
            fn syn_path_to_type(
                &mut self,
                p: SynPath,
                types: &mut UniqueTypes,
            ) -> TriResult<Type<'gcx>, ()> {
                let ident = p.path.get_ident().unwrap().to_string();
                if ident == "T" {
                    let tid_i32 = types.insert_type(Type::Scalar(TypeScalar::I32));
                    let tid_u32 = types.insert_type(Type::Scalar(TypeScalar::U32));
                    let i = Param::Other {
                        name: self.gcx.intern_str("i"),
                        tid: tid_i32,
                    };
                    let u = Param::Other {
                        name: self.gcx.intern_str("u"),
                        tid: tid_u32,
                    };
                    let res = Type::Named(TypeNamed {
                        name: self.gcx.intern_str("T"),
                        params: [i, u].into(),
                    });
                    Ok(res)
                } else {
                    unreachable!()
                }
            }
        }

        impl<'gcx> logic::Host<'gcx> for TestHost<'gcx> {
            fn ident_to_npath(&mut self, ident: &syn::Ident) -> TriResult<String, ()> {
                logic::Host::ident_to_npath(&mut *self.inner, ident)
            }
        }

        impl<'gcx> EvaluateArrayLength<'gcx> for TestHost<'gcx> {
            fn eval_array_len(&mut self, expr: &syn::Expr) -> TriResult<crate::ArrayLen, ()> {
                EvaluateArrayLength::eval_array_len(&mut *self.inner, expr)
            }
        }

        crate::impl_empty_method_host!(TestHost<'_>);
        crate::impl_empty_scoping!(TestHost<'_>);

        let top_block = syn::parse_str::<syn::Block>(code).unwrap();

        let pinned = Pin::new(&top_block);
        pinned.locate_as_entry(crate::cur_path!(), code).unwrap();

        let gcx = GlobalCx::default();
        let mut inferer = test_inferer(&gcx);
        let mut logic = test_logic(&gcx);
        let mut infer_logic_host = TestHost {
            gcx: &gcx,
            inner: Box::new(TestInferLogicHost::new(&gcx)),
        };
        inferer
            .infer_block(&mut logic, &mut infer_logic_host, &top_block, None)
            .unwrap();

        // stmt0: struct T { i: i32, u: u32 }
        // stmt1: fn f1(T { i, u }: T) {}
        let mut stmt_i = 1;
        let syn::Stmt::Item(syn::Item::Fn(item_fn)) = &top_block.stmts[stmt_i] else {
            unreachable!()
        };
        let i = type_of_ident(&inferer, &item_fn.sig, "i");
        assert_eq!(
            i,
            OwnedType::Named {
                name: "i32".into(),
                params: [].into()
            }
        );
        let u = type_of_ident(&inferer, &item_fn.sig, "u");
        assert_eq!(
            u,
            OwnedType::Named {
                name: "u32".into(),
                params: [].into()
            }
        );
        stmt_i += 1;

        // let T { i, .. } = T { i: 0, u: 0 };
        let i = type_of_ident(&inferer, &top_block.stmts[stmt_i], "i");
        assert_eq!(
            i,
            OwnedType::Named {
                name: "i32".into(),
                params: [].into()
            }
        );
        stmt_i += 1;

        // let T { u, .. } = T { i: 0, u: 0 };
        let u = type_of_ident(&inferer, &top_block.stmts[stmt_i], "u");
        assert_eq!(
            u,
            OwnedType::Named {
                name: "u32".into(),
                params: [].into()
            }
        );
        stmt_i += 1;

        // let (i, u) = (0_i16, 0_u16);
        let i = type_of_ident(&inferer, &top_block.stmts[stmt_i], "i");
        assert_eq!(
            i,
            OwnedType::Named {
                name: "i16".into(),
                params: [].into()
            }
        );
        let u = type_of_ident(&inferer, &top_block.stmts[stmt_i], "u");
        assert_eq!(
            u,
            OwnedType::Named {
                name: "u16".into(),
                params: [].into()
            }
        );
        stmt_i += 1;

        // let (i, u, .., b, f) = (0_i8, 0_u8, 0, 0, true, 0_f32);
        let i = type_of_ident(&inferer, &top_block.stmts[stmt_i], "i");
        assert_eq!(
            i,
            OwnedType::Named {
                name: "i8".into(),
                params: [].into()
            }
        );
        let u = type_of_ident(&inferer, &top_block.stmts[stmt_i], "u");
        assert_eq!(
            u,
            OwnedType::Named {
                name: "u8".into(),
                params: [].into()
            }
        );
        let b = type_of_ident(&inferer, &top_block.stmts[stmt_i], "b");
        assert_eq!(
            b,
            OwnedType::Named {
                name: "bool".into(),
                params: [].into()
            }
        );
        let f = type_of_ident(&inferer, &top_block.stmts[stmt_i], "f");
        assert_eq!(
            f,
            OwnedType::Named {
                name: "f32".into(),
                params: [].into()
            }
        );
        stmt_i += 1;

        // let [a, b] = [0_i8, 1];
        let a = type_of_ident(&inferer, &top_block.stmts[stmt_i], "a");
        assert_eq!(
            a,
            OwnedType::Named {
                name: "i8".into(),
                params: [].into()
            }
        );
        let b = type_of_ident(&inferer, &top_block.stmts[stmt_i], "b");
        assert_eq!(
            b,
            OwnedType::Named {
                name: "i8".into(),
                params: [].into()
            }
        );
        let expr: &syn::Expr = top_block.stmts[stmt_i].find("1").unwrap();
        let one = inferer.get_owned_type_of_expr(expr).unwrap();
        assert_eq!(
            one,
            OwnedType::Named {
                name: "i8".into(),
                params: [].into()
            }
        );
        stmt_i += 1;

        // let [a, ..] = [0_u8, 1, 2];
        let a = type_of_ident(&inferer, &top_block.stmts[stmt_i], "a");
        assert_eq!(
            a,
            OwnedType::Named {
                name: "u8".into(),
                params: [].into()
            }
        );
    }

    #[test]
    #[rustfmt::skip]
    fn test_infer_expr() {
        let code = "1_u32 + 2";

        let expr = syn::parse_str::<syn::Expr>(code).unwrap();
        let gcx = GlobalCx::default();
        let mut inferer = test_inferer(&gcx);
        let mut logic = test_logic(&gcx);
        let mut infer_logic_host = TestInferLogicHost::new(&gcx);
        inferer.infer_expr(&mut logic, &mut infer_logic_host, &expr, None).unwrap();

        // Is symbol table empty?
        assert!(inferer.symbols.is_empty());

        let ty = inferer.get_owned_type_of_expr(&expr).unwrap();
        assert_eq!(ty, OwnedType::Named { name: "u32".into(), params: [].into() });
    }
}
