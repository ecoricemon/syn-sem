use super::{
    infer_eval::{EvalHelper, Inspector},
    task::TaskQueue,
};
use crate::{
    ds::vec::BoxedSlice,
    err,
    etc::util::{IntoPathSegments, PathSegments},
    semantic::{
        analyze::task::TaskFixImplType,
        entry::GlobalCx,
        eval::{self, Evaluated},
        logic::Logic,
        tree::{
            filter, ArrayLen, Const, Enum, Field, Fn, ItemTrait, Local, Mod, NodeIndex, Param,
            PathId, PathTree, PathVis, PrivItem, PrivPathTree, RawConst, RawFn, RawUse,
            SearchTypeNotFound, SearchTypeNotReady, SearchTypeOk, Struct, SynToPath, Trait, Type,
            TypeAlias, TypeArray, TypeId, TypeMut, TypePath, TypeRef, TypeTuple, Use, Variant,
        },
    },
    syntax::{common::IdentifySyn, SyntaxTree},
    Intern, Map, Set, TriOption, TriResult, Which2,
};
use std::{
    collections::VecDeque,
    iter,
    ptr::{self, NonNull},
};

const TREE_ROOT: NodeIndex = PathTree::<()>::ROOT;

#[derive(Debug, Default)]
pub(super) struct Resolver {/* Nothing for now */}

impl Resolver {
    // The context is not using `Resolver` yet, but we create resolving context through `Resolver`
    // for consistency.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn as_cx<'a, 'gcx>(
        gcx: &'gcx GlobalCx<'gcx>,
        stree: &'a mut SyntaxTree,
        ptree: &'a mut PrivPathTree<'gcx>,
        s2p: &'a mut SynToPath,
        evaluated: &'a mut Evaluated<'gcx>,
        type_inspector: &'a mut Inspector<'gcx>,
        logic: &'a mut Logic<'gcx>,
        tasks: &'a mut TaskQueue<'gcx>,
    ) -> ResolveCx<'a, 'gcx> {
        ResolveCx {
            gcx,
            stree,
            ptree,
            s2p,
            evaluated,
            type_inspector,
            logic,
            tasks,
        }
    }
}

/// Converts raw items into resolved items in the path tree.
pub(super) struct ResolveCx<'a, 'gcx> {
    gcx: &'gcx GlobalCx<'gcx>,
    stree: &'a mut SyntaxTree,
    ptree: &'a mut PrivPathTree<'gcx>,
    s2p: &'a mut SynToPath,
    evaluated: &'a mut Evaluated<'gcx>,
    type_inspector: &'a mut Inspector<'gcx>,
    logic: &'a mut Logic<'gcx>,
    tasks: &'a mut TaskQueue<'gcx>,
}

impl<'a, 'gcx> ResolveCx<'a, 'gcx> {
    pub(super) fn resolve_const_vis(&mut self, pid: PathId) {
        let raw = self.ptree[pid].as_raw_const();
        let vis_node = help_vis::scoped_vis_node(self.ptree, pid.ni, &raw.visibility());
        match self.ptree.get_mut_item(pid).as_raw_const() {
            RawConst::Free { vis_node: dst, .. } => *dst = Some(vis_node),
        }
    }

    pub(super) fn resolve_enum_vis(&mut self, pid: PathId) {
        let raw = self.ptree[pid].as_raw_enum();
        let vis_node = help_vis::scoped_vis_node(self.ptree, pid.ni, &raw.visibility());
        self.ptree.get_mut_item(pid).as_raw_enum().vis_node = Some(vis_node);
    }

    pub(super) fn resolve_field_vis(&mut self, pid: PathId) {
        let raw = self.ptree[pid].as_raw_field();
        let vis_node = help_vis::scoped_vis_node(self.ptree, pid.ni, &raw.visibility());
        self.ptree.get_mut_item(pid).as_raw_field().vis_node = Some(vis_node);
    }

    pub(super) fn resolve_fn_vis(&mut self, pid: PathId) {
        let raw = self.ptree[pid].as_raw_fn();
        let vis_node = if let Some(unscoped_base) = raw.unscoped_base {
            help_vis::unscoped_vis_node(self.ptree, unscoped_base, &raw.visibility())
        } else {
            help_vis::scoped_vis_node(self.ptree, pid.ni, &raw.visibility())
        };
        self.ptree.get_mut_item(pid).as_raw_fn().vis_node = Some(vis_node);
    }

    pub(super) fn resolve_mod_vis(&mut self, pid: PathId) {
        let raw = self.ptree[pid].as_raw_mod();
        let vis_node = help_vis::scoped_vis_node(self.ptree, pid.ni, &raw.visibility());
        let resolved = PrivItem::Mod(Mod {
            ptr_mod: raw.ptr_mod,
            ptr_file: raw.ptr_file,
            vis_node,
            fpath: raw.fpath.clone(),
            mod_rs: raw.mod_rs,
        });
        self.ptree.set_item(pid, resolved);
    }

    pub(super) fn resolve_struct_vis(&mut self, pid: PathId) {
        let raw = self.ptree[pid].as_raw_struct();
        let vis_node = help_vis::scoped_vis_node(self.ptree, pid.ni, &raw.visibility());
        self.ptree.get_mut_item(pid).as_raw_struct().vis_node = Some(vis_node);
    }

    pub(super) fn resolve_trait_vis(&mut self, pid: PathId) {
        let raw = self.ptree[pid].as_raw_trait();
        let vis_node = help_vis::scoped_vis_node(self.ptree, pid.ni, &raw.visibility());
        let resolved = PrivItem::Trait(Trait {
            ptr_trait: raw.ptr_trait,
            vis_node,
        });
        self.ptree.set_item(pid, resolved);
    }

    pub(super) fn resolve_type_alias_vis(&mut self, pid: PathId) {
        let raw = self.ptree[pid].as_raw_type_alias();
        let vis_node = help_vis::scoped_vis_node(self.ptree, pid.ni, &raw.visibility());
        self.ptree.get_mut_item(pid).as_raw_type_alias().vis_node = Some(vis_node);
    }

    pub(super) fn resolve_use_vis(&mut self, pid: PathId) {
        let raw = self.ptree[pid].as_raw_use();
        let vis_node = help_vis::scoped_vis_node(self.ptree, pid.ni, &raw.visibility());
        self.ptree.get_mut_item(pid).as_raw_use().vis_node = Some(vis_node);
    }

