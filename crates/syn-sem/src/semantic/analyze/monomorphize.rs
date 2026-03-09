use super::task::{Task, TaskDynInput, TaskQueue};
use crate::{
    ds::vec::BoxedSlice,
    err,
    semantic::{
        entry::GlobalCx,
        eval,
        infer::{self, Inferer},
        tree::{PrivPathTree, SynToPath, TypeId},
    },
    syntax::{
        common::{FindChildren, IdentifySyn, SynId},
        SyntaxTree,
    },
    Intern, Result, TriResult,
};
use any_intern::Interned;
use std::{any, fmt::Write, mem};
use syn_locator::{Locate, LocateEntry};

#[derive(Debug)]
pub(super) struct Monomorphizer {/* Nothing for now */}

impl Monomorphizer {
    // The context is not using `Monomorphizer` yet, but we create monomorphization context through
    // `Monomorphizer` for consistency.
    pub(super) fn as_cx<'a, 'gcx>(
        gcx: &'gcx GlobalCx<'gcx>,
        stree: &'a mut SyntaxTree,
        ptree: &'a PrivPathTree<'gcx>,
        s2p: &'a SynToPath,
        inferer: &'a mut Inferer<'gcx>,
        tasks: &'a mut TaskQueue<'gcx>,
    ) -> MonomorphizeCx<'a, 'gcx> {
        MonomorphizeCx {
            gcx,
            stree,
            ptree,
            s2p,
            inferer,
            tasks,
        }
    }
}

pub(super) struct MonomorphizeCx<'a, 'gcx> {
    gcx: &'gcx GlobalCx<'gcx>,
    stree: &'a mut SyntaxTree,
    ptree: &'a PrivPathTree<'gcx>,
    s2p: &'a SynToPath,
    inferer: &'a mut Inferer<'gcx>,
    tasks: &'a mut TaskQueue<'gcx>,
}

