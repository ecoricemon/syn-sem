pub mod logic {
    use crate::{
        semantic::{
            entry::GlobalCx,
            eval::{Evaluated, Value},
            logic::{name_to_ptr, term, var_name, ImplLogic},
            tree::{ArrayLen, AsPrivPathTree, NodeIndex, PrivPathTree, Type, TypeId, TypeScalar},
        },
        NameIn,
    };
    use logic_eval::{Expr, Name, Term, VAR_PREFIX};

    pub fn type_to_term<'gcx, T: AsPrivPathTree<'gcx>>(
        ptree: &T,
        tid: TypeId,
        gcx: &'gcx GlobalCx<'gcx>,
    ) -> Term<NameIn<'gcx>> {
        TypeToTermCx {
            gcx,
            ptree: ptree.as_private_path_tree(),
            var_ident: 0,
        }
        .type_to_term(tid)
    }

    pub fn has_trait<'gcx, T: AsPrivPathTree<'gcx>>(
        impl_logic: &mut ImplLogic<'gcx>,
        ptree: &T,
        self_ty: TypeId,
        trait_node: NodeIndex,
        gcx: &'gcx GlobalCx<'gcx>,
    ) -> bool {
        let ptree = ptree.as_private_path_tree();
        let self_ty = type_to_term(ptree, self_ty, gcx);
        let trait_npath = ptree.get_name_path(trait_node);
        let trait_ = term::path_to_list(trait_npath.as_str(), gcx);
        let query_term = term::impl_2(self_ty, trait_, gcx);
        let query = Expr::Term(query_term);

        impl_logic.query(query).prove_next().is_some()
    }

    pub fn get_trait_assoc_const_value<'e, 'gcx, T: AsPrivPathTree<'gcx>>(
        impl_logic: &mut ImplLogic<'gcx>,
        ptree: &T,
        evaluated: &'e Evaluated<'gcx>,
        self_ty: TypeId,
        trait_node: NodeIndex,
        const_ident: &str,
        gcx: &'gcx GlobalCx<'gcx>,
    ) -> Option<&'e Value<'gcx>> {
        const CONST_ID_VAR: &str = "$X";
        const _: () = assert!(VAR_PREFIX == '$');

        // Makes a query for `impl_logic`.
        let ptree = ptree.as_private_path_tree();
        let self_ty = type_to_term(ptree, self_ty, gcx);
        let trait_npath = ptree.get_name_path(trait_node);
        let trait_ = term::path_to_list(trait_npath.as_str(), gcx);
        let const_ident = Term {
            functor: Name::with_intern(const_ident, gcx),
            args: [term::arg_n([].into(), gcx)].into(),
        };
        let const_id = Term {
            functor: Name::with_intern(CONST_ID_VAR, gcx),
            args: [].into(),
        };
        let query_term = term::assoc_const_val_4(self_ty, trait_, const_ident, const_id, gcx);
        let query = Expr::Term(query_term);

        // Finds evaluated value through the `impl_logic`.
        let mut best_match: Option<&Value> = None;
        let mut prove_cx = impl_logic.query(query);
        while let Some(eval) = prove_cx.prove_next() {
            let assign = eval
                .into_iter()
                .find(|assign| assign.get_lhs_variable().as_ref() == CONST_ID_VAR)
                .unwrap();
            let value_id = assign.rhs();
            let expr_ptr = name_to_ptr::<syn::Expr>(&value_id.functor).unwrap();
            let value = evaluated.get_mapped_value_by_expr_ptr(expr_ptr).unwrap();

            // If we have two answers, const generic and non const generic, we choose more concrete
            // answer.
            if let Some(best) = best_match {
                match (
                    best.contains_const_generic(),
                    value.contains_const_generic(),
                ) {
                    (true, false) => best_match = Some(value),
                    (false, true) => {}
                    (true, true) | (false, false) if best == value => {
                        // Monomorphization can make this happen. Assume we have code looking like
                        // `impl<N> .. { const A: i32 = N; const B: i32 = 1; }`
                        // When we monomorphize the impl block on "N" to a concrete value "0", we
                        // will have monomorphized block `{ A = 0; B = 1 }`. "B" here is duplicated
                    }
                    (true, true) | (false, false) => unreachable!(),
                }
            } else {
                best_match = Some(value);
            }
        }

        best_match
    }

    struct TypeToTermCx<'a, 'gcx> {
        gcx: &'gcx GlobalCx<'gcx>,
        ptree: &'a PrivPathTree<'gcx>,
        var_ident: u32,
    }

    impl<'gcx> TypeToTermCx<'_, 'gcx> {
        fn type_to_term(&self, tid: TypeId) -> Term<NameIn<'gcx>> {
            match &self.ptree.types()[tid] {
                Type::Scalar(scalar) => self.scalar_to_term(scalar),
                Type::Path(path) => {
                    let npath = self.ptree.get_name_path(path.pid.ni);
                    term::path_to_list(npath.as_str(), self.gcx)
                }
                Type::Tuple(tuple) => {
                    let elems = tuple
                        .elems
                        .iter()
                        .map(|elem| self.type_to_term(*elem))
                        .collect();
                    term::tuple_n(elems, self.gcx)
                }
                Type::Array(arr) => {
                    let elem = self.type_to_term(arr.elem);
                    match arr.len {
                        ArrayLen::Fixed(len) => {
                            let len = Term {
                                functor: Name::with_intern(&len.to_string(), self.gcx),
                                args: [].into(),
                            };
                            term::array_2(elem, len, self.gcx)
                        }
                        ArrayLen::Dynamic => term::array_1(elem, self.gcx),
                        ArrayLen::Generic => {
                            let var = Term {
                                functor: var_name(&self.var_ident, self.gcx),
                                args: [].into(),
                            };
                            term::array_2(elem, var, self.gcx)
                        }
                    }
                }
                Type::Ref(ref_) => {
                    let elem = self.type_to_term(ref_.elem);
                    term::ref_1(elem, self.gcx)
                }
                Type::Mut(mut_) => {
                    let elem = self.type_to_term(mut_.elem);
                    term::mut_1(elem, self.gcx)
                }
                Type::Unit => term::unit_0(self.gcx),
            }
        }

        fn scalar_to_term(&self, scalar: &TypeScalar) -> Term<NameIn<'gcx>> {
            fn functor_only<'gcx>(functor: &str, gcx: &'gcx GlobalCx<'gcx>) -> Term<NameIn<'gcx>> {
                Term {
                    functor: Name::with_intern(functor, gcx),
                    args: [].into(),
                }
            }

            match scalar {
                TypeScalar::Int => {
                    let int = Term {
                        functor: var_name(&self.var_ident, self.gcx),
                        args: [].into(),
                    };
                    term::int_1(int, self.gcx)
                }
                TypeScalar::Float => {
                    let float = Term {
                        functor: var_name(&self.var_ident, self.gcx),
                        args: [].into(),
                    };
                    term::float_1(float, self.gcx)
                }
                TypeScalar::I8 => {
                    let int = functor_only("i8", self.gcx);
                    term::int_1(int, self.gcx)
                }
                TypeScalar::I16 => {
                    let int = functor_only("i16", self.gcx);
                    term::int_1(int, self.gcx)
                }
                TypeScalar::I32 => {
                    let int = functor_only("i32", self.gcx);
                    term::int_1(int, self.gcx)
                }
                TypeScalar::I64 => {
                    let int = functor_only("i64", self.gcx);
                    term::int_1(int, self.gcx)
                }
                TypeScalar::I128 => {
                    let int = functor_only("i128", self.gcx);
                    term::int_1(int, self.gcx)
                }
                TypeScalar::Isize => {
                    let int = functor_only("isize", self.gcx);
                    term::int_1(int, self.gcx)
                }
                TypeScalar::U8 => {
                    let int = functor_only("u8", self.gcx);
                    term::int_1(int, self.gcx)
                }
                TypeScalar::U16 => {
                    let int = functor_only("u16", self.gcx);
                    term::int_1(int, self.gcx)
                }
                TypeScalar::U32 => {
                    let int = functor_only("u32", self.gcx);
                    term::int_1(int, self.gcx)
                }
                TypeScalar::U64 => {
                    let int = functor_only("u64", self.gcx);
                    term::int_1(int, self.gcx)
                }
                TypeScalar::U128 => {
                    let int = functor_only("u128", self.gcx);
                    term::int_1(int, self.gcx)
                }
                TypeScalar::Usize => {
                    let int = functor_only("usize", self.gcx);
                    term::int_1(int, self.gcx)
                }
                TypeScalar::F32 => {
                    let float = functor_only("f32", self.gcx);
                    term::float_1(float, self.gcx)
                }
                TypeScalar::F64 => {
                    let float = functor_only("f64", self.gcx);
                    term::float_1(float, self.gcx)
                }
                TypeScalar::Bool => functor_only("bool", self.gcx),
            }
        }
    }
}

