use super::{
    format::{self, BriefDebugItem, DebugBriefHow, DebugItem, PrintFilter},
    item::{self, EffectiveItemKind, ItemTrait},
    node::{Node, NodeIndex, NodeIter, NodeIterMut},
    ty::{
        ArrayLen, CreateOwnedType, OwnedParam, OwnedType, Param, Type, TypeArray, TypeId, TypeMut,
        TypePath, TypeRef, TypeScalar, TypeTuple, UniqueTypes, OWNED_TYPE_CREATOR,
    },
    PathId,
};
use crate::{
    etc::util::{self, IntoPathSegments, PathSegments},
    GetOwned, Set, TriOption,
};
use smallvec::SmallVec;
use std::{
    cell::{Cell, UnsafeCell},
    fmt,
    hash::Hash,
    iter, mem, ops,
    path::PathBuf,
    ptr::NonNull,
    result::Result as StdResult,
};

pub struct PathTree<'gcx, T> {
    pub(crate) nodes: Vec<Node<T>>,
    pub(crate) types: UnsafeCell<UniqueTypes<'gcx>>,
    pub(crate) crate_node: NodeIndex,
}

impl<'gcx, T: ItemTrait> PathTree<'gcx, T> {
    pub(crate) fn new<F>(crate_root_gen: F) -> Self
    where
        F: FnOnce(item::Mod) -> T,
    {
        // Empty root node.
        let root_node = Node {
            items: SmallVec::new(),
            parent: Self::ROOT,
            children: Vec::new(),
        };

        // Tree.
        let mut this = Self {
            nodes: vec![root_node],
            types: UnsafeCell::new(UniqueTypes::new()),
            crate_node: NodeIndex(usize::MAX), // will be replaced soon
        };

        // Crate root node.
        let crate_root = crate_root_gen(item::Mod {
            ptr_mod: None,
            ptr_file: None,
            vis_node: Self::ROOT, // will be replaced soon
            fpath: PathBuf::default(),
            mod_rs: false,
        });
        let crate_: &str = &util::get_crate_name();
        let Ok(crate_pid) = this.try_add_item(Self::ROOT, crate_, crate_root) else {
            unreachable!()
        };
        this[crate_pid].as_mut_mod().unwrap().vis_node = crate_pid.ni;

        // Replaces dummy crate node.
        this.crate_node = crate_pid.ni;

        this
    }

    pub(crate) fn norm_search<I>(&self, base: NodeIndex, key: I) -> Option<NodeIndex>
    where
        I: IntoPathSegments,
    {
        let (base, key) = self.normalize_key(base, key.segments());
        self.search(base, PathSegments(key))
    }

    pub(crate) fn norm_search_type<I>(&self, base: NodeIndex, key: I) -> SearchTypeResult
    where
        I: IntoPathSegments,
    {
        let (base, key) = self.normalize_key(base, key.segments());
        self.search_type(base, PathSegments(key))
    }

    /// Finds a type from the given condition, `base` and `key`.
    ///
    /// This method may look up the given `key` with other conditions inherited from the given
    /// `base`. See example below.
    ///
    /// ```ignore
    /// // Let `base` is `a::b::foo::{0}::bar::{0}`
    /// // a::b::foo::{0}::bar::{0} // Searches in the block
    /// // a::b::foo::{0}           // Searches in the block
    /// // a::b                     // Searches in the module
    /// mod a {
    ///     mod b {            // Maybe `T` is defined or imported here
    ///         fn foo() {     // Maybe `T` is defined or imported here
    ///             fn bar() { // Maybe `T` is defined or imported here
    ///                 T
    ///             }
    ///         }
    ///     }
    /// }
    /// ```
    ///
    /// # Caution
    ///
    /// This method ignores unresolved 'use'.
    pub(crate) fn search_type<I>(&self, base: NodeIndex, key: I) -> SearchTypeResult
    where
        I: IntoPathSegments,
    {
        match self.search_type_from_nodes(base, key.clone()) {
            res @ SearchTypeOk(_) | res @ SearchTypeNotReady(_) => res,
            SearchTypeNotFound(_) => self.search_scalar(key),
        }
    }