    pub(super) fn resolve_variant_vis(&mut self, pid: PathId) -> TriResult<(), ()> {
        let parent = self.ptree.parent_item(pid.ni, filter::enum_);
        match self.ptree[parent].vis_node() {
            TriOption::Some(vis_node) => {
                self.ptree.get_mut_item(pid).as_raw_variant().vis_node = Some(vis_node);
                Ok(())
            }
            TriOption::NotYet(()) => err!(soft, ()),
            TriOption::None => err!(hard, ""),
        }
    }

    pub(super) fn resolve_const_type(&mut self, pid: PathId) -> TriResult<(), ()> {
        let raw = self.ptree[pid].as_raw_const();
        let base = self.ptree.parent_item(pid.ni, filter::block_mod).ni;

        match raw {
            RawConst::Free {
                ptr_const,
                vis_node,
                tid,
            } => {
                let tid = if let Some(tid) = tid {
                    *tid
                } else if let Some(tid) =
                    help_ty::syn_type_to_type_id(raw.syn_type(), self.stree, self.ptree, base)
                {
                    tid
                } else {
                    return err!(soft, ());
                };
                let resolved = PrivItem::Const(Const::Free {
                    ptr_const: *ptr_const,
                    vis_node: vis_node.unwrap(),
                    tid,
                });
                self.ptree.set_item(pid, resolved);
                Ok(())
            }
        }
    }

    pub(super) fn resolve_enum_type(&mut self, pid: PathId) -> TriResult<(), ()> {
        let raw = self.ptree[pid].as_raw_enum();

        let tid = self.ptree.insert_type(Type::Path(TypePath {
            pid,
            params: [].into(),
        }));

        let resolved = PrivItem::Enum(Enum {
            ptr_enum: raw.ptr_enum,
            vis_node: raw.vis_node.unwrap(),
            tid,
        });
        self.ptree.set_item(pid, resolved);
        Ok(())
    }

    pub(super) fn resolve_field_type(&mut self, pid: PathId) -> TriResult<(), ()> {
        let raw = self.ptree[pid].as_raw_field();
        let base = self.ptree.parent_item(pid.ni, filter::block_mod).ni;

        let Some(tid) =
            help_ty::syn_type_to_type_id(&raw.as_syn().ty, self.stree, self.ptree, base)
        else {
            return err!(soft, ());
        };

        let resolved = PrivItem::Field(Field {
            ptr_field: raw.ptr_field,
            vis_node: raw.vis_node.unwrap(),
            tid,
        });
        self.ptree.set_item(pid, resolved);
        Ok(())
    }

    pub(super) fn resolve_fn_type(&mut self, pid: PathId) -> TriResult<(), ()> {
        let raw = self.ptree[pid].as_raw_fn();
        let base = self.ptree.parent_item(pid.ni, filter::block_mod_struct).ni;

        let output = match &raw.as_syn_sig().output {
            syn::ReturnType::Default => self.ptree.insert_type(Type::Unit),
            syn::ReturnType::Type(_, ty) => {
                match help_ty::syn_type_to_type_id(ty, self.stree, self.ptree, base) {
                    Some(tid) => tid,
                    None => return err!(soft, ()),
                }
            }
        };

        let inputs = raw.as_syn_sig().inputs.iter().filter_map(|arg| match arg {
            syn::FnArg::Receiver(v) => {
                let self_pid = self.ptree.parent_item(pid.ni, filter::struct_);

                let mut self_tid = match &self.ptree[self_pid] {
                    PrivItem::Struct(v) => v.tid,
                    PrivItem::RawStruct(_) => return None,
                    o => todo!("{o:?}"),
                };

                if v.mutability.is_some() {
                    self_tid = self
                        .ptree
                        .insert_type(Type::Mut(TypeMut { elem: self_tid }));
                }

                if v.reference.is_some() {
                    self_tid = self
                        .ptree
                        .insert_type(Type::Ref(TypeRef { elem: self_tid }));
                }

                Some(self_tid)
            }
            syn::FnArg::Typed(v) => {
                help_ty::syn_type_to_type_id(&v.ty, self.stree, self.ptree, base)
            }
        });

        let params: BoxedSlice<Param> = iter::once(output)
            .chain(inputs)
            .enumerate()
            .map(|(i, tid)| Param::Other {
                name: self.gcx.intern_str(&i.to_string()),
                tid,
            })
            .collect();

        if params.len() != raw.as_syn_sig().inputs.len() + 1 {
            return err!(soft, ());
        }

        let tid = self.ptree.insert_type(Type::Path(TypePath { pid, params }));

        let resolved = PrivItem::Fn(Fn {
            ptr_attr: raw.ptr_attr,
            ptr_sig: raw.ptr_sig,
            ptr_block: raw.ptr_block,
            vis_node: raw.vis_node.unwrap(),
            tid,
        });
        self.ptree.set_item(pid, resolved);
        Ok(())
    }

    pub(super) fn resolve_local_type(&mut self, pid: PathId) -> TriResult<(), ()> {
        let PrivItem::RawLocal(raw) = &self.ptree[pid] else {
            return Ok(());
        };

        let mut fn_args = Vec::new();

        let mut top_blocks = help_local::TopBlocks {
            ptree: self.ptree,
            tops: Map::default(),
        };

        let parent = self.ptree[pid.ni].parent;
        for (_, parent_item) in self.ptree[parent].iter() {
            match parent_item {
                PrivItem::Fn(Fn { ptr_sig, .. }) | PrivItem::RawFn(RawFn { ptr_sig, .. }) => {
                    let Some(arg) = self.stree.get_ancestor1::<syn::FnArg>(raw.syn_id()) else {
                        continue;
                    };

                    let sig = unsafe { ptr_sig.as_ref() };
                    if sig.inputs.iter().any(|input| input == arg) {
                        fn_args.push((arg as *const syn::FnArg, pid));
                        break;
                    }
                }
                PrivItem::Block(_) => {
                    top_blocks.find_top_block(pid);
                    break;
                }
                _ => {}
            }
        }

        let top_blocks = top_blocks.tops;

        for (fn_arg, pid) in fn_args {
            let fn_arg = unsafe { fn_arg.as_ref().unwrap() };
            let local_syn = self.ptree[pid].as_raw_local().ptr_ident;
            FnArgResolver {
                stree: self.stree,
                ptree: self.ptree,
                local_pid: pid,
                local_syn,
            }
            .resolve(fn_arg)?;
        }

        if top_blocks.is_empty() {
            return err!(soft, ());
        }

        debug_assert!(!top_blocks.is_empty());

        for (block, local_pids) in top_blocks {
            let block = unsafe { block.as_ref().unwrap() };
            self.resolve_local_in_block(block, local_pids.iter().cloned())?;
        }
        Ok(())
    }

