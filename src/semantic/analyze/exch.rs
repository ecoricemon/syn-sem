use super::infer_eval::InferHelper;
use crate::{
    ds::vec::BoxedSlice,
    err,
    etc::util,
    helper,
    semantic::{
        entry::GlobalCx,
        eval::{self, Evaluated},
        infer::{self, InferArrayLen},
        tree::{
            filter, ArrayLen, EffectiveItemKind, ItemTrait, NodeIndex, Param, PathId, PathTree,
            SearchTypeNotFound, SearchTypeNotReady, SearchTypeOk, SearchTypeResult, Type,
            TypeArray, TypeId, TypeMut, TypePath, TypeRef, TypeScalar, TypeTuple,
        },
    },
    syntax::SyntaxTree,
    Intern, TriResult,
};

impl TypeId {
    pub(super) fn from_syn_path<T: ItemTrait>(
        _qself: Option<&syn::QSelf>,
        path: &syn::Path,
        stree: &SyntaxTree,
        ptree: &PathTree<'_, T>,
        base: NodeIndex,
    ) -> SearchTypeResult {
        // Self keyword?
        if path
            .get_ident()
            .map(|ident| ident == "Self")
            .unwrap_or(false)
        {
            let self_pid = ptree.nearest_item(base, filter::enum_struct);
            let self_tid = ptree[self_pid].as_struct().unwrap().tid;
            SearchTypeOk(self_tid)
        }
        // Const generic param? then we can find `syn::Type`.
        else if let Some(sid) = helper::generic::find_const_generic_param(path, stree) {
            let param = sid.as_any().downcast_ref::<syn::ConstParam>().unwrap();
            Self::from_syn_type(&param.ty, stree, ptree, base)
        } else {
            let key = util::get_name_path_from_syn_path(path);
            ptree.norm_search_type(base, key.as_str())
        }
    }

    pub(super) fn from_syn_type<T: ItemTrait>(
        ty: &syn::Type,
        stree: &SyntaxTree,
        ptree: &PathTree<'_, T>,
        base: NodeIndex,
    ) -> SearchTypeResult {
        match ty {
            syn::Type::Array(ty_arr) => {
                let elem = match Self::from_syn_type(&ty_arr.elem, stree, ptree, base) {
                    SearchTypeOk(tid) => tid,
                    o => return o,
                };

                // TODO: Here the length being assigned is dummy unique value. It will be fixed by
                // a task(FixType) in the future. But anyway, this is a bad pattern. It should be
                // revised in a clean way.
                let len = ty as *const _ as usize;

                let tid = ptree.insert_type(Type::Array(TypeArray {
                    elem,
                    len: ArrayLen::Fixed(len),
                }));
                SearchTypeOk(tid)
            }
            syn::Type::Path(ty_path) => {
                Self::from_syn_path(ty_path.qself.as_ref(), &ty_path.path, stree, ptree, base)
            }
            syn::Type::Reference(ty_ref) => {
                let mut elem = match Self::from_syn_type(&ty_ref.elem, stree, ptree, base) {
                    SearchTypeOk(tid) => tid,
                    o => return o,
                };

                if ty_ref.mutability.is_some() {
                    elem = ptree.insert_type(Type::Mut(TypeMut { elem }));
                }

                let tid = ptree.insert_type(Type::Ref(TypeRef { elem }));
                SearchTypeOk(tid)
            }
            syn::Type::Tuple(ty_tuple) => {
                let elems = ty_tuple
                    .elems
                    .iter()
                    .map(|elem| Self::from_syn_type(elem, stree, ptree, base))
                    .collect::<crate::Which3<BoxedSlice<TypeId>, _, _>>();
                let elems = match elems {
                    SearchTypeOk(elems) => elems,
                    SearchTypeNotReady(v) => return SearchTypeNotReady(v),
                    SearchTypeNotFound(v) => return SearchTypeNotFound(v),
                };
                let tid = ptree.insert_type(Type::Tuple(TypeTuple { elems }));
                SearchTypeOk(tid)
            }
            o => todo!("{o:?}"),
        }
    }

