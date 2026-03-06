use super::task::{Task, TaskQueue};
use crate::{
    err,
    etc::{abs_fs::AbstractFiles, util},
    semantic::tree::{
        filter, Block, NodeIndex, PathTree, PathVis, PrivItem, PrivPathTree, RawConst, RawEnum,
        RawField, RawFn, RawLocal, RawMod, RawStruct, RawTrait, RawTypeAlias, RawUse, RawVariant,
        SearchTypeOk, SynToPath, Type, TypeId, TypePath,
    },
    syntax::{
        common::{AttributeHelper, IdentifySyn, SynId},
        file::File,
        SyntaxTree,
    },
    Result, TriResult, Which2,
};
use quote::ToTokens;
use std::{fmt::Write, path::PathBuf};
use syn_locator::Locate;

const TREE_ROOT: NodeIndex = PathTree::<()>::ROOT;

/// Constructor adds nodes to the path tree.
pub(super) struct Constructor<'a, 'gcx> {
    pub(super) files: &'a mut AbstractFiles,
    pub(super) stree: &'a mut SyntaxTree,
    pub(super) ptree: &'a mut PrivPathTree<'gcx>,
    pub(super) s2p: &'a mut SynToPath,
}

impl<'gcx> Constructor<'_, 'gcx> {
    pub(super) fn construct_by_file(
        &mut self,
        fpath: PathBuf,
        npath: String,
        tasks: &mut TaskQueue<'gcx>,
    ) -> Result<()> {
        let fpath = self.files.to_absolute_path(&fpath)?;

        // Files must have been added without any errors because file processing doesn't allow soft
        // error (return type is Result, not TriResult). So, we can exit early.
        if self.stree.contains_file(&fpath) {
            return Ok(());
        }

        let code = self.files.read(&fpath)?;
        let file = File::new(fpath.clone(), code)?;
        self.stree.insert_file(fpath.clone(), file);

        let mut cx = ConstructCx {
            files: self.files,
            stree: self.stree,
            ptree: self.ptree,
            s2p: self.s2p,
            tasks,
            path: npath,
        };
        let file = self.stree.get_file(&fpath).unwrap();
        cx.add_file(file)?;

        Ok(())
    }

    pub(super) fn construct_by_impl(
        &mut self,
        item_impl: *const syn::ItemImpl,
        base: NodeIndex,
        tasks: &mut TaskQueue<'gcx>,
    ) -> TriResult<(), ()> {
        let item_impl = unsafe { item_impl.as_ref().unwrap() };

        let npath = self.ptree.get_name_path(base);

        let mut cx = ConstructCx {
            files: self.files,
            stree: self.stree,
            ptree: self.ptree,
            s2p: self.s2p,
            tasks,
            path: npath,
        };
        cx.add_item_impl(item_impl)
    }
}

struct ConstructCx<'a, 'gcx> {
    files: &'a AbstractFiles,
    stree: &'a SyntaxTree,
    ptree: &'a mut PrivPathTree<'gcx>,
    s2p: &'a mut SynToPath,
    tasks: &'a mut TaskQueue<'gcx>,
    path: String,
}