    pub(crate) fn search_type_from_nodes<I>(&self, base: NodeIndex, key: I) -> SearchTypeResult
    where
        I: IntoPathSegments,
    {
        // Determines where we are, 'mod' or 'block'.
        enum Context {
            Mod,
            Block,
        }

        let mut cx = None;

        for (_, item) in self[base].iter() {
            match item.effective_kind() {
                EffectiveItemKind::Mod => {
                    cx = Some(Context::Mod);
                    break;
                }
                EffectiveItemKind::Block | EffectiveItemKind::Fn => {
                    cx = Some(Context::Block);
                    break;
                }
                _ => {}
            }
        }

        let cx = if let Some(cx) = cx {
            cx
        } else {
            let parent_pid = self.parent_item(base, |item| {
                matches!(
                    item.effective_kind(),
                    EffectiveItemKind::Block | EffectiveItemKind::Fn | EffectiveItemKind::Mod
                )
            });
            if parent_pid.ni == Self::ROOT {
                Context::Mod
            } else {
                match self[parent_pid].effective_kind() {
                    EffectiveItemKind::Mod => Context::Mod,
                    _ => Context::Block,
                }
            }
        };

        let base = Cell::new(base);

        let cb = |vis: MaybeVisible, pid: PathId, item: &T| -> Option<SearchTypeResult> {
            match vis {
                MaybeVisible::Visible => match item.type_id() {
                    TriOption::Some(tid) => Some(SearchTypeOk(tid)),
                    TriOption::NotYet(()) => Some(SearchTypeNotReady(pid)),
                    TriOption::None => None,
                },
                MaybeVisible::UnknownYet => Some(SearchTypeNotReady(pid)),
            }
        };

        // In a module context, we cannot see items defined in parent blocks. Therefore, we don't
        // have to search on other conditions.
        if matches!(cx, Context::Mod) {
            match self.traverse(base.get(), key, cb) {
                Some(res) => res,
                None => SearchTypeNotFound(()),
            }
        }
        // In a block context, we can also see items defined in parant blocks without explicit
        // prefix like 'super' or something like that. So, we have to consider parent blocks as
        // well.
        else {
            'find: loop {
                let res = self.traverse(base.get(), key.clone(), cb);
                if let Some(res) = res {
                    return res;
                }

                for (_, item) in self[base.get()].iter() {
                    match item.effective_kind() {
                        EffectiveItemKind::Block | EffectiveItemKind::Fn => {
                            let parent = self[base.get()].parent;
                            base.set(parent);
                            continue 'find;
                        }
                        EffectiveItemKind::Mod => break 'find,
                        _ => {}
                    }
                }

                break 'find;
            }

            SearchTypeNotFound(())
        }
    }

    pub(crate) fn norm_search_item<I>(
        &self,
        base: NodeIndex,
        key: I,
        pivot: usize,
    ) -> Option<PathId>
    where
        I: IntoPathSegments,
    {
        let (base, key) = self.normalize_key(base, key.segments());
        self.search_item(base, PathSegments(key), pivot)
    }

    /// Finds the nearest item from the given condition, `base`, `key` and `pivot`. The nearest item
    /// could be chosen differently by the given pivot, so be careful about some cases like below.
    ///
    /// ```ignore
    /// let x = ...;
    /// let x = x ...; // Pivot should be less than lvalue 'x', not rvalue 'x'.
    /// ```
    ///
    /// Plus, this method looks up the given `key` with other conditions inherited from the given
    /// `base`. See example below.
    ///
    /// ```ignore
    /// // Let `base` is `a::b::foo::{0}::bar::{0}`
    /// // a::b::foo::{0}::bar::{0} // Searches in the block
    /// // a::b::foo::{0}::bar      // Searches in the function arguments
    /// // a::b                     // Searches in the module
    /// mod a {
    ///     mod b {
    ///         fn foo() {
    ///             fn bar() {
    ///                 // Searches here
    ///                 pivot
    ///                 // Does not search here
    ///             }
    ///         }
    ///     }
    /// }
    /// ```
    ///
    /// # Caution
    ///
    /// This method ignores unresolved `use`.
    //
    // NOTE: May work incorrectly with blocks since current logic just finds previous occurences.
    // Symbol table approach would fix that, or we could fix the logic to consider blocks.
    pub(crate) fn search_item<I>(&self, mut base: NodeIndex, key: I, pivot: usize) -> Option<PathId>
    where
        I: IntoPathSegments,
    {
        let found = Cell::new(None);
        let mut least_diff = usize::MAX;

        'outer: loop {
            self.traverse_wo_vis_check(base, key.clone(), |pid, item| {
                const CONTINUE_TRAVERSING: Option<()> = None;
                const STOP_TRAVERSING: Option<()> = Some(());

                // Looks into `Local` without checking the visibility (it doesn't have).
                if item.effective_kind() == EffectiveItemKind::Local {
                    let end = item.syn_id().unwrap().as_identify_syn().location().end;
                    if end < pivot && (pivot - end) < least_diff {
                        least_diff = pivot - end;
                        found.set(Some(pid));
                    }
                    return CONTINUE_TRAVERSING;
                }

                // Not a `Local` but visible, then we found it.
                match item.vis_node() {
                    TriOption::Some(item_vis) if self.is_descendant(base, item_vis) => {
                        found.set(Some(pid));
                        STOP_TRAVERSING
                    }
                    _ => CONTINUE_TRAVERSING,
                }
            });

            if found.get().is_some() {
                break;
            }

            for (_, item) in self[base].iter() {
                match item.effective_kind() {
                    EffectiveItemKind::Block => {
                        base = self[base].parent;
                        break;
                    }
                    EffectiveItemKind::Fn => {
                        let parent_pid = self.parent_item(base, filter::mod_);
                        base = parent_pid.ni;
                        break;
                    }
                    EffectiveItemKind::Mod => break 'outer,
                    _ => {}
                }
            }
        }

        found.get()
    }

    pub(crate) fn norm_traverse<I, F, R>(&self, base: NodeIndex, key: I, f: F) -> Option<R>
    where
        I: IntoPathSegments,
        F: FnMut(MaybeVisible, PathId, &T) -> Option<R>,
    {
        let (base, key) = self.normalize_key(base, key.segments());
        self.traverse(base, PathSegments(key), f)
    }

    /// Visits either visible or maybe visible items matched by the given condition, `base` and
    /// `key`.
    ///
    /// If the given function returns Some value, then it is returned immediately without further
    /// traversing.
    ///
    /// # Caution
    ///
    /// This method ignores unresolved `use` and 'type'.
    pub(crate) fn traverse<I, F, R>(&self, base: NodeIndex, key: I, mut f: F) -> Option<R>
    where
        I: IntoPathSegments,
        F: FnMut(MaybeVisible, PathId, &T) -> Option<R>,
    {
        let key = key.segments();

        // Stack storing (base node, key advance count, MaybeVisible)
        let mut stack = Vec::new();
        stack.push((Self::ROOT, 0, MaybeVisible::Visible)); // from the root for extern crate
        stack.push((base, 0, MaybeVisible::Visible)); // from the base for this crate

        let mut visited = Set::<NodeIndex>::default();

        // If the item is "definitely visible" or "maybe visible", returns it within `Some`,
        // otherwise, returns `None`.
        let maybe_visible = |item: &T, parent_vis: MaybeVisible| match item.vis_node() {
            TriOption::Some(vis_node) if self.is_descendant(base, vis_node) => {
                Some(parent_vis.or(MaybeVisible::Visible))
            }
            TriOption::NotYet(()) => Some(MaybeVisible::UnknownYet),
            _ => None,
        };

        // If possible to jump, then puts new "base", "key offset", and "visibility" in the stack.
        let jump_if_possible = |stack: &mut Vec<(NodeIndex, usize, MaybeVisible)>,
                                mid: NodeIndex,
                                advance: usize,
                                parent_vis: MaybeVisible| {
            for (_, item) in self[mid].iter() {
                // Allows only "definitely visible" and "maybe visible".
                let Some(vis) = maybe_visible(item, parent_vis) else {
                    continue;
                };

                // We can jump over `TypeAlias` and `Use`.
                if let Some(item::TypeAlias { tid, .. }) = item.as_type_alias() {
                    if let Type::Path(TypePath { pid: dst, .. }) = self.get_type(*tid) {
                        stack.push((dst.ni, advance, vis));
                    }
                } else if let Some(item::Use { dst, .. }) = item.as_use() {
                    stack.push((dst.ni, advance, vis));
                }
            }
        };

        while let Some((advanced_base, key_off, cur_vis)) = stack.pop() {
            let advanced_key = key.clone().skip(key_off);
            let advanced_key = PathSegments(advanced_key);

            match self.search_best(advanced_base, advanced_key) {
                Ok((dst, advance)) => {
                    if !visited.insert(dst) {
                        continue;
                    }

                    for (ii, item) in self[dst].iter() {
                        // Allows only "definitely visible" and "maybe visible".
                        let Some(vis) = maybe_visible(item, cur_vis) else {
                            continue;
                        };

                        let ret = f(vis, dst.to_path_id(ii), item);
                        if ret.is_some() {
                            return ret;
                        }
                    }

                    jump_if_possible(&mut stack, dst, advance, cur_vis);
                }
                Err((mid, advance)) => {
                    if !visited.insert(mid) {
                        continue;
                    }

                    jump_if_possible(&mut stack, mid, advance, cur_vis);
                }
            }
        }
        None
    }

    /// Similar to [`traverse`](Self::traverse), but this method traverses all matches items
    /// regardless of visibility.
    pub(crate) fn traverse_wo_vis_check<I, F, R>(
        &self,
        base: NodeIndex,
        key: I,
        mut f: F,
    ) -> Option<R>
    where
        I: IntoPathSegments,
        F: FnMut(PathId, &T) -> Option<R>,
    {
        let key = key.segments();

        // Stack storing (base node, key advance count)
        let mut stack = Vec::new();
        stack.push((Self::ROOT, 0)); // from the root for extern crate
        stack.push((base, 0)); // from the base for this crate

        let mut visited = Set::<NodeIndex>::default();

        // If possible to jump, then puts new "base" and "key offset" in the stack.
        let jump_if_possible =
            |stack: &mut Vec<(NodeIndex, usize)>, mid: NodeIndex, advance: usize| {
                for (_, item) in self[mid].iter() {
                    // We can jump over `TypeAlias` and `Use`.
                    if let Some(item::TypeAlias { tid, .. }) = item.as_type_alias() {
                        if let Type::Path(TypePath { pid: dst, .. }) = self.get_type(*tid) {
                            stack.push((dst.ni, advance));
                        }
                    } else if let Some(item::Use { dst, .. }) = item.as_use() {
                        stack.push((dst.ni, advance));
                    }
                }
            };

        while let Some((advanced_base, key_off)) = stack.pop() {
            let advanced_key = key.clone().skip(key_off);
            let advanced_key = PathSegments(advanced_key);

            match self.search_best(advanced_base, advanced_key) {
                Ok((dst, advance)) => {
                    if !visited.insert(dst) {
                        continue;
                    }

                    for (ii, item) in self[dst].iter() {
                        let ret = f(dst.to_path_id(ii), item);
                        if ret.is_some() {
                            return ret;
                        }
                    }

                    jump_if_possible(&mut stack, dst, advance);
                }
                Err((mid, advance)) => {
                    if !visited.insert(mid) {
                        continue;
                    }

                    jump_if_possible(&mut stack, mid, advance);
                }
            }
        }
        None
    }

    pub(crate) fn normalize_key<I, II>(
        &self,
        mut base: NodeIndex,
        key: I,
    ) -> (NodeIndex, std::iter::Skip<I>)
    where
        I: Iterator<Item = II> + Clone,
        II: AsRef<str>,
    {
        let mut key_off = 0;
        for segment in key.clone() {
            if segment.as_ref() == "crate" {
                base = self.crate_node;
                key_off += 1;
            } else if segment.as_ref() == "super" {
                let parent_pid = self.parent_item(base, filter::mod_);
                base = parent_pid.ni;
                key_off += 1;
            } else {
                break;
            }
        }
        (base, key.clone().skip(key_off))
    }
}