    pub(super) fn from_infer_type<'gcx>(
        ty: infer::Type<'gcx>,
        infer: &InferHelper<'_, 'gcx>,
    ) -> TriResult<Self, ()> {
        match ty {
            infer::Type::Scalar(v) => {
                let ty = Type::Scalar(TypeScalar::from_infer_scalar(v));
                let tid = infer.ptree.insert_type(ty);
                Ok(tid)
            }
            infer::Type::Named(infer::TypeNamed { name, .. }) => {
                match infer.ptree.norm_search_type(infer.base, name.as_ref()) {
                    SearchTypeOk(tid) => Ok(tid),
                    SearchTypeNotReady(_) | SearchTypeNotFound(()) => err!(soft, ()),
                }
            }
            infer::Type::Tuple(infer::TypeTuple { elems }) => {
                let elems = elems
                    .into_iter()
                    .map(|elem| {
                        let elem_ty = infer.find_type(elem).clone();
                        Self::from_infer_type(elem_ty, infer)
                    })
                    .collect::<TriResult<BoxedSlice<_>, ()>>()?;
                let ty = Type::Tuple(TypeTuple { elems });
                let tid = infer.ptree.insert_type(ty);
                Ok(tid)
            }
            infer::Type::Array(infer::TypeArray { elem, len }) => {
                let elem_ty = infer.find_type(elem).clone();
                let elem_tid = Self::from_infer_type(elem_ty, infer)?;
                let len = match len {
                    InferArrayLen::Fixed(value) => ArrayLen::Fixed(value),
                    InferArrayLen::Dynamic => ArrayLen::Dynamic,
                    InferArrayLen::Generic => ArrayLen::Generic,
                    InferArrayLen::Unknown => return err!(soft, ()),
                };
                let ty = Type::Array(TypeArray {
                    elem: elem_tid,
                    len,
                });
                let tid = infer.ptree.insert_type(ty);
                Ok(tid)
            }
            infer::Type::Ref(infer::TypeRef { elem }) => {
                let elem_ty = infer.find_type(elem).clone();
                let elem_tid = Self::from_infer_type(elem_ty, infer)?;
                let ty = Type::Ref(TypeRef { elem: elem_tid });
                let tid = infer.ptree.insert_type(ty);
                Ok(tid)
            }
            infer::Type::Mut(infer::TypeMut { elem }) => {
                let elem_ty = infer.find_type(elem).clone();
                let elem_tid = Self::from_infer_type(elem_ty, infer)?;
                let ty = Type::Mut(TypeMut { elem: elem_tid });
                let tid = infer.ptree.insert_type(ty);
                Ok(tid)
            }
            infer::Type::Unit => {
                let tid = infer.ptree.insert_type(Type::Unit);
                Ok(tid)
            }
            infer::Type::Var(_) | infer::Type::Composed(_) | infer::Type::Unknown => {
                panic!("infer type `{ty:?}` cannot be a tree type. it must be unified")
            }
        }
    }
}

impl TypeScalar {
    fn from_infer_scalar(scalar: infer::TypeScalar) -> Self {
        match scalar {
            infer::TypeScalar::Int { .. } => Self::Int,
            infer::TypeScalar::Float { .. } => Self::Float,
            infer::TypeScalar::I8 => Self::I8,
            infer::TypeScalar::I16 => Self::I16,
            infer::TypeScalar::I32 => Self::I32,
            infer::TypeScalar::I64 => Self::I64,
            infer::TypeScalar::I128 => Self::I128,
            infer::TypeScalar::Isize => Self::Isize,
            infer::TypeScalar::U8 => Self::U8,
            infer::TypeScalar::U16 => Self::U16,
            infer::TypeScalar::U32 => Self::U32,
            infer::TypeScalar::U64 => Self::U64,
            infer::TypeScalar::U128 => Self::U128,
            infer::TypeScalar::Usize => Self::Usize,
            infer::TypeScalar::F32 => Self::F32,
            infer::TypeScalar::F64 => Self::F64,
            infer::TypeScalar::Bool => Self::Bool,
        }
    }
}