    fn resolve_local_in_block<I: Iterator<Item = PathId>>(
        &mut self,
        block: &syn::Block,
        local_pids: I,
    ) -> TriResult<(), ()> {
        // Infers about the top block.
        // TODO: infer_signature_and_block
        // TODO: Need a test case to test inference about a block return type.
        let base = self.s2p.get_path_id(block.syn_id()).unwrap().ni;
        self.type_inspector
            .as_infer_helper(
                self.gcx,
                self.stree,
                self.ptree,
                self.s2p,
                self.evaluated,
                self.logic,
                self.tasks,
                base,
            )
            .infer_block(block, None)?;

        for local_pid in local_pids {
            if !matches!(self.ptree[local_pid], PrivItem::RawLocal(_)) {
                continue;
            }

            let Which2::A(pat_ident) = &self.ptree[local_pid].as_raw_local().as_syn() else {
                // It returns PatIdent or Receiver, but Receiver is not expected here because we
                // are in a block, not a function.
                unreachable!()
            };

            let infer_ty = self
                .type_inspector
                .get_infer_type(&pat_ident.ident)
                .unwrap()
                .clone();
            let base = self.ptree.parent_item(local_pid.ni, filter::block_mod).ni;
            let infer = self.type_inspector.as_infer_helper(
                self.gcx,
                self.stree,
                self.ptree,
                self.s2p,
                self.evaluated,
                self.logic,
                self.tasks,
                base,
            );
            let tid = TypeId::from_infer_type(infer_ty, &infer)?;

            let raw = self.ptree[local_pid].as_raw_local();
            let resolved = PrivItem::Local(Local {
                ptr_attr: raw.ptr_attr,
                ptr_ident: raw.ptr_ident,
                ptr_ty: raw.ptr_ty,
                tid,
            });
            self.ptree.set_item(local_pid, resolved);
        }
        Ok(())
    }

    pub(super) fn resolve_struct_type(&mut self, pid: PathId) -> TriResult<(), ()> {
        let raw = self.ptree[pid].as_raw_struct();
        let base = self.ptree.parent_item(pid.ni, filter::block_mod).ni;

        let fields = raw
            .as_syn()
            .fields
            .iter()
            .enumerate()
            .filter_map(|(i, field)| {
                let name = field
                    .ident
                    .as_ref()
                    .map(|ident| ident.to_string())
                    .unwrap_or_else(|| (i + 1).to_string());
                let tid = help_ty::syn_type_to_type_id(&field.ty, self.stree, self.ptree, base)?;
                Some(Param::Other {
                    name: self.gcx.intern_str(&name),
                    tid,
                })
            });

        let params = iter::once(Param::Self_)
            .chain(fields)
            .collect::<BoxedSlice<_>>();

        if params.len() != raw.as_syn().fields.len() + 1 {
            return err!(soft, ());
        }

        let tid = self.ptree.insert_type(Type::Path(TypePath { pid, params }));

        let resolved = PrivItem::Struct(Struct {
            ptr_struct: raw.ptr_struct,
            vis_node: raw.vis_node.unwrap(),
            tid,
        });
        self.ptree.set_item(pid, resolved);
        Ok(())
    }

    pub(super) fn resolve_type_alias_type(&mut self, pid: PathId) -> TriResult<(), ()> {
        let raw = self.ptree[pid].as_raw_type_alias();
        let base = self.ptree.parent_item(pid.ni, filter::block_mod).ni;

        let Some(tid) =
            help_ty::syn_type_to_type_id(&raw.as_syn().ty, self.stree, self.ptree, base)
        else {
            return err!(soft, ());
        };

        let resolved = PrivItem::TypeAlias(TypeAlias {
            ptr_type: raw.ptr_type,
            vis_node: raw.vis_node.unwrap(),
            tid,
        });
        self.ptree.set_item(pid, resolved);
        Ok(())
    }

    pub(super) fn resolve_variant_disc(&mut self, pid: PathId) -> TriResult<(), ()> {
        let raw = self.ptree[pid].as_raw_variant();

        // Variant follows type id of its parent enum.
        let parent = self.ptree.parent_item(pid.ni, filter::enum_);
        let tid = if let PrivItem::Enum(enum_) = &self.ptree[parent] {
            enum_.tid
        } else {
            return err!(soft, ());
        };

        let disc = if let Some((_, expr)) = &raw.as_syn().discriminant {
            let base = self.ptree.parent_item(pid.ni, filter::block_mod).ni;
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
            match eval.eval_expr(expr)? {
                eval::Value::Scalar(eval::Scalar::Int(i)) => i as isize,
                eval::Value::Scalar(eval::Scalar::Isize(i)) => i,
                o => {
                    return err!(
                        hard,
                        "expected isize for an enum discriminant, but found {o:?}"
                    );
                }
            }
        } else if raw.nth > 0 {
            let parent = self.ptree.parent_item(pid.ni, filter::enum_).ni;

            let find_prev_disc = |node: NodeIndex| {
                self.ptree[node]
                    .iter()
                    .find_map(|(_, child_item)| match child_item {
                        PrivItem::Variant(Variant { nth, disc, .. }) if nth + 1 == raw.nth => {
                            Some(*disc)
                        }
                        _ => None,
                    })
            };

            if let Some(prev_disc) = self.ptree[parent]
                .children
                .iter()
                .find_map(|(_, child_node)| find_prev_disc(*child_node))
            {
                prev_disc + 1
            } else {
                return err!(soft, ());
            }
        } else {
            0
        };

        let resolved = PrivItem::Variant(Variant {
            ptr_variant: raw.ptr_variant,
            vis_node: raw.vis_node.unwrap(),
            tid,
            nth: raw.nth,
            disc,
        });
        self.ptree.set_item(pid, resolved);
        Ok(())
    }