impl<'gcx, T> PathTree<'gcx, T> {
    pub(crate) const ROOT: NodeIndex = NodeIndex(0);

    pub fn iter(&self) -> PathIter<'_, T> {
        PathIter::new(self)
    }

    pub fn iter_mut(&mut self) -> PathIterMut<'_, T> {
        PathIterMut::new(self)
    }

    pub(crate) fn types(&self) -> &UniqueTypes<'gcx> {
        unsafe { self.types.get().as_ref().unwrap_unchecked() }
    }

    pub(crate) fn get_type(&self, tid: TypeId) -> &Type<'gcx> {
        let types = self.types();
        &types[tid]
    }

    /// Returns true if the given type contains generic parameter in it.
    pub(crate) fn contains_generic_param_in_type(&self, tid: TypeId) -> bool {
        match self.get_type(tid) {
            Type::Path(TypePath { params, .. }) => {
                params.iter().any(|param| {
                    matches!(param, Param::Other { tid, .. } if self.contains_generic_param_in_type(*tid))
                })
            }
            Type::Tuple(TypeTuple { elems }) => {
                elems.iter().any(|elem| self.contains_generic_param_in_type(*elem))
            }
            Type::Array(TypeArray { elem, len }) => {
                matches!(len, ArrayLen::Generic)
                || self.contains_generic_param_in_type(*elem)
            }
            Type::Ref(TypeRef { elem })
            | Type::Mut(TypeMut { elem }) => self.contains_generic_param_in_type(*elem),
            Type::Scalar(_)
            | Type::Unit => false,
        }
    }

    pub(crate) fn get_type_id_of<Q>(&self, ty: &Q) -> Option<TypeId>
    where
        Q: Hash + PartialEq<Type<'gcx>> + ?Sized,
    {
        let types = unsafe { self.types.get().as_ref().unwrap_unchecked() };
        types.find(ty).map(TypeId)
    }

    /// Inserts the given type in the tree.
    ///
    /// If the same type was found by [`PartialEq<Type>`], then the old type is replaced with the
    /// given new type.
    pub(crate) fn insert_type(&self, ty: Type<'gcx>) -> TypeId {
        let types = unsafe { self.types.get().as_mut().unwrap_unchecked() };
        types.insert(ty)
    }

    pub(crate) fn replace_type(&self, old: Type<'gcx>, new: Type<'gcx>) -> bool {
        let types = unsafe { self.types.get().as_mut().unwrap_unchecked() };
        types.replace(&old, new)
    }

    pub(crate) fn get_name_path(&self, index: NodeIndex) -> String {
        self.get_name_path_between(Self::ROOT, index).unwrap()
    }

    pub(crate) fn get_name_path_between(
        &self,
        ancestor: NodeIndex,
        child: NodeIndex,
    ) -> Option<String> {
        let mut cur = child;
        let mut segments = Vec::new();
        while cur != Self::ROOT && cur != ancestor {
            let c = &self[cur];
            let p = &self[c.parent];
            let (k, _) = p.children.iter().find(|(_, h)| *h == cur).unwrap();
            segments.push(k.clone());
            cur = c.parent;
        }
        if cur != Self::ROOT || ancestor == Self::ROOT {
            segments.reverse();
            let npath = segments.join("::");
            Some(npath)
        } else {
            None
        }
    }

    pub(crate) fn search<I>(&self, base: NodeIndex, key: I) -> Option<NodeIndex>
    where
        I: IntoPathSegments,
    {
        let mut cur = base;
        for seg in key.segments() {
            if let Some(next) = self[cur].children.iter().find(|(k, _)| seg.as_ref() == k) {
                cur = next.1;
            } else {
                return None;
            }
        }
        Some(cur)
    }

    /// Ok: node index to the target and advance count
    /// Err: node index to a node in the middle of search and advance count
    pub(crate) fn search_best<I>(
        &self,
        base: NodeIndex,
        key: I,
    ) -> StdResult<(NodeIndex, usize), (NodeIndex, usize)>
    where
        I: IntoPathSegments,
    {
        let mut cur = base;
        let mut advance = 0;
        for seg in key.segments() {
            if let Some(next) = self[cur].children.iter().find(|(k, _)| seg.as_ref() == k) {
                cur = next.1;
                advance += 1;
            } else {
                return Err((cur, advance));
            }
        }
        Ok((cur, advance))
    }

    /// Searches type from [`Self::types`]. This method is intended to be called inside
    /// [`Self::search_type`].
    //
    // Scalar types exist in the type container only. We cannot find them from the tree nodes.
    pub(crate) fn search_scalar<I>(&self, key: I) -> SearchTypeResult
    where
        I: IntoPathSegments,
    {
        let mut segments = key.segments();

        if segments.clone().count() == 1 {
            if let Some(segment) = segments.next() {
                if let Some(scalar) = TypeScalar::from_type_name(segment.as_ref()) {
                    if let Some(tid) = self.get_type_id_of(&Type::Scalar(scalar)) {
                        return SearchTypeOk(tid);
                    }
                }
            }
        }

        SearchTypeNotFound(())
    }

    pub(crate) fn is_descendant(&self, descendant: NodeIndex, ancestor: NodeIndex) -> bool {
        let mut cur = descendant;
        while cur != ancestor {
            if cur == Self::ROOT {
                return false;
            }
            cur = self[cur].parent;
        }
        true
    }

    pub(crate) fn parent_item<F>(&self, index: NodeIndex, filter: F) -> PathId
    where
        F: FnMut(&T) -> bool,
    {
        self.nearest_item(self[index].parent, filter)
    }

    /// Returns first matched item while traversing from the given node to the root node.
    ///
    /// If the given node contains a matched item, then it will be returned.
    pub(crate) fn nearest_item<F>(&self, mut node: NodeIndex, mut filter: F) -> PathId
    where
        F: FnMut(&T) -> bool,
    {
        while node != Self::ROOT {
            // There could be empty node like ancestor nodes of the entry mod.
            while node != Self::ROOT && self[node].items.is_empty() {
                node = self[node].parent;
            }

            for (ii, item) in self[node].iter() {
                if filter(item) {
                    return node.to_path_id(ii);
                }
            }

            node = self[node].parent;
        }

        node.to_path_id(0) // Invalid id
    }

    pub(crate) fn try_add_item<I>(
        &mut self,
        base: NodeIndex,
        key: I,
        item: T,
    ) -> StdResult<PathId, (PathId, T)>
    where
        I: IntoPathSegments,
        T: ItemTrait,
    {
        self.insert_node_then(base, key, |ni, node| {
            for (ii, exist_item) in node.iter() {
                if item.is_effective_same(exist_item) {
                    let exist_pid = ni.to_path_id(ii);
                    return Err((exist_pid, item));
                }
            }

            let ii = node.push(item);
            let new_pid = ni.to_path_id(ii);
            Ok(new_pid)
        })
    }

    pub(crate) fn insert_node_then<I, F, R>(&mut self, base: NodeIndex, key: I, then: F) -> R
    where
        I: IntoPathSegments,
        F: FnOnce(NodeIndex, &mut Node<T>) -> R,
    {
        let mut cur = base;
        for seg in key.segments() {
            if let Some(next) = self[cur].children.iter().find(|(k, _)| seg.as_ref() == k) {
                cur = next.1;
            } else {
                let ni = self.new_node(cur);
                self[cur].children.push((seg.as_ref().to_owned(), ni));
                cur = ni;
            }
        }
        then(cur, &mut self[cur])
    }

    pub(crate) fn new_node(&mut self, parent: NodeIndex) -> NodeIndex {
        self.nodes.push(Node {
            items: SmallVec::new(),
            parent,
            children: Vec::new(),
        });
        NodeIndex(self.nodes.len() - 1)
    }

    pub(crate) fn take_item(&mut self, id: PathId) -> T
    where
        T: Default,
    {
        self[id.ni].take(id.ii)
    }
}

