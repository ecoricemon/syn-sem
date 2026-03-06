use super::{
    construct::Constructor,
    find_known::KnownLibFinder,
    infer_eval::Inspector,
    monomorphize::Monomorphizer,
    resolve::Resolver,
    task::{self, Task, TaskDynInput, TaskQueue},
};
use crate::{
    err,
    etc::abs_fs::AbstractFiles,
    semantic::{
        entry::GlobalCx,
        eval::Evaluated,
        infer::{self, Inferer},
        logic::Logic,
        tree::{filter, NodeIndex, PathId, PrivItem, PrivPathTree, SynToPath},
    },
    syntax::{
        common::{IdentifySyn, SynId},
        SyntaxTree,
    },
    Result, TriError, TriResult, TriResultHelper,
};
use std::path::PathBuf;

pub(super) struct TaskConstructPathTreeHandler<'a, 'gcx> {
    pub(super) files: &'a mut AbstractFiles,
    pub(super) stree: &'a mut SyntaxTree,
    pub(super) ptree: &'a mut PrivPathTree<'gcx>,
    pub(super) s2p: &'a mut SynToPath,
    pub(super) tasks: &'a mut TaskQueue<'gcx>,
}

impl<'gcx> TaskConstructPathTreeHandler<'_, 'gcx> {
    pub(super) fn handle_task(
        &mut self,
        task: task::TaskConstructPathTree,
    ) -> TriResult<(), task::TaskConstructPathTree> {
        match task {
            task::TaskConstructPathTree::File { fpath, npath } => self
                .construct_path_tree_for_file(fpath, npath)
                .map_err(TriError::from),
            task::TaskConstructPathTree::Impl {
                ref item_impl,
                base,
            } => self
                .construct_path_tree_for_impl(*item_impl, base)
                .map_soft_err(|()| task),
        }
    }

    fn construct_path_tree_for_file(&mut self, fpath: PathBuf, npath: String) -> Result<()> {
        let fpath = self.files.to_absolute_path(&fpath)?;

        Constructor {
            files: self.files,
            stree: self.stree,
            ptree: self.ptree,
            s2p: self.s2p,
        }
        .construct_by_file(fpath.clone(), npath, self.tasks)?;

        let f = self.stree.get_file(&fpath).unwrap();
        let task = Task::load_logic_for_file(f.file.syn_id());
        let _ = self.tasks.push_back(task);
        Ok(())
    }

    fn construct_path_tree_for_impl(
        &mut self,
        item_impl: SynId,
        base: NodeIndex,
    ) -> TriResult<(), ()> {
        let item_impl = item_impl.as_const_ptr::<syn::ItemImpl>().unwrap();

        Constructor {
            files: self.files,
            stree: self.stree,
            ptree: self.ptree,
            s2p: self.s2p,
        }
        .construct_by_impl(item_impl, base, self.tasks)
    }
}

pub(super) struct TaskFindKnownLibHandler<'a, 'gcx> {
    pub(super) files: &'a AbstractFiles,
    pub(super) known_finder: &'a mut KnownLibFinder,
    pub(super) tasks: &'a mut TaskQueue<'gcx>,
}

impl<'gcx> TaskFindKnownLibHandler<'_, 'gcx> {
    pub(super) fn handle_task(&mut self, task: task::TaskFindKnownLibFrom) {
        self.known_finder
            .find_known_lib(task, self.files, |known_name| {
                let fpath = known_name.into();
                let npath = known_name.to_owned();
                let task = Task::construct_path_tree_for_file(fpath, npath);
                let _ = self.tasks.push_back(task);
            });
    }
}

pub(super) struct TaskLoadLogicHandler<'a, 'gcx> {
    pub(super) gcx: &'gcx GlobalCx<'gcx>,
    pub(super) ptree: &'a PrivPathTree<'gcx>,
    pub(super) s2p: &'a SynToPath,
    pub(super) evaluated: &'a Evaluated<'gcx>,
    pub(super) type_inspector: &'a mut Inspector<'gcx>,
    pub(super) logic: &'a mut Logic<'gcx>,
    pub(super) tasks: &'a mut TaskQueue<'gcx>,
}