impl<'gcx> MonomorphizeCx<'_, 'gcx> {
    pub(super) fn monomorphize_impl(
        &mut self,
        item_impl: SynId,
        concrete_self_ty: Option<TypeId>,
    ) -> TriResult<(), ()> {
        // Finds 'base' node for the impl block.
        let Some(base) =
            crate::helper::ptree::find_base_node_of_item_impl(self.stree, self.s2p, item_impl)
        else {
            return err!(
                hard,
                "failed to find the target ancestor node of an impl block"
            );
        };

        // Clones the impl block except generics. We're going to replace those generic symbols with
        // some concrete values through host functions.
        let item_impl = item_impl.as_ref::<syn::ItemImpl>().unwrap();
        let (sid_impl, generics) = self.clone_impl_except_generics(item_impl)?;
        let item_impl = sid_impl.as_any().downcast_ref::<syn::ItemImpl>().unwrap();

        // Self type may contains generic symbols. In that case, we can find concrete types and
        // values corresponding to the generic symbols through the `concrete_self_ty`. Let's
        // collect required information (symbol name, type, value).
        type NameTypeValue<'gcx> = (
            Interned<'gcx, str>,
            infer::Type<'gcx>,
            Option<eval::Value<'gcx>>,
        );
        let name_ty_val = generics
            .params
            .iter()
            .filter_map(|param| {
                let mut finder = find::GenericFinder::new(self.ptree);
                let portion = finder.find_generic_portion_of_type(param, &item_impl.self_ty)?;

                // Generic symbol name
                let ident = match param {
                    syn::GenericParam::Lifetime(_) => panic!("lifetime is not allowed"),
                    syn::GenericParam::Type(ty_param) => &ty_param.ident,
                    syn::GenericParam::Const(const_param) => &const_param.ident,
                };
                let name = self.gcx.intern_str(&ident.to_string());

                // Corresponding concrete type to the symbol
                let concrete_self =
                    concrete_self_ty.expect("generic symbol found, concrete self type is expected");
                let tree_ty = self.ptree.get_type(concrete_self);
                let matched = finder
                    .find_corresponding_with_type(portion, &item_impl.self_ty, tree_ty)
                    .unwrap();
                let ty = infer::Type::from_tree_type(
                    matched.ty,
                    self.ptree,
                    &mut self.inferer.types,
                    self.gcx,
                );

                // Corresponding optional value to the symbol
                let value = matched.value;

                Some((name, ty, value))
            })
            .collect::<BoxedSlice<NameTypeValue<'gcx>>>();

        // Setup & Cleanup task id using unique input combination
        let mut unique = String::new();
        write!(&mut unique, "{:?}", item_impl as *const _).unwrap();
        write!(&mut unique, "{:?}", concrete_self_ty.map(|inner| inner.0)).unwrap();
        let (mut setup_task_id, mut cleanup_task_id) = (unique.clone(), unique);
        setup_task_id.push('s');
        cleanup_task_id.push('c');

        // To replace symbols with concrete types and values for following tasks, we have to put
        // them in `lasting_cx` in a setup task. The setup task will be processed before start of
        // each following task.
        let setup_fn = move |input: TaskDynInput<'gcx>| {
            let mut lasting_symbols = input.gcx.lasting_symbols();

            lasting_symbols.infer_type_symbols.push_opaque_block();
            lasting_symbols.eval_value_symbols.push_opaque_block();

            for (name, ty, value) in &name_ty_val {
                lasting_symbols.infer_type_symbols.push(*name, ty.clone());

                if let Some(value) = value {
                    lasting_symbols
                        .eval_value_symbols
                        .push(*name, value.clone());
                }
            }

            Ok(())
        };
        let setup_task = Task::dyn_(setup_fn, self.gcx.intern_str(&setup_task_id));
        let parent_node = self.tasks.just_popped_task_node();
        self.tasks.set_setup_task(parent_node, setup_task);

        // Cleanup task will clear the `lasting_cx` after end of each following task.
        let cleanup_fn = move |input: TaskDynInput<'gcx>| {
            let mut lasting_symbols = input.gcx.lasting_symbols();

            lasting_symbols.infer_type_symbols.pop_block();
            lasting_symbols.eval_value_symbols.pop_block();
            Ok(())
        };
        let cleanup_task = Task::dyn_(cleanup_fn, self.gcx.intern_str(&cleanup_task_id));
        self.tasks.set_cleanup_task(parent_node, cleanup_task);

        // Now, makes logic for the cloned impl block.
        let task = Task::load_logic_for_impl(sid_impl, base);
        self.tasks.push_back(task).unwrap();

        // Evaluates all constants in the impl block.
        let descendants = [any::TypeId::of::<syn::ImplItemConst>()];
        item_impl.visit_descendant(&descendants, &mut |_, sid| {
            let item_const = sid.as_any().downcast_ref::<syn::ImplItemConst>().unwrap();
            let expr = item_const.expr.syn_id();
            let ty = item_const.ty.syn_id();
            let task = Task::eval_const_trait_impl(expr, ty, base);
            self.tasks.push_back(task).unwrap();
        });

        Ok(())
    }

    /// Clones [`syn::ItemImpl`] except [`syn::Generics`], then registers the impl block to the
    /// [`syn_locator`].
    ///
    /// * Output - (syn id to cloned [`syn::ItemImpl`], excepted [`syn::Generics`])
    fn clone_impl_except_generics(
        &mut self,
        item_impl: &syn::ItemImpl,
    ) -> Result<(SynId, syn::Generics)> {
        let mut dst_code = item_impl.code();
        let loc_impl = item_impl.location();

        // Removes generics from the code.
        if let Some(where_clause) = &item_impl.generics.where_clause {
            let loc_where = where_clause.location();
            let l = loc_where.start - loc_impl.start;
            let r = loc_where.end - loc_impl.start;
            dst_code.replace_range(l..r, "");
        }
        if let Some(lt_token) = &item_impl.generics.lt_token {
            if let Some(gt_token) = &item_impl.generics.gt_token {
                let l = lt_token.location().start - loc_impl.start;
                let r = gt_token.location().end - loc_impl.start;
                dst_code.replace_range(l..r, "");
            }
        }

        // Takes `syn::Generics` out of the cloned `syn::ItemImpl`.
        let mut dst_impl = item_impl.clone();
        let generics = mem::take(&mut dst_impl.generics);

        // We made a new syntax tree and its code, so let's add it to `syn_locator` like
        // `syn::File`.
        let pinned_impl = Box::pin(dst_impl);
        let sid_impl = pinned_impl.as_ref().syn_id();
        let file_path = format!("{}:{}", loc_impl.file_path, loc_impl.start);
        pinned_impl.as_ref().locate_as_entry(&file_path, dst_code)?;
        self.stree.insert_impl(file_path.into(), pinned_impl);

        Ok((sid_impl, generics))
    }
}

/// Finds the first corresponding tree type to the generic parameter in the syn type.
///
/// If it's possible to make evaluated value from the type, then returns the value together.
///
/// e.g.
/// * syn_ty is given like "[T; N]" in "impl<const N: usize> Trait for [T; N]"
/// * tree_ty is given like "[T; 42]"
///   -> generic parameter in the syn_ty is "N" which is corresponding to "42", so returns
///   (usize, 42)
//
// TODO: e.g. in self type "[T; { const N: usize, .. N .. }]", "N" hiddens const generic "N". So
// does types, therefore we need a common way to figure it out. We don't have path tree for blocks
// like these yet.
mod find {
    use crate::{
        err,
        semantic::{
            eval,
            tree::{self, PrivPathTree},
        },
        Result,
    };
    use std::ptr;

    /// Finds generic parameter portion of syn item.
    pub(super) struct GenericFinder<'a, 'gcx> {
        ptree: &'a PrivPathTree<'gcx>,
        ns: Namespace,
    }