impl<'gcx> infer::Type<'gcx> {
    pub(super) fn from_syn_type<T: ItemTrait>(
        ty: &syn::Type,
        stree: &SyntaxTree,
        ptree: &PathTree<'gcx, T>,
        base: NodeIndex,
        types: &mut infer::UniqueTypes<'gcx>,
        gcx: &'gcx GlobalCx<'gcx>,
    ) -> TriResult<Self, ()> {
        let tree_tid = match TypeId::from_syn_type(ty, stree, ptree, base) {
            SearchTypeOk(tid) => tid,
            _ => return err!(soft, ()),
        };
        let tree_ty = ptree.get_type(tree_tid);
        let infer_ty = Self::from_tree_type(tree_ty, ptree, types, gcx);
        Ok(infer_ty)
    }

    pub(super) fn from_tree_type<T: ItemTrait>(
        ty: &Type<'gcx>,
        ptree: &PathTree<'gcx, T>,
        types: &mut infer::UniqueTypes<'gcx>,
        gcx: &'gcx GlobalCx<'gcx>,
    ) -> Self {
        match ty {
            Type::Path(TypePath { pid, .. }) => Self::from_tree_path_id(*pid, ptree, types, gcx),
            Type::Scalar(scalar) => {
                let scalar = infer::TypeScalar::from_tree_scalar(*scalar);
                Self::Scalar(scalar)
            }
            Type::Tuple(TypeTuple { elems }) => Self::Tuple(infer::TypeTuple {
                elems: elems
                    .iter()
                    .map(|elem| infer::TypeId::from_tree_type_id(*elem, ptree, types, gcx))
                    .collect(),
            }),
            Type::Array(TypeArray { elem, len }) => {
                let len = match len {
                    ArrayLen::Fixed(v) => InferArrayLen::Fixed(*v),
                    ArrayLen::Dynamic => InferArrayLen::Dynamic,
                    ArrayLen::Generic => InferArrayLen::Generic,
                };
                Self::Array(infer::TypeArray {
                    elem: infer::TypeId::from_tree_type_id(*elem, ptree, types, gcx),
                    len,
                })
            }
            Type::Ref(TypeRef { elem }) => Self::Ref(infer::TypeRef {
                elem: infer::TypeId::from_tree_type_id(*elem, ptree, types, gcx),
            }),
            Type::Mut(TypeMut { elem }) => Self::Mut(infer::TypeMut {
                elem: infer::TypeId::from_tree_type_id(*elem, ptree, types, gcx),
            }),
            Type::Unit => Self::Unit,
        }
    }

    fn from_tree_path_id<T: ItemTrait>(
        pid: PathId,
        ptree: &PathTree<'gcx, T>,
        types: &mut infer::UniqueTypes<'gcx>,
        gcx: &'gcx GlobalCx<'gcx>,
    ) -> Self {
        let item = &ptree[pid];
        let ty = if let Some(f) = item.as_fn() {
            let Type::Path(TypePath { pid: _pid, params }) = ptree.get_type(f.tid) else {
                unreachable!()
            };
            debug_assert_eq!(pid, *_pid);

            from_tree_type_path(pid, params, ptree, types, gcx)
        } else if let Some(st) = item.as_struct() {
            let Type::Path(TypePath { pid: _pid, params }) = ptree.get_type(st.tid) else {
                unreachable!()
            };
            debug_assert_eq!(pid, *_pid);

            from_tree_type_path(pid, params, ptree, types, gcx)
        } else {
            Self::Named(infer::TypeNamed {
                name: gcx.intern_str(&ptree.get_name_path(pid.ni)),
                params: [].into(),
            })
        };
        return ty;

        // === Internal helper functions ===

        fn from_tree_type_path<'gcx, T: ItemTrait>(
            pid: PathId,
            params: &[Param<'gcx>],
            ptree: &PathTree<'gcx, T>,
            types: &mut infer::UniqueTypes<'gcx>,
            gcx: &'gcx GlobalCx<'gcx>,
        ) -> infer::Type<'gcx> {
            infer::Type::Named(infer::TypeNamed {
                name: gcx.intern_str(&ptree.get_name_path(pid.ni)),
                params: params
                    .iter()
                    .map(|param| match param {
                        Param::Self_ => infer::Param::Self_,
                        Param::Other { name, tid } => infer::Param::Other {
                            name: *name,
                            tid: infer::TypeId::from_tree_type_id(*tid, ptree, types, gcx),
                        },
                    })
                    .collect(),
            })
        }
    }
}

