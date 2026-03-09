use super::task::{Task, TaskQueue};
use crate::{
    err,
    etc::{
        syn::SynPath,
        util::{self, IntoPathSegments, PathSegments},
    },
    helper,
    semantic::{
        basic_traits::{EvaluateArrayLength, Scope, Scoping},
        entry::GlobalCx,
        eval::{self, Evaluated, Evaluator},
        infer::{self, Inferable, Inferer},
        logic::{self, find_method, Logic},
        tree::{
            self, filter, EffectiveItemKind, ItemTrait, NodeIndex, PathTree, PrivPathTree,
            SearchTypeOk, SynToPath, TypeId,
        },
    },
    syntax::{
        common::{IdentifySyn, SynId},
        SyntaxTree,
    },
    Map, TriOption, TriResult,
};
use logic_eval_util::str::StrPath;
use syn_locator::Locate;

#[derive(Debug)]
pub(super) struct Inspector<'gcx> {
    pub(super) inferer: Inferer<'gcx>,
    pub(super) evaluator: Evaluator<'gcx>,
    pub(super) hints: Map<*const syn::Expr, infer::Type<'gcx>>,
}

impl<'gcx> Inspector<'gcx> {
    pub(super) fn new(gcx: &'gcx GlobalCx<'gcx>) -> Self {
        Self {
            inferer: Inferer::new(gcx),
            evaluator: Evaluator::new(gcx),
            hints: Map::default(),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn as_infer_helper<'a>(
        &'a mut self,
        gcx: &'gcx GlobalCx<'gcx>,
        stree: &'a SyntaxTree,
        ptree: &'a PrivPathTree<'gcx>,
        s2p: &'a SynToPath,
        evaluated: &'a Evaluated<'gcx>,
        logic: &'a mut Logic<'gcx>,
        tasks: &'a mut TaskQueue<'gcx>,
        base: NodeIndex,
    ) -> InferHelper<'a, 'gcx> {
        InferHelper {
            gcx,
            stree,
            ptree,
            s2p,
            evaluated,
            inferer: &mut self.inferer,
            logic,
            hints: &mut self.hints,
            tasks,
            base,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn load_logic_for_file<T: ItemTrait>(
        &mut self,
        gcx: &'gcx GlobalCx<'gcx>,
        ptree: &PathTree<'gcx, T>,
        s2p: &SynToPath,
        evaluated: &Evaluated<'gcx>,
        logic: &mut Logic<'gcx>,
        tasks: &mut TaskQueue<'gcx>,
        base: NodeIndex,
        file: &syn::File,
    ) -> TriResult<(), ()> {
        let mut host = LogicHost {
            gcx,
            ptree,
            s2p,
            evaluated,
            hints: &mut self.hints,
            tasks,
            base,
        };
        logic.load_file(&mut host, file)
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn load_logic_for_impl<T: ItemTrait>(
        &mut self,
        gcx: &'gcx GlobalCx<'gcx>,
        ptree: &PathTree<'gcx, T>,
        s2p: &SynToPath,
        evaluated: &Evaluated<'gcx>,
        logic: &mut Logic<'gcx>,
        tasks: &mut TaskQueue<'gcx>,
        base: NodeIndex,
        item_impl: &syn::ItemImpl,
    ) -> TriResult<(), ()> {
        let mut host = LogicHost {
            gcx,
            ptree,
            s2p,
            evaluated,
            hints: &mut self.hints,
            tasks,
            base,
        };
        logic.load_item_impl(&mut host, item_impl)
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn as_eval_helper<'a, T>(
        &'a mut self,
        gcx: &'gcx GlobalCx<'gcx>,
        stree: &'a SyntaxTree,
        ptree: &'a PathTree<'gcx, T>,
        s2p: &'a SynToPath,
        evaluated: &'a mut Evaluated<'gcx>,
        logic: &'a mut Logic<'gcx>,
        tasks: &'a mut TaskQueue<'gcx>,
        base: NodeIndex,
    ) -> EvalHelper<'a, 'gcx, T> {
        EvalHelper {
            gcx,
            stree,
            ptree,
            s2p,
            evaluated,
            inferer: &mut self.inferer,
            evaluator: &mut self.evaluator,
            logic,
            hints: &mut self.hints,
            tasks,
            base,
        }
    }

    pub(super) fn get_infer_type<T: Inferable + ?Sized>(
        &self,
        syn: &T,
    ) -> Option<&infer::Type<'gcx>> {
        self.inferer.get_type(syn)
    }
}

// === Inference helper ===

pub(super) struct InferHelper<'a, 'gcx> {
    pub(super) gcx: &'gcx GlobalCx<'gcx>,
    pub(super) stree: &'a SyntaxTree,
    pub(super) ptree: &'a PrivPathTree<'gcx>,
    pub(super) s2p: &'a SynToPath,
    pub(super) evaluated: &'a Evaluated<'gcx>,
    pub(super) inferer: &'a mut Inferer<'gcx>,
    pub(super) logic: &'a mut Logic<'gcx>,
    pub(super) hints: &'a mut Map<*const syn::Expr, infer::Type<'gcx>>,
    pub(super) tasks: &'a mut TaskQueue<'gcx>,
    pub(super) base: NodeIndex,
}

impl<'gcx> InferHelper<'_, 'gcx> {
    pub(super) fn infer_block(
        &mut self,
        block: &syn::Block,
        type_hint: Option<infer::Type<'gcx>>,
    ) -> TriResult<(), ()> {
        let mut infer_logic_host = InferLogicHost {
            gcx: self.gcx,
            stree: self.stree,
            ptree: self.ptree,
            s2p: self.s2p,
            evaluated: self.evaluated,
            hints: self.hints,
            tasks: self.tasks,
            base: self.base,
        };
        self.inferer
            .infer_block(self.logic, &mut infer_logic_host, block, type_hint)
    }

    pub(super) fn infer_expr(
        &mut self,
        expr: &syn::Expr,
        type_hint: Option<infer::Type<'gcx>>,
    ) -> TriResult<(), ()> {
        let mut infer_logic_host = InferLogicHost {
            gcx: self.gcx,
            stree: self.stree,
            ptree: self.ptree,
            s2p: self.s2p,
            evaluated: self.evaluated,
            hints: self.hints,
            tasks: self.tasks,
            base: self.base,
        };
        self.inferer
            .infer_expr(self.logic, &mut infer_logic_host, expr, type_hint)
    }

    pub(super) fn find_type(&self, tid: infer::TypeId) -> &infer::Type<'gcx> {
        self.inferer.find_type(tid)
    }
}

// === InferLogicHost ===

struct InferLogicHost<'a, 'gcx, T> {
    gcx: &'gcx GlobalCx<'gcx>,
    stree: &'a SyntaxTree,
    ptree: &'a PathTree<'gcx, T>,
    s2p: &'a SynToPath,
    evaluated: &'a Evaluated<'gcx>,
    hints: &'a mut Map<*const syn::Expr, infer::Type<'gcx>>,
    tasks: &'a mut TaskQueue<'gcx>,
    base: NodeIndex,
}

impl<'gcx, T> InferLogicHost<'_, 'gcx, T> {
    fn as_infer_host(&mut self) -> InferHost<'_, 'gcx, T> {
        InferHost {
            gcx: self.gcx,
            stree: self.stree,
            ptree: self.ptree,
            s2p: self.s2p,
            hints: self.hints,
            evaluated: self.evaluated,
            tasks: self.tasks,
            base: self.base,
        }
    }