    pub(super) fn fix_const_type_len(&mut self, pid: PathId) -> TriResult<(), ()> {
        let PrivItem::Const(const_) = &self.ptree[pid] else {
            return err!(soft, ());
        };

        let base = self.ptree.parent_item(pid.ni, filter::block_mod).ni;
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

        help_ty::fix_array_type(const_.syn_type(), const_.type_id(), &mut eval)
    }

    pub(super) fn fix_field_type_len(&mut self, pid: PathId) -> TriResult<(), ()> {
        let PrivItem::Field(field) = &self.ptree[pid] else {
            return err!(soft, ());
        };

        let base = self.ptree.parent_item(pid.ni, filter::block_mod).ni;
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

        help_ty::fix_array_type(&field.as_syn().ty, field.tid, &mut eval)
    }

    pub(super) fn fix_fn_type_len(&mut self, pid: PathId) -> TriResult<(), ()> {
        let PrivItem::Fn(fn_) = &self.ptree[pid] else {
            return err!(soft, ());
        };

        let base = self.ptree.parent_item(pid.ni, filter::block_mod_struct).ni;
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

        let ty = eval.ptree.get_type(fn_.tid);

        let Type::Path(TypePath { params, .. }) = ty else {
            return Ok(());
        };

        let tree_output = match params.iter().next().unwrap() {
            Param::Self_ => unreachable!(),
            Param::Other { tid, .. } => *tid,
        };
        let tree_inputs = params.iter().skip(1).map(|param| match param {
            Param::Self_ => unreachable!(),
            Param::Other { tid, .. } => *tid,
        });

        match &fn_.syn_sig().output {
            syn::ReturnType::Default => {}
            syn::ReturnType::Type(_, o_ty) => {
                help_ty::fix_array_type(o_ty, tree_output, &mut eval)?;
            }
        }

        for (syn_input, tree_input) in fn_.syn_sig().inputs.iter().zip(tree_inputs) {
            match syn_input {
                syn::FnArg::Receiver(_) => {}
                syn::FnArg::Typed(syn::PatType { ty: o_ty, .. }) => {
                    help_ty::fix_array_type(o_ty, tree_input, &mut eval)?;
                }
            }
        }
        Ok(())
    }

    pub(super) fn fix_local_type_len(&mut self, pid: PathId) -> TriResult<(), ()> {
        let PrivItem::Local(local) = &self.ptree[pid] else {
            return err!(soft, ());
        };

        if let Some(ty) = local.syn_type() {
            let base = self.ptree.parent_item(pid.ni, filter::block_fn).ni;
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

            help_ty::fix_array_type(ty, local.tid, &mut eval)?;
        }
        Ok(())
    }

    pub(super) fn fix_struct_type_len(&mut self, pid: PathId) -> TriResult<(), ()> {
        let PrivItem::Struct(st) = &self.ptree[pid] else {
            return err!(soft, ());
        };

        let base = self.ptree.parent_item(pid.ni, filter::block_mod).ni;
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

        let Type::Path(ty) = &self.ptree.types()[st.tid] else {
            unreachable!()
        };

        let syn_fields = st.as_syn().fields.iter();
        let ty_params = ty.params.iter().filter_map(|param| match param {
            Param::Self_ => None,
            Param::Other { tid, .. } => Some(*tid),
        });

        for (syn_field, ty_param) in syn_fields.zip(ty_params) {
            help_ty::fix_array_type(&syn_field.ty, ty_param, &mut eval)?;
        }
        Ok(())
    }

    pub(super) fn fix_type_alias_type_len(&mut self, pid: PathId) -> TriResult<(), ()> {
        let PrivItem::TypeAlias(alias) = &self.ptree[pid] else {
            return err!(soft, ());
        };

        let base = self.ptree.parent_item(pid.ni, filter::block_mod).ni;
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

        help_ty::fix_array_type(&alias.as_syn().ty, alias.tid, &mut eval)
    }

    pub(super) fn fix_type_impl_type_len(&mut self, task: TaskFixImplType) -> TriResult<(), ()> {
        let TaskFixImplType {
            ty,
            generics,
            self_ty,
            base,
        } = task;

        let syn_ty = ty.as_ref::<syn::Type>().unwrap();
        let generics = generics.as_ref::<syn::Generics>().unwrap();

        // If the self type is a generic array, then processes it here.
        let mut is_generic_array = false;
        if let syn::Type::Array(syn::TypeArray {
            len: syn::Expr::Path(expr_path),
            ..
        }) = syn_ty
        {
            if let Some(len_ident) = expr_path.path.get_ident() {
                if generics.params.iter().any(|param| {
                    matches!(
                        param,
                        syn::GenericParam::Const(syn::ConstParam { ident, .. })
                        if ident == len_ident
                    )
                }) {
                    is_generic_array = true;
                }
            }
        }
        if is_generic_array {
            let self_ty = self.ptree.get_type(self_ty).clone();

            let Type::Array(TypeArray { elem, .. }) = self_ty else {
                return Ok(());
            };

            let resolved = Type::Array(TypeArray {
                elem,
                len: ArrayLen::Generic,
            });
            self.ptree.replace_type(self_ty, resolved);
            return Ok(());
        }

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

        help_ty::fix_array_type(syn_ty, self_ty, &mut eval)
    }

