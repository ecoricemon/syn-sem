use super::{
    term,
    util::{ptr_to_name, try_make_int_or_float_term, var_name},
};
use crate::{
    ds::vec::BoxedSlice,
    etc::util::{push_colon_path, IntoPathSegments},
    semantic::{
        basic_traits::{EvaluateArrayLength, RawScope, Scope, Scoping},
        entry::GlobalCx,
        tree::ArrayLen,
    },
    ClauseIn, ExprIn, Map, NameIn, PredicateIn, Set, TermIn, TriResult,
};
use logic_eval::{Clause, ClauseIter, Database, Expr, Name, ProveCx, Term, VAR_PREFIX};
use std::{borrow, hash::Hash, iter, ops};
use syn_locator::Locate;

/// Auto-generated variables will be named in order of '$#A', '$#B', ..., '$#Z', '$#0', '$#1', ...
const AUTO_VAR_PREFIX: &str = "$#";
const CUSTOM_PREFIX_LETTERS: [char; 1] = ['#' /* used at AUTO_VAR_PREFIX */];
const _: () = assert!(VAR_PREFIX == '$');

pub(crate) trait Host<'gcx>: Scoping + EvaluateArrayLength<'gcx> {
    fn ident_to_npath(&mut self, ident: &syn::Ident) -> TriResult<String, ()>;
}

pub(super) struct HostWrapper<'a, 'gcx, H> {
    inner: &'a mut H,
    scope_stack: Vec<RawScope>,
    gcx: &'gcx GlobalCx<'gcx>,
}

impl<'a, 'gcx, H: Host<'gcx>> HostWrapper<'a, 'gcx, H> {
    pub(super) fn new(gcx: &'gcx GlobalCx<'gcx>, host: &'a mut H) -> Self {
        Self {
            inner: host,
            scope_stack: Vec::new(),
            gcx,
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

    /// e.g.
    /// * Input - ident: Z / arg: arg($T, $U)
    /// * Output - list(x, y, Z($T, $U))
    pub(super) fn ident_to_term(
        &mut self,
        ident: &syn::Ident,
        arg: TermIn<'gcx>,
    ) -> TriResult<TermIn<'gcx>, ()> {
        let npath = self.inner.ident_to_npath(ident)?;

        // e.g. i32 -> int(i32), w/o arg
        if let Some(term) = try_make_int_or_float_term(&npath, self.gcx) {
            debug_assert!(arg.args.is_empty());
            return Ok(term);
        }

        // bool or char, w/o arg
        if ["bool", "char"].contains(&npath.as_str()) {
            debug_assert!(arg.args.is_empty());
            return Ok(Term {
                functor: Name::with_intern(&npath, self.gcx),
                args: [].into(),
            });
        }

        let num_seg = npath.segments().count();

        let term = match num_seg as u32 {
            0 => unreachable!(),
            1 => Term {
                functor: Name::with_intern(&npath, self.gcx),
                args: [arg].into(),
            },
            2.. => {
                let empty_arg = || term::arg_n([].into(), self.gcx);

                let elems: Vec<TermIn<'gcx>> = npath
                    .segments()
                    .take(num_seg - 1)
                    .map(|segment| Term {
                        functor: Name::with_intern(segment, self.gcx),
                        args: [empty_arg()].into(),
                    })
                    .chain(iter::once(Term {
                        functor: Name::with_intern(npath.segments().last().unwrap(), self.gcx),
                        args: [arg].into(),
                    }))
                    .collect();

                term::list_n(elems, self.gcx)
            }
        };
        Ok(term)
    }
}

impl<'gcx, H: Host<'gcx>> Host<'gcx> for HostWrapper<'_, 'gcx, H> {
    fn ident_to_npath(&mut self, ident: &syn::Ident) -> TriResult<String, ()> {
        Host::ident_to_npath(self.inner, ident)
    }
}

impl<'gcx, H: Host<'gcx>> Scoping for HostWrapper<'_, 'gcx, H> {
    fn on_enter_scope(&mut self, scope: Scope) {
        <Self>::on_enter_scope(self, scope)
    }

    fn on_exit_scope(&mut self, _: Scope) {
        <Self>::on_exit_scope(self)
    }
}

impl<'gcx, H: Host<'gcx>> EvaluateArrayLength<'gcx> for HostWrapper<'_, 'gcx, H> {
    fn eval_array_len(&mut self, expr: &syn::Expr) -> TriResult<crate::ArrayLen, ()> {
        EvaluateArrayLength::eval_array_len(self.inner, expr)
    }
}

trait Loadable {}
impl Loadable for syn::File {}
impl Loadable for syn::Item {}
impl Loadable for syn::ItemImpl {}

/// Logic DB about `impl` blocks. The DB also will include something associated with the impl
/// blocks such as `trait` or `struct`.
#[derive(Debug)]
pub struct ImplLogic<'gcx> {
    gcx: &'gcx GlobalCx<'gcx>,

    db: DatabaseWrapper<'gcx>,

    /// Stores already loaded items such as files, impl blocks, and so on in order to avoid
    /// duplicate loading for the same item.
    loaded: Set<*const ()>,

    /// Default types of generic parameters of structs, traits, enums, and type aliases.
    default_generics: DefaultGenericMap<'gcx>,
}

impl<'gcx> ImplLogic<'gcx> {
    pub(crate) fn new(gcx: &'gcx GlobalCx<'gcx>) -> Self {
        Self {
            gcx,
            db: DatabaseWrapper::new(gcx),
            loaded: Set::default(),
            default_generics: Map::default(),
        }
    }

    pub fn query(&mut self, expr: ExprIn<'gcx>) -> ProveCx<'_, 'gcx, GlobalCx<'gcx>> {
        self.db.query(expr)
    }

    pub fn clauses(&self) -> ClauseIter<'_, 'gcx, GlobalCx<'gcx>> {
        self.db.clauses()
    }

    pub fn to_prolog(&self) -> String {
        self.db.to_prolog()
    }

    pub fn insert_clause(&mut self, clause: ClauseIn<'gcx>) {
        self.db.insert_clause(clause);
    }

    pub fn commit(&mut self) {
        self.db.commit();
    }

    // TODO: Loading a huge file like "core" is taking a lot of time.
    pub(crate) fn load_file<H: Host<'gcx>>(
        &mut self,
        host: &mut H,
        file: &syn::File,
    ) -> TriResult<(), ()> {
        let gcx = self.gcx;
        LoadCx {
            gcx,
            logic: self,
            host: HostWrapper::new(gcx, host),
        }
        .load_file(file)
    }

    pub(crate) fn load_item_impl<H: Host<'gcx>>(
        &mut self,
        item_impl: &syn::ItemImpl,
        host: &mut H,
    ) -> TriResult<(), ()> {
        let gcx = self.gcx;
        LoadCx {
            gcx,
            logic: self,
            host: HostWrapper::new(gcx, host),
        }
        .load_item_impl(item_impl)
    }
}

struct LoadCx<'a, 'gcx, H> {
    gcx: &'gcx GlobalCx<'gcx>,
    logic: &'a mut ImplLogic<'gcx>,
    host: HostWrapper<'a, 'gcx, H>,
}