    fn as_logic_host(&mut self) -> LogicHost<'_, 'gcx, T> {
        LogicHost {
            gcx: self.gcx,
            ptree: self.ptree,
            s2p: self.s2p,
            evaluated: self.evaluated,
            hints: self.hints,
            tasks: self.tasks,
            base: self.base,
        }
    }

    fn as_array_len_eval(&mut self) -> ArrayLenEval<'_, 'gcx> {
        ArrayLenEval {
            gcx: self.gcx,
            evaluated: self.evaluated,
            hints: self.hints,
            tasks: self.tasks,
            base: self.base,
        }
    }
}

impl<'gcx, T: ItemTrait> infer::Host<'gcx> for InferLogicHost<'_, 'gcx, T> {
    fn syn_path_to_type(
        &mut self,
        syn_path: SynPath,
        types: &mut infer::UniqueTypes<'gcx>,
    ) -> TriResult<infer::Type<'gcx>, ()> {
        let mut infer_host = self.as_infer_host();
        infer::Host::syn_path_to_type(&mut infer_host, syn_path, types)
    }
}

impl<'a, 'gcx, T: ItemTrait> logic::Host<'gcx> for InferLogicHost<'a, 'gcx, T> {
    fn ident_to_npath(&mut self, ident: &syn::Ident) -> TriResult<String, ()> {
        let mut logic_host = self.as_logic_host();
        logic::Host::ident_to_npath(&mut logic_host, ident)
    }
}