impl<'gcx> TaskLoadLogicHandler<'_, 'gcx> {
    pub(super) fn handle_task(
        &mut self,
        task: task::TaskLoadLogic,
    ) -> TriResult<(), task::TaskLoadLogic> {
        let res = match task {
            task::TaskLoadLogic::ImplsInFile { file } => {
                let base = self.s2p.get_path_id(file).unwrap().ni;
                let file = file.as_any().downcast_ref::<syn::File>().unwrap();
                self.type_inspector.load_logic_for_file(
                    self.gcx,
                    self.ptree,
                    self.s2p,
                    self.evaluated,
                    self.logic,
                    self.tasks,
                    base,
                    file,
                )
            }
            task::TaskLoadLogic::Impl { item_impl, base } => {
                let item_impl = item_impl.as_any().downcast_ref::<syn::ItemImpl>().unwrap();
                self.type_inspector.load_logic_for_impl(
                    self.gcx,
                    self.ptree,
                    self.s2p,
                    self.evaluated,
                    self.logic,
                    self.tasks,
                    base,
                    item_impl,
                )
            }
        };
        res.map_soft_err(|()| task)
    }
}

pub(super) struct TaskResolveHandler<'a, 'gcx> {
    pub(super) gcx: &'gcx GlobalCx<'gcx>,
    pub(super) stree: &'a mut SyntaxTree,
    pub(super) ptree: &'a mut PrivPathTree<'gcx>,
    pub(super) s2p: &'a mut SynToPath,
    pub(super) evaluated: &'a mut Evaluated<'gcx>,
    pub(super) type_inspector: &'a mut Inspector<'gcx>,
    pub(super) logic: &'a mut Logic<'gcx>,
    pub(super) tasks: &'a mut TaskQueue<'gcx>,
}

impl<'gcx> TaskResolveHandler<'_, 'gcx> {
    pub(super) fn handle_task(
        &mut self,
        task: task::TaskResolve,
    ) -> TriResult<(), task::TaskResolve> {
        let mut cx = Resolver::as_cx(
            self.gcx,
            self.stree,
            self.ptree,
            self.s2p,
            self.evaluated,
            self.type_inspector,
            self.logic,
            self.tasks,
        );
        match task.clone() {
            task::TaskResolve::Const(inner) => match inner {
                task::TaskResolveConst::ResolveVis(pid) => cx.resolve_const_vis(pid),
                task::TaskResolveConst::ResolveType(pid) => {
                    cx.resolve_const_type(pid).map_soft_err(|()| task)?
                }
            },
            task::TaskResolve::Enum(inner) => match inner {
                task::TaskResolveEnum::ResolveVis(pid) => cx.resolve_enum_vis(pid),
                task::TaskResolveEnum::ResolveType(pid) => {
                    cx.resolve_enum_type(pid).map_soft_err(|()| task)?
                }
            },
            task::TaskResolve::Field(inner) => match inner {
                task::TaskResolveField::ResolveVis(pid) => cx.resolve_field_vis(pid),
                task::TaskResolveField::ResolveType(pid) => {
                    cx.resolve_field_type(pid).map_soft_err(|()| task)?
                }
            },
            task::TaskResolve::Fn(inner) => match inner {
                task::TaskResolveFn::ResolveVis(pid) => cx.resolve_fn_vis(pid),
                task::TaskResolveFn::ResolveType(pid) => {
                    cx.resolve_fn_type(pid).map_soft_err(|()| task)?
                }
            },
            task::TaskResolve::Local(inner) => match inner {
                task::TaskResolveLocal::ResolveType(pid) => {
                    cx.resolve_local_type(pid).map_soft_err(|()| task)?
                }
            },
            task::TaskResolve::Mod(inner) => match inner {
                task::TaskResolveMod::ResolveVis(pid) => cx.resolve_mod_vis(pid),
            },
            task::TaskResolve::Struct(inner) => match inner {
                task::TaskResolveStruct::ResolveVis(pid) => cx.resolve_struct_vis(pid),
                task::TaskResolveStruct::ResolveType(pid) => {
                    cx.resolve_struct_type(pid).map_soft_err(|()| task)?
                }
            },
            task::TaskResolve::Trait(inner) => match inner {
                task::TaskResolveTrait::ResolveVis(pid) => cx.resolve_trait_vis(pid),
            },
            task::TaskResolve::TypeAlias(inner) => match inner {
                task::TaskResolveTypeAlias::ResolveVis(pid) => cx.resolve_type_alias_vis(pid),
                task::TaskResolveTypeAlias::ResolveType(pid) => {
                    cx.resolve_type_alias_type(pid).map_soft_err(|()| task)?
                }
            },
            task::TaskResolve::Use(inner) => match inner {
                task::TaskResolveUse::ResolveVis(pid) => cx.resolve_use_vis(pid),
                task::TaskResolveUse::ResolveDst => cx.resolve_use_dst().map_soft_err(|()| task)?,
            },
            task::TaskResolve::Variant(inner) => match inner {
                task::TaskResolveVariant::ResolveVis(pid) => {
                    cx.resolve_variant_vis(pid).map_soft_err(|()| task)?
                }
                task::TaskResolveVariant::ResolveDisc(pid) => {
                    cx.resolve_variant_disc(pid).map_soft_err(|()| task)?
                }
            },
        };
        Ok(())
    }
}