    impl<'a, 'gcx> GenericFinder<'a, 'gcx> {
        pub(super) fn new(ptree: &'a PrivPathTree<'gcx>) -> Self {
            Self {
                ptree,
                ns: Namespace::Type,
            }
        }

        /// In the given tree type, finds a type, and an optional value, that corresponds to the
        /// given generic parameter.
        ///
        /// Returns Err if failed to find.
        pub(super) fn find_corresponding_with_type(
            &self,
            portion: GenericPortion<'_>,
            syn_ty: &syn::Type,
            tree_ty: &tree::Type<'gcx>,
        ) -> Result<MatchedType<'_, 'gcx>> {
            match (syn_ty, tree_ty) {
                (syn::Type::Array(l), tree::Type::Array(r)) => match portion.ns {
                    Namespace::Type => {
                        let r_elem = self.ptree.get_type(r.elem);
                        self.find_corresponding_with_type(portion, &l.elem, r_elem)
                    }
                    Namespace::Value => self.find_corresponding_array_len(portion, &l.len, r.len),
                },
                _ => todo!(),
            }
        }

        fn find_corresponding_array_len(
            &self,
            portion: GenericPortion<'_>,
            syn_expr: &syn::Expr,
            arr_len: tree::ArrayLen,
        ) -> Result<MatchedType<'_, 'gcx>> {
            if let syn::Expr::Path(expr_path) = syn_expr {
                if ptr::eq(&expr_path.path, portion.path) {
                    let tid = self
                        .ptree
                        .insert_type(tree::Type::Scalar(tree::TypeScalar::Usize));
                    let ty = self.ptree.get_type(tid);
                    let tree::ArrayLen::Fixed(len) = arr_len else {
                        unreachable!("generic corresponding array length must be fixed");
                    };
                    let value = eval::Value::Scalar(eval::Scalar::Usize(len));
                    return Ok(MatchedType {
                        ty,
                        value: Some(value),
                    });
                }
            }
            err!("expr: {syn_expr:?}, portion: {portion:?}")
        }

        /// In the given syn type, finds a portion that corresponds to the given generic parameter.
        ///
        /// Returns None if failed to find.
        pub(super) fn find_generic_portion_of_type<'syn>(
            &mut self,
            param: &syn::GenericParam,
            ty: &'syn syn::Type,
        ) -> Option<GenericPortion<'syn>> {
            match ty {
                syn::Type::Array(ty_arr) => {
                    // Array element (type namespace)
                    self.ns = Namespace::Type;
                    let ret = self.find_generic_portion_of_type(param, &ty_arr.elem);
                    if ret.is_some() {
                        return ret;
                    }

                    // Array length (value namespace)
                    self.ns = Namespace::Value;
                    self.find_generic_portion_of_expr(param, &ty_arr.len)
                }
                syn::Type::Path(ty_path) => {
                    debug_assert!(ty_path.qself.is_none()); // Not yet considered
                    self.compare(param, &ty_path.path)
                }
                _ => todo!("{ty:?}"),
            }
        }

        fn find_generic_portion_of_expr<'syn>(
            &mut self,
            param: &syn::GenericParam,
            expr: &'syn syn::Expr,
        ) -> Option<GenericPortion<'syn>> {
            match expr {
                syn::Expr::Path(expr_path) if expr_path.qself.is_none() => {
                    self.compare(param, &expr_path.path)
                }
                _ => todo!(),
            }
        }

        fn compare<'syn>(
            &mut self,
            param: &syn::GenericParam,
            path: &'syn syn::Path,
        ) -> Option<GenericPortion<'syn>> {
            let ident = path.get_ident()?;

            match self.ns {
                Namespace::Type => {
                    if let syn::GenericParam::Type(ty_param) = param {
                        if &ty_param.ident == ident {
                            return Some(GenericPortion { path, ns: self.ns });
                        }
                    }
                    None
                }
                Namespace::Value => {
                    if let syn::GenericParam::Const(const_param) = param {
                        if &const_param.ident == ident {
                            return Some(GenericPortion { path, ns: self.ns });
                        }
                    }
                    None
                }
            }
        }
    }

    pub(super) struct MatchedType<'a, 'gcx> {
        pub(super) ty: &'a tree::Type<'gcx>,
        pub(super) value: Option<eval::Value<'gcx>>,
    }

    #[derive(Debug)]
    pub(super) struct GenericPortion<'a> {
        /// Generic symbol inside a syn item.
        ///
        /// e.g. latter "N" in "impl<const N: usize> ... [T; N]"
        path: &'a syn::Path,

        /// Namespace where this generic symbol belongs
        ns: Namespace,
    }

    #[derive(Debug, Clone, Copy)]
    enum Namespace {
        Type,
        Value,
    }
}