impl<'gcx, T> GetOwned<TypeId> for PathTree<'gcx, T> {
    type Owned = OwnedType;

    fn get_owned(&self, tid: TypeId) -> Self::Owned {
        match self.get_type(tid) {
            Type::Scalar(TypeScalar::Int) => OwnedType::Path {
                name: "int".into(),
                params: [].into(),
            },
            Type::Scalar(TypeScalar::Float) => OwnedType::Path {
                name: "float".into(),
                params: [].into(),
            },
            Type::Scalar(TypeScalar::I8) => OwnedType::Path {
                name: "i8".into(),
                params: [].into(),
            },
            Type::Scalar(TypeScalar::I16) => OwnedType::Path {
                name: "i16".into(),
                params: [].into(),
            },
            Type::Scalar(TypeScalar::I32) => OwnedType::Path {
                name: "i32".into(),
                params: [].into(),
            },
            Type::Scalar(TypeScalar::I64) => OwnedType::Path {
                name: "i64".into(),
                params: [].into(),
            },
            Type::Scalar(TypeScalar::I128) => OwnedType::Path {
                name: "i128".into(),
                params: [].into(),
            },
            Type::Scalar(TypeScalar::Isize) => OwnedType::Path {
                name: "isize".into(),
                params: [].into(),
            },
            Type::Scalar(TypeScalar::U8) => OwnedType::Path {
                name: "u8".into(),
                params: [].into(),
            },
            Type::Scalar(TypeScalar::U16) => OwnedType::Path {
                name: "u16".into(),
                params: [].into(),
            },
            Type::Scalar(TypeScalar::U32) => OwnedType::Path {
                name: "u32".into(),
                params: [].into(),
            },
            Type::Scalar(TypeScalar::U64) => OwnedType::Path {
                name: "u64".into(),
                params: [].into(),
            },
            Type::Scalar(TypeScalar::U128) => OwnedType::Path {
                name: "u128".into(),
                params: [].into(),
            },
            Type::Scalar(TypeScalar::Usize) => OwnedType::Path {
                name: "usize".into(),
                params: [].into(),
            },
            Type::Scalar(TypeScalar::F32) => OwnedType::Path {
                name: "f32".into(),
                params: [].into(),
            },
            Type::Scalar(TypeScalar::F64) => OwnedType::Path {
                name: "f64".into(),
                params: [].into(),
            },
            Type::Scalar(TypeScalar::Bool) => OwnedType::Path {
                name: "bool".into(),
                params: [].into(),
            },
            Type::Path(TypePath { pid, params }) => OwnedType::Path {
                name: self.get_name_path(pid.ni),
                params: params
                    .iter()
                    .map(|param| match param {
                        Param::Self_ => OwnedParam::Self_,
                        Param::Other { name, tid } => OwnedParam::Other {
                            name: (**name).to_owned(),
                            ty: self.get_owned(*tid),
                        },
                    })
                    .collect(),
            },
            Type::Tuple(TypeTuple { elems }) => {
                OwnedType::Tuple(elems.iter().map(|elem| self.get_owned(*elem)).collect())
            }
            Type::Array(TypeArray { elem, len }) => OwnedType::Array {
                elem: Box::new(self.get_owned(*elem)),
                len: *len,
            },
            Type::Ref(TypeRef { elem }) => OwnedType::Ref {
                elem: Box::new(self.get_owned(*elem)),
            },
            Type::Mut(TypeMut { elem }) => OwnedType::Mut {
                elem: Box::new(self.get_owned(*elem)),
            },
            Type::Unit => OwnedType::Unit,
        }
    }
}