pub(super) struct TaskFixTypeHandler<'a, 'gcx> {
    pub(super) gcx: &'gcx GlobalCx<'gcx>,
    pub(super) stree: &'a mut SyntaxTree,
    pub(super) ptree: &'a mut PrivPathTree<'gcx>,
    pub(super) s2p: &'a mut SynToPath,
    pub(super) evaluted: &'a mut Evaluated<'gcx>,
    pub(super) type_inspector: &'a mut Inspector<'gcx>,
    pub(super) logic: &'a mut Logic<'gcx>,
    pub(super) tasks: &'a mut TaskQueue<'gcx>,
}

impl<'gcx> TaskFixTypeHandler<'_, 'gcx> {
    pub(super) fn handle_task(
        &mut self,
        task: task::TaskFixType,
    ) -> TriResult<(), task::TaskFixType> {
        let mut cx = Resolver::as_cx(
            self.gcx,
            self.stree,
            self.ptree,
            self.s2p,
            self.evaluted,
            self.type_inspector,
            self.logic,
            self.tasks,
        );
        let res = match task.clone() {
            task::TaskFixType::Const(pid) => cx.fix_const_type_len(pid),
            task::TaskFixType::Field(pid) => cx.fix_field_type_len(pid),
            task::TaskFixType::Fn(pid) => cx.fix_fn_type_len(pid),
            task::TaskFixType::Local(pid) => cx.fix_local_type_len(pid),
            task::TaskFixType::Struct(pid) => cx.fix_struct_type_len(pid),
            task::TaskFixType::TypeAlias(pid) => cx.fix_type_alias_type_len(pid),
            task::TaskFixType::ImplType(inner) => cx.fix_type_impl_type_len(inner),
        };
        res.map_soft_err(|()| task)
    }
}

pub(super) struct TaskEvalConstHandler<'a, 'gcx> {
    pub(super) gcx: &'gcx GlobalCx<'gcx>,
    pub(super) stree: &'a SyntaxTree,
    pub(super) ptree: &'a mut PrivPathTree<'gcx>,
    pub(super) s2p: &'a SynToPath,
    pub(super) evaluated: &'a mut Evaluated<'gcx>,
    pub(super) type_inspector: &'a mut Inspector<'gcx>,
    pub(super) logic: &'a mut Logic<'gcx>,
    pub(super) tasks: &'a mut TaskQueue<'gcx>,
}