    // TODO: The whole 'use' resolving and shortening should be reconsidered.
    pub(super) fn resolve_use_dst(&mut self) -> TriResult<(), ()> {
        // Returns if visibilities are not completely resolved yet.
        if self.ptree.unresolved().any(|pid| {
            matches!(
                &self.ptree[pid],
                PrivItem::RawUse(raw)
                if raw.vis_node.is_none()
            )
        }) {
            return err!(soft, ());
        }

        // Gathers glob and non-glob `UseRaw`s.
        let mut globs = VecDeque::new();
        let mut non_globs = VecDeque::new();
        for pid in self.ptree.unresolved() {
            if let PrivItem::RawUse(raw) = &self.ptree[pid] {
                if matches!(raw.npath.segments().last(), Some("*")) {
                    globs.push_back(pid);
                } else {
                    non_globs.push_back(pid);
                }
            }
        }

        let mut changed = true;
        while changed && (!globs.is_empty() || !non_globs.is_empty()) {
            changed = false;

            // Tries to expand all globs.
            let n = globs.len();
            for _ in 0..n {
                let pid = globs.pop_front().unwrap();
                let mut visit = Set::default();
                if !expand_glob(self.ptree, pid, &mut visit, &mut non_globs)? {
                    globs.push_back(pid);
                    continue;
                }

                self.ptree.take_item(pid);
                changed = true;
            }

            // Tries to search destinations of `UseRaw`s.
            for pid in &non_globs {
                if self.ptree[*pid].as_raw_use().dst_node.is_some() {
                    continue;
                }

                let dst = search_dst_of_use_raw(self.ptree, *pid);
                let mut item = self.ptree.get_mut_item(*pid);
                let raw = item.as_raw_use();
                changed |= raw.dst_node != dst;
                raw.dst_node = dst;
            }

            // Tries to turn `UseRaw`s into `Use`s.
            let n = non_globs.len();
            for _ in 0..n {
                let pid = non_globs.pop_front().unwrap();

                if !self.ptree[pid].is_raw() {
                    continue;
                }

                let mut visit = Set::default();
                let mut terminals = Vec::new();
                if !find_terminal_of_use_raw(self.ptree, pid, &mut visit, &mut terminals) {
                    non_globs.push_back(pid);
                    continue;
                }

                match terminals.len() as u32 {
                    0 => {}
                    1 => {
                        let raw = self.ptree[pid].as_raw_use();
                        let resolved = PrivItem::Use(Use {
                            ptr_group: raw.ptr_group,
                            syn_part: raw.syn_part,
                            vis_node: raw.vis_node.unwrap(),
                            dst: terminals[0],
                        });
                        self.ptree.set_item(pid, resolved);
                        changed = true;
                    }
                    2.. => {
                        let old = self.ptree.take_item(pid);
                        let raw = old.as_raw_use();
                        for terminal in terminals {
                            if terminal.ni == pid.ni {
                                continue;
                            }
                            let resolved = PrivItem::Use(Use {
                                ptr_group: raw.ptr_group,
                                syn_part: raw.syn_part,
                                vis_node: raw.vis_node.unwrap(),
                                dst: terminal,
                            });
                            self.ptree.add_item(pid.ni, "", resolved);
                        }
                        changed = true;
                    }
                }
            }
        }

        remove_duplicate(self.ptree);

        if self
            .ptree
            .unresolved()
            .any(|pid| matches!(self.ptree[pid], PrivItem::RawUse(_)))
        {
            return err!(soft, ());
        }

        return Ok(());

        // === Internal helper functions ===

        fn expand_glob<'gcx>(
            ptree: &mut PrivPathTree<'gcx>,
            src: PathId,
            visit: &mut Set<PathId>,
            non_globs: &mut VecDeque<PathId>,
        ) -> TriResult<bool, ()> {
            if !visit.insert(src) {
                return Ok(true);
            }

            let dst = search_dst_of_use_raw(ptree, src);
            let Some(dst) = dst else {
                return err!(soft, ());
            };

            // 1st traverse: Expands globs in the destination module first.
            let num_children = ptree[dst].children.len();
            for child in 0..num_children {
                let (path, node) = &ptree[dst].children[child];
                if path != "*" {
                    continue;
                }
                let node = *node;

                let num_items = ptree[node].items.len();
                for ii in 0..num_items {
                    let pid = node.to_path_id(ii);
                    let item = &ptree[pid];
                    if matches!(item, PrivItem::None) {
                        continue;
                    }
                    let vis_node = match item.vis_node() {
                        TriOption::Some(vis_node) => vis_node,
                        TriOption::NotYet(()) => return err!(soft, ()),
                        TriOption::None => continue,
                    };
                    if !ptree.is_descendant(pid.ni, vis_node) {
                        continue;
                    }
                    if !expand_glob(ptree, pid, visit, non_globs)? {
                        return Ok(false);
                    }
                }
            }

            // 2nd traverse
            // - Visits non-glob items of the destination module including
            //   expanded items through the 1st traverse.
            // - Expands the path tree for the items.
            // - The expansion is not recursive, which means it brings direct
            //   children only.
            let mut entries = Vec::new();
            for (path, dst_ni) in &ptree[dst].children {
                if path == "*" {
                    continue;
                }

                for (_, item) in ptree[*dst_ni].iter() {
                    let vis_node = match item.vis_node() {
                        TriOption::Some(vis_node) => vis_node,
                        TriOption::NotYet(()) => return err!(soft, ()),
                        TriOption::None => continue,
                    };
                    if !ptree.is_descendant(src.ni, vis_node) {
                        continue;
                    }

                    let raw = ptree[src].as_raw_use();
                    entries.push((
                        path.clone(),
                        PrivItem::RawUse(RawUse {
                            ptr_group: raw.ptr_group,
                            syn_part: raw.syn_part,
                            vis_node: raw.vis_node,
                            npath: raw.npath.clone(),
                            dst_node: Some(*dst_ni),
                        }),
                    ));
                }
            }

            let base = ptree[src.ni].parent;
            for entry in entries {
                let (path, new_item) = entry;

                let new_pid = ptree.add_item(base, path.as_str(), new_item);
                non_globs.push_back(new_pid);
            }

            Ok(true)
        }

        /// Traverses `UseRaw` chain then finds terminal items.
        ///
        /// * src - Path id to a `UseRaw` item.
        fn find_terminal_of_use_raw(
            ptree: &PrivPathTree,
            src: PathId,
            visit: &mut Set<PathId>,
            terminals: &mut Vec<PathId>,
        ) -> bool {
            if !visit.insert(src) {
                return true;
            }

            let raw = ptree[src].as_raw_use();
            let Some(dst_ni) = raw.dst_node else {
                return false;
            };

            for (dst_ii, dst_item) in ptree[dst_ni].iter() {
                let dst_pid = dst_ni.to_path_id(dst_ii);
                match dst_item {
                    PrivItem::Use(_) => {
                        if !find_terminal_of_use(ptree, dst_pid, visit, terminals) {
                            return false;
                        }
                    }
                    PrivItem::RawUse(_) => {
                        if !find_terminal_of_use_raw(ptree, dst_pid, visit, terminals) {
                            return false;
                        }
                    }
                    _ => terminals.push(dst_pid),
                }
            }
            true
        }