impl<T> ops::Index<PathId> for PathTree<'_, T> {
    type Output = T;

    fn index(&self, id: PathId) -> &Self::Output {
        let node = &self[id.ni];
        &node.items[id.ii.0]
    }
}

impl<T> ops::IndexMut<PathId> for PathTree<'_, T> {
    fn index_mut(&mut self, id: PathId) -> &mut Self::Output {
        let node = &mut self[id.ni];
        &mut node.items[id.ii.0]
    }
}

impl<T> ops::Index<NodeIndex> for PathTree<'_, T> {
    type Output = Node<T>;

    fn index(&self, index: NodeIndex) -> &Self::Output {
        &self.nodes[index]
    }
}

impl<T> ops::IndexMut<NodeIndex> for PathTree<'_, T> {
    fn index_mut(&mut self, index: NodeIndex) -> &mut Self::Output {
        &mut self.nodes[index]
    }
}

impl<T> CreateOwnedType for PathTree<'_, T> {
    fn create_owned_type(&self, tid: TypeId) -> OwnedType {
        self.get_owned(tid)
    }
}

impl<T> fmt::Debug for PathTree<'_, T>
where
    T: fmt::Debug + 'static,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fn write<E>(
            this: &PathTree<E>,
            ni: NodeIndex,
            key: &mut String,
            debug_set: &mut fmt::DebugSet,
        ) where
            E: fmt::Debug,
        {
            for (seg, child) in &this[ni].children {
                let org_len = key.len();
                util::push_colon_path(key, seg);

                for (ii, item) in this[*child].iter() {
                    let pid = child.to_path_id(ii);

                    debug_set.entry(&DebugItem {
                        id: &pid,
                        path: key,
                        item,
                    });
                }

                write(this, *child, key, debug_set);

                key.truncate(org_len);
            }
        }

        // Safety: We're setting `OWNED_TYPE_CREATOR` with static lifetime, but it will be reset
        // right after use of it. (this safety based on single thread environment)
        unsafe {
            let ptr_ptree = NonNull::new_unchecked((self as *const dyn CreateOwnedType).cast_mut());
            type Src<'a> = NonNull<dyn CreateOwnedType + 'a>;
            type Dst = NonNull<dyn CreateOwnedType>;
            let ptr_ptree = mem::transmute::<Src<'_>, Dst>(ptr_ptree);
            OWNED_TYPE_CREATOR.with(|creator| creator.set(Some(ptr_ptree)));
        }

        let mut debug_set = f.debug_set();
        let mut key = String::new();
        write(self, Self::ROOT, &mut key, &mut debug_set);

        OWNED_TYPE_CREATOR.with(|creator| creator.set(None));
        debug_set.finish()?;
        Ok(())
    }
}