impl<'gcx> TaskEvalConstHandler<'_, 'gcx> {
    pub(super) fn handle_task(
        &mut self,
        task: task::TaskEvalConst,
    ) -> TriResult<(), task::TaskEvalConst> {
        let res = match task.clone() {
            task::TaskEvalConst::Free { const_pid } => self.eval_free_const(const_pid),
            task::TaskEvalConst::Inher { expr, ty, base } => self.eval_inher_const(expr, ty, base),
            task::TaskEvalConst::TraitDefault { expr, ty, base } => {
                self.eval_trait_default_const(expr, ty, base)
            }
            task::TaskEvalConst::TraitImpl { expr, ty, base } => {
                self.eval_trait_impl_const(expr, ty, base)
            }
        };
        res.map_soft_err(|()| task)
    }

    fn eval_free_const(&mut self, pid: PathId) -> TriResult<(), ()> {
        // Not resolved yet? retries later.
        let PrivItem::Const(const_) = &self.ptree[pid] else {
            return err!(soft, ());
        };

        // Const doesn't belong to the path tree. So unlike other path items, inference doesn't
        // occur by tasks. So infers the type of the expression before evaluation.
        let const_ = const_.clone();
        let expr = const_.syn_expr().unwrap(); // free const has init expr
        let ty = &const_.syn_type();
        let base = self.ptree.parent_item(pid.ni, filter::block_mod).ni;
        let ty_hint = self.infer_const_expr(expr, ty, base)?;

        // Evaluates the expression.
        let mut eval = self.type_inspector.as_eval_helper(
            self.gcx,
            self.stree,
            self.ptree,
            self.s2p,
            self.evaluated,
            self.logic,
            self.tasks,
            base,
        );
        match eval.eval_expr_with_hint(expr, ty_hint) {
            Ok(value) => {
                let expr: &syn::Expr = expr;
                self.evaluated.insert_mapped_value2(expr, pid, value);
                Ok(())
            }
            e => e.map(|_value| ()),
        }
    }

    fn eval_inher_const(&mut self, expr: SynId, ty: SynId, base: NodeIndex) -> TriResult<(), ()> {
        let expr = expr.as_ref::<syn::Expr>().unwrap();
        let ty = ty.as_ref::<syn::Type>().unwrap();

        // Const doesn't belong to the path tree. So unlike other path items, inference doesn't
        // occur by tasks. So infers the type of the expression before evaluation.
        let ty_hint = self.infer_const_expr(expr, ty, base)?;

        // Evaluates the expression.
        let mut eval = self.type_inspector.as_eval_helper(
            self.gcx,
            self.stree,
            self.ptree,
            self.s2p,
            self.evaluated,
            self.logic,
            self.tasks,
            base,
        );
        match eval.eval_expr_with_hint(expr, ty_hint) {
            Ok(value) => {
                self.evaluated.insert_mapped_value(expr, value);
                Ok(())
            }
            e => e.map(|_value| ()),
        }
    }

    fn eval_trait_default_const(
        &mut self,
        expr: SynId,
        ty: SynId,
        base: NodeIndex,
    ) -> TriResult<(), ()> {
        let expr = expr.as_ref::<syn::Expr>().unwrap();
        let ty = ty.as_ref::<syn::Type>().unwrap();

        // Const doesn't belong to the path tree. So inference doesn't occur by tasks unlike other
        // path items. Therefore we infer the type of the expression here before evaluation.
        let ty_hint = self.infer_const_expr(expr, ty, base)?;

        // Evaluates the expression.
        let mut eval = self.type_inspector.as_eval_helper(
            self.gcx,
            self.stree,
            self.ptree,
            self.s2p,
            self.evaluated,
            self.logic,
            self.tasks,
            base,
        );
        match eval.eval_expr_with_hint(expr, ty_hint) {
            Ok(value) => {
                self.evaluated.insert_mapped_value(expr, value);
                Ok(())
            }
            e => e.map(|_value| ()),
        }
    }

    fn eval_trait_impl_const(
        &mut self,
        expr: SynId,
        ty: SynId,
        base: NodeIndex,
    ) -> TriResult<(), ()> {
        let expr = expr.as_ref::<syn::Expr>().unwrap();
        let ty = ty.as_ref::<syn::Type>().unwrap();

        // Const doesn't belong to the path tree. So unlike other path items, inference doesn't
        // occur by tasks. So infers the type of the expression before evaluation.
        let ty_hint = self.infer_const_expr(expr, ty, base)?;

        // Evaluates the expression.
        let mut eval = self.type_inspector.as_eval_helper(
            self.gcx,
            self.stree,
            self.ptree,
            self.s2p,
            self.evaluated,
            self.logic,
            self.tasks,
            base,
        );
        match eval.eval_expr_with_hint(expr, ty_hint) {
            Ok(value) => {
                self.evaluated.insert_mapped_value(expr, value);
                Ok(())
            }
            e => e.map(|_value| ()),
        }
    }

    /// Infers the given expression then returns type hint for further evaluation.
    fn infer_const_expr(
        &mut self,
        expr: &syn::Expr,
        ty: &syn::Type,
        base: NodeIndex,
    ) -> TriResult<infer::Type<'gcx>, ()> {
        // Creates type hint from the `syn::Type`.
        let infer_types = &mut self.type_inspector.inferer.types;
        let ty_hint =
            infer::Type::from_syn_type(ty, self.stree, self.ptree, base, infer_types, self.gcx)?;

        // Infers type of the expression. This will infer types about all nested expressions too.
        let mut infer = self.type_inspector.as_infer_helper(
            self.gcx,
            self.stree,
            self.ptree,
            self.s2p,
            self.evaluated,
            self.logic,
            self.tasks,
            base,
        );
        infer.infer_expr(expr, Some(ty_hint.clone()))?;

        Ok(ty_hint)
    }
}