        /// * src - Path id to a `UseRaw` item.
        fn find_terminal_of_use(
            ptree: &PrivPathTree,
            src: PathId,
            visit: &mut Set<PathId>,
            terminals: &mut Vec<PathId>,
        ) -> bool {
            if !visit.insert(src) {
                return true;
            }

            let dst_pid = ptree[src].as_use().dst;
            match &ptree[dst_pid] {
                PrivItem::Use(_) => find_terminal_of_use(ptree, dst_pid, visit, terminals),
                PrivItem::RawUse(_) => find_terminal_of_use_raw(ptree, dst_pid, visit, terminals),
                _ => {
                    terminals.push(dst_pid);
                    true
                }
            }
        }

        /// Removes duplicate 'Use'.
        ///
        /// Duplication can occur due to recursive imports. By that, a single node can have multiple
        /// same 'Use's, or 'Use' can points to something in the same node. Also, glob and normal
        /// imports can be duplicate.
        fn remove_duplicate(ptree: &mut PrivPathTree) {
            let num_nodes = ptree.num_nodes();
            for ni in 0..num_nodes {
                let ni = NodeIndex(ni);

                let mut items = &ptree[ni].items;
                let num_items = items.len();
                for ri in (0..num_items).rev() {
                    let PrivItem::Use(right) = &items[ri] else {
                        break;
                    };

                    // Destination of a 'Use' pointing to the same node is redundant.
                    if right.dst.ni == ni {
                        ptree.take_item(ni.to_path_id(ri));
                        items = &ptree[ni].items; // Due to the borrow rule
                        continue;
                    }

                    // If we find the same 'Use's in one node, removes right one and keep more
                    // general visibility.
                    for li in 0..ri {
                        let PrivItem::Use(left) = &items[li] else {
                            continue;
                        };

                        if right.dst != left.dst {
                            continue;
                        }

                        let left_vis = left.vis_node;
                        let right_vis = right.vis_node;
                        if ptree.is_descendant(left_vis, right_vis) {
                            let pid = ni.to_path_id(li);
                            ptree.get_mut_item(pid).as_use().vis_node = right_vis;
                        }

                        // Assigns again due to the borrow rule.
                        ptree.take_item(ni.to_path_id(ri));
                        items = &ptree[ni].items; // Due to the borrow rule
                        break;
                    }
                }
            }
        }
    }
}

/// Visibility helper
mod help_vis {
    use super::*;

    /// * ni - Node index to an item that has the given visibility.
    pub(super) fn scoped_vis_node(ptree: &PrivPathTree, ni: NodeIndex, vis: &PathVis) -> NodeIndex {
        enum Wrapper {
            Root,
            Mod,
            Block,
            Fn,
            Struct,
            Trait,
        }

        let climb = |ni: NodeIndex| {
            let mut parent = ptree[ni].parent;

            // There are no items between 'crate' and the entry module. In
            // that case, goes up to the 'crate'.
            while parent != TREE_ROOT && ptree[parent].items.is_empty() {
                parent = ptree[parent].parent;
            }

            for (_, item) in ptree[parent].iter() {
                match item {
                    PrivItem::Mod(_) | PrivItem::RawMod(_) => {
                        return (Wrapper::Mod, parent);
                    }
                    PrivItem::Block(_) => return (Wrapper::Block, parent),
                    PrivItem::Fn(_) | PrivItem::RawFn(_) => {
                        return (Wrapper::Fn, parent);
                    }
                    PrivItem::Struct(_) | PrivItem::RawStruct(_) => {
                        return (Wrapper::Struct, parent);
                    }
                    PrivItem::Trait(_) | PrivItem::RawTrait(_) => {
                        return (Wrapper::Trait, parent);
                    }
                    _ => {}
                }
            }

            debug_assert_eq!(parent, TREE_ROOT);
            (Wrapper::Root, parent)
        };

        // If we meet a block during climbing, then stops.
        let climb_to = |mut ni: NodeIndex, dst: NodeIndex| {
            while ni != dst {
                let (w, p) = climb(ni);
                ni = p;
                if matches!(w, Wrapper::Block) {
                    break;
                }
            }
            ni
        };

        match vis {
            PathVis::Pub => climb_to(ni, TREE_ROOT),
            PathVis::PubCrate => {
                let dst = ptree.crate_node();
                climb_to(ni, dst)
            }
            PathVis::PubSuper => {
                let cont = ptree.parent_item(ni, filter::mod_).ni;
                let dst = ptree.parent_item(cont, filter::mod_).ni;
                climb_to(ni, dst)
            }
            PathVis::PubPath(path) => {
                let cont = ptree.parent_item(ni, filter::mod_).ni;
                let dst = ptree.norm_search(cont, path.as_str()).unwrap();
                climb_to(ni, dst)
            }
            PathVis::Private => {
                let dst = ptree.parent_item(ni, filter::mod_).ni;
                climb_to(ni, dst)
            }
        }
    }

    /// * base_mod - Node index to the nearest ancestor mod that contains
    ///   the given unscoped visibility.
    pub(super) fn unscoped_vis_node(
        ptree: &PrivPathTree,
        base_mod: NodeIndex,
        vis: &PathVis,
    ) -> NodeIndex {
        match vis {
            PathVis::Pub => TREE_ROOT,
            PathVis::PubCrate => ptree.crate_node(),
            PathVis::PubSuper => ptree.parent_item(base_mod, filter::mod_).ni,
            PathVis::PubPath(path) => ptree.norm_search(base_mod, path.as_str()).unwrap(),
            PathVis::Private => base_mod,
        }
    }
}

/// Type helper
mod help_ty {
    use super::*;

    pub(super) fn syn_type_to_type_id(
        ty: &syn::Type,
        stree: &SyntaxTree,
        ptree: &PrivPathTree,
        base: NodeIndex,
    ) -> Option<TypeId> {
        match TypeId::from_syn_type(ty, stree, ptree, base) {
            SearchTypeOk(tid) => Some(tid),
            SearchTypeNotReady(_) | SearchTypeNotFound(()) => None,
        }
    }

