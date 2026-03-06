//! [`TaskFindKnownLib`] handler

use super::task::TaskFindKnownLibFrom;
use crate::{
    etc::{abs_fs::AbstractFiles, util::IntoPathSegments},
    Set,
};

#[derive(Debug)]
pub(super) struct KnownLibFinder {
    /// Known library names that we've seen.
    seen: SeenLibs,
}

impl KnownLibFinder {
    pub(super) fn new() -> Self {
        Self {
            seen: SeenLibs::new(),
        }
    }

    /// * collector - A function receiving known library names such as "core".
    pub(super) fn find_known_lib<F: FnMut(&str)>(
        &mut self,
        task: TaskFindKnownLibFrom,
        files: &AbstractFiles,
        mut collector: F,
    ) -> bool {
        match task {
            TaskFindKnownLibFrom::Name(name) => {
                self.find_known_lib_in_name(&name, files, &mut collector)
            }
            TaskFindKnownLibFrom::Type(ty) => {
                let ty = ty.as_ref::<syn::Type>().unwrap();
                self.find_known_lib_in_type(ty, files, &mut collector)
            }
            TaskFindKnownLibFrom::Sig(sig) => {
                let sig = sig.as_ref::<syn::Signature>().unwrap();
                self.find_known_lib_in_sig(sig, files, &mut collector)
            }
            TaskFindKnownLibFrom::Block(block) => {
                let block = block.as_ref::<syn::Block>().unwrap();
                self.find_known_lib_in_block(block, files, &mut collector)
            }
        }
    }

    fn find_known_lib_in_name<F: FnMut(&str)>(
        &mut self,
        name: &str,
        files: &AbstractFiles,
        collector: &mut F,
    ) -> bool {
        let first_segment = name.segments().next().unwrap();
        if !files.is_known_library(first_segment) || self.seen.contains_name(first_segment) {
            return false;
        }

        collector(first_segment);
        self.seen.insert_name(first_segment.to_owned());
        true
    }

    fn find_known_lib_in_type<F: FnMut(&str)>(
        &mut self,
        ty: &syn::Type,
        files: &AbstractFiles,
        collector: &mut F,
    ) -> bool {
        if self.seen.contains_addr(ty) {
            return false;
        }

        let mut found = false;

        match ty {
            syn::Type::Array(ty_arr) => {
                found |= self.find_known_lib_in_type(&ty_arr.elem, files, collector);
            }
            syn::Type::Path(ty_path) => {
                if let Some(qself) = &ty_path.qself {
                    found |= self.find_known_lib_in_type(&qself.ty, files, collector);
                }
                found |= self.find_known_lib_in_path(&ty_path.path, files, collector);
            }
            syn::Type::Reference(ty_ref) => {
                found |= self.find_known_lib_in_type(&ty_ref.elem, files, collector);
            }
            syn::Type::Tuple(ty_tuple) => {
                for elem in &ty_tuple.elems {
                    found |= self.find_known_lib_in_type(elem, files, collector);
                }
            }
            o => todo!("{o:?}"),
        }

        if found {
            self.seen.insert_addr(ty);
        }
        found
    }

    fn find_known_lib_in_path<F: FnMut(&str)>(
        &mut self,
        path: &syn::Path,
        files: &AbstractFiles,
        collector: &mut F,
    ) -> bool {
        if self.seen.contains_addr(path) {
            return false;
        }

        let first_segment = path.segments.first().unwrap().ident.to_string();
        if !files.is_known_library(&first_segment) || self.seen.contains_name(&first_segment) {
            return false;
        }

        collector(&first_segment);
        self.seen.insert_name(first_segment);
        self.seen.insert_addr(path);
        true
    }

    fn find_known_lib_in_sig<F: FnMut(&str)>(
        &mut self,
        sig: &syn::Signature,
        files: &AbstractFiles,
        collector: &mut F,
    ) -> bool {
        if self.seen.contains_addr(sig) {
            return false;
        }

        let mut found = false;

        for input in &sig.inputs {
            if let syn::FnArg::Typed(pat_type) = input {
                found |= self.find_known_lib_in_type(&pat_type.ty, files, collector);
            }
        }

        if let syn::ReturnType::Type(_, ty) = &sig.output {
            found |= self.find_known_lib_in_type(ty, files, collector);
        }

        found
    }

    fn find_known_lib_in_block<F: FnMut(&str)>(
        &mut self,
        block: &syn::Block,
        files: &AbstractFiles,
        collector: &mut F,
    ) -> bool {
        if self.seen.contains_addr(block) {
            return false;
        }

        let mut found = false;

        for stmt in &block.stmts {
            match stmt {
                syn::Stmt::Local(local) => {
                    if let Some(init) = &local.init {
                        found |= self.find_known_lib_in_expr(&init.expr, files, collector);
                    }
                }
                syn::Stmt::Item(_item) => { /* Todo */ }
                syn::Stmt::Expr(expr, _) => {
                    found |= self.find_known_lib_in_expr(expr, files, collector);
                }
                syn::Stmt::Macro(_) => {}
            }
        }

        if found {
            self.seen.insert_addr(block);
        }
        found
    }

    fn find_known_lib_in_expr<F: FnMut(&str)>(
        &mut self,
        expr: &syn::Expr,
        files: &AbstractFiles,
        collector: &mut F,
    ) -> bool {
        if self.seen.contains_addr(expr) {
            return false;
        }

        let mut found = false;

        match expr {
            syn::Expr::Path(expr_path) => {
                if let Some(qself) = &expr_path.qself {
                    found |= self.find_known_lib_in_type(&qself.ty, files, collector);
                }
                found |= self.find_known_lib_in_path(&expr_path.path, files, collector)
            }
            syn::Expr::Call(expr_call) => {
                found |= self.find_known_lib_in_expr(&expr_call.func, files, collector);
                for arg in &expr_call.args {
                    found |= self.find_known_lib_in_expr(arg, files, collector);
                }
            }
            _ => { /* Todo */ }
        };

        if found {
            self.seen.insert_addr(expr);
        }
        found
    }
}

impl Default for KnownLibFinder {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
struct SeenLibs {
    seen_names: Set<String>,
    seen_addrs: Set<*const ()>,
}

impl SeenLibs {
    fn new() -> Self {
        Self {
            seen_names: Set::default(),
            seen_addrs: Set::default(),
        }
    }

    fn contains_name(&self, name: &str) -> bool {
        self.seen_names.contains(name)
    }

    fn contains_addr<T>(&self, ref_: &T) -> bool {
        let ptr = ref_ as *const T as *const ();
        self.seen_addrs.contains(&ptr)
    }

    fn insert_name(&mut self, name: String) -> bool {
        self.seen_names.insert(name)
    }

    fn insert_addr<T>(&mut self, ref_: &T) -> bool {
        let ptr = ref_ as *const T as *const ();
        self.seen_addrs.insert(ptr)
    }
}