impl<'gcx, T: ItemTrait> find_method::Host<'gcx> for InferLogicHost<'_, 'gcx, T> {
    fn is_visible(&mut self, parent_path: &str, fn_ident: &str) -> TriResult<bool, ()> {
        let mut infer_host = self.as_infer_host();
        find_method::Host::is_visible(&mut infer_host, parent_path, fn_ident)
    }
}

impl<T> Scoping for InferLogicHost<'_, '_, T> {
    fn on_enter_scope(&mut self, scope: Scope) {
        self.base = common_on_enter_scope(scope, self.s2p);
    }

    fn on_exit_scope(&mut self, scope: Scope) {
        common_on_exit_scope(scope);
    }
}

impl<'gcx, T: ItemTrait> EvaluateArrayLength<'gcx> for InferLogicHost<'_, 'gcx, T> {
    fn eval_array_len(&mut self, expr: &syn::Expr) -> TriResult<tree::ArrayLen, ()> {
        let mut len_eval = self.as_array_len_eval();
        EvaluateArrayLength::eval_array_len(&mut len_eval, expr)
    }
}

// === InferHost ===

struct InferHost<'a, 'gcx, T> {
    gcx: &'gcx GlobalCx<'gcx>,
    stree: &'a SyntaxTree,
    ptree: &'a PathTree<'gcx, T>,
    s2p: &'a SynToPath,
    evaluated: &'a Evaluated<'gcx>,
    hints: &'a mut Map<*const syn::Expr, infer::Type<'gcx>>,
    tasks: &'a mut TaskQueue<'gcx>,
    base: NodeIndex,
}

impl<'gcx, T> InferHost<'_, 'gcx, T> {
    fn as_array_len_eval(&mut self) -> ArrayLenEval<'_, 'gcx> {
        ArrayLenEval {
            gcx: self.gcx,
            evaluated: self.evaluated,
            hints: self.hints,
            tasks: self.tasks,
            base: self.base,
        }
    }
}

impl<'gcx, T: ItemTrait> infer::Host<'gcx> for InferHost<'_, 'gcx, T> {
    fn syn_path_to_type(
        &mut self,
        syn_path: SynPath,
        types: &mut infer::UniqueTypes<'gcx>,
    ) -> TriResult<infer::Type<'gcx>, ()> {
        // If we can find the path from path tree, then returns it.
        if let SearchTypeOk(tid) = TypeId::from_syn_path(
            syn_path.qself,
            syn_path.path,
            self.stree,
            self.ptree,
            self.base,
        ) {
            let ty = self.ptree.get_type(tid);
            let ty = infer::Type::from_tree_type(ty, self.ptree, types, self.gcx);
            return Ok(ty);
        }

        // If the path is a symbol like used in monomorphization, then returns it.
        if syn_path.qself.is_none() {
            if let Some(ident) = syn_path.path.get_ident() {
                if let Some(ty) = self
                    .gcx
                    .lasting_symbols()
                    .infer_type_symbols
                    .get(&*ident.to_string())
                {
                    return Ok(ty.clone());
                }
            }
        }

        err!(soft, ())
    }
}

impl<'gcx, T: ItemTrait> find_method::Host<'gcx> for InferHost<'_, 'gcx, T> {
    fn is_visible(&mut self, parent_path: &str, fn_ident: &str) -> TriResult<bool, ()> {
        let Some(node) = self.ptree.search(PathTree::<T>::ROOT, parent_path) else {
            return err!(soft, ());
        };

        for (_, item) in self.ptree[node].iter() {
            // Trait? then checks trait's visibility
            if item.effective_kind() == EffectiveItemKind::Trait {
                match item.vis_node() {
                    TriOption::Some(vis_node) => {
                        let is_visible = self.ptree.is_descendant(self.base, vis_node);
                        return Ok(is_visible);
                    }
                    TriOption::NotYet(()) | TriOption::None => {}
                }
            }
            // Non-trait? then checks fn's visibility
            else {
                let Some(node) = self.ptree.search(node, fn_ident) else {
                    return err!(soft, ());
                };

                for (_, item) in self.ptree[node].iter() {
                    match item.vis_node() {
                        TriOption::Some(vis_node) => {
                            let is_visible = self.ptree.is_descendant(self.base, vis_node);
                            return Ok(is_visible);
                        }
                        TriOption::NotYet(()) | TriOption::None => {}
                    }
                }
            }
        }

        err!(soft, ())
    }
}