// TODO
// * We should check if `syn::Path` is a generic parameter or something else.
//   e.g.
//   fn foo<const N: usize>() {
//      let N: bool = true;
//      1 + N // This is a value 'true' of type 'bool', not generic param.
//   }
//   This is a problem when we check if '1 + N' contains generic params, but 'N' is not a generic
//   param, but we are not processing correctly.
//   - Precedence considerations
//     1. How to know whether 'N' is overwritten. That is not in the path tree.
//     2. When we visit paths in an expr, maybe we need to distinguish type & value namespaces
//        because overwriting happens in each namespace.
pub(crate) mod generic {
    use super::path;
    use crate::syntax::{
        common::{IdentifySyn, SynId},
        SyntaxTree,
    };
    use std::any;

    /// Returns true if the given expression contains const generic parameters.
    ///
    /// e.g.
    /// * code - impl<const N: usize> .. { .. N + 1 .. }
    /// * input - 'N + 1'
    /// * output - true
    pub(crate) fn contains_const_generic_param_in_expr(
        expr: &syn::Expr,
        stree: &SyntaxTree,
    ) -> bool {
        match expr {
            syn::Expr::Array(arr) => arr
                .elems
                .iter()
                .any(|elem| contains_const_generic_param_in_expr(elem, stree)),
            syn::Expr::Assign(assign) => {
                contains_const_generic_param_in_expr(&assign.left, stree)
                    || contains_const_generic_param_in_expr(&assign.right, stree)
            }
            syn::Expr::Async(_) => panic!("`async` is not supported"),
            syn::Expr::Await(_) => panic!("`await` is not supported"),
            syn::Expr::Binary(bin) => {
                contains_const_generic_param_in_expr(&bin.left, stree)
                    || contains_const_generic_param_in_expr(&bin.right, stree)
            }
            syn::Expr::Block(block) => block
                .block
                .stmts
                .iter()
                .any(|stmt| contains_const_generic_param_in_stmt(stmt, stree)),
            syn::Expr::Break(break_) => {
                if let Some(expr) = break_.expr.as_ref() {
                    contains_const_generic_param_in_expr(expr, stree)
                } else {
                    false
                }
            }
            syn::Expr::Call(call) => {
                contains_const_generic_param_in_expr(&call.func, stree)
                    || call
                        .args
                        .iter()
                        .any(|arg| contains_const_generic_param_in_expr(arg, stree))
            }
            syn::Expr::Cast(cast) => {
                contains_const_generic_param_in_expr(&cast.expr, stree)
                    || contains_const_generic_param_in_type(&cast.ty, stree)
            }
            syn::Expr::Closure(_) => panic!("`closure` is not supported"),
            syn::Expr::Const(const_) => const_
                .block
                .stmts
                .iter()
                .any(|stmt| contains_const_generic_param_in_stmt(stmt, stree)),
            syn::Expr::Let(let_) => {
                contains_const_generic_param_in_pat(&let_.pat, stree)
                    || contains_const_generic_param_in_expr(&let_.expr, stree)
            }
            syn::Expr::Lit(_) => false,
            syn::Expr::Loop(loop_) => loop_
                .body
                .stmts
                .iter()
                .any(|stmt| contains_const_generic_param_in_stmt(stmt, stree)),
            syn::Expr::Macro(_) => panic!("macro is not supported"),
            syn::Expr::Match(match_) => {
                let contains_in_expr = contains_const_generic_param_in_expr(&match_.expr, stree);

                let contains_in_arms = match_.arms.iter().any(|arm| {
                    let contains_in_pat = contains_const_generic_param_in_pat(&arm.pat, stree);

                    let contains_in_guard = if let Some((_, expr)) = arm.guard.as_ref() {
                        contains_const_generic_param_in_expr(expr, stree)
                    } else {
                        false
                    };

                    let contains_in_body = contains_const_generic_param_in_expr(&arm.body, stree);

                    contains_in_pat || contains_in_guard || contains_in_body
                });

                contains_in_expr || contains_in_arms
            }
            syn::Expr::MethodCall(method_call) => {
                // Ignores turbofish for now.
                contains_const_generic_param_in_expr(&method_call.receiver, stree)
                    || method_call
                        .args
                        .iter()
                        .any(|arg| contains_const_generic_param_in_expr(arg, stree))
            }
            syn::Expr::Paren(paren) => contains_const_generic_param_in_expr(&paren.expr, stree),
            syn::Expr::Path(path) => is_const_generic_param(&path.path, stree),
            syn::Expr::Range(range) => {
                let contains_in_start = if let Some(expr) = range.start.as_ref() {
                    contains_const_generic_param_in_expr(expr, stree)
                } else {
                    false
                };

                let contains_in_end = if let Some(expr) = range.end.as_ref() {
                    contains_const_generic_param_in_expr(expr, stree)
                } else {
                    false
                };

                contains_in_start || contains_in_end
            }
            syn::Expr::RawAddr(raw_addr) => {
                contains_const_generic_param_in_expr(&raw_addr.expr, stree)
            }
            syn::Expr::Reference(ref_) => contains_const_generic_param_in_expr(&ref_.expr, stree),
            syn::Expr::Repeat(repeat) => {
                contains_const_generic_param_in_expr(&repeat.expr, stree)
                    || contains_const_generic_param_in_expr(&repeat.len, stree)
            }
            syn::Expr::Return(ret) => {
                if let Some(expr) = ret.expr.as_ref() {
                    contains_const_generic_param_in_expr(expr, stree)
                } else {
                    false
                }
            }
            syn::Expr::Struct(v) => todo!("{v:#?}"),
            syn::Expr::Try(v) => todo!("{v:#?}"),
            syn::Expr::TryBlock(v) => todo!("{v:#?}"),
            syn::Expr::Tuple(v) => todo!("{v:#?}"),
            syn::Expr::Unary(unary) => contains_const_generic_param_in_expr(&unary.expr, stree),
            syn::Expr::Unsafe(v) => todo!("{v:#?}"),
            syn::Expr::Verbatim(v) => todo!("{v:#?}"),
            syn::Expr::While(v) => todo!("{v:#?}"),
            syn::Expr::Yield(v) => todo!("{v:#?}"),
            _ => todo!(),
        }
    }