impl<'a, 'gcx, H: Host<'gcx>> LoadCx<'a, 'gcx, H> {
    fn load_file(&mut self, file: &syn::File) -> TriResult<(), ()> {
        if self.has_loaded(file) {
            return Ok(());
        }

        for item in &file.items {
            self.load_item(item)?;
        }

        self.save_change(file);
        Ok(())
    }

    fn load_item(&mut self, item: &syn::Item) -> TriResult<(), ()> {
        if self.has_loaded(item) {
            return Ok(());
        }

        match item {
            syn::Item::Impl(v) => self.load_item_impl(v)?,
            syn::Item::Mod(v) => self.load_item_mod(v)?,
            syn::Item::Struct(v) => self.load_item_struct(v)?,
            syn::Item::Trait(v) => self.load_item_trait(v)?,
            _ => {}
        }

        self.save_change(item);
        Ok(())
    }

    /// This method could generate clauses as follows.
    /// * impl(SelfTy).
    /// * impl(SelfTy, Trait).
    /// * assoc_ty(SelfTy, Trait, AssocTy, AssignTy) :- impl(SelfTy, Trait).
    /// * assoc_const_val(SelfTy, Trait, ConstName, ConstId) :-
    ///   assoc_const_ty(Trait, ConstName, _).
    /// * inher_fn(SelfTy, FnName, sig(..)) :- impl(SelfTy).
    /// * inher_const(SelfTy, ConstName, ConstTy, ConstId) :- impl(SelfTy).
    fn load_item_impl(&mut self, item_impl: &syn::ItemImpl) -> TriResult<(), ()> {
        if self.has_loaded(item_impl) {
            return Ok(());
        }

        // Generics
        let mut bound = Bound::new(self.gcx);
        bound.push_generics(&item_impl.generics, &mut self.host)?;

        // Self type
        let self_ty = Finder::new(self.gcx, &mut self.host, &mut bound)
            .with_default_generic_context(&self.logic.default_generics, None)
            .type_to_term(&item_impl.self_ty)?;

        // Trait
        let (impl_, trait_) = if let Some((_, path, _)) = &item_impl.trait_ {
            let trait_ = Finder::new(self.gcx, &mut self.host, &mut bound)
                .with_default_generic_context(&self.logic.default_generics, Some(self_ty.clone()))
                .path_to_term(path)?;
            let impl_ = term::impl_2(self_ty.clone(), trait_.clone(), self.gcx);
            (impl_, Some(trait_))
        } else {
            let impl_ = term::impl_1(self_ty.clone(), self.gcx);
            (impl_, None)
        };

        // e.g. `impl(SelfTy).` or `impl(SelfTy, Trait).`
        self.logic.db.insert_clause(Clause {
            head: impl_.clone(),
            body: bound.take_bound_expr(),
        });

        // Now we go into inside 'impl' block, then make clauses for the associated items of the
        // block.
        if let Some(trait_) = trait_ {
            LoadTraitImplItemCx {
                gcx: self.gcx,
                logic: self.logic,
                host: &mut self.host,
                bound: &mut bound,
                self_ty: &self_ty,
                trait_: &trait_,
            }
            .load_impl_items(&item_impl.items)?;
        } else {
            LoadInherentImplItemCx {
                gcx: self.gcx,
                logic: self.logic,
                host: &mut self.host,
                bound: &mut bound,
                self_ty: &self_ty,
                impl_: &impl_,
            }
            .load_impl_items(&item_impl.items)?;
        }

        self.save_change(item_impl);
        Ok(())
    }

    fn load_item_mod(&mut self, item_mod: &syn::ItemMod) -> TriResult<(), ()> {
        self.host.on_enter_scope(Scope::Mod(item_mod));

        if let Some((_, items)) = &item_mod.content {
            for item in items {
                self.load_item(item)?;
            }
        }

        self.host.on_exit_scope();
        Ok(())
    }

    /// This method could generate clauses as follows.
    /// * impl(Struct, Sized) :- impl(Field, Sized).
    fn load_item_struct(&mut self, item_struct: &syn::ItemStruct) -> TriResult<(), ()> {
        // Generics
        let mut bound = Bound::new(self.gcx);
        bound.push_generics(&item_struct.generics, &mut self.host)?;

        // Struct
        let arg = generics_to_arg(&item_struct.generics, self.gcx);
        let struct_ = self.host.ident_to_term(&item_struct.ident, arg)?;

        // Adds default generics info.
        let path = self.host.ident_to_npath(&item_struct.ident)?;
        let path = Name::with_intern(&path, self.gcx);
        self.set_default_generics(path, &item_struct.generics, &mut bound)?;

        // e.g. impl(Struct, Sized)
        let sized = Term {
            functor: Name::with_intern("Sized", self.gcx),
            args: [].into(),
        };
        let head = term::impl_2(struct_, sized.clone(), self.gcx);

        // e.g. impl(Field, Sized)
        let mut fill = |fields: syn::punctuated::Iter<'_, syn::Field>| -> TriResult<(), ()> {
            for field in fields {
                let field =
                    Finder::new(self.gcx, &mut self.host, &mut bound).type_to_term(&field.ty)?;
                let impl_ = term::impl_2(field, sized.clone(), self.gcx);
                bound.push_bound_term(impl_);
            }
            Ok(())
        };
        match &item_struct.fields {
            syn::Fields::Named(fields) => fill(fields.named.iter())?,
            syn::Fields::Unnamed(fields) => fill(fields.unnamed.iter())?,
            syn::Fields::Unit => {}
        }
        let body = bound.take_bound_expr();

        self.logic.db.insert_clause(Clause { head, body });
        Ok(())
    }

    /// This method could generate clauses as follows.
    /// * trait(Trait).
    /// * assoc_fn(Trait, FnName, sig(..)) :- trait(Trait).
    /// * assoc_const_ty(Trait, ConstName, ConstTy) :- trait(Trait).
    /// * assoc_const_val(Trait, ConstName, ConstId) :- assoc_const_ty(Trait, ConstName, _).
    fn load_item_trait(&mut self, item_trait: &syn::ItemTrait) -> TriResult<(), ()> {
        // Generics
        let mut bound = Bound::new(self.gcx);
        bound.push_generics(&item_trait.generics, &mut self.host)?;

        // Trait
        let arg = generics_to_arg(&item_trait.generics, self.gcx);
        let trait_ = self.host.ident_to_term(&item_trait.ident, arg)?;

        // Adds default generics info.
        let path = self.host.ident_to_npath(&item_trait.ident)?;
        let path = Name::with_intern(&path, self.gcx);
        self.set_default_generics(path, &item_trait.generics, &mut bound)?;

        // e.g. trait(Trait).
        let head = term::trait_1(trait_, self.gcx);
        let body = bound.take_bound_expr();
        self.logic.db.insert_clause(Clause {
            head: head.clone(),
            body,
        });

        // Inside the trait definition block.
        LoadTraitItemCx {
            gcx: self.gcx,
            logic: self.logic,
            host: &mut self.host,
            bound: &mut bound,
            trait_: &head,
        }
        .load_trait_items(&item_trait.items)
    }

    fn set_default_generics(
        &mut self,
        path: NameIn<'gcx>,
        generics: &syn::Generics,
        bound: &mut Bound<'gcx>,
    ) -> TriResult<(), ()> {
        let mut default_generics = Vec::with_capacity(generics.params.len());
        for param in &generics.params {
            match param {
                syn::GenericParam::Type(ty_param) => {
                    let term = if let Some(ty) = &ty_param.default {
                        let term = Finder::new(self.gcx, &mut self.host, bound).type_to_term(ty)?;
                        Some(term)
                    } else {
                        None
                    };
                    default_generics.push(term);
                }
                _ => todo!(),
            }
        }

        self.logic
            .default_generics
            .insert(path, default_generics.into());
        Ok(())
    }

    /// Returns true if the syn node has already been loaded. We don't need to load it again.
    fn has_loaded<T: Loadable>(&mut self, syn: &T) -> bool {
        let ptr = syn as *const T as *const ();
        self.logic.loaded.contains(&ptr)
    }

    /// Records that the syn node has been fully loaded. [`has_loaded`](Self::has_loaded) will
    /// return true for the syn node after call to this method.
    fn save_change<T: Loadable>(&mut self, syn: &T) {
        let ptr = syn as *const T as *const ();
        self.logic.loaded.insert(ptr);

        // Confirms the change.
        self.logic.db.commit();
    }
}

// TODO: do we really need three lifetimes?
/// For associated items in a trait impl block.
struct LoadTraitImplItemCx<'i, 'o, 'gcx, H> {
    gcx: &'gcx GlobalCx<'gcx>,
    logic: &'o mut ImplLogic<'gcx>,
    host: &'o mut HostWrapper<'i, 'gcx, H>,
    bound: &'o mut Bound<'gcx>,
    self_ty: &'o TermIn<'gcx>,
    trait_: &'o TermIn<'gcx>,
}

impl<'i, 'o, 'gcx, H: Host<'gcx>> LoadTraitImplItemCx<'i, 'o, 'gcx, H> {
    fn load_impl_items(&mut self, impl_items: &[syn::ImplItem]) -> TriResult<(), ()> {
        for impl_item in impl_items {
            match impl_item {
                syn::ImplItem::Const(c) => self._load_impl_item_const(c)?,
                syn::ImplItem::Fn(_) => { /* No corresponding term yet */ }
                syn::ImplItem::Type(ty) => self._load_impl_item_type(ty)?,
                _ => {}
            }
        }
        Ok(())
    }

    /// This method could generate clauses as follows.
    /// * assoc_const_val(SelfTy, Trait, ConstName, ConstId) :-
    ///   assoc_const_ty(Trait, ConstName, _).
    fn _load_impl_item_const(&mut self, item_const: &syn::ImplItemConst) -> TriResult<(), ()> {
        // Note: `item_const.generics` is experimental at the time of writing, but just keep this
        // for consistency.

        debug_assert!(self.bound.bound_terms.is_empty());

        // e.g. CONST<T, U> -> CONST(arg($T, $U))
        let arg = generics_to_arg(&item_const.generics, self.gcx);
        let const_name = Term {
            functor: Name::with_intern(&item_const.ident.to_string(), self.gcx),
            args: [arg].into(),
        };

        let const_id = Term {
            functor: ptr_to_name(&item_const.expr, self.gcx),
            args: [].into(),
        };

        let const_ty = Finder::new(self.gcx, self.host, self.bound).type_to_term(&item_const.ty)?;

        let head = term::assoc_const_val_4(
            self.self_ty.clone(),
            self.trait_.clone(),
            const_name.clone(),
            const_id,
            self.gcx,
        );
        let body = term::assoc_const_ty_3(self.trait_.clone(), const_name, const_ty, self.gcx);
        let body = Some(Expr::Term(body));
        self.logic.db.insert_clause(Clause { head, body });

        Ok(())
    }

    /// This method could generate clauses as follows.
    /// * assoc_ty(SelfTy, Trait, AssocTy, AssignTy) :- impl(SelfTy, Trait).
    fn _load_impl_item_type(&mut self, item_ty: &syn::ImplItemType) -> TriResult<(), ()> {
        // 'assoc_ty' should be implied(:-) by generic parameter's bounds.
        debug_assert!(self.bound.bound_terms.is_empty());
        self.bound.push_generics(&item_ty.generics, self.host)?;

        let arg = generics_to_arg(&item_ty.generics, self.gcx);
        let assoc_ty = Term {
            functor: Name::with_intern(&item_ty.ident.to_string(), self.gcx),
            args: [arg].into(),
        };

        // 'assoc_ty' should be implied(:-) by 'impl(SelfTy, Trait)'.
        let impl_ = term::impl_2(self.self_ty.clone(), self.trait_.clone(), self.gcx);
        self.bound.push_bound_term(impl_);

        let assign_ty = Finder::new(self.gcx, self.host, self.bound)
            .with_default_generic_context(&self.logic.default_generics, Some(self.self_ty.clone()))
            .type_to_term(&item_ty.ty)?;

        let head = term::assoc_ty_4(
            self.self_ty.clone(),
            self.trait_.clone(),
            assoc_ty,
            assign_ty,
            self.gcx,
        );
        let body = self.bound.take_bound_expr();
        self.logic.db.insert_clause(Clause { head, body });

        self.bound.pop_generics();
        Ok(())
    }
}