impl<T> format::DebugBriefly for PathTree<'_, T>
where
    T: format::DebugBriefly + 'static,
{
    fn fmt_briefly(&self, f: &mut fmt::Formatter<'_>, filter: &PrintFilter) -> fmt::Result {
        fn write<Item>(
            this: &PathTree<Item>,
            ni: NodeIndex,
            key: &mut String,
            debug_set: &mut fmt::DebugSet,
            filter: &PrintFilter,
        ) where
            Item: format::DebugBriefly,
        {
            for (seg, child) in &this[ni].children {
                let org_len = key.len();
                util::push_colon_path(key, seg);

                for (ii, item) in this[*child].iter() {
                    let pid = child.to_path_id(ii);

                    // Precedence 1. Filter by item name
                    let mut how_to_print = None;
                    if let Some(how) = filter.is_item_of.get(item.name()) {
                        how_to_print = Some(*how);
                    }
                    // Precedence 2. Filter by 'starts_with'
                    if how_to_print.is_none() {
                        if let Some(first_segment) = key.segments().next() {
                            if let Some(how) = filter.starts_with.get(first_segment) {
                                how_to_print = Some(*how);
                            }
                        }
                    }
                    // Precedence 3. Filter by 'contains'
                    if how_to_print.is_none() {
                        if let Some((_, how)) = filter
                            .contains
                            .iter()
                            .find(|(test, _)| key.contains(test.as_str()))
                        {
                            how_to_print = Some(*how);
                        }
                    }

                    match how_to_print {
                        Some(DebugBriefHow::ShowDetail) => {
                            debug_set.entry(&DebugItem {
                                id: &pid,
                                path: key,
                                item,
                            });
                        }
                        Some(DebugBriefHow::Hide) => {}
                        None => {
                            debug_set.entry(&BriefDebugItem {
                                id: &pid,
                                path: key,
                                item,
                                filter,
                            });
                        }
                    }
                }

                write(this, *child, key, debug_set, filter);

                key.truncate(org_len);
            }
        }

        // Safety: We're setting `OWNED_TYPE_CREATOR` with static lifetime, but it will be reset
        // right after use of it. (this safety based on single thread environment)
        unsafe {
            let ptr_ptree = NonNull::new_unchecked((self as *const dyn CreateOwnedType).cast_mut());
            type Src<'a> = NonNull<dyn CreateOwnedType + 'a>;
            type Dst = NonNull<dyn CreateOwnedType>;
            let ptr_ptree = mem::transmute::<Src<'_>, Dst>(ptr_ptree);
            OWNED_TYPE_CREATOR.with(|creator| creator.set(Some(ptr_ptree)));
        }

        let mut debug_set = f.debug_set();
        let mut key = String::new();
        write(self, Self::ROOT, &mut key, &mut debug_set, filter);

        OWNED_TYPE_CREATOR.with(|creator| creator.set(None));
        debug_set.finish()?;
        Ok(())
    }

    fn name(&self) -> &'static str {
        "PathTree"
    }
}