    // Returns true if the statement contains const generic params in it.
    pub(crate) fn contains_const_generic_param_in_stmt(
        _stmt: &syn::Stmt,
        _stree: &SyntaxTree,
    ) -> bool {
        todo!()
    }

    // Returns true if the type contains const generic params in it.
    pub(crate) fn contains_const_generic_param_in_type(
        _ty: &syn::Type,
        _stree: &SyntaxTree,
    ) -> bool {
        todo!()
    }

    // Returns true if the pattern contains const generic params in it.
    pub(crate) fn contains_const_generic_param_in_pat(
        _pat: &syn::Pat,
        _stree: &SyntaxTree,
    ) -> bool {
        todo!()
    }

    /// Finds ancestor syntax nodes that declare generic parameters in the given expression.
    ///
    /// e.g.
    /// * code - impl<const N: usize> .. { .. fn foo<const M: usize>() { .. N + M .. } }
    /// * input - 'N + M'
    /// * output - iterator over 'impl .. { .. }' and 'fn foo .. { .. }' in the order of occurrence
    ///   of generic params in the expression.
    pub(crate) fn find_generic_decls<'a>(
        expr: &'a syn::Expr,
        stree: &'a SyntaxTree,
    ) -> GenericDecls<'a> {
        GenericDecls {
            stree,
            paths: path::paths_in_expr(expr),
        }
    }

    pub(crate) struct GenericDecls<'a> {
        stree: &'a SyntaxTree,
        paths: Box<dyn Iterator<Item = &'a syn::Path> + 'a>,
    }

    impl<'a> Iterator for GenericDecls<'a> {
        type Item = GenericDecl;

        fn next(&mut self) -> Option<Self::Item> {
            for path in self.paths.by_ref() {
                let ret = find_generic_decl(path, self.stree);
                if ret.is_some() {
                    return ret;
                }
            }
            None
        }
    }

    /// Returns true if the given path is a const generic parameter.
    ///
    /// e.g.
    /// * code - impl<const N: usize> .. { .. N .. }
    /// * input - 'N'
    /// * output - true
    pub(crate) fn is_const_generic_param(path: &syn::Path, stree: &SyntaxTree) -> bool {
        find_const_generic_param(path, stree).is_some()
    }

    /// Finds the [`syn::ConstParam`] for the given path.
    ///
    /// The returned syn id can be casted into [`syn::ConstParam`] through [`SynId::as_any`].
    ///
    /// e.g.
    /// * code - impl<const N: usize> .. { .. N .. }
    /// * input - 'N'
    /// * output - syn id to the 'const N: usize'
    pub(crate) fn find_const_generic_param(path: &syn::Path, stree: &SyntaxTree) -> Option<SynId> {
        // Finds `syn::GenericParam` for the given path.
        let decl = find_generic_decl(path, stree)?;
        let generic_param = match decl.downcast() {
            CastedGenericDecl::ItemFn { generic_param, .. } => generic_param,
            CastedGenericDecl::TraitItemFn { generic_param, .. } => generic_param,
            CastedGenericDecl::ImplItemFn { generic_param, .. } => generic_param,
            CastedGenericDecl::ItemImpl { generic_param, .. } => generic_param,
        };

        // Is `syn::ConstParam`? then returns it.
        if let syn::GenericParam::Const(const_param) = generic_param {
            Some(const_param.syn_id())
        } else {
            None
        }
    }

    /// Finds the ancestor syntax node that declares generic parameter for the given path.
    ///
    /// e.g.
    /// * code - impl<const N: usize> .. { .. N .. }
    /// * input - 'N'
    /// * output - info about 'impl .. { .. }' and 'const N: usize'
    pub(crate) fn find_generic_decl(path: &syn::Path, stree: &SyntaxTree) -> Option<GenericDecl> {
        let input_ident = path.get_ident()?;

        // For now, we look for ancestor functions and impl blocks only.
        let tid_item_fn = any::TypeId::of::<syn::ItemFn>();
        let tid_trait_item_fn = any::TypeId::of::<syn::TraitItemFn>();
        let tid_impl_item_fn = any::TypeId::of::<syn::ImplItemFn>();
        let tid_item_impl = any::TypeId::of::<syn::ItemImpl>();

        let mut targets = [
            tid_item_fn,
            tid_trait_item_fn,
            tid_impl_item_fn,
            tid_item_impl,
        ];
        let mut num_targets = targets.len();

        let child = path.syn_id();
        while let Some((i, ancestor)) = stree.get_ancestor(child, &targets[..num_targets]) {
            // If the nearest ancestor dosen't have a generic param for the input, then we remove
            // similar targets because those targets are impossible to be accessed from the input
            // from then on. For example, inside of nested functions cannot see generic params of
            // the outer functions.

            // Generic param in a free-standing function?
            if targets[i] == tid_item_fn {
                let syn = ancestor.as_any().downcast_ref::<syn::ItemFn>().unwrap();
                if let Some(param) = find_generic_param(input_ident, &syn.sig.generics) {
                    return Some(GenericDecl::ItemFn {
                        item_fn: ancestor,
                        generic_param: param.syn_id(),
                    });
                }
                remove_fn_target(&mut targets, &mut num_targets);
            }
            // Generic param in a function in a trait definition?
            else if targets[i] == tid_trait_item_fn {
                let syn = ancestor
                    .as_any()
                    .downcast_ref::<syn::TraitItemFn>()
                    .unwrap();
                if let Some(param) = find_generic_param(input_ident, &syn.sig.generics) {
                    return Some(GenericDecl::TraitItemFn {
                        trait_item_fn: ancestor,
                        generic_param: param.syn_id(),
                    });
                }
                remove_fn_target(&mut targets, &mut num_targets);
            }
            // Generic param in a function in an impl block?
            else if targets[i] == tid_impl_item_fn {
                let syn = ancestor.as_any().downcast_ref::<syn::ImplItemFn>().unwrap();
                if let Some(param) = find_generic_param(input_ident, &syn.sig.generics) {
                    return Some(GenericDecl::ImplItemFn {
                        impl_item_fn: ancestor,
                        generic_param: param.syn_id(),
                    });
                }
                remove_fn_target(&mut targets, &mut num_targets);
            }
            // Generic param in an impl block?
            else if targets[i] == tid_item_impl {
                let syn = ancestor.as_any().downcast_ref::<syn::ItemImpl>().unwrap();
                if let Some(param) = find_generic_param(input_ident, &syn.generics) {
                    return Some(GenericDecl::ItemImpl {
                        item_impl: ancestor,
                        generic_param: param.syn_id(),
                    });
                }
                remove_impl_target(&mut targets, &mut num_targets);
            }
        }

        None
    }

    /// Returns generic parameter declaration of the given ident.
    ///
    /// e.g.
    /// * code - impl<const N: usize> .. { .. N .. }
    /// * input - 'N'
    /// * output - reference to 'const N: usize'
    fn find_generic_param<'g>(
        ident: &syn::Ident,
        generics: &'g syn::Generics,
    ) -> Option<&'g syn::GenericParam> {
        generics.params.iter().find(|param| match param {
            syn::GenericParam::Lifetime(param) => ident == &param.lifetime.ident,
            syn::GenericParam::Type(param) => ident == &param.ident,
            syn::GenericParam::Const(param) => ident == &param.ident,
        })
    }

    fn remove_fn_target(targets: &mut [any::TypeId; 4], num_targets: &mut usize) {
        let removes = [
            any::TypeId::of::<syn::ItemFn>(),
            any::TypeId::of::<syn::TraitItemFn>(),
            any::TypeId::of::<syn::ImplItemFn>(),
        ];
        remove_certain_target(targets, num_targets, &removes);
    }

    fn remove_impl_target(targets: &mut [any::TypeId; 4], num_targets: &mut usize) {
        let removes = [any::TypeId::of::<syn::ItemImpl>()];
        remove_certain_target(targets, num_targets, &removes);
    }

    fn remove_certain_target(
        targets: &mut [any::TypeId; 4],
        num_targets: &mut usize,
        removes: &[any::TypeId],
    ) {
        let mut n = *num_targets;
        let mut i = 0;
        while i < n {
            let target = &targets[i];
            if removes.contains(target) {
                targets.swap(i, n - 1);
                n -= 1;
            }
            i += 1;
        }
        *num_targets = n;
    }

    pub(crate) enum GenericDecl {
        ItemFn {
            /// Syn id to a [`syn::ItemFn`].
            item_fn: SynId,
            /// Syn id to a [`syn::GenericParam`]
            generic_param: SynId,
        },
        TraitItemFn {
            /// Syn id to a [`syn::TraitItemFn`].
            trait_item_fn: SynId,
            /// Syn id to a [`syn::GenericParam`]
            generic_param: SynId,
        },
        ImplItemFn {
            /// Syn id to a [`syn::ImplItemFn`].
            impl_item_fn: SynId,
            /// Syn id to a [`syn::GenericParam`]
            generic_param: SynId,
        },
        ItemImpl {
            /// Syn id to a [`syn::ItemImpl`].
            item_impl: SynId,
            /// Syn id to a [`syn::GenericParam`]
            generic_param: SynId,
        },
    }

    #[allow(dead_code)]
    #[derive(Debug)]
    pub(crate) enum CastedGenericDecl<'a> {
        ItemFn {
            item_fn: &'a syn::ItemFn,
            generic_param: &'a syn::GenericParam,
        },
        TraitItemFn {
            trait_item_fn: &'a syn::TraitItemFn,
            generic_param: &'a syn::GenericParam,
        },
        ImplItemFn {
            impl_item_fn: &'a syn::ImplItemFn,
            generic_param: &'a syn::GenericParam,
        },
        ItemImpl {
            item_impl: &'a syn::ItemImpl,
            generic_param: &'a syn::GenericParam,
        },
    }

    impl GenericDecl {
        pub(crate) fn downcast(&self) -> CastedGenericDecl<'_> {
            match self {
                Self::ItemFn {
                    item_fn,
                    generic_param,
                } => CastedGenericDecl::ItemFn {
                    item_fn: item_fn.as_any().downcast_ref().unwrap(),
                    generic_param: generic_param.as_any().downcast_ref().unwrap(),
                },
                Self::TraitItemFn {
                    trait_item_fn,
                    generic_param,
                } => CastedGenericDecl::TraitItemFn {
                    trait_item_fn: trait_item_fn.as_any().downcast_ref().unwrap(),
                    generic_param: generic_param.as_any().downcast_ref().unwrap(),
                },
                Self::ImplItemFn {
                    impl_item_fn,
                    generic_param,
                } => CastedGenericDecl::ImplItemFn {
                    impl_item_fn: impl_item_fn.as_any().downcast_ref().unwrap(),
                    generic_param: generic_param.as_any().downcast_ref().unwrap(),
                },
                Self::ItemImpl {
                    item_impl,
                    generic_param,
                } => CastedGenericDecl::ItemImpl {
                    item_impl: item_impl.as_any().downcast_ref().unwrap(),
                    generic_param: generic_param.as_any().downcast_ref().unwrap(),
                },
            }
        }
    }
}