// TODO: do we really need three lifetimes?
/// For associated items in a trait definition block.
struct LoadTraitItemCx<'i, 'o, 'gcx, H> {
    gcx: &'gcx GlobalCx<'gcx>,
    logic: &'o mut ImplLogic<'gcx>,
    host: &'o mut HostWrapper<'i, 'gcx, H>,
    bound: &'o mut Bound<'gcx>,
    /// e.g. trait(Trait).
    trait_: &'o TermIn<'gcx>,
}

impl<'i, 'o, 'gcx, H: Host<'gcx>> LoadTraitItemCx<'i, 'o, 'gcx, H> {
    fn load_trait_items(&mut self, trait_items: &[syn::TraitItem]) -> TriResult<(), ()> {
        for trait_item in trait_items {
            match trait_item {
                syn::TraitItem::Const(c) => self.load_trait_item_const(c)?,
                syn::TraitItem::Fn(f) => self.load_trait_item_fn(f)?,
                syn::TraitItem::Type(_) => {}
                o => todo!("{o:?}"),
            }
        }
        Ok(())
    }

    /// This method could generate clauses as follows.
    /// * assoc_const_ty(Trait, ConstName, ConstTy) :- trait(Trait).
    /// * assoc_const_val(Trait, ConstName, ConstId) :- assoc_const_ty(Trait, ConstName, _).
    fn load_trait_item_const(&mut self, item_const: &syn::TraitItemConst) -> TriResult<(), ()> {
        // Note: `item_const.generics` is experimental at the time of writing, but just keep this
        // for consistency.

        // `assoc_const_ty` should be implied(:-) by generic parameter's bounds.
        debug_assert!(self.bound.bound_terms.is_empty());
        self.bound.push_generics(&item_const.generics, self.host)?;

        // `assoc_const_ty` should be implied(:-) by `trait(Trait)`.
        self.bound.push_bound_term(self.trait_.clone());

        // e.g. Takes `Trait` out of `trait(Trait)`.
        let trait_ = self.trait_.args[0].clone();

        // e.g. CONST<T, U> -> CONST(arg($T, $U))
        let arg = generics_to_arg(&item_const.generics, self.gcx);
        let const_name = Term {
            functor: Name::with_intern(&item_const.ident.to_string(), self.gcx),
            args: [arg].into(),
        };

        let const_ty = Finder::new(self.gcx, self.host, self.bound).type_to_term(&item_const.ty)?;

        // Inserts `assoc_const_ty(..)`.
        let head = term::assoc_const_ty_3(trait_.clone(), const_name.clone(), const_ty, self.gcx);
        let body = self.bound.take_bound_expr();
        self.logic.db.insert_clause(Clause { head, body });

        self.bound.pop_generics();

        // Inserts `assoc_const_val(..) :- assoc_const_ty(..)` if initialization expression exists.
        if let Some((_, expr)) = &item_const.default {
            let const_id = Term {
                functor: ptr_to_name(expr, self.gcx),
                args: [].into(),
            };
            let head =
                term::assoc_const_val_3(trait_.clone(), const_name.clone(), const_id, self.gcx);
            let anonymous_ty = Term {
                functor: var_name("_", self.gcx),
                args: [].into(),
            };
            let body = term::assoc_const_ty_3(trait_, const_name, anonymous_ty, self.gcx);
            let body = Some(Expr::Term(body));
            self.logic.db.insert_clause(Clause { head, body });
        }

        Ok(())
    }

    /// This method could generate clauses as follows.
    /// * assoc_fn(Trait, FnName, sig(..)) :- trait(Trait).
    fn load_trait_item_fn(&mut self, item_fn: &syn::TraitItemFn) -> TriResult<(), ()> {
        let sig = &item_fn.sig;

        // `assoc_fn` should be implied(:-) by generic parameter's bounds.
        debug_assert!(self.bound.bound_terms.is_empty());
        self.bound.push_generics(&sig.generics, self.host)?;

        // `assoc_fn` should be implied(:-) by `trait(Trait)`.
        self.bound.push_bound_term(self.trait_.clone());

        // e.g. Takes `Trait` out of `trait(Trait)`.
        let trait_ = self.trait_.args[0].clone();

        // e.g. foo<T, U> -> foo(arg($T, $U))
        let arg = generics_to_arg(&sig.generics, self.gcx);
        let fn_name = Term {
            functor: Name::with_intern(&sig.ident.to_string(), self.gcx),
            args: [arg].into(),
        };

        let sig =
            Finder::new(self.gcx, self.host, self.bound).trait_fn_sig_to_term(item_fn, &trait_)?;

        let head = term::assoc_fn_3(trait_, fn_name, sig, self.gcx);
        let body = self.bound.take_bound_expr();
        self.logic.db.insert_clause(Clause { head, body });

        self.bound.pop_generics();
        Ok(())
    }
}

// TODO: do we really need three lifetimes?
/// For associated items in an inherent impl block.
struct LoadInherentImplItemCx<'i, 'o, 'gcx, H> {
    gcx: &'gcx GlobalCx<'gcx>,
    logic: &'o mut ImplLogic<'gcx>,
    host: &'o mut HostWrapper<'i, 'gcx, H>,
    bound: &'o mut Bound<'gcx>,
    self_ty: &'o TermIn<'gcx>,
    impl_: &'o TermIn<'gcx>,
}

impl<'i, 'o, 'gcx, H: Host<'gcx>> LoadInherentImplItemCx<'i, 'o, 'gcx, H> {
    fn load_impl_items(&mut self, impl_items: &[syn::ImplItem]) -> TriResult<(), ()> {
        for impl_item in impl_items {
            match impl_item {
                syn::ImplItem::Const(c) => self._load_impl_item_const(c)?,
                syn::ImplItem::Fn(f) => self._load_impl_item_fn(f)?,
                syn::ImplItem::Type(_) => {} // Not supported by the language yet
                _ => {}
            }
        }
        Ok(())
    }

    /// This method could generate clauses as follows.
    /// * inher_const(SelfTy, ConstName, ConstTy, ConstId) :- impl(SelfTy).
    fn _load_impl_item_const(&mut self, item_const: &syn::ImplItemConst) -> TriResult<(), ()> {
        // Note: `item_const.generics` is experimental at the time of writing, but just keep this
        // for consistency.

        // `inher_const` should be implied(:-) by generic parameter's bounds.
        debug_assert!(self.bound.bound_terms.is_empty());
        self.bound.push_generics(&item_const.generics, self.host)?;

        // `inher_const` should be implied(:-) by `impl(SelfTy)`.
        self.bound.push_bound_term(self.impl_.clone());

        // e.g. CONST<T, U> -> CONST(arg($T, $U))
        let arg = generics_to_arg(&item_const.generics, self.gcx);
        let const_name = Term {
            functor: Name::with_intern(&item_const.ident.to_string(), self.gcx),
            args: [arg].into(),
        };

        let const_ty = Finder::new(self.gcx, self.host, self.bound)
            .with_default_generic_context(&self.logic.default_generics, Some(self.self_ty.clone()))
            .type_to_term(&item_const.ty)?;

        // const_id is the address of the init expression.
        let const_id = Term {
            functor: ptr_to_name(&item_const.expr, self.gcx),
            args: [].into(),
        };

        let head = term::inher_const_4(
            self.self_ty.clone(),
            const_name,
            const_ty,
            const_id,
            self.gcx,
        );
        let body = self.bound.take_bound_expr();
        self.logic.db.insert_clause(Clause { head, body });

        self.bound.pop_generics();
        Ok(())
    }

    /// This method could generate clauses as follows.
    /// * inher_fn(SelfTy, FnName, sig(..)) :- impl(SelfTy).
    fn _load_impl_item_fn(&mut self, item_fn: &syn::ImplItemFn) -> TriResult<(), ()> {
        let sig = &item_fn.sig;

        // `inher_fn` should be implied(:-) by generic parameter's bounds.
        debug_assert!(self.bound.bound_terms.is_empty());
        self.bound.push_generics(&sig.generics, self.host)?;

        // `inher_fn` should be implied(:-) by `impl(SelfTy)`.
        self.bound.push_bound_term(self.impl_.clone());

        // e.g. foo<T, U> -> foo(arg($T, $U))
        let arg = generics_to_arg(&sig.generics, self.gcx);
        let fn_name = Term {
            functor: Name::with_intern(&sig.ident.to_string(), self.gcx),
            args: [arg].into(),
        };

        let sig = Finder::new(self.gcx, self.host, self.bound)
            .with_default_generic_context(&self.logic.default_generics, Some(self.self_ty.clone()))
            .inherent_fn_sig_to_term(sig)?;

        let head = term::inher_fn_3(self.self_ty.clone(), fn_name, sig, self.gcx);
        let body = self.bound.take_bound_expr();
        self.logic.db.insert_clause(Clause { head, body });

        self.bound.pop_generics();
        Ok(())
    }
}