impl<T> Scoping for InferHost<'_, '_, T> {
    fn on_enter_scope(&mut self, scope: Scope) {
        self.base = common_on_enter_scope(scope, self.s2p);
    }

    fn on_exit_scope(&mut self, scope: Scope) {
        common_on_exit_scope(scope);
    }
}

impl<'gcx, T> EvaluateArrayLength<'gcx> for InferHost<'_, 'gcx, T> {
    fn eval_array_len(&mut self, expr: &syn::Expr) -> TriResult<tree::ArrayLen, ()> {
        let mut len_eval = self.as_array_len_eval();
        EvaluateArrayLength::eval_array_len(&mut len_eval, expr)
    }
}

// === LogicHost ===

struct LogicHost<'a, 'gcx, T> {
    gcx: &'gcx GlobalCx<'gcx>,
    ptree: &'a PathTree<'gcx, T>,
    s2p: &'a SynToPath,
    evaluated: &'a Evaluated<'gcx>,
    hints: &'a mut Map<*const syn::Expr, infer::Type<'gcx>>,
    tasks: &'a mut TaskQueue<'gcx>,
    base: NodeIndex,
}

impl<'gcx, T> LogicHost<'_, 'gcx, T> {
    fn as_array_len_eval(&mut self) -> ArrayLenEval<'_, 'gcx> {
        ArrayLenEval {
            gcx: self.gcx,
            evaluated: self.evaluated,
            hints: self.hints,
            tasks: self.tasks,
            base: self.base,
        }
    }
}

impl<'a, 'gcx, T: ItemTrait> logic::Host<'gcx> for LogicHost<'a, 'gcx, T> {
    fn ident_to_npath(&mut self, ident: &syn::Ident) -> TriResult<String, ()> {
        let ident = ident.to_string();
        let (base, key) = self.ptree.normalize_key(self.base, ident.segments());

        let res = self
            .ptree
            .traverse(base, PathSegments(key.clone()), |_vis, pid, value| {
                if matches!(
                    value.effective_kind(),
                    EffectiveItemKind::Mod
                        | EffectiveItemKind::Struct
                        | EffectiveItemKind::Trait
                        | EffectiveItemKind::Enum
                ) {
                    Some(pid.ni)
                } else {
                    None
                }
            });

        if let Some(node) = res {
            let npath = self.ptree.get_name_path(node);
            return Ok(npath);
        }

        let SearchTypeOk(tid) = self.ptree.search_type(base, PathSegments(key)) else {
            return err!(soft, ());
        };

        match self.ptree.get_type(tid) {
            tree::Type::Scalar(scalar) => Ok(scalar.to_type_name().to_owned()),
            o => todo!("{o:?}"),
        }
    }
}

impl<T: ItemTrait> Scoping for LogicHost<'_, '_, T> {
    fn on_enter_scope(&mut self, scope: Scope) {
        self.base = common_on_enter_scope(scope, self.s2p);
    }

    fn on_exit_scope(&mut self, scope: Scope) {
        common_on_exit_scope(scope);
    }
}

impl<'gcx, T> EvaluateArrayLength<'gcx> for LogicHost<'_, 'gcx, T> {
    fn eval_array_len(&mut self, expr: &syn::Expr) -> TriResult<tree::ArrayLen, ()> {
        let mut len_eval = self.as_array_len_eval();
        EvaluateArrayLength::eval_array_len(&mut len_eval, expr)
    }
}

// === ArrayLenEval ===

struct ArrayLenEval<'a, 'gcx> {
    gcx: &'gcx GlobalCx<'gcx>,
    evaluated: &'a Evaluated<'gcx>,
    hints: &'a mut Map<*const syn::Expr, infer::Type<'gcx>>,
    tasks: &'a mut TaskQueue<'gcx>,
    base: NodeIndex,
}