    pub(super) fn fix_array_type<'gcx, T: ItemTrait>(
        syn_ty: &syn::Type,
        tid: TypeId,
        eval: &mut EvalHelper<'_, 'gcx, T>,
    ) -> TriResult<(), ()> {
        let ty = eval.ptree.get_type(tid).clone();

        let Type::Array(TypeArray { elem, .. }) = ty else {
            return Ok(());
        };

        let len = match syn_ty {
            syn::Type::Array(syn::TypeArray { len, .. }) => match eval.eval_expr(len)? {
                eval::Value::Scalar(eval::Scalar::Usize(len)) => ArrayLen::Fixed(len),
                _ => unreachable!(),
            },
            syn::Type::Slice(syn::TypeSlice { .. }) => ArrayLen::Dynamic,
            _ => return Ok(()),
        };

        let resolved = Type::Array(TypeArray { elem, len });
        eval.ptree.replace_type(ty, resolved);
        Ok(())
    }
}

mod help_local {
    use super::*;

    pub(super) struct TopBlocks<'a, 'gcx> {
        pub(super) ptree: &'a PrivPathTree<'gcx>,
        pub(super) tops: Map<*const syn::Block, Vec<PathId>>,
    }

    impl<'gcx> TopBlocks<'_, 'gcx> {
        /// * pid - Path id to a local variable.
        ///   (can be a path id to a block internally)
        pub(super) fn find_top_block(&mut self, local_pid: PathId) -> PathId {
            let parent = self.ptree.parent_item(local_pid.ni, filter::block);
            if parent.ni == TREE_ROOT {
                return local_pid;
            }

            let top = self.find_top_block(parent);

            if matches!(self.ptree[local_pid], PrivItem::RawLocal(_)) {
                self.tops
                    .entry(self.ptree[top].as_block().ptr_syn())
                    .and_modify(|pids| pids.push(local_pid))
                    .or_insert(vec![local_pid]);
            }

            top
        }
    }
}

struct FnArgResolver<'a, 'gcx> {
    stree: &'a SyntaxTree,
    ptree: &'a mut PrivPathTree<'gcx>,
    local_pid: PathId,
    local_syn: Which2<NonNull<syn::PatIdent>, NonNull<syn::Receiver>>,
}

impl<'gcx> FnArgResolver<'_, 'gcx> {
    fn resolve(&mut self, fn_arg: &syn::FnArg) -> TriResult<(), ()> {
        match fn_arg {
            syn::FnArg::Receiver(v) => self.resolve_receiver(v),
            syn::FnArg::Typed(v) => self.resolve_pat_type(v),
        }
    }

    fn resolve_receiver(&mut self, recv: &syn::Receiver) -> TriResult<(), ()> {
        let base = self.ptree[self.local_pid.ni].parent;
        let SearchTypeOk(tid) = TypeId::from_syn_type(&recv.ty, self.stree, self.ptree, base)
        else {
            unreachable!()
        };

        let raw = self.ptree[self.local_pid].as_raw_local();
        let resolved = PrivItem::Local(Local {
            ptr_attr: raw.ptr_attr,
            ptr_ident: raw.ptr_ident,
            ptr_ty: raw.ptr_ty,
            tid,
        });
        self.ptree.set_item(self.local_pid, resolved);
        Ok(())
    }

    fn resolve_pat(&mut self, pat: &syn::Pat, tid: Option<TypeId>) -> TriResult<(), ()> {
        if let Which2::A(pat_ident) = &self.local_syn {
            if !pat.contains(pat_ident.as_ptr().cast_const()) {
                return Ok(());
            }
        }

        match pat {
            syn::Pat::Ident(v) => self.resolve_ident(&v.ident, tid.unwrap()),
            syn::Pat::Struct(v) => self.resolve_pat_struct(v)?,
            syn::Pat::Tuple(v) => self.resolve_pat_tuple(v)?,
            syn::Pat::Type(v) => self.resolve_pat_type(v)?,
            _ => todo!(),
        }
        Ok(())
    }

    fn resolve_pat_struct(&mut self, pat_struct: &syn::PatStruct) -> TriResult<(), ()> {
        let base = self.ptree[self.local_pid.ni].parent;
        let SearchTypeOk(tid) = TypeId::from_syn_path(
            pat_struct.qself.as_ref(),
            &pat_struct.path,
            self.stree,
            self.ptree,
            base,
        ) else {
            return err!(soft, ());
        };

        let Type::Path(TypePath { pid, .. }) = self.ptree.get_type(tid) else {
            unreachable!()
        };
        let ni = pid.ni;

        for field in &pat_struct.fields {
            let key = match &field.member {
                syn::Member::Named(v) => v.to_string(),
                syn::Member::Unnamed(v) => v.index.to_string(),
            };
            let tid = match self.ptree.norm_search_type(ni, key.as_str()) {
                SearchTypeOk(tid) => tid,
                SearchTypeNotReady(_) => return err!(soft, ()),
                SearchTypeNotFound(()) => {
                    return err!(
                        hard,
                        "could not find {}::{key}",
                        self.ptree.get_name_path(ni)
                    );
                }
            };
            self.resolve_pat(&field.pat, Some(tid))?;
        }
        Ok(())
    }

    fn resolve_pat_tuple(&mut self, _pat_tuple: &syn::PatTuple) -> TriResult<(), ()> {
        todo!()
    }

    fn resolve_pat_type(&mut self, pat_type: &syn::PatType) -> TriResult<(), ()> {
        let base = self.ptree[self.local_pid.ni].parent;
        match TypeId::from_syn_type(&pat_type.ty, self.stree, self.ptree, base) {
            SearchTypeOk(tid) => self.resolve_pat(&pat_type.pat, Some(tid)),
            _ => err!(soft, ()),
        }
    }

    fn resolve_ident(&mut self, ident: &syn::Ident, tid: TypeId) {
        let raw = self.ptree[self.local_pid].as_raw_local();
        if let Which2::A(pat_ident) = &raw.as_syn() {
            if &pat_ident.ident != ident {
                return;
            }
        }

        let resolved = PrivItem::Local(Local {
            ptr_attr: raw.ptr_attr,
            ptr_ident: raw.ptr_ident,
            ptr_ty: raw.ptr_ty,
            tid,
        });
        self.ptree.set_item(self.local_pid, resolved);
    }
}

trait ContainsPatIdent {
    fn contains(&self, target: *const syn::PatIdent) -> bool;
}