// TODO: do we really need three lifetimes?
pub(super) struct Finder<'a, 'b, 'gcx, H> {
    gcx: &'gcx GlobalCx<'gcx>,
    host: &'b mut HostWrapper<'a, 'gcx, H>,
    bound: &'b mut Bound<'gcx>,

    /// Default generic type context.
    default_cx: DefaultGenericCx<'b, 'gcx>,

    /// If occupied, it means we're solving something related to the trait.
    trait_: Option<&'b TermIn<'gcx>>,
    cx: u8,
}

impl<'a, 'b, 'gcx, H: Host<'gcx>> Finder<'a, 'b, 'gcx, H> {
    const CX_UNKNOWN: u8 = 0;
    const CX_TYPE: u8 = 1;

    pub(super) fn new(
        gcx: &'gcx GlobalCx<'gcx>,
        host: &'b mut HostWrapper<'a, 'gcx, H>,
        bound: &'b mut Bound<'gcx>,
    ) -> Self {
        Self {
            gcx,
            host,
            bound,
            default_cx: DefaultGenericCx {
                map: None,
                self_ty: None,
            },
            trait_: None,
            cx: Self::CX_UNKNOWN,
        }
    }

    fn with_default_generic_context(
        &mut self,
        map: &'b DefaultGenericMap<'gcx>,
        self_ty: Option<TermIn<'gcx>>,
    ) -> &mut Self {
        self.default_cx.map = Some(map);
        self.default_cx.self_ty = self_ty;
        self
    }

    /// * trait_ - e.g. Add, not trait(Add).
    fn trait_fn_sig_to_term(
        &mut self,
        trait_item_fn: &syn::TraitItemFn,
        trait_: &'b TermIn<'gcx>,
    ) -> TriResult<TermIn<'gcx>, ()> {
        debug_assert_ne!(trait_.functor.as_ref(), term::FUNCTOR_TRAIT);

        self.trait_ = Some(trait_);
        self._fn_sig_to_term(&trait_item_fn.sig)
    }

    fn inherent_fn_sig_to_term(&mut self, sig: &syn::Signature) -> TriResult<TermIn<'gcx>, ()> {
        debug_assert!(self.trait_.is_none());
        self._fn_sig_to_term(sig)
    }

    pub(super) fn type_to_term(&mut self, ty: &syn::Type) -> TriResult<TermIn<'gcx>, ()> {
        self.cx = Self::CX_TYPE;

        let term = match ty {
            syn::Type::Path(syn::TypePath { qself, path }) => {
                if let Some(qself) = qself {
                    self._qpath_to_term(&qself.ty, path.segments.iter())?
                } else {
                    self.path_to_term(path)?
                }
            }
            syn::Type::Reference(syn::TypeReference {
                mutability, elem, ..
            }) => {
                let mut elem = self.type_to_term(elem)?;
                if mutability.is_some() {
                    elem = term::mut_1(elem, self.gcx);
                }
                term::ref_1(elem, self.gcx)
            }
            syn::Type::Array(syn::TypeArray { elem, len, .. }) => {
                let elem = self.type_to_term(elem)?;
                let len = match self.host.eval_array_len(len)? {
                    ArrayLen::Fixed(n) => Term {
                        functor: Name::with_intern(&n.to_string(), self.gcx),
                        args: [].into(),
                    },
                    ArrayLen::Dynamic => unreachable!(),
                    ArrayLen::Generic => Term {
                        functor: var_name(&len.code(), self.gcx), // TODO: if len is complex expr?
                        args: [].into(),
                    },
                };
                term::array_2(elem, len, self.gcx)
            }
            syn::Type::Slice(syn::TypeSlice { elem, .. }) => {
                let elem = self.type_to_term(elem)?;
                term::array_1(elem, self.gcx)
            }
            syn::Type::Tuple(syn::TypeTuple { elems, .. }) => {
                let elems: Vec<TermIn<'gcx>> = elems
                    .iter()
                    .map(|elem| self.type_to_term(elem))
                    .collect::<TriResult<_, ()>>()?;
                term::tuple_n(elems, self.gcx)
            }
            o => unreachable!("{o:#?}"),
        };
        Ok(term)
    }

    fn path_to_term(&mut self, path: &syn::Path) -> TriResult<TermIn<'gcx>, ()> {
        self.path_segments_to_term(path.segments.iter())
    }

    /// This method is meant to be called for a 'Fn' signature, not 'Closure'.
    fn _fn_sig_to_term(&mut self, sig: &syn::Signature) -> TriResult<TermIn<'gcx>, ()> {
        let mut args = Vec::with_capacity(sig.inputs.len() + 1);

        let output_arg = match &sig.output {
            syn::ReturnType::Default => term::unit_0(self.gcx),
            syn::ReturnType::Type(_, ty) => self.type_to_term(ty)?,
        };
        args.push(output_arg);

        for input in &sig.inputs {
            let input_arg = match input {
                syn::FnArg::Receiver(recv) => {
                    // * If it's inherent method - We know the self type, so it will be a concrete
                    // type.
                    // * If it's trait associated function - We don't know self type, so it will be
                    // something like ref($Self) or Box($Self) or ...
                    let self_ty = self.type_to_term(&recv.ty)?;

                    // If it's trait associated function, $Self needs a bound to impl($Self, Trait).
                    if let Some(trait_) = self.trait_ {
                        let self_var = Term {
                            functor: var_name("Self", self.gcx),
                            args: [].into(),
                        };
                        let impl_ = term::impl_2(self_var, trait_.clone(), self.gcx);
                        self.bound.push_bound_term(impl_);
                    }

                    self_ty
                }
                syn::FnArg::Typed(pat_ty) => self.type_to_term(&pat_ty.ty)?,
            };
            args.push(input_arg);
        }

        let sig_term = term::sig_n(args, self.gcx);
        Ok(sig_term)
    }

    pub(super) fn path_segments_to_term<
        'item,
        I: ExactSizeIterator<Item = &'item syn::PathSegment> + Clone,
    >(
        &mut self,
        mut segments: I,
    ) -> TriResult<TermIn<'gcx>, ()> {
        let num_segments = segments.len();

        // First segment -> canonical path segments
        // e.g. Shl<i8> -> [core, ops, Shl(i8)]
        let first = segments.clone().next().unwrap();

        // If the path is something like `T::Assoc` and we're in a trait, then we turn it into
        // `<T as Trait>::Assoc`.
        if self.bound.contains_var(&first.ident) && self.trait_.is_some() && num_segments == 2 {
            return self._qself_assoc_to_term(first, segments.nth(1).unwrap());
        }

        let mut elems = Vec::new();
        let mut path = String::new();

        if first.ident == "Self" && self.default_cx.get_self_type().is_some() {
            debug_assert!(first.arguments.is_empty());
            let self_ty = self.default_cx.get_self_type().unwrap();
            elems.push(self_ty.clone());
        } else if self.bound.contains_var(&first.ident) {
            debug_assert!(first.arguments.is_empty());
            let var = Term {
                functor: var_name(&first.ident, self.gcx),
                args: [].into(),
            };
            elems.push(var);
        } else {
            path = self.host.ident_to_npath(&first.ident)?;
            let arg = self.path_arguments_to_arg(&path, &first.arguments)?;
            let term = self.host.ident_to_term(&first.ident, arg)?;
            if term.functor.as_ref() == term::FUNCTOR_LIST {
                elems.extend(term.args);
            } else {
                elems.push(term);
            }
        }

        // The rest of segments.
        let term = if elems.len() == 1 && num_segments == 1 {
            elems.pop().unwrap()
        } else {
            for segment in segments.skip(1) {
                let functor = Name::with_intern(&segment.ident.to_string(), self.gcx);

                push_colon_path(&mut path, &segment.ident);
                let arg = self.path_arguments_to_arg(&path, &segment.arguments)?;

                elems.push(Term {
                    functor,
                    args: [arg].into(),
                });
            }
            term::list_n(elems, self.gcx)
        };
        Ok(term)
    }

    /// e.g.
    /// If we have Trait, T::Assoc -> <T as Trait>::Assoc -> Term
    /// If we don't have Trait, T::Assoc -> ConcreteTy::Assoc
    fn _qself_assoc_to_term(
        &mut self,
        qself_segment: &syn::PathSegment,
        assoc_segment: &syn::PathSegment,
    ) -> TriResult<TermIn<'gcx>, ()> {
        if let Some(trait_) = self.trait_ {
            debug_assert!(qself_segment.arguments.is_empty());

            let self_ty = Term {
                functor: var_name(&qself_segment.ident, self.gcx),
                args: [].into(),
            };

            let trait_ = trait_.clone();

            let arg = self.path_arguments_to_arg("", &assoc_segment.arguments)?;
            let assoc_item = Term {
                functor: Name::with_intern(&assoc_segment.ident.to_string(), self.gcx),
                args: [arg].into(),
            };

            let var = self._qpath_to_var(self_ty, trait_, assoc_item);
            Ok(var)
        } else {
            // We're not inside a trait...
            todo!()
        }
    }

    /// * Input - <qself_ty as trait_path>::trait_path
    /// * Output - A variable($X) with bounds
    fn _qpath_to_term<'item, I: ExactSizeIterator<Item = &'item syn::PathSegment> + Clone>(
        &mut self,
        qself_ty: &syn::Type,
        trait_path: I,
    ) -> TriResult<TermIn<'gcx>, ()> {
        let num_segments = trait_path.len();
        debug_assert!(num_segments > 1);

        let self_ty = self.type_to_term(qself_ty)?;

        // Replaces the self type of the defualt generic context.
        self.default_cx.replace_self_type(self_ty.clone());

        let trait_ = self.path_segments_to_term(trait_path.clone().take(num_segments - 1))?;

        let last_segment = trait_path.clone().last().unwrap();

        let mut iter = trait_path.clone();
        let mut path = self.host.ident_to_npath(&iter.next().unwrap().ident)?;
        for segment in iter {
            push_colon_path(&mut path, &segment.ident);
        }

        let arg = self.path_arguments_to_arg(&path, &last_segment.arguments)?;
        let assoc_item = Term {
            functor: Name::with_intern(&last_segment.ident.to_string(), self.gcx),
            args: [arg].into(),
        };

        let var = self._qpath_to_var(self_ty, trait_, assoc_item);
        Ok(var)
    }

    /// * Input - <self_ty as trait_>::assoc_item
    /// * Output - A variable($X) with bounds shown below
    /// * Bound - assoc_ty(self_ty, trait_, assoc_item, $X)
    /// * Bound - impl(self_ty, trait_)
    fn _qpath_to_var(
        &mut self,
        self_ty: TermIn<'gcx>,
        trait_: TermIn<'gcx>,
        assoc_item: TermIn<'gcx>,
    ) -> TermIn<'gcx> {
        let auto_var = self.bound.next_auto_var();

        let trait_assoc = if self.cx == Self::CX_TYPE {
            // 'assoc_ty' should be implied(:-) by 'impl(SelfTy, Trait)'.
            let impl_ = term::impl_2(self_ty.clone(), trait_.clone(), self.gcx);
            self.bound.push_bound_term(impl_);

            term::assoc_ty_4(self_ty, trait_, assoc_item, auto_var.clone(), self.gcx)
        } else {
            todo!("no related term yet")
        };

        self.bound.push_bound_term(trait_assoc);

        auto_var
    }

    /// * path - e.g. `a::b::C`
    /// * args - e.g. `<X, Y>` in `a::b::C<X, Y>`
    /// * Output - arg(..)
    pub(super) fn path_arguments_to_arg(
        &mut self,
        path: &str,
        args: &syn::PathArguments,
    ) -> TriResult<TermIn<'gcx>, ()> {
        let args = match args {
            syn::PathArguments::None => {
                if let Some(defaults) = self.default_cx.get_default_types(path) {
                    defaults.map(|(_i, term)| term.clone()).collect()
                } else {
                    [].into()
                }
            }
            syn::PathArguments::AngleBracketed(syn::AngleBracketedGenericArguments {
                args,
                ..
            }) => {
                let total = self.default_cx.get_generics_len(path).unwrap_or(args.len());

                let mut buf = Vec::with_capacity(total);

                for arg in args {
                    let arg = match arg {
                        syn::GenericArgument::Type(ty) => self.type_to_term(ty)?,
                        _ => todo!(),
                    };
                    buf.push(arg);
                }
                if let Some(defaults) = self.default_cx.get_default_types(path) {
                    for (j, default) in defaults {
                        if j >= buf.len() {
                            buf.push(default.clone());
                        }
                    }
                }

                buf
            }
            syn::PathArguments::Parenthesized(_) => {
                todo!()
            }
        };
        Ok(term::arg_n(args, self.gcx))
    }
}