impl<'gcx> EvaluateArrayLength<'gcx> for ArrayLenEval<'_, 'gcx> {
    fn eval_array_len(&mut self, expr: &syn::Expr) -> TriResult<tree::ArrayLen, ()> {
        self.hints.insert(
            expr as *const _,
            infer::Type::Scalar(infer::TypeScalar::Usize),
        );

        // If we have an evaluated value for the expression, we can return it.
        // - We may have it by evaluation in the past.
        let value = self
            .evaluated
            .get_value_by_expr(expr)
            .cloned()
            // - We may have it by temporary symbols used in monomorphization.
            .or_else(|| {
                if let syn::Expr::Path(expr_path) = expr {
                    if expr_path.qself.is_none() {
                        if let Some(ident) = expr_path.path.get_ident() {
                            if let Some(value) = self
                                .gcx
                                .lasting_symbols()
                                .eval_value_symbols
                                .get(&*ident.to_string())
                            {
                                return Some(value.clone());
                            }
                        }
                    }
                }
                None
            });
        if let Some(value) = value {
            match value {
                eval::Value::Scalar(eval::Scalar::Usize(n)) => Ok(tree::ArrayLen::Fixed(n)),
                eval::Value::ConstGeneric(_) => Ok(tree::ArrayLen::Generic),
                _ => unreachable!(),
            }
        }
        // We don't have an evaluated value for the expression for now. Requests evaluation.
        else {
            let task = Task::eval_expr(expr.syn_id(), self.base);
            let _ = self.tasks.push_back(task);
            err!(soft, ())
        }
    }
}

// === Evaluation helper ===

pub(super) struct EvalHelper<'a, 'gcx, T> {
    pub(super) gcx: &'gcx GlobalCx<'gcx>,
    pub(super) stree: &'a SyntaxTree,
    pub(super) ptree: &'a PathTree<'gcx, T>,
    pub(super) s2p: &'a SynToPath,
    pub(super) evaluated: &'a mut Evaluated<'gcx>,
    pub(super) inferer: &'a mut Inferer<'gcx>,
    pub(super) evaluator: &'a mut Evaluator<'gcx>,
    pub(super) logic: &'a mut Logic<'gcx>,
    pub(super) hints: &'a mut Map<*const syn::Expr, infer::Type<'gcx>>,
    pub(super) tasks: &'a mut TaskQueue<'gcx>,
    pub(super) base: NodeIndex,
}

impl<'gcx, T: ItemTrait> EvalHelper<'_, 'gcx, T> {
    pub(super) fn eval_expr_with_hint(
        &mut self,
        expr: &syn::Expr,
        hint: infer::Type<'gcx>,
    ) -> TriResult<eval::Value<'gcx>, ()> {
        self.hints.insert(expr as *const _, hint);
        self.eval_expr(expr)
    }

    pub(super) fn eval_expr(&mut self, expr: &syn::Expr) -> TriResult<eval::Value<'gcx>, ()> {
        // Does the expression contain const generic params? then it should be evaluated with
        // concrete types later. To do that, we leave some info behind.
        let value = if helper::generic::contains_const_generic_param_in_expr(expr, self.stree) {
            eval::Value::ConstGeneric(eval::ConstGeneric {
                expr,
                base: self.base,
            })
        }
        // The espression doesn't contain const generic params, then evaluates it now.
        else {
            let infer_logic_host = InferLogicHost {
                gcx: self.gcx,
                stree: self.stree,
                ptree: self.ptree,
                s2p: self.s2p,
                evaluated: self.evaluated,
                hints: self.hints,
                tasks: self.tasks,
                base: self.base,
            };
            let mut eval_host = EvalHost {
                gcx: self.gcx,
                stree: self.stree,
                inferer: self.inferer,
                logic: self.logic,
                infer_logic_host,
            };
            self.evaluator.eval_expr(&mut eval_host, expr)?
        };

        self.evaluated.insert_mapped_value(expr, value.clone());
        Ok(value)
    }
}

struct EvalHost<'a, 'gcx, T> {
    gcx: &'gcx GlobalCx<'gcx>,
    stree: &'a SyntaxTree,
    inferer: &'a mut Inferer<'gcx>,
    logic: &'a mut Logic<'gcx>,
    infer_logic_host: InferLogicHost<'a, 'gcx, T>,
}

impl<'gcx, T: ItemTrait> eval::Host<'gcx> for EvalHost<'_, 'gcx, T> {
    fn find_type(&mut self, expr: &syn::Expr) -> TriResult<infer::Type<'gcx>, ()> {
        if let Some(ty) = self
            .inferer
            .get_type(expr)
            .or_else(|| self.infer_logic_host.hints.get(&(expr as *const _)))
        {
            return Ok(ty.clone());
        }

        let (kind, top, type_hint) = find_top_element(expr, self.stree);
        infer_top_element(
            kind,
            top,
            self.inferer,
            self.logic,
            &mut self.infer_logic_host,
            type_hint,
        )?;