impl infer::TypeId {
    pub(super) fn from_tree_type_id<'gcx, T: ItemTrait>(
        tid: TypeId,
        ptree: &PathTree<'gcx, T>,
        types: &mut infer::UniqueTypes<'gcx>,
        gcx: &'gcx GlobalCx<'gcx>,
    ) -> Self {
        let tree_ty = ptree.get_type(tid);
        let infer_ty = infer::Type::from_tree_type(tree_ty, ptree, types, gcx);
        types.insert_type(infer_ty)
    }
}

impl infer::TypeScalar {
    fn from_tree_scalar(scalar: TypeScalar) -> Self {
        match scalar {
            TypeScalar::Int => Self::Int { reserved: None },
            TypeScalar::Float => Self::Float { reserved: None },
            TypeScalar::I8 => Self::I8,
            TypeScalar::I16 => Self::I16,
            TypeScalar::I32 => Self::I32,
            TypeScalar::I64 => Self::I64,
            TypeScalar::I128 => Self::I128,
            TypeScalar::Isize => Self::Isize,
            TypeScalar::U8 => Self::U8,
            TypeScalar::U16 => Self::U16,
            TypeScalar::U32 => Self::U32,
            TypeScalar::U64 => Self::U64,
            TypeScalar::U128 => Self::U128,
            TypeScalar::Usize => Self::Usize,
            TypeScalar::F32 => Self::F32,
            TypeScalar::F64 => Self::F64,
            TypeScalar::Bool => Self::Bool,
        }
    }
}

impl<'gcx> eval::Value<'gcx> {
    pub(super) fn from_path_id<T: ItemTrait>(
        pid: PathId,
        ptree: &PathTree<'gcx, T>,
        evaluated: &Evaluated<'gcx>,
        gcx: &'gcx GlobalCx<'gcx>,
    ) -> TriResult<Self, ()> {
        let item = &ptree[pid];
        match item.effective_kind() {
            EffectiveItemKind::Fn => {
                let Some(fn_) = item.as_fn() else {
                    return err!(soft, ());
                };
                let sig = fn_.syn_sig();
                let block = fn_.syn_block();
                let value = Self::Fn(eval::Fn::from_signature_and_block(sig, block));
                Ok(value)
            }
            EffectiveItemKind::Const => {
                if let Some(c) = evaluated.get_mapped_value_by_path_id(pid) {
                    Ok(c.clone())
                } else {
                    err!(soft, ())
                }
            }
            EffectiveItemKind::Struct => {
                let fields = ptree[pid.ni]
                    .children
                    .iter()
                    .flat_map(|(_, child_node)| {
                        ptree[*child_node].iter().filter_map(|(ii, item)| {
                            (item.effective_kind() == EffectiveItemKind::Field)
                                .then_some(child_node.to_path_id(ii))
                        })
                    })
                    .map(|child_pid| {
                        // We cannot fill in field's value here. It should
                        // be done by caller.
                        let field_name = ptree.get_name_path_between(pid.ni, child_pid.ni).unwrap();
                        eval::Field {
                            name: gcx.intern_str(&field_name),
                            value: eval::Value::Unit,
                        }
                    })
                    .collect();

                let value = eval::Value::Composed(fields);
                Ok(value)
            }
            EffectiveItemKind::Variant => {
                let Some(var) = item.as_variant() else {
                    return err!(soft, ());
                };
                let value = eval::Value::Enum(eval::Enum {
                    path: ptree.get_name_path(pid.ni),
                    disc: var.disc,
                });
                Ok(value)
            }
            o => {
                err!(hard, "couldn't get an evaluated value from `{o:?}`")
            }
        }
    }
}