impl<'gcx> ConstructCx<'_, 'gcx> {
    fn add_file(&mut self, file: &File) -> Result<()> {
        let sid = file.file.syn_id();
        if let Some(ni) = self.ptree.search(TREE_ROOT, self.path.as_str()) {
            // If the file was loaded by 'mod' from another file, we just set the file pointer.
            for (ii, item) in self.ptree[ni].iter() {
                let pid = ni.to_path_id(ii);
                match item {
                    PrivItem::Mod(_) => {
                        let mut item = self.ptree.get_mut_item(pid);
                        let mod_ = item.as_mod();

                        // The crate root is initialized with an empty fpath; update it with the
                        // real entry file path so that child modules can compute their directory
                        // correctly.
                        if mod_.fpath.as_os_str().is_empty() {
                            mod_.fpath = file.abs_path.clone();
                            mod_.mod_rs = self.files.is_mod_rs(&file.abs_path);
                        }

                        mod_.ptr_file = Some((&file.file).into());
                        self.s2p.add_syn_to_path(sid, pid);
                        break; // No more mods can exist in the same node.
                    }
                    PrivItem::RawMod(_) => {
                        self.ptree.get_mut_item(pid).as_raw_mod().ptr_file =
                            Some((&file.file).into());
                        self.s2p.add_syn_to_path(sid, pid);
                        break; // No more mods can exist in the same node.
                    }
                    _ => {}
                }
            }
        } else {
            // If the file was loaded as entry file, we need to append new module for the file.
            let item = PrivItem::RawMod(RawMod {
                ptr_mod: None,
                ptr_file: Some((&file.file).into()),
                vis_node: None,
                fpath: file.abs_path.clone(),
                mod_rs: self.files.is_mod_rs(&file.abs_path),
            });
            let pid = self.ptree.add_item(TREE_ROOT, self.path.as_str(), item);
            self.s2p.add_syn_to_path(sid, pid);
        }

        // Regardless of how the file was loaded, inserts items in the path tree.
        for item in &file.file.items {
            self.add_item(item)?;
        }
        Ok(())
    }

    fn add_item(&mut self, item: &syn::Item) -> Result<()> {
        // Tracks only when the path has been initialzied by tracking a file.
        if self.path.is_empty() {
            return Ok(());
        }

        // Items that make 'new' paths need to be inserted to the path tree.
        match item {
            syn::Item::Const(v) => self.add_item_const(v),
            syn::Item::Enum(v) => self.add_item_enum(v)?,
            syn::Item::ExternCrate(_) => {}
            syn::Item::Fn(v) => self.add_item_fn(v)?,
            syn::Item::ForeignMod(_) => {}
            syn::Item::Impl(v) => self.reserve_item_impl(v),
            syn::Item::Macro(_) => {}
            syn::Item::Mod(v) => self.add_item_mod(v)?,
            syn::Item::Static(_) => {}
            syn::Item::Struct(v) => self.add_item_struct(v)?,
            syn::Item::Trait(v) => self.add_item_trait(v),
            syn::Item::TraitAlias(_) => {}
            syn::Item::Type(v) => self.add_item_type(v),
            syn::Item::Union(_) => {}
            syn::Item::Use(v) => self.add_item_use(v),
            syn::Item::Verbatim(_) => {}
            _ => {} // Non-exhaustive
        }
        Ok(())
    }

    fn add_item_const(&mut self, item_const: &syn::ItemConst) {
        // We can make path items for free standing constants unlike associated constants.
        let org_len = self.path.len();
        util::push_colon_path(&mut self.path, &item_const.ident);

        let pid = self.ptree.add_item(
            TREE_ROOT,
            self.path.as_str(),
            PrivItem::RawConst(RawConst::Free {
                ptr_const: item_const.into(),
                vis_node: None,
                tid: None,
            }),
        );
        let sid = item_const.syn_id();
        self.s2p.add_syn_to_path(sid, pid);

        self.path.truncate(org_len);

        // Also, appends a task to evaluate the constant.
        let _ = self.tasks.push_back(Task::eval_const_free(pid));
    }

    fn add_item_enum(&mut self, item_enum: &syn::ItemEnum) -> Result<()> {
        if !item_enum.generics.params.is_empty() {
            return err!("generic is not allowed yet: `{}`", item_enum.ident.code());
        }

        let org_len = self.path.len();
        util::push_colon_path(&mut self.path, &item_enum.ident);

        let pid = self.ptree.add_item(
            TREE_ROOT,
            self.path.as_str(),
            PrivItem::RawEnum(RawEnum {
                ptr_enum: item_enum.into(),
                vis_node: None,
            }),
        );
        let sid = item_enum.syn_id();
        self.s2p.add_syn_to_path(sid, pid);

        // Variants
        for (nth, var) in item_enum.variants.iter().enumerate() {
            self.add_variant(var, nth);
        }

        self.path.truncate(org_len);
        Ok(())
    }

    fn add_item_fn(&mut self, item_fn: &syn::ItemFn) -> Result<()> {
        if !item_fn.sig.generics.params.is_empty() {
            return err!("generic is not allowed for now: `{}`", item_fn.sig.code());
        }

        let org_len = self.path.len();
        util::push_colon_path(&mut self.path, &item_fn.sig.ident);

        let pid = self.ptree.add_item(
            TREE_ROOT,
            self.path.as_str(),
            PrivItem::RawFn(RawFn {
                ptr_attr: (&item_fn.attrs).into(),
                ptr_vis: (&item_fn.vis).into(),
                ptr_sig: (&item_fn.sig).into(),
                ptr_block: (&*item_fn.block).into(),
                vis_node: None,
                unscoped_base: None,
            }),
        );
        let sid = item_fn.syn_id();
        self.s2p.add_syn_to_path(sid, pid);

        // Signature & Block
        self.add_signature(&item_fn.sig);
        let mut state = BlockState { nth: 0 };
        self.add_block(&item_fn.block, &mut state)?;

        self.path.truncate(org_len);
        Ok(())
    }

    fn add_item_impl(&mut self, item_impl: &syn::ItemImpl) -> TriResult<(), ()> {
        // TODO: associated items cannot be added to the path tree due to their generics. We should
        // look into logic DB when we search the associated items. But for now, associated
        // functions belong to the path tree.

        // Finds the self type.
        let node = self.ptree.search(TREE_ROOT, self.path.as_str()).unwrap();
        let base = self.ptree.nearest_item(node, filter::block_mod).ni;
        let SearchTypeOk(self_ty) =
            TypeId::from_syn_type(&item_impl.self_ty, self.stree, self.ptree, base)
        else {
            return err!(soft, ());
        };

        // Fixes the self type later.
        let _ = self.tasks.push_back(Task::fix_impl_type(
            item_impl.self_ty.syn_id(),
            item_impl.generics.syn_id(),
            self_ty,
            base,
        ));

        // If self type is a path, then we can add it to the path tree.
        if let Type::Path(TypePath { pid, .. }) = self.ptree.get_type(self_ty) {
            let org_path = self.path.clone();
            self.path = self.ptree.get_name_path(pid.ni);

            for item in &item_impl.items {
                if let syn::ImplItem::Fn(v) = item {
                    self.add_impl_item_fn(v, base)?;
                }
            }

            self.path = org_path;
        }

        // Regardless whether self type is a path or not, we can do something about the type like,
        // - Evaluate associated constants and store them.
        for item in &item_impl.items {
            if let syn::ImplItem::Const(v) = item {
                let const_task = if item_impl.trait_.is_some() {
                    Task::eval_const_trait_impl(v.expr.syn_id(), v.ty.syn_id(), base)
                } else {
                    Task::eval_const_inher(v.expr.syn_id(), v.ty.syn_id(), base)
                };
                let _ = self.tasks.push_back(const_task);
            }
        }

        Ok(())
    }

    fn add_impl_item_fn(
        &mut self,
        item_fn: &syn::ImplItemFn,
        unscoped_base: NodeIndex,
    ) -> Result<()> {
        if !item_fn.sig.generics.params.is_empty() {
            return err!("generic is not allowed for now: `{}`", item_fn.sig.code());
        }

        let org_len = self.path.len();
        util::push_colon_path(&mut self.path, &item_fn.sig.ident);

        let pid = self.ptree.add_item(
            TREE_ROOT,
            self.path.as_str(),
            PrivItem::RawFn(RawFn {
                ptr_attr: (&item_fn.attrs).into(),
                ptr_vis: (&item_fn.vis).into(),
                ptr_sig: (&item_fn.sig).into(),
                ptr_block: (&item_fn.block).into(),
                vis_node: None,
                unscoped_base: Some(unscoped_base),
            }),
        );
        let sid = item_fn.syn_id();
        self.s2p.add_syn_to_path(sid, pid);

        // Signature & Block
        self.add_signature(&item_fn.sig);
        let mut state = BlockState { nth: 0 };
        self.add_block(&item_fn.block, &mut state)?;

        self.path.truncate(org_len);
        Ok(())
    }

    fn add_item_mod(&mut self, item_mod: &syn::ItemMod) -> Result<()> {
        // Terminology
        // * inline - Module is inline when it contains its content within {}.
        // * fpath - File path of the module.
        //   - "/home/a.rs" if non-inline module.
        //   - "/home/a" if inline module.
        // * mod_rs - Whether the module is a `mod-rs` or not.
        //   - `mod-rs` if the file is one of "mod.rs", "main.rs", or "lib.rs".
        //   - `mod-rs` if the file is determined by "path" attribute and the module is not inline
        //     (e.g. #[path = "a.rs"] mod foo;)
        //   - `non-mod-rs` otherwise.
        // ref: https://doc.rust-lang.org/reference/items/modules.html

        // Retrieves parent module info.
        let ni = self.ptree.search(TREE_ROOT, self.path.as_str()).unwrap();
        let parent_pid = self.ptree.nearest_item(ni, filter::mod_);
        let (parent_fpath, parent_mod_rs, parent_inline) = match &self.ptree[parent_pid] {
            PrivItem::Mod(v) => (&v.fpath, v.mod_rs, v.is_inline()),
            PrivItem::RawMod(v) => (&v.fpath, v.mod_rs, v.is_inline()),
            _ => unreachable!(),
        };

        // Directory containing a file that declares this module with or without content.
        let dir = parent_fpath.parent().unwrap();

        let inline = item_mod.content.is_some();

        // Determines this module's file path and `mod-rs`.
        let fpath;
        let mod_rs;
        if let Some(path_value) = item_mod.get_attribute_value("path") {
            let path_value = path_value.to_token_stream().to_string();
            let path_value = path_value.trim_matches('"');
            fpath = if !parent_inline {
                dir.join(path_value)
            } else {
                parent_fpath.with_extension("").join(path_value)
            };
            mod_rs = !inline;
        } else {
            let base_buf;
            let base = if parent_mod_rs {
                dir
            } else {
                base_buf = parent_fpath.with_extension("");
                base_buf.as_ref()
            };

            if inline {
                fpath = base.join(format!("{}", item_mod.ident));
                mod_rs = false;
            } else {
                let fpath_a = base.join(format!("{}.rs", item_mod.ident));
                let fpath_b = base.join(format!("{}/mod.rs", item_mod.ident));

                match (self.files.exists(&fpath_a), self.files.exists(&fpath_b)) {
                    (true, false) => {
                        fpath = fpath_a;
                        mod_rs = false;
                    }
                    (false, true) => {
                        fpath = fpath_b;
                        mod_rs = true;
                    }
                    (true, true) => {
                        return err!(
                            "found conflicting paths: `{}` and `{}`",
                            fpath_a.to_string_lossy(),
                            fpath_b.to_string_lossy()
                        );
                    }
                    (false, false) => {
                        return err!(
                            "expected `{}` or `{}`, but not found",
                            fpath_a.to_string_lossy(),
                            fpath_b.to_string_lossy()
                        );
                    }
                }
            }
        };

        // Enters the scope.
        let org_len = self.path.len();
        util::push_colon_path(&mut self.path, &item_mod.ident);

        let pid = self.ptree.add_item(
            TREE_ROOT,
            self.path.as_str(),
            PrivItem::RawMod(RawMod {
                ptr_mod: Some(item_mod.into()),
                ptr_file: None,
                vis_node: None,
                fpath,
                mod_rs,
            }),
        );
        let sid = item_mod.syn_id();
        self.s2p.add_syn_to_path(sid, pid);

        // Internal items.
        if let Some((_, items)) = &item_mod.content {
            for item in items.iter() {
                self.add_item(item)?;
            }
        }

        // Exits the scope.
        self.path.truncate(org_len);

        // Appends a task for the new file if necessary.
        if let PrivItem::RawMod(raw_mod) = &self.ptree[pid] {
            if !inline && !self.stree.contains_file(&raw_mod.fpath) {
                let fpath = raw_mod.fpath.clone();
                let npath = self.ptree.get_name_path(pid.ni);
                let task = Task::construct_path_tree_for_file(fpath, npath);
                let _ = self.tasks.push_front(task);
            }
        }

        Ok(())
    }

    fn add_item_struct(&mut self, item_struct: &syn::ItemStruct) -> Result<()> {
        if !item_struct.generics.params.is_empty() {
            return err!(
                "generic is not allowed for now: `{}`",
                item_struct.ident.code()
            );
        }

        let org_len = self.path.len();
        util::push_colon_path(&mut self.path, &item_struct.ident);

        let pid = self.ptree.add_item(
            TREE_ROOT,
            self.path.as_str(),
            PrivItem::RawStruct(RawStruct {
                ptr_struct: item_struct.into(),
                vis_node: None,
            }),
        );
        let sid = item_struct.syn_id();
        self.s2p.add_syn_to_path(sid, pid);

        // Fields
        for (nth, field) in item_struct.fields.iter().enumerate() {
            self.add_field(field, nth as u32);
        }

        self.path.truncate(org_len);
        Ok(())
    }

    fn add_item_trait(&mut self, item_trait: &syn::ItemTrait) {
        let org_len = self.path.len();
        util::push_colon_path(&mut self.path, &item_trait.ident);

        let pid = self.ptree.add_item(
            TREE_ROOT,
            self.path.as_str(),
            PrivItem::RawTrait(RawTrait {
                ptr_trait: item_trait.into(),
                vis_node: None,
            }),
        );
        let sid = item_trait.syn_id();
        self.s2p.add_syn_to_path(sid, pid);

        // Trait items
        for item in &item_trait.items {
            // TODO: other variants
            if let syn::TraitItem::Const(v) = item {
                self.add_trait_item_const(v);
            }
        }

        self.path.truncate(org_len);
    }

    fn add_trait_item_const(&mut self, item_const: &syn::TraitItemConst) {
        let node = self.ptree.search(TREE_ROOT, self.path.as_str()).unwrap();
        let base = self.ptree.nearest_item(node, filter::block_mod).ni;

        if let Some((_, expr)) = &item_const.default {
            let task = Task::eval_const_trait_default(expr.syn_id(), item_const.ty.syn_id(), base);
            let _ = self.tasks.push_back(task);
        }
    }

    fn add_item_type(&mut self, item_type: &syn::ItemType) {
        let org_len = self.path.len();
        util::push_colon_path(&mut self.path, &item_type.ident);

        let pid = self.ptree.add_item(
            TREE_ROOT,
            self.path.as_str(),
            PrivItem::RawTypeAlias(RawTypeAlias {
                ptr_type: item_type.into(),
                vis_node: None,
            }),
        );
        let sid = item_type.syn_id();
        self.s2p.add_syn_to_path(sid, pid);

        self.path.truncate(org_len);
    }

    fn add_item_use(&mut self, item_use: &syn::ItemUse) {
        fn dfs(
            this: &mut ConstructCx,
            item: &syn::ItemUse,
            node: &syn::UseTree,
            vis: PathVis,
            buf: &mut String,
        ) {
            match node {
                syn::UseTree::Path(v) => {
                    let org_len = buf.len();
                    write!(buf, "{}::", v.ident).unwrap();
                    dfs(this, item, &v.tree, vis, buf);
                    buf.truncate(org_len);
                }
                syn::UseTree::Name(v) => {
                    let org_len = this.path.len();
                    util::push_colon_path(&mut this.path, &v.ident);

                    let syn_part = v.syn_id();
                    add_path_item(this, item, syn_part, format!("{buf}{}", v.ident));

                    this.path.truncate(org_len);
                }
                syn::UseTree::Rename(v) => {
                    let org_len = this.path.len();
                    util::push_colon_path(&mut this.path, &v.rename);

                    let syn_part = v.syn_id();
                    add_path_item(this, item, syn_part, format!("{buf}{}", v.ident));

                    this.path.truncate(org_len);
                }
                syn::UseTree::Glob(v) => {
                    let org_len = this.path.len();
                    this.path.push_str("::*");

                    let syn_part = v.syn_id();
                    add_path_item(this, item, syn_part, format!("{buf}*"));

                    this.path.truncate(org_len);
                }
                syn::UseTree::Group(v) => {
                    for node in &v.items {
                        dfs(this, item, node, vis.clone(), buf);
                    }
                }
            }
        }

        fn add_path_item(
            this: &mut ConstructCx,
            item: &syn::ItemUse,
            syn_part: SynId,
            npath: String,
        ) {
            let pid = this.ptree.add_item(
                TREE_ROOT,
                this.path.as_str(),
                PrivItem::RawUse(RawUse {
                    ptr_group: item.into(),
                    syn_part,
                    vis_node: None,
                    npath,
                    dst_node: None,
                }),
            );
            this.s2p.add_syn_to_path(syn_part, pid);
        }

        let mut buf = String::new();
        dfs(
            self,
            item_use,
            &item_use.tree,
            PathVis::new(&item_use.vis),
            &mut buf,
        );
    }

    fn add_field(&mut self, field: &syn::Field, nth: u32) {
        let org_len = self.path.len();
        if let Some(ident) = &field.ident {
            util::push_colon_path(&mut self.path, ident);
        } else {
            util::push_colon_path(&mut self.path, nth);
        }

        let pid = self.ptree.add_item(
            TREE_ROOT,
            self.path.as_str(),
            PrivItem::RawField(RawField {
                ptr_field: field.into(),
                vis_node: None,
            }),
        );
        let sid = field.syn_id();
        self.s2p.add_syn_to_path(sid, pid);

        self.path.truncate(org_len);
    }

    fn add_variant(&mut self, var: &syn::Variant, nth: usize) {
        let org_len = self.path.len();
        util::push_colon_path(&mut self.path, &var.ident);

        let pid = self.ptree.add_item(
            TREE_ROOT,
            self.path.as_str(),
            PrivItem::RawVariant(RawVariant {
                ptr_variant: var.into(),
                vis_node: None,
                nth,
            }),
        );
        let sid = var.syn_id();
        self.s2p.add_syn_to_path(sid, pid);

        self.path.truncate(org_len);
    }

    fn add_signature(&mut self, sig: &syn::Signature) {
        for input in &sig.inputs {
            match input {
                syn::FnArg::Receiver(v) => self.add_receiver(v),
                syn::FnArg::Typed(v) => self.add_local_pat_type(v, &v.attrs),
            }
        }
    }

    fn add_receiver(&mut self, recv: &syn::Receiver) {
        let org_len = self.path.len();
        util::push_colon_path(&mut self.path, "self");

        let pid = self.ptree.add_item(
            TREE_ROOT,
            self.path.as_str(),
            PrivItem::RawLocal(RawLocal {
                ptr_attr: (&recv.attrs).into(),
                ptr_ident: Which2::B(recv.into()),
                ptr_ty: Some((&*recv.ty).into()),
                mut_: false, // TODO: Remove me
            }),
        );
        let sid = recv.syn_id();
        self.s2p.add_syn_to_path(sid, pid);

        self.path.truncate(org_len);
    }

    fn add_block(&mut self, block: &syn::Block, state: &mut BlockState) -> Result<()> {
        let org_len = self.path.len();
        util::push_colon_path(&mut self.path, format!("{{{}}}", state.nth));
        state.nth += 1;

        let pid = self.ptree.add_item(
            TREE_ROOT,
            self.path.as_str(),
            PrivItem::Block(Block {
                ptr_block: block.into(),
            }),
        );
        let sid = block.syn_id();
        self.s2p.add_syn_to_path(sid, pid);

        // Statements
        let mut state = BlockState { nth: 0 };
        for stmt in &block.stmts {
            self.add_stmt(stmt, &mut state)?;
        }

        self.path.truncate(org_len);
        Ok(())
    }

    fn add_stmt(&mut self, stmt: &syn::Stmt, state: &mut BlockState) -> Result<()> {
        match stmt {
            syn::Stmt::Local(v) => self.add_local_pat(&v.pat, None, &v.attrs),
            syn::Stmt::Item(v) => self.add_item(v)?,
            syn::Stmt::Expr(v, _) => self.add_expr(v, state)?,
            syn::Stmt::Macro(..) => {}
        }
        Ok(())
    }

    fn add_expr(&mut self, expr: &syn::Expr, state: &mut BlockState) -> Result<()> {
        if let syn::Expr::Block(syn::ExprBlock { block, .. }) = expr {
            self.add_block(block, state)
        } else {
            Ok(())
        }
    }

    fn add_local_pat(
        &mut self,
        pat: &syn::Pat,
        ty: Option<&syn::Type>,
        attr: &Vec<syn::Attribute>,
    ) {
        match pat {
            syn::Pat::Ident(v) => self.add_local_pat_ident(v, ty, attr),
            syn::Pat::Struct(v) => self.add_local_pat_struct(v, attr),
            syn::Pat::Tuple(v) => self.add_local_pat_tuple(v, ty, attr),
            syn::Pat::Type(v) => self.add_local_pat_type(v, attr),
            _ => {}
        }
    }

    fn add_local_pat_struct(&mut self, pat_struct: &syn::PatStruct, attr: &Vec<syn::Attribute>) {
        for field in &pat_struct.fields {
            self.add_local_pat(&field.pat, None, attr);
        }
    }

    fn add_local_pat_tuple(
        &mut self,
        pat_tuple: &syn::PatTuple,
        ty: Option<&syn::Type>,
        attr: &Vec<syn::Attribute>,
    ) {
        match ty {
            Some(syn::Type::Tuple(ty_tuple)) => {
                for (pat, ty) in pat_tuple.elems.iter().zip(&ty_tuple.elems) {
                    self.add_local_pat(pat, Some(ty), attr);
                }
            }
            _ => {
                for pat in &pat_tuple.elems {
                    self.add_local_pat(pat, None, attr);
                }
            }
        }
    }

    fn add_local_pat_type(&mut self, pat_type: &syn::PatType, attr: &Vec<syn::Attribute>) {
        let pat = &*pat_type.pat;
        let ty = &*pat_type.ty;

        match ty {
            syn::Type::Path(_)
            | syn::Type::Array(_)
            | syn::Type::Slice(_)
            | syn::Type::Reference(_) => self.add_local_pat(pat, Some(ty), attr),
            syn::Type::Tuple(ty_tuple) => match pat {
                syn::Pat::Ident(pat_ident) => {
                    self.add_local_pat_ident(pat_ident, Some(ty), attr);
                }
                syn::Pat::Tuple(pat_tuple) => {
                    for (pat, ty) in pat_tuple.elems.iter().zip(&ty_tuple.elems) {
                        self.add_local_pat(pat, Some(ty), attr);
                    }
                }
                _ => unreachable!(),
            },
            _ => {}
        }
    }

    fn add_local_pat_ident(
        &mut self,
        pat_ident: &syn::PatIdent,
        ty: Option<&syn::Type>,
        attr: &Vec<syn::Attribute>,
    ) {
        let org_len = self.path.len();
        util::push_colon_path(&mut self.path, &pat_ident.ident);

        let pid = self.ptree.add_item(
            TREE_ROOT,
            self.path.as_str(),
            PrivItem::RawLocal(RawLocal {
                ptr_attr: attr.into(),
                ptr_ident: Which2::A(pat_ident.into()),
                ptr_ty: ty.map(|ty| ty.into()),
                mut_: pat_ident.mutability.is_some(),
            }),
        );
        let sid = pat_ident.syn_id();
        self.s2p.add_syn_to_path(sid, pid);

        self.path.truncate(org_len);
    }

    fn reserve_item_impl(&mut self, item_impl: &syn::ItemImpl) {
        let base = self.ptree.search(TREE_ROOT, self.path.as_str()).unwrap();
        let task = Task::construct_path_tree_for_impl(item_impl.syn_id(), base);
        let _ = self.tasks.push_front(task);
    }
}

#[derive(Clone, Copy)]
struct BlockState {
    nth: u32,
}