pub(crate) mod path {
    use std::iter;

    /// Returns iterator traversing all paths in the given expression.
    pub(crate) fn paths_in_expr<'a>(
        expr: &'a syn::Expr,
    ) -> Box<dyn Iterator<Item = &'a syn::Path> + 'a> {
        match expr {
            syn::Expr::Array(arr) => {
                let iter_elems = arr.elems.iter().flat_map(|elem| paths_in_expr(elem));
                Box::new(iter_elems)
            }
            syn::Expr::Assign(assign) => {
                let iter_left = paths_in_expr(&assign.left);
                let iter_right = paths_in_expr(&assign.right);
                Box::new(iter_left.chain(iter_right))
            }
            syn::Expr::Async(_) => panic!("`async` is not supported"),
            syn::Expr::Await(_) => panic!("`await` is not supported"),
            syn::Expr::Binary(bin) => {
                let iter_left = paths_in_expr(&bin.left);
                let iter_right = paths_in_expr(&bin.right);
                Box::new(iter_left.chain(iter_right))
            }
            syn::Expr::Block(block) => {
                let iter_block = block
                    .block
                    .stmts
                    .iter()
                    .flat_map(|stmt| paths_in_stmt(stmt));
                Box::new(iter_block)
            }
            syn::Expr::Break(break_) => break_
                .expr
                .as_ref()
                .map(|expr| paths_in_expr(expr))
                .unwrap_or_else(|| Box::new(iter::empty())),
            syn::Expr::Call(call) => {
                let iter_func = paths_in_expr(&call.func);
                let iter_args = call.args.iter().flat_map(|arg| paths_in_expr(arg));
                Box::new(iter_func.chain(iter_args))
            }
            syn::Expr::Cast(cast) => {
                let iter_expr = paths_in_expr(&cast.expr);
                let iter_ty = paths_in_type(&cast.ty);
                Box::new(iter_expr.chain(iter_ty))
            }
            syn::Expr::Closure(_) => panic!("`closure` is not supported"),
            syn::Expr::Const(const_) => {
                let iter_block = const_
                    .block
                    .stmts
                    .iter()
                    .flat_map(|stmt| paths_in_stmt(stmt));
                Box::new(iter_block)
            }
            syn::Expr::Let(let_) => {
                let iter_pat = paths_in_pat(&let_.pat);
                let iter_expr = paths_in_expr(&let_.expr);
                Box::new(iter_pat.chain(iter_expr))
            }
            syn::Expr::Lit(_) => Box::new(iter::empty()),
            syn::Expr::Loop(loop_) => {
                let iter_body = loop_.body.stmts.iter().flat_map(|stmt| paths_in_stmt(stmt));
                Box::new(iter_body)
            }
            syn::Expr::Macro(_) => panic!("macro is not supported"),
            syn::Expr::Match(match_) => {
                let iter_expr = paths_in_expr(&match_.expr);
                let iter_arms = match_.arms.iter().flat_map(|arm| {
                    let iter_pat = paths_in_pat(&arm.pat);
                    let iter_guard = arm
                        .guard
                        .as_ref()
                        .map(|(_, guard)| paths_in_expr(guard))
                        .unwrap_or_else(|| Box::new(iter::empty()));
                    let iter_body = paths_in_expr(&arm.body);
                    iter_pat.chain(iter_guard).chain(iter_body)
                });
                Box::new(iter_expr.chain(iter_arms))
            }
            syn::Expr::MethodCall(method_call) => {
                // Ignores turbofish for now.
                let iter_recv = paths_in_expr(&method_call.receiver);
                let iter_args = method_call.args.iter().flat_map(|arg| paths_in_expr(arg));
                Box::new(iter_recv.chain(iter_args))
            }
            syn::Expr::Paren(paren) => paths_in_expr(&paren.expr),
            syn::Expr::Path(path) => Box::new(iter::once(&path.path)),
            syn::Expr::Range(range) => {
                let iter_start = range
                    .start
                    .as_ref()
                    .map(|start| paths_in_expr(start))
                    .unwrap_or_else(|| Box::new(iter::empty()));
                let iter_end = range
                    .end
                    .as_ref()
                    .map(|end| paths_in_expr(end))
                    .unwrap_or_else(|| Box::new(iter::empty()));
                Box::new(iter_start.chain(iter_end))
            }
            syn::Expr::RawAddr(raw_addr) => paths_in_expr(&raw_addr.expr),
            syn::Expr::Reference(ref_) => paths_in_expr(&ref_.expr),
            syn::Expr::Repeat(repeat) => {
                let iter_expr = paths_in_expr(&repeat.expr);
                let iter_len = paths_in_expr(&repeat.len);
                Box::new(iter_expr.chain(iter_len))
            }
            syn::Expr::Return(ret) => ret
                .expr
                .as_ref()
                .map(|expr| paths_in_expr(expr))
                .unwrap_or_else(|| Box::new(iter::empty())),
            syn::Expr::Struct(v) => todo!("{v:#?}"),
            syn::Expr::Try(v) => todo!("{v:#?}"),
            syn::Expr::TryBlock(v) => todo!("{v:#?}"),
            syn::Expr::Tuple(v) => todo!("{v:#?}"),
            syn::Expr::Unary(unary) => paths_in_expr(&unary.expr),
            syn::Expr::Unsafe(v) => todo!("{v:#?}"),
            syn::Expr::Verbatim(v) => todo!("{v:#?}"),
            syn::Expr::While(v) => todo!("{v:#?}"),
            syn::Expr::Yield(v) => todo!("{v:#?}"),
            _ => todo!(),
        }
    }

    pub(crate) fn paths_in_stmt<'a>(
        _stmt: &'a syn::Stmt,
    ) -> Box<dyn Iterator<Item = &'a syn::Path> + 'a> {
        todo!()
    }

    pub(crate) fn paths_in_type<'a>(
        _ty: &'a syn::Type,
    ) -> Box<dyn Iterator<Item = &'a syn::Path> + 'a> {
        todo!()
    }

    pub(crate) fn paths_in_pat<'a>(
        _pat: &'a syn::Pat,
    ) -> Box<dyn Iterator<Item = &'a syn::Path> + 'a> {
        todo!()
    }
}