pub(super) struct Bound<'gcx> {
    gcx: &'gcx GlobalCx<'gcx>,

    /// Bound terms that will appear in a clause's body.
    ///
    /// e.g. some head :- impl(Foo, Clone), impl(Foo, Sized).
    bound_terms: Vec<TermIn<'gcx>>,

    /// An integer for auto-generated variables.
    next_auto_var: u32,

    /// Variable symbols.
    var_symbols: Vec<String>,
}

impl<'gcx> Bound<'gcx> {
    pub(super) fn new(gcx: &'gcx GlobalCx<'gcx>) -> Self {
        // `Self` is a default variable symbol.
        let var_symbols = vec!["Self".to_owned()];

        Self {
            gcx,
            bound_terms: Vec::new(),
            next_auto_var: 0,
            var_symbols,
        }
    }

    fn push_bound_term(&mut self, bound_term: TermIn<'gcx>) {
        // Rejects duplicate bound.
        if self.bound_terms.iter().all(|exist| exist != &bound_term) {
            self.bound_terms.push(bound_term);
        }
    }

    fn take_bound_expr(&mut self) -> Option<ExprIn<'gcx>> {
        // Removes redundant conditions. See detailed comments in the loop below.
        // - Preventing them from being inserted is quite complex. I think this is a little bit
        // inefficient, but looks better.
        let mut redundant = Vec::new();

        for (lt, (ri, rt)) in self
            .bound_terms
            .iter()
            .flat_map(|l| self.bound_terms.iter().enumerate().map(move |r| (l, r)))
        {
            // Case   : head :- assoc_ty(SelfTy, Trait), impl(SelfTy, Trait)
            // Reason : `assoc_ty` as a bound doesn't have to have `impl` bound. `assoc_ty` as the
            // head of a clause will have the `impl` bound.
            if lt.functor.as_ref() == term::FUNCTOR_ASSOC_TY
                && rt.functor.as_ref() == term::FUNCTOR_IMPL
                && lt.args[0..2] == rt.args[0..2]
            {
                redundant.push(ri);
            }
        }

        for r in redundant {
            self.bound_terms.remove(r);
        }

        match self.bound_terms.len() as u32 {
            0 | 1 => {
                let term = self.bound_terms.pop()?;
                Some(Expr::Term(term))
            }
            2.. => {
                let args = self.bound_terms.drain(..).map(Expr::Term).collect();
                Some(Expr::And(args))
            }
        }
    }

    /// Don't forget to call [`Self::pop_generics`] after calling this method.
    fn push_generics<H: Host<'gcx>>(
        &mut self,
        generics: &syn::Generics,
        host: &mut HostWrapper<'_, 'gcx, H>,
    ) -> TriResult<(), ()> {
        // Stores idents of generic parameters. When we meet those idents later, we will make them
        // variables.
        self.var_symbols.push(String::new()); // Empty string is a seperator
        for param in &generics.params {
            match param {
                syn::GenericParam::Type(ty_param) => {
                    self.var_symbols.push(ty_param.ident.to_string());
                }
                syn::GenericParam::Const(_) | syn::GenericParam::Lifetime(_) => {
                    // No terms related to this yet
                }
            }
        }

        // Generic parameters that are not bound to ?Sized.
        let mut sized: Vec<TermIn<'gcx>> = Vec::new();

        // Appends generic bound.
        //
        // NOTE: For now, we're considering type parameters only.
        for type_param in generics.type_params() {
            let bounded_term = Term {
                functor: var_name(&type_param.ident, self.gcx),
                args: [].into(),
            };
            sized.push(bounded_term.clone());

            for param_bound in &type_param.bounds {
                let syn::TypeParamBound::Trait(trait_bound) = param_bound else {
                    // For now, consider trait bounds only.
                    continue;
                };

                push_trait_bound(self, host, &mut sized, &bounded_term, trait_bound)?;
            }
        }

        if let Some(where_clause) = &generics.where_clause {
            for predicate in &where_clause.predicates {
                match predicate {
                    syn::WherePredicate::Type(ty) => {
                        let bounded_term =
                            Finder::new(self.gcx, host, self).type_to_term(&ty.bounded_ty)?;
                        for bound in &ty.bounds {
                            match bound {
                                syn::TypeParamBound::Trait(trait_bound) => {
                                    push_trait_bound(
                                        self,
                                        host,
                                        &mut sized,
                                        &bounded_term,
                                        trait_bound,
                                    )?;
                                }
                                _ => todo!(),
                            }
                        }
                    }
                    _ => todo!(),
                }
            }
        }

        for bounded_term in sized {
            // e.g. $G implements Sized.
            let trait_ = Term {
                functor: Name::with_intern("Sized", self.gcx),
                args: [].into(),
            };
            let trait_impl = term::impl_2(bounded_term, trait_, self.gcx);
            self.push_bound_term(trait_impl)
        }

        return Ok(());

        // === Internal helper functions ===

        fn push_trait_bound<'gcx, H: Host<'gcx>>(
            this: &mut Bound<'gcx>,
            host: &mut HostWrapper<'_, 'gcx, H>,
            sized: &mut Vec<TermIn<'gcx>>,
            bounded_term: &TermIn<'gcx>,
            trait_bound: &syn::TraitBound,
        ) -> TriResult<(), ()> {
            let trait_ = Finder::new(this.gcx, host, this).path_to_term(&trait_bound.path)?;

            if matches!(trait_bound.modifier, syn::TraitBoundModifier::Maybe(_)) {
                // Bounds like `?Sized` will be added later at once.
                if trait_.functor.as_ref() == "Sized" {
                    if let Some(i) = sized
                        .iter()
                        .enumerate()
                        .find_map(|(i, term)| (term == bounded_term).then_some(i))
                    {
                        sized.swap_remove(i);
                    }
                }
                return Ok(());
            }

            // e.g. `bounded_term` implements `trait_`.
            let trait_impl = term::impl_2(bounded_term.clone(), trait_, this.gcx);
            this.push_bound_term(trait_impl);
            Ok(())
        }
    }

    fn pop_generics(&mut self) {
        loop {
            match self.var_symbols.last() {
                // Empty string is a seperator.
                Some(ident) if ident.is_empty() => {
                    self.var_symbols.pop();
                    break;
                }
                Some(_) => {
                    self.var_symbols.pop();
                }
                None => break,
            }
        }
    }

    fn contains_var<T: PartialEq<str>>(&self, ident: &T) -> bool {
        self.var_symbols
            .iter()
            .any(|generic| ident == generic.as_str())
    }

    fn next_auto_var(&mut self) -> TermIn<'gcx> {
        let functor = if self.next_auto_var < 26 {
            format!(
                "{}{}",
                AUTO_VAR_PREFIX,
                (self.next_auto_var as u8 + b'A') as char
            )
        } else {
            format!("{}{}", AUTO_VAR_PREFIX, self.next_auto_var - 26)
        };

        self.next_auto_var += 1;

        Term {
            functor: Name::with_intern(&functor, self.gcx),
            args: [].into(),
        }
    }
}