impl ContainsPatIdent for syn::Pat {
    fn contains(&self, target: *const syn::PatIdent) -> bool {
        match self {
            syn::Pat::Ident(v) => v.contains(target),
            syn::Pat::Struct(v) => v.contains(target),
            syn::Pat::Tuple(v) => v.contains(target),
            syn::Pat::Type(v) => v.contains(target),
            _ => false,
        }
    }
}

impl ContainsPatIdent for syn::PatIdent {
    fn contains(&self, target: *const syn::PatIdent) -> bool {
        ptr::eq(self, target)
    }
}

impl ContainsPatIdent for syn::PatStruct {
    fn contains(&self, target: *const syn::PatIdent) -> bool {
        self.fields.iter().any(|field| field.pat.contains(target))
    }
}

impl ContainsPatIdent for syn::PatTuple {
    fn contains(&self, target: *const syn::PatIdent) -> bool {
        self.elems.iter().any(|elem| elem.contains(target))
    }
}

impl ContainsPatIdent for syn::PatType {
    fn contains(&self, target: *const syn::PatIdent) -> bool {
        self.pat.contains(target)
    }
}

fn search_dst_of_use_raw(ptree: &PrivPathTree, pid: PathId) -> Option<NodeIndex> {
    let base = ptree.parent_item(pid.ni, filter::mod_).ni;
    let key = ptree[pid].as_raw_use().npath.segments();

    // Filters out destination that is the same as the input.
    // e.g. mod m { use foo as bar; }
    // In the case above, `traverse` will reach not only "foo"(extern) but also "m::foo"(input).
    // But we definitely don't want "m::foo"(input).
    if matches!(key.clone().last(), Some("*")) {
        let key_len = key.clone().count();
        let key = PathSegments(key.take(key_len - 1));
        ptree.norm_traverse(base, key, |_vis, dst_pid, _| {
            (pid != dst_pid).then_some(dst_pid.ni)
        })
    } else {
        let key = PathSegments(key);
        ptree.norm_traverse(base, key, |_vis, dst_pid, _| {
            (pid != dst_pid).then_some(dst_pid.ni)
        })
    }
}

pub(super) fn shorten_chain(ptree: &mut PrivPathTree) {
    let mut visit = Visit {
        visited_pid: Map::default(),
        visited_tid: Set::default(),
        to_update: Map::default(),
    };

    let num_nodes = ptree.num_nodes();
    for ni in 0..num_nodes {
        let ni = NodeIndex(ni);

        let num_items = ptree[ni].items.len();
        for ii in 0..num_items {
            let pid = ni.to_path_id(ii);
            let item = &ptree[pid];
            if matches!(item, PrivItem::None) {
                continue;
            }

            if matches!(item, PrivItem::Use(..) | PrivItem::TypeAlias(..)) {
                find_path_end(ptree, pid, &mut visit);
            }
        }
    }

    for (pid, update) in visit.to_update {
        match update {
            PathOrType::Path(dst) => {
                ptree.get_mut_item(pid).as_use().dst = dst;
            }
            PathOrType::Type(dst) => {
                ptree.get_mut_item(pid).as_type_alias().tid = dst;
            }
        }
    }

    // === Internal helper ===

    #[derive(Clone, Copy, PartialEq, Eq, Hash)]
    enum PathOrType {
        Path(PathId),
        Type(TypeId),
    }

    struct Visit {
        visited_pid: Map<PathId, PathOrType>,
        visited_tid: Set<TypeId>,
        to_update: Map<PathId, PathOrType>,
    }

    fn find_path_end(ptree: &PrivPathTree, input: PathId, visit: &mut Visit) -> PathOrType {
        if let Some(output) = visit.visited_pid.get(&input) {
            return *output;
        }

        let output = match &ptree[input] {
            PrivItem::Use(v) => {
                let res = find_path_end(ptree, v.dst, visit);
                match res {
                    PathOrType::Path(pid) => {
                        visit.to_update.insert(input, PathOrType::Path(pid));
                    }
                    PathOrType::Type(tid) => {
                        let ty = ptree.get_type(tid);
                        if let Type::Path(TypePath { pid, .. }) = ty {
                            visit.to_update.insert(input, PathOrType::Path(*pid));
                        }
                    }
                }
                res
            }
            PrivItem::TypeAlias(alias) => {
                let res = find_type_end(ptree, alias.tid, visit);
                visit.to_update.insert(input, PathOrType::Type(res));
                PathOrType::Type(res)
            }
            PrivItem::Struct(v) => PathOrType::Type(v.tid),
            PrivItem::Fn(v) => PathOrType::Type(v.tid),
            _ => PathOrType::Path(input),
        };

        visit.visited_pid.insert(input, output);
        output
    }

    fn find_type_end(ptree: &PrivPathTree, input: TypeId, visit: &mut Visit) -> TypeId {
        if visit.visited_tid.contains(&input) {
            return input;
        }

        let output = match ptree.get_type(input) {
            Type::Path(TypePath { pid, params }) => {
                for param in params {
                    let Param::Other {
                        name: _,
                        tid: param_tid,
                    } = param
                    else {
                        continue;
                    };
                    find_type_end(ptree, *param_tid, visit);
                }
                let PathOrType::Type(fin) = find_path_end(ptree, *pid, visit) else {
                    // TypeAlias must have pointed to a type.
                    unreachable!()
                };
                fin
            }
            Type::Tuple(TypeTuple { elems }) => {
                let optimized_type = Type::Tuple(TypeTuple {
                    elems: elems
                        .iter()
                        .map(|elem| find_type_end(ptree, *elem, visit))
                        .collect(),
                });
                ptree.insert_type(optimized_type)
            }
            Type::Array(TypeArray { elem, len }) => {
                let optimized_type = Type::Array(TypeArray {
                    elem: find_type_end(ptree, *elem, visit),
                    len: *len,
                });
                ptree.insert_type(optimized_type)
            }
            Type::Ref(TypeRef { elem }) => {
                let optimized_type = Type::Ref(TypeRef {
                    elem: find_type_end(ptree, *elem, visit),
                });
                ptree.insert_type(optimized_type)
            }
            Type::Mut(TypeMut { elem }) => {
                let optimized_type = Type::Mut(TypeMut {
                    elem: find_type_end(ptree, *elem, visit),
                });
                ptree.insert_type(optimized_type)
            }
            Type::Scalar(_) | Type::Unit => input,
        };

        visit.visited_tid.insert(input);
        output
    }
}