pub mod trait_ {
    use super::{
        generic::{self, GenericDecl},
        logic,
    };
    use crate::{
        etc::util::IntoPathSegments,
        semantic::{
            analyze::Semantics,
            entry::GlobalCx,
            eval::{self, ConstGeneric},
            tree,
        },
    };

    /// Finds associated const value.
    ///
    /// Returns the value if the value is already concrete. Otherwise, tries monomorphization about
    /// the block of the const value then returns monomorphized value.
    pub fn find_assoc_const_value<'a, 'gcx, I>(
        self_ty: tree::TypeId,
        trait_: I,
        const_ident: &str,
        sem: &'a mut Semantics<'gcx>,
        gcx: &'gcx GlobalCx<'gcx>,
    ) -> Option<&'a eval::Value<'gcx>>
    where
        I: IntoPathSegments,
    {
        // Finds trait associated const value, but it may not concrete yet.
        let trait_node = sem.ptree.search(crate::PathTree::ROOT, trait_)?;
        let value = logic::get_trait_assoc_const_value(
            &mut sem.logic.impl_,
            &sem.ptree,
            &sem.evaluated,
            self_ty,
            trait_node,
            const_ident,
            gcx,
        )?;

        // If the value is not a const generic, then we can return it.
        if !value.contains_const_generic() {
            // Safety: Returning lifetime `a makes borrowing `sem` is maintained till end of
            // function. But we're going to return here and there is no more access to `sem`.
            let value = unsafe {
                let ptr = value as *const eval::Value;
                ptr.as_ref().unwrap_unchecked()
            };
            return Some(value);
        }

        // NOTE: For now, we assume that there is only one generic param in the value. If not, we
        // need to monomorphize nested block in some way.

        // The value contains const generic in it. We need monomorphization.
        let ConstGeneric { expr, .. } = value.iter_const_generic().next().unwrap();
        let expr = unsafe { expr.as_ref().unwrap() };
        let generic_decl = generic::find_generic_decls(expr, &sem.stree)
            .next()
            .unwrap();
        match generic_decl {
            GenericDecl::ItemFn { .. } => todo!(),
            GenericDecl::TraitItemFn { .. } => todo!(),
            GenericDecl::ImplItemFn { .. } => todo!(),
            GenericDecl::ItemImpl { item_impl, .. } => {
                sem.monomorphize_impl(item_impl, Some(self_ty)).unwrap();
            }
        }

        // Retries, then the value will not be const generic any longer.
        let value = logic::get_trait_assoc_const_value(
            &mut sem.logic.impl_,
            &sem.ptree,
            &sem.evaluated,
            self_ty,
            trait_node,
            const_ident,
            gcx,
        )?;
        assert!(!value.contains_const_generic());
        Some(value)
    }
}