/// An iterator traversing the whole tree and yielding pairs of [`PathId`] and reference to a
/// [`Item`].
pub struct PathIter<'a, T> {
    nodes: &'a [Node<T>],
    node_iter: NodeIter<'a, T>,
    ni: NodeIndex,
}

impl<'a, T> PathIter<'a, T> {
    fn new(tree: &'a PathTree<T>) -> Self {
        Self {
            nodes: &tree.nodes,
            node_iter: NodeIter::empty(),
            ni: NodeIndex(0),
        }
    }
}

impl<T> Clone for PathIter<'_, T> {
    fn clone(&self) -> Self {
        Self {
            nodes: self.nodes,
            node_iter: self.node_iter.clone(),
            ni: self.ni,
        }
    }
}

impl<'a, T> Iterator for PathIter<'a, T> {
    type Item = (PathId, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some((ii, item)) = self.node_iter.next() {
                let ni = self.ni - 1;
                return Some((ni.to_path_id(ii), item));
            }

            if self.ni < self.nodes.len() {
                self.node_iter = self.nodes[self.ni].iter();
                self.ni += 1;
            } else {
                return None;
            }
        }
    }
}

impl<'a, T> iter::FusedIterator for PathIter<'a, T> {}

/// An iterator traversing the whole tree and yielding pairs of [`PathId`] and mutable reference to
/// a [`Item`].
pub struct PathIterMut<'a, T> {
    nodes: &'a mut [Node<T>],
    node_iter: NodeIterMut<'a, T>,
    ni: NodeIndex,
}