/// * Key - The path of a struct, trait, enum or type alias.
/// * Value - Default generic types such as [None, Some(a::St), Some(Self)]
type DefaultGenericMap<'gcx> = Map<NameIn<'gcx>, BoxedSlice<Option<TermIn<'gcx>>>>;

struct DefaultGenericCx<'a, 'gcx> {
    /// Optional default generic map.
    map: Option<&'a DefaultGenericMap<'gcx>>,

    /// Optional self type for switching `Self` in the default generics.
    self_ty: Option<TermIn<'gcx>>,
}

impl<'a, 'gcx> DefaultGenericCx<'a, 'gcx> {
    fn replace_self_type(&mut self, self_ty: TermIn<'gcx>) {
        self.self_ty = Some(self_ty);
    }

    fn get_self_type(&self) -> Option<&TermIn<'gcx>> {
        self.self_ty.as_ref()
    }

    /// e.g. [None, Soem(A), Some(Self)] -> 3
    fn get_generics_len<Q>(&self, name: &Q) -> Option<usize>
    where
        NameIn<'gcx>: borrow::Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.map?.get(name).map(|defaults| defaults.len())
    }

    /// e.g. [None, Soem(A), Some(Self)] -> A, self_ty
    fn get_default_types<Q>(
        &self,
        name: &Q,
    ) -> Option<impl Iterator<Item = (usize, &TermIn<'gcx>)> + Clone>
    where
        NameIn<'gcx>: borrow::Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        let defaults = self.map?.get(name)?;
        let iter = defaults.iter().enumerate().filter_map(|(i, default)| {
            default.as_ref().map(|term| {
                // Switches `Self` or `$Self` to `self_ty`
                if term.functor.as_ref() == "Self"
                    || term.is_variable() && &term.functor[1..] == "Self"
                {
                    let self_ty = self.self_ty.as_ref().unwrap_or_else(|| {
                        panic!("self_ty is not given");
                    });
                    (i, self_ty)
                } else {
                    (i, term)
                }
            })
        });
        Some(iter)
    }
}

#[derive(Debug)]
pub(crate) struct DatabaseWrapper<'gcx> {
    db: Database<'gcx, GlobalCx<'gcx>>,

    /// Added well known 'Sized' types such as 'i32', 'u32', 'arr', and so on.
    added_sized_known: Set<PredicateIn<'gcx>>,

    gcx: &'gcx GlobalCx<'gcx>,
}

impl<'gcx> DatabaseWrapper<'gcx> {
    fn new(gcx: &'gcx GlobalCx<'gcx>) -> Self {
        Self {
            db: Database::with_interner(gcx),
            added_sized_known: Set::default(),
            gcx,
        }
    }

    fn insert_clause(&mut self, mut clause: ClauseIn<'gcx>) {
        // impl(X, Sized) plays kind of an important role in our logic. If we ask logic engine show
        // all possible types with a certain constraint, the engine will give us types that
        // appeared in impl(Ty, Sized).
        self._insert_impl_sized_for_known_on_demand(&clause);

        // If the clause contains singleton variables(not being referred to), then puts prefix '_'
        // in front of them.
        Self::_add_prefix_for_singleton_vars(&mut clause, self.gcx);

        self.db.insert_clause(clause)
    }

    fn commit(&mut self) {
        self.db.commit();
    }