        let ty = self.inferer.get_type(expr).unwrap().clone();
        return Ok(ty);

        // === Internal helper function ===

        #[derive(Debug)]
        enum Kind {
            ItemFn,
            Block,
            Expr,
        }

        fn find_top_element<'gcx>(
            expr: &syn::Expr,
            stree: &SyntaxTree,
        ) -> (Kind, SynId, Option<infer::Type<'gcx>>) {
            let mut top_kind = Kind::Expr;
            let mut top = expr.syn_id();
            let mut type_hint = None;

            // If found top element for infering the expression, exits the loop.
            let mut cur = top;
            while let Some(parent) = stree.get_parent(cur).cloned() {
                let any = parent.as_any();
                if any.is::<syn::ItemFn>() {
                    top_kind = Kind::ItemFn;
                    top = parent;
                    break;
                } else if any.is::<syn::TypeArray>() {
                    type_hint = Some(infer::Type::Scalar(infer::TypeScalar::Usize));
                    break;
                } else if any.is::<syn::Block>() {
                    top_kind = Kind::Block;
                    top = parent;
                } else if any.is::<syn::Expr>() {
                    top_kind = Kind::Expr;
                    top = parent;
                }

                cur = parent;
            }

            (top_kind, top, type_hint)
        }

        fn infer_top_element<'gcx, H: infer::Host<'gcx> + logic::Host<'gcx>>(
            kind: Kind,
            top: SynId,
            inferer: &mut Inferer<'gcx>,
            logic: &mut Logic<'gcx>,
            infer_logic_host: &mut H,
            type_hint: Option<infer::Type<'gcx>>,
        ) -> TriResult<(), ()> {
            match kind {
                Kind::ItemFn => {
                    let v = top.as_any().downcast_ref::<syn::ItemFn>().unwrap();
                    inferer.infer_signature_and_block(logic, infer_logic_host, &v.sig, &v.block)
                }
                Kind::Block => {
                    let v = top.as_any().downcast_ref::<syn::Block>().unwrap();
                    inferer.infer_block(logic, infer_logic_host, v, type_hint)
                }
                Kind::Expr => {
                    let v = top.as_any().downcast_ref::<syn::Expr>().unwrap();
                    inferer.infer_expr(logic, infer_logic_host, v, type_hint)
                }
            }
        }
    }

    fn find_fn(&mut self, name: StrPath, types: &[infer::Type]) -> eval::Fn {
        todo!(
            "{}, input name: {name:?}, types: {types:?}",
            crate::cur_path!()
        );
    }

    fn syn_path_to_value(&mut self, syn_path: SynPath) -> TriResult<eval::Value<'gcx>, ()> {
        let Self {
            infer_logic_host:
                InferLogicHost {
                    ptree,
                    evaluated,
                    base,
                    ..
                },
            ..
        } = self;

        let key = util::get_name_path_from_syn_path(syn_path.path);

        // The given path is 'Self'?
        if key == "Self" {
            // TODO: Can we find what `Self` is in another way? This approach looks unstable
            // because it too much depends on that the current `base` should be a child of
            // enum/struct.
            let pid = ptree.nearest_item(*base, filter::enum_struct);
            eval::Value::from_path_id(pid, ptree, evaluated, self.gcx)
        }
        // Or a temporary symbol like during monomorphization?
        else if let Some(value) = self.gcx.lasting_symbols().eval_value_symbols.get(&*key) {
            Ok(value.clone())
        }
        // Or an item like constant declared outside?
        else if let Some(pid) =
            ptree.norm_search_item(*base, key.as_str(), syn_path.path.location().start)
        {
            eval::Value::from_path_id(pid, ptree, evaluated, self.gcx)
        } else {
            err!(soft, ())
        }
    }
}

impl<T> Scoping for EvalHost<'_, '_, T> {
    fn on_enter_scope(&mut self, scope: Scope) {
        self.infer_logic_host.base = common_on_enter_scope(scope, self.infer_logic_host.s2p);
    }

    fn on_exit_scope(&mut self, scope: Scope) {
        common_on_exit_scope(scope);
    }
}

fn common_on_enter_scope(scope: Scope, s2p: &SynToPath) -> NodeIndex {
    let sid = match scope {
        Scope::Mod(v) => v.syn_id(),
        Scope::ItemFn(v) => v.syn_id(),
        Scope::Block(v) => v.syn_id(),
    };
    s2p.get_path_id(sid).unwrap().ni
}

fn common_on_exit_scope(_: Scope) {}