pub mod ptree {
    use crate::{
        semantic::tree::{NodeIndex, SynToPath},
        syntax::{common::SynId, SyntaxTree},
    };
    use std::any;

    /// Finds `base` node, which is used when searching path tree, for the given expression.
    pub fn find_base_node_of_expr(
        stree: &SyntaxTree,
        s2p: &SynToPath,
        expr: SynId,
    ) -> Option<NodeIndex> {
        // Targets may have the given expression as its descendant.
        let target_ancestors = [
            any::TypeId::of::<syn::Block>(),
            any::TypeId::of::<syn::ItemMod>(),
            any::TypeId::of::<syn::File>(),
        ];
        find_base_node(stree, s2p, expr, &target_ancestors)
    }

    pub fn find_base_node_of_item_impl(
        stree: &SyntaxTree,
        s2p: &SynToPath,
        item_impl: SynId,
    ) -> Option<NodeIndex> {
        // Targets may have the given impl block as its descendant.
        let target_ancestors = [
            any::TypeId::of::<syn::ItemMod>(),
            any::TypeId::of::<syn::File>(),
        ];
        find_base_node(stree, s2p, item_impl, &target_ancestors)
    }

    fn find_base_node(
        stree: &SyntaxTree,
        s2p: &SynToPath,
        sid: SynId,
        target_ancestors: &[any::TypeId],
    ) -> Option<NodeIndex> {
        let mut cur = sid;
        while let Some((_, ancestor)) = stree.get_ancestor(cur, target_ancestors) {
            if let Some(ancestor_pid) = s2p.get_path_id(ancestor) {
                return Some(ancestor_pid.ni);
            }
            cur = ancestor;
        }
        None
    }
}