    fn query(&mut self, expr: ExprIn<'gcx>) -> ProveCx<'_, 'gcx, GlobalCx<'gcx>> {
        self.db.query(expr)
    }

    pub(crate) fn to_prolog(&self) -> String {
        // Removes custom prefix letters. `VAR_PREFIX`, on the other hand, would be removed by
        // logic_eval crate.
        self.db.to_prolog(|name| {
            for custom_letter in CUSTOM_PREFIX_LETTERS {
                if let Some(stripped) = name.strip_prefix(custom_letter) {
                    return stripped;
                }
            }
            name
        })
    }

    fn _insert_impl_sized_for_known_on_demand(&mut self, clause: &ClauseIn<'gcx>) {
        term_helper(self, &clause.head);
        if let Some(body) = &clause.body {
            expr_helper(self, body);
        }

        // === Internal helper functions ===

        fn expr_helper<'gcx>(this: &mut DatabaseWrapper<'gcx>, expr: &ExprIn<'gcx>) {
            match expr {
                Expr::Term(term) => term_helper(this, term),
                Expr::Not(inner) => expr_helper(this, inner),
                Expr::And(args) | Expr::Or(args) => {
                    for arg in args {
                        expr_helper(this, arg);
                    }
                }
            }
        }

        fn term_helper<'gcx>(this: &mut DatabaseWrapper<'gcx>, term: &TermIn<'gcx>) {
            #[rustfmt::skip]
            const SORTED_KNOWN: [&str; 19] = [
                term::FUNCTOR_ARRAY,
                term::FUNCTOR_REF,
                term::FUNCTOR_TUPLE,
                "bool", "char",
                "f32", "f64",
                "i128", "i16", "i32", "i64", "i8", "isize",
                "u128", "u16", "u32", "u64", "u8", "usize",
            ];
            debug_assert!(SORTED_KNOWN.windows(2).all(|w| w[0] <= w[1]));

            if SORTED_KNOWN.binary_search(&term.functor.as_ref()).is_err() {
                for arg in &term.args {
                    term_helper(this, arg);
                }
                return;
            }

            if !this.added_sized_known.insert(term.predicate()) {
                return;
            }

            for arg in &term.args {
                term_helper(this, arg);
            }

            let sized = Term {
                functor: Name::with_intern("Sized", this.gcx),
                args: [].into(),
            };

            // Inserts an additional clause shown below.
            // impl(array($_E, $_L), Sized).
            if term.functor.as_ref() == term::FUNCTOR_ARRAY {
                let var_elem = Term {
                    functor: var_name("_E", this.gcx),
                    args: [].into(),
                };
                let var_len = Term {
                    functor: var_name("_L", this.gcx),
                    args: [].into(),
                };
                let arr = term::array_2(var_elem, var_len, this.gcx);
                let head = term::impl_2(arr, sized, this.gcx);
                this.db.insert_clause(Clause { head, body: None });
            }
            // Inserts an additional clause shown below.
            // impl(ref($_), Sized).
            else if term.functor.as_ref() == term::FUNCTOR_REF {
                let var = Term {
                    functor: var_name("_", this.gcx), // Anonymous variable in prolog
                    args: [].into(),
                };
                let ref_ = term::ref_1(var, this.gcx);
                let head = term::impl_2(ref_, sized, this.gcx);
                this.db.insert_clause(Clause { head, body: None });
            }
            // Inserts additional clauses as follows.
            // e.g. impl(tuple($A, $B), Sized) :- impl($A, Sized), impl($B, Sized).
            // TODO: May cause neverend unifying in `logic-eval`.
            else if term.functor.as_ref() == term::FUNCTOR_TUPLE {
                let arity = term.args.len();
                let mut elems = Vec::with_capacity(arity);
                let mut bounds = Vec::with_capacity(arity);

                for i in 0..term.args.len() {
                    let functor = if i < 26 {
                        var_name(&((i as u8 + b'A') as char), this.gcx)
                    } else {
                        var_name(&(i - 26), this.gcx)
                    };
                    let var = Term {
                        functor,
                        args: [].into(),
                    };
                    let bound = term::impl_2(var.clone(), sized.clone(), this.gcx);
                    elems.push(var);
                    bounds.push(Expr::Term(bound));
                }

                let tuple = term::tuple_n(elems, this.gcx);
                let head = term::impl_2(tuple, sized, this.gcx);
                this.db.insert_clause(Clause {
                    head,
                    body: Some(Expr::And(bounds)),
                });
            }
            // Inserts additional clauses as follows.
            // e.g. impl(int(i32), Sized).
            else {
                debug_assert!(term.args.is_empty());

                // e.g. i32 -> int(i32)
                let term = try_make_int_or_float_term(&term.functor, this.gcx)
                    .unwrap_or_else(|| term.clone());

                let head = term::impl_2(term, sized, this.gcx);
                this.db.insert_clause(Clause { head, body: None });
            }
        }
    }

    /// e.g. foo($X, $Y) :- bar($X) => foo($X, $_Y) :- bar($X)
    fn _add_prefix_for_singleton_vars(clause: &mut ClauseIn<'gcx>, gcx: &'gcx GlobalCx<'gcx>) {
        let ptr_term = &mut clause.head as *mut TermIn<'gcx>;
        let ptr_clause = (clause as *mut ClauseIn<'gcx>).cast_const();
        fix_term(ptr_term, ptr_clause, gcx);

        if let Some(body) = &mut clause.body {
            let ptr_expr = body as *mut ExprIn<'gcx>;
            let ptr_clause = (clause as *mut ClauseIn<'gcx>).cast_const();
            fix_expr(ptr_expr, ptr_clause, gcx);
        }

        // === Internal helper functions ===

        fn fix_term<'gcx>(
            term: *mut TermIn<'gcx>,
            clause: *const ClauseIn<'gcx>,
            gcx: &'gcx GlobalCx<'gcx>,
        ) {
            let is_singleton = unsafe {
                let term = &*term;
                let clause = &*clause;
                is_singleton_var(term, clause)
            };

            if is_singleton {
                unsafe {
                    let term = &mut *term;
                    if term.functor.starts_with(AUTO_VAR_PREFIX) {
                        let functor = format!(
                            "{}_{}",
                            AUTO_VAR_PREFIX,
                            &term.functor[AUTO_VAR_PREFIX.len()..]
                        );
                        term.functor = Name::with_intern(&functor, gcx);
                    } else {
                        let functor =
                            format!("{}_{}", VAR_PREFIX, &term.functor[VAR_PREFIX.len_utf8()..]);
                        term.functor = Name::with_intern(&functor, gcx);
                    }
                }
            } else {
                let (args, num_args) = unsafe {
                    let term = &mut *term;
                    (term.args.as_mut_ptr(), term.args.len())
                };

                for i in 0..num_args {
                    unsafe { fix_term(args.add(i), clause, gcx) };
                }
            }
        }

        fn fix_expr<'gcx>(
            expr: *mut ExprIn<'gcx>,
            clause: *const ClauseIn<'gcx>,
            gcx: &'gcx GlobalCx<'gcx>,
        ) {
            unsafe {
                let expr = &mut *expr;
                match expr {
                    Expr::Term(term) => {
                        fix_term(term as *mut _, clause, gcx);
                    }
                    Expr::Not(inner) => {
                        let inner = &mut **inner;
                        fix_expr(inner as *mut _, clause, gcx);
                    }
                    Expr::And(args) | Expr::Or(args) => {
                        let num_args = args.len();
                        let args = args.as_mut_ptr();
                        for i in 0..num_args {
                            fix_expr(args.add(i), clause, gcx);
                        }
                    }
                }
            }
        }

        /// Singleton variable here means a variable that appears only once in the clause. But,
        /// this method returns false even if it's singleton when it starts with '_' because we
        /// don't have things to do on that.
        fn is_singleton_var<'gcx>(lhs: &TermIn<'gcx>, clause: &ClauseIn<'gcx>) -> bool {
            if !lhs.is_variable() {
                return false;
            }

            let off = if lhs.functor.starts_with(AUTO_VAR_PREFIX) {
                AUTO_VAR_PREFIX.len()
            } else {
                VAR_PREFIX.len_utf8()
            };
            if lhs.functor[off..].starts_with('_') {
                return false;
            }

            fn count_term<'gcx>(var: &TermIn<'gcx>, term: &TermIn<'gcx>, same_cnt: &mut u32) {
                if var == term {
                    *same_cnt += 1;
                } else {
                    for arg in &term.args {
                        count_term(var, arg, same_cnt);
                    }
                }
            }

            fn count_expr<'gcx>(var: &TermIn<'gcx>, expr: &ExprIn<'gcx>, same_cnt: &mut u32) {
                match expr {
                    Expr::Term(term) => count_term(var, term, same_cnt),
                    Expr::Not(inner) => count_expr(var, inner, same_cnt),
                    Expr::And(args) | Expr::Or(args) => {
                        for arg in args {
                            count_expr(var, arg, same_cnt);
                        }
                    }
                }
            }

            let mut same_cnt = 0;

            count_term(lhs, &clause.head, &mut same_cnt);
            if let Some(body) = &clause.body {
                count_expr(lhs, body, &mut same_cnt);
            }

            same_cnt == 1
        }
    }
}

impl<'gcx> ops::Deref for DatabaseWrapper<'gcx> {
    type Target = Database<'gcx, GlobalCx<'gcx>>;

    fn deref(&self) -> &Self::Target {
        &self.db
    }
}

/// * generics - e.g. <T, U>
/// * Output   - e.g. arg($T, $U)
fn generics_to_arg<'gcx>(generics: &syn::Generics, gcx: &'gcx GlobalCx<'gcx>) -> TermIn<'gcx> {
    let args = generics
        .params
        .iter()
        .map(|param| match param {
            syn::GenericParam::Type(ty_param) => Term {
                functor: var_name(&ty_param.ident, gcx),
                args: [].into(),
            },
            o => todo!("{o:#?}"),
        })
        .collect();
    term::arg_n(args, gcx)
}

#[cfg(test)]
#[rustfmt::skip]
pub(crate) mod tests {
    use super::{
        ImplLogic,
    };
    use crate::{
        syntax::file::SmFile,
        semantic::{
            logic::test_help::TestLogicHost,
            entry::GlobalCx,
        },
    };
    use pretty_assertions::assert_eq;

    #[test]
    fn test_logic_construction() {
        test_impl();
        test_inherent_fn();
        test_trait_impl();
        test_trait_impl_assoc_ty();
        test_trait_assoc_fn();
        test_trait_complex_assoc_fn();
        test_sized_for_struct();
        test_default_generic_type();
        test_various_self();
    }