pub(super) struct TaskEvalExprHandler<'a, 'gcx> {
    pub(super) gcx: &'gcx GlobalCx<'gcx>,
    pub(super) stree: &'a SyntaxTree,
    pub(super) ptree: &'a PrivPathTree<'gcx>,
    pub(super) s2p: &'a SynToPath,
    pub(super) evaluated: &'a mut Evaluated<'gcx>,
    pub(super) type_inspector: &'a mut Inspector<'gcx>,
    pub(super) logic: &'a mut Logic<'gcx>,
    pub(super) tasks: &'a mut TaskQueue<'gcx>,
}

impl<'gcx> TaskEvalExprHandler<'_, 'gcx> {
    pub(super) fn handle_task(
        &mut self,
        task: task::TaskEvalExpr,
    ) -> TriResult<(), task::TaskEvalExpr> {
        let mut eval = self.type_inspector.as_eval_helper(
            self.gcx,
            self.stree,
            self.ptree,
            self.s2p,
            self.evaluated,
            self.logic,
            self.tasks,
            task.base,
        );

        let expr = task.expr.as_ref::<syn::Expr>().unwrap();

        eval.eval_expr(expr)
            .map(|value| {
                self.evaluated.insert_mapped_value(expr, value);
            })
            .map_soft_err(|()| task)
    }
}

pub(super) struct TaskMonomorphizeHandler<'a, 'gcx> {
    pub(super) gcx: &'gcx GlobalCx<'gcx>,
    pub(super) stree: &'a mut SyntaxTree,
    pub(super) ptree: &'a PrivPathTree<'gcx>,
    pub(super) s2p: &'a SynToPath,
    pub(super) inferer: &'a mut Inferer<'gcx>,
    pub(super) tasks: &'a mut TaskQueue<'gcx>,
}

impl<'gcx> TaskMonomorphizeHandler<'_, 'gcx> {
    // TODO
    // - Return type may need to be extended to include 'no change', which could be ignored
    // - Trait generics are ignored for now.
    // - Nested generics should be considered.
    pub(super) fn handle_task(
        &mut self,
        task: task::TaskMonomorphize,
    ) -> TriResult<(), task::TaskMonomorphize> {
        let res = match task {
            task::TaskMonomorphize::Impl { item_impl, self_ty } => Monomorphizer::as_cx(
                self.gcx,
                self.stree,
                self.ptree,
                self.s2p,
                self.inferer,
                self.tasks,
            )
            .monomorphize_impl(item_impl, self_ty),
        };
        res.map_soft_err(|()| task)
    }
}

pub(super) struct TaskDynHandler<'gcx> {
    pub(super) gcx: &'gcx GlobalCx<'gcx>,
}

impl<'gcx> TaskDynHandler<'gcx> {
    pub(super) fn handle_task(
        &mut self,
        mut task: task::TaskDyn<'gcx>,
    ) -> TriResult<(), task::TaskDyn<'gcx>> {
        let input = TaskDynInput { gcx: self.gcx };
        task.custom.run(input).map_soft_err(|()| task)
    }
}