impl<'a, T> PathIterMut<'a, T> {
    fn new(tree: &'a mut PathTree<T>) -> Self {
        Self {
            nodes: &mut tree.nodes,
            node_iter: NodeIterMut::empty(),
            ni: NodeIndex(0),
        }
    }
}

impl<'a, T> Iterator for PathIterMut<'a, T> {
    type Item = (PathId, &'a mut T);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some((ii, item)) = self.node_iter.next() {
                let ni = self.ni - 1;
                return Some((ni.to_path_id(ii), item));
            }

            match mem::take(&mut self.nodes) {
                [] => return None,
                [head, tail @ ..] => {
                    self.nodes = tail;
                    self.node_iter = head.iter_mut();
                    self.ni += 1;
                }
            }
        }
    }
}

impl<'a, T> iter::FusedIterator for PathIterMut<'a, T> {}

pub(crate) type SearchTypeResult = crate::Which3<TypeId, PathId, ()>;
pub(crate) use crate::Which3::A as SearchTypeOk;
pub(crate) use crate::Which3::B as SearchTypeNotReady;
pub(crate) use crate::Which3::C as SearchTypeNotFound;

pub(crate) mod filter {
    use super::*;

    pub(crate) fn block<T: ItemTrait>(item: &T) -> bool {
        item.effective_kind() == EffectiveItemKind::Block
    }

    pub(crate) fn block_fn<T: ItemTrait>(item: &T) -> bool {
        matches!(
            item.effective_kind(),
            EffectiveItemKind::Block | EffectiveItemKind::Fn
        )
    }

    pub(crate) fn block_mod<T: ItemTrait>(item: &T) -> bool {
        matches!(
            item.effective_kind(),
            EffectiveItemKind::Block | EffectiveItemKind::Mod
        )
    }

    pub(crate) fn block_mod_struct<T: ItemTrait>(item: &T) -> bool {
        matches!(
            item.effective_kind(),
            EffectiveItemKind::Block | EffectiveItemKind::Mod | EffectiveItemKind::Struct
        )
    }

    pub(crate) fn enum_<T: ItemTrait>(item: &T) -> bool {
        matches!(item.effective_kind(), EffectiveItemKind::Enum)
    }

    pub(crate) fn enum_struct<T: ItemTrait>(item: &T) -> bool {
        matches!(
            item.effective_kind(),
            EffectiveItemKind::Enum | EffectiveItemKind::Struct
        )
    }

    pub(crate) fn mod_<T: ItemTrait>(item: &T) -> bool {
        item.effective_kind() == EffectiveItemKind::Mod
    }

    pub(crate) fn struct_<T: ItemTrait>(item: &T) -> bool {
        item.effective_kind() == EffectiveItemKind::Struct
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum MaybeVisible {
    Visible,
    UnknownYet,
}

impl MaybeVisible {
    #[must_use]
    fn or(self, other: Self) -> Self {
        match self {
            Self::Visible => other,
            Self::UnknownYet => self,
        }
    }
}