    fn test_impl() {
        let clauses = load(r"
            impl<T: A + B> S<T> {}
        ");
        let expected = remove_whitespace(&[
            "#impl(S(#arg($T))) :- 
                #impl($T, A(#arg)), 
                #impl($T, B(#arg)), 
                #impl($T, Sized)."
        ]);
        assert_eq!(clauses, expected);
    }

    fn test_inherent_fn() {
        let clauses = load(r"
            impl<T: A> S<T> {
                fn f0() {}
                fn f1<U: B>() {}
                fn f2(u: U) -> T {}
            }
        ");
        let expected = remove_whitespace(&[
            "#impl(S(#arg($T))) :- 
                #impl($T, A(#arg)),
                #impl($T, Sized).",
            "#inher_fn(S(#arg($T)), f0(#arg), #sig(#unit)) :-
                #impl(S(#arg($T))).",
            "#inher_fn(S(#arg($T)), f1(#arg($U)), #sig(#unit)) :-
                #impl($U, B(#arg)),
                #impl($U, Sized),
                #impl(S(#arg($T))).",
            "#inher_fn(S(#arg($T)), f2(#arg), #sig($T, U(#arg))) :-
                #impl(S(#arg($T))).",
        ]);
        assert_eq!(clauses, expected);
    }

    fn test_trait_impl() {
        let clauses = load(r"
            impl<T, U: ?Sized> Trait<T> for S<U> {}
        ");
        let expected = remove_whitespace(&[
            "#impl(S(#arg($_U)), Trait(#arg($T))) :- 
                #impl($T, Sized)."
        ]);
        assert_eq!(clauses, expected);
    }

    fn test_trait_impl_assoc_ty() {
        let clauses = load(r"
            impl Trait for S {
                type AssocTy = T;
            }
        ");
        let expected = remove_whitespace(&[
            "#impl(S(#arg), Trait(#arg)).",
            "#assoc_ty(S(#arg), Trait(#arg), AssocTy(#arg), T(#arg)) :- 
                #impl(S(#arg), Trait(#arg)).",
        ]);
        assert_eq!(clauses, expected);
    }

    fn test_trait_assoc_fn() {
        let clauses = load(r"
            trait Trait<T: A> {
                fn f0();
                fn f1<U: B>();
                fn f2(u: U) -> T;
            }
        ");
        let expected = remove_whitespace(&[
            "#trait(Trait(#arg($T))) :- 
                #impl($T, A(#arg)), 
                #impl($T, Sized).",
            "#assoc_fn(Trait(#arg($T)), f0(#arg), #sig(#unit)) :- 
                #trait(Trait(#arg($T))).",
            "#assoc_fn(Trait(#arg($T)), f1(#arg($U)), #sig(#unit)) :- 
                #impl($U, B(#arg)), 
                #impl($U, Sized), 
                #trait(Trait(#arg($T))).",
            "#assoc_fn(Trait(#arg($T)), f2(#arg), #sig($T, U(#arg))) :- 
                #trait(Trait(#arg($T))).",
        ]);
        assert_eq!(clauses, expected);
    }

    fn test_trait_complex_assoc_fn() {
        let clauses = load(r"
            trait MyShl<Rhs = Self> {
                type Output;
                fn shl(self, rhs: Rhs) -> Self::Output;
            }
        ");
        let expected = remove_whitespace(&[
            "#trait(MyShl(#arg($Rhs))) :- 
                #impl($Rhs, Sized).",
            "#assoc_fn(MyShl(#arg($Rhs)), shl(#arg), #sig($#A, $Self, $Rhs)) :- 
                #trait(MyShl(#arg($Rhs))), 
                #assoc_ty($Self, MyShl(#arg($Rhs)), Output(#arg), $#A).",
        ]);
        assert_eq!(clauses, expected);
    }

    fn test_sized_for_struct() {
        // impl(X, Sized) could automatically be generated on demand.

        // Array (Sized)
        let clauses = load(r"
            struct A { a: [i32; 1] }
        ");
        let expected = remove_whitespace(&[
            "#impl(#int(i32), Sized).",
            "#impl(#array($_E, $_L), Sized).",
            "#impl(A(#arg), Sized) :- #impl(#array(#int(i32), 1), Sized).",
        ]);
        assert_eq!(clauses, expected);

        // Tuple (Sized)
        let clauses = load(r"
            struct A { a: (i32, f32) }
        ");
        let expected = remove_whitespace(&[
            "#impl(#int(i32), Sized).",
            "#impl(#float(f32), Sized).",
            "#impl(#tuple($A, $B), Sized) :- #impl($A, Sized), #impl($B, Sized).",
            "#impl(A(#arg), Sized) :- #impl(#tuple(#int(i32), #float(f32)), Sized).",
        ]);
        assert_eq!(clauses, expected);

        // User-defined structs (Sized)
        let clauses = load(r"
            struct A { a: i32 }
            struct B { a1: A, a2: A, b: f32 }
        ");
        let expected = remove_whitespace(&[
            "#impl(#int(i32), Sized).",
            "#impl(A(#arg), Sized) :- #impl(#int(i32), Sized).",
            "#impl(#float(f32), Sized).",
            "#impl(B(#arg), Sized) :- #impl(A(#arg), Sized), #impl(#float(f32), Sized).",
        ]);
        assert_eq!(clauses, expected);

        // Reference
        let clauses = load(r"
            struct A { a: &i32 }
            struct B { a: &mut i32 }
        ");
        let expected = remove_whitespace(&[
            "#impl(#int(i32), Sized).",
            "#impl(#ref($_), Sized).",
            "#impl(A(#arg), Sized) :- #impl(#ref(#int(i32)), Sized).",
            "#impl(B(#arg), Sized) :- #impl(#ref(#mut(#int(i32))), Sized).",
        ]);
        assert_eq!(clauses, expected);

        // Slice (Unsized)
        let clauses = load(r"
            struct A { a: [i32] }
        ");
        let expected = remove_whitespace(&[
            "#impl(#int(i32), Sized).",
            "#impl(#array($_E, $_L), Sized).",
            "#impl(A(#arg), Sized) :- #impl(#array(#int(i32), #dyn), Sized).",
        ]);
        assert_eq!(clauses, expected);
    }

    fn test_default_generic_type() {
        // Default generic types for a trait
        let clauses = load(r"
            trait Trait<A = Self, B = X> {}
            impl Trait for X {}
            impl Trait<W> for Y {}
            struct W;
            struct X;
            struct Y;
        ");
        let expected = remove_whitespace(&[
            "#trait(Trait(#arg($A, $B))) :- #impl($A, Sized), #impl($B, Sized).",
            "#impl(X(#arg), Trait(#arg(X(#arg), X(#arg)))).",
            "#impl(Y(#arg), Trait(#arg(W(#arg), X(#arg)))).",
            "#impl(W(#arg), Sized).",
            "#impl(X(#arg), Sized).",
            "#impl(Y(#arg), Sized).",
        ]);
        assert_eq!(clauses, expected);

        // Default generic types for a struct
        let clauses = load(r"
            struct St<A = X, B = Y> { a: A, b: B }
            impl St {}
            impl St<W> {}
            struct W;
            struct X;
            struct Y;
        ");
        let expected = remove_whitespace(&[
            "#impl(St(#arg($A, $B)) ,Sized) :- #impl($A, Sized), #impl($B, Sized).",
            "#impl(W(#arg), Sized).",
            "#impl(X(#arg), Sized).",
            "#impl(Y(#arg), Sized).",
            "#impl(St(#arg(X(#arg), Y(#arg)))).",
            "#impl(St(#arg(W(#arg), Y(#arg)))).",
        ]);
        assert_eq!(clauses, expected);

        // Default generic types in a qself position.
        let clauses = load(r"
            trait Trait<A = X> {
                type Assoc;
            }
            impl Trait for X {
                type Assoc = X;
            }
            impl Trait for Y {
                type Assoc = <X as Trait>::Assoc;
            }
            struct X;
            struct Y;
        ");
        let expected = remove_whitespace(&[
            "#trait(Trait(#arg($A))) :- #impl($A, Sized).",
            "#impl(X(#arg), Trait(#arg(X(#arg)))).",
            "#impl(Y(#arg), Trait(#arg(X(#arg)))).",
            "#impl(X(#arg), Sized).",
            "#impl(Y(#arg), Sized).",
            "#assoc_ty(X(#arg), Trait(#arg(X(#arg))), Assoc(#arg), X(#arg)) :-
                #impl(X(#arg), Trait(#arg(X(#arg)))).",
            "#assoc_ty(Y(#arg), Trait(#arg(X(#arg))), Assoc(#arg), $#A) :-
                #impl(Y(#arg), Trait(#arg(X(#arg)))),
                #assoc_ty(X(#arg), Trait(#arg(X(#arg))), Assoc(#arg), $#A).",
        ]);
        assert_eq!(clauses, expected);
    }

    fn test_various_self() {
        // Self in a generic parameter.
        let clauses = load(r"
            trait Trait<A = Self> {}
            impl Trait for X {}
            struct X;
        ");
        let expected = remove_whitespace(&[
            "#trait(Trait(#arg($A))) :- #impl($A, Sized).",
            "#impl(X(#arg), Trait(#arg(X(#arg)))).",
            "#impl(X(#arg), Sized).",
        ]);
        assert_eq!(clauses, expected);

        // Self in a associated type.
        let clauses = load(r"
            trait Trait {
                type Assoc;
            }
            impl Trait for X {
                type Assoc = Self;
            }
            struct X;
        ");
        let expected = remove_whitespace(&[
            "#trait(Trait(#arg)).",
            "#impl(X(#arg), Trait(#arg)).",
            "#impl(X(#arg), Sized).",
            "#assoc_ty(X(#arg), Trait(#arg), Assoc(#arg), X(#arg)) :-
                #impl(X(#arg), Trait(#arg)).",
        ]);
        assert_eq!(clauses, expected);

        // Self in inherent methods.
        let clauses = load(r"
            struct X;
            impl X {
                fn f0(self) {}
                fn f1(&self) {}
                fn f2(&mut self) {}
                fn f3(self: Box<Self>) {}
            }
        ");
        let expected = remove_whitespace(&[
            "#impl(X(#arg), Sized).",
            "#impl(#ref($_), Sized).",
            "#impl(X(#arg)).",
            "#inher_fn(X(#arg), f0(#arg), #sig(#unit, X(#arg))) :-
                #impl(X(#arg)).",
            "#inher_fn(X(#arg), f1(#arg), #sig(#unit, #ref(X(#arg)))) :-
                #impl(X(#arg)).",
            "#inher_fn(X(#arg), f2(#arg), #sig(#unit, #ref(#mut(X(#arg))))) :-
                #impl(X(#arg)).",
            "#inher_fn(X(#arg), f3(#arg), #sig(#unit, Box(#arg(X(#arg))))) :-
                #impl(X(#arg)).",
        ]);
        assert_eq!(clauses, expected);
    }

    fn load(code: &str) -> Vec<String> {
        let file = "test";
        let gcx = GlobalCx::default();
        let mut logic = ImplLogic::new(&gcx);
        let mut host = TestLogicHost::new();
        let file = SmFile::new(file.into(), code).unwrap();
        logic.load_file(&mut host, &file.file).unwrap();
        logic
            .db
            .clauses()
            .map(|clause| {
                let mut clause = clause.to_string();
                clause.retain(|c| !c.is_whitespace());
                clause
            })
            .collect()
    }

    fn remove_whitespace(clauses: &[&str]) -> Vec<String> {
        clauses
            .iter()
            .map(|clause| clause.chars().filter(|c| !c.is_whitespace()).collect())
            .collect()
    }
}
