use crate::{
    Binding, Def, DefId, DefKind, Import, ImportId, ImportKind, ImportStatus, Name, Namespace,
    Origin, Scope, ScopeId, ScopeKind, Visibility,
};
use std::ops::{Index, IndexMut};

/// Name-resolution database.
#[derive(Debug, Clone)]
pub struct NameDb<'cx> {
    scopes: Vec<Scope<'cx>>,
    defs: Vec<Def<'cx>>,
    imports: Vec<Import<'cx>>,
}

impl<'cx> NameDb<'cx> {
    /// Returns the crate-root scope.
    pub const fn root_scope(&self) -> ScopeId {
        ScopeId::new(0)
    }

    /// Returns all scopes.
    pub fn scopes(&self) -> &[Scope<'cx>] {
        &self.scopes
    }

    /// Returns all definitions.
    pub fn defs(&self) -> &[Def<'cx>] {
        &self.defs
    }

    /// Returns all imports.
    pub fn imports(&self) -> &[Import<'cx>] {
        &self.imports
    }

    /// Returns the binding for `name` directly declared in `scope` and `namespace`.
    pub fn binding(
        &self,
        scope: ScopeId,
        namespace: Namespace,
        name: Name<'cx>,
    ) -> Option<&Binding> {
        self[scope].bindings.get(namespace, name)
    }

    /// Adds a scope under `parent`.
    pub fn add_scope(&mut self, kind: ScopeKind, parent: Option<ScopeId>) -> ScopeId {
        let id = ScopeId::new(self.scopes.len());
        self.scopes.push(Scope::new(id, kind, parent));
        id
    }

    /// Adds a definition and binds it in the default namespaces for its kind.
    pub fn add_def(
        &mut self,
        parent_scope: ScopeId,
        kind: DefKind,
        name: Option<Name<'cx>>,
        visibility: Visibility,
        origin: Origin,
    ) -> DefId {
        self.add_def_in_namespaces(
            parent_scope,
            kind,
            name,
            kind.namespaces(),
            visibility,
            origin,
        )
    }

    /// Adds a definition and binds it in explicit namespaces.
    pub fn add_def_in_namespaces(
        &mut self,
        parent_scope: ScopeId,
        kind: DefKind,
        name: Option<Name<'cx>>,
        namespaces: &[Namespace],
        visibility: Visibility,
        origin: Origin,
    ) -> DefId {
        let id = DefId::new(self.defs.len());
        self.defs.push(Def {
            id,
            name,
            kind,
            parent_scope,
            child_scope: None,
            target: None,
            visibility,
            origin,
        });

        if let Some(name) = name {
            for &namespace in namespaces {
                self[parent_scope].bindings.insert(namespace, name, id);
            }
        }

        id
    }

    /// Adds an unresolved import.
    pub fn add_import(
        &mut self,
        scope: ScopeId,
        source_path: Vec<Name<'cx>>,
        kind: ImportKind<'cx>,
        visibility: Visibility,
        origin: Origin,
    ) -> ImportId {
        let id = ImportId::new(self.imports.len());
        self.imports.push(Import {
            id,
            scope,
            source_path,
            kind,
            visibility,
            status: ImportStatus::Unresolved,
            origin,
        });
        id
    }

    /// Links a definition to a child scope that contains its importable members.
    pub fn set_child_scope(&mut self, def: DefId, child_scope: ScopeId) {
        self.defs[def.index()].child_scope = Some(child_scope);
    }

    /// Follows `DefKind::Use` alias definitions to their underlying definition.
    ///
    /// For example, if `use b::C` points at `pub use a::C`, and that re-export points at the
    /// original struct `C`, this returns the struct definition.
    pub fn follow_aliases(&self, mut def: DefId) -> DefId {
        let mut remaining = self.defs.len();
        while remaining > 0 {
            let Some(target) = self[def].target else {
                return def;
            };
            def = target;
            remaining -= 1;
        }
        def
    }

    /// Resolves all currently collected imports to local-crate bindings.
    pub fn resolve_imports(&mut self) {
        loop {
            let mut changed = false;
            for index in 0..self.imports.len() {
                if self.imports[index].status != ImportStatus::Unresolved {
                    continue;
                }

                match self.resolve_import(ImportId::new(index)) {
                    ImportResolve::Resolved => {
                        self.imports[index].status = ImportStatus::Resolved;
                        changed = true;
                    }
                    ImportResolve::Ambiguous => {
                        self.imports[index].status = ImportStatus::Ambiguous;
                        changed = true;
                    }
                    ImportResolve::Pending => {}
                }
            }

            if !changed {
                break;
            }
        }

        for import in &mut self.imports {
            if import.status == ImportStatus::Unresolved {
                import.status = ImportStatus::NotFound;
            }
        }
    }

    fn get_or_insert_import_def(
        &mut self,
        parent_scope: ScopeId,
        name: Option<Name<'cx>>,
        visibility: Visibility,
        origin: Origin,
        target: DefId,
    ) -> DefId {
        if let Some(def) = self.defs.iter().find(|def| {
            def.kind == DefKind::Use
                && def.parent_scope == parent_scope
                && def.name == name
                && def.visibility == visibility
                && def.origin == origin
                && def.target == Some(target)
        }) {
            return def.id;
        }

        let id = DefId::new(self.defs.len());
        self.defs.push(Def {
            id,
            name,
            kind: DefKind::Use,
            parent_scope,
            child_scope: None,
            target: Some(target),
            visibility,
            origin,
        });
        id
    }

    fn insert_unique_binding(
        &mut self,
        scope: ScopeId,
        namespace: Namespace,
        name: Name<'cx>,
        def: DefId,
    ) -> bool {
        self[scope].bindings.insert_unique(namespace, name, def)
    }

    fn to_import_alias_target(&self, bindings: &[(Namespace, DefId)]) -> Option<ImportAliasTarget> {
        let (_, first) = *bindings
            .first()
            .expect("import alias target requires at least one resolved binding");
        let target = self.follow_aliases(first);
        let mut namespaces = Vec::with_capacity(bindings.len());

        for &(namespace, def) in bindings {
            if self.follow_aliases(def) != target {
                return None;
            }
            namespaces.push(namespace);
        }

        Some(ImportAliasTarget { target, namespaces })
    }

    fn resolve_lexical(
        &self,
        mut scope: ScopeId,
        namespace: Namespace,
        name: Name<'cx>,
    ) -> ResolveResult {
        loop {
            if let Some(binding) = self[scope].bindings.get(namespace, name) {
                let mut defs = binding.iter();
                return match defs.len() {
                    0 => ResolveResult::NotFound,
                    1 => ResolveResult::Found(defs.next().unwrap()),
                    _ => ResolveResult::Ambiguous(defs.collect()),
                };
            }

            let Some(parent) = self[scope].parent else {
                return ResolveResult::NotFound;
            };
            scope = parent;
        }
    }

    fn is_descendant_scope(&self, mut descendant: ScopeId, ancestor: ScopeId) -> bool {
        loop {
            if descendant == ancestor {
                return true;
            }

            let Some(parent) = self[descendant].parent else {
                return false;
            };
            descendant = parent;
        }
    }

    /// Resolves one collected `use` declaration into local bindings.
    ///
    /// The original [`Import`] entry remains the source-level record. Successful resolution adds
    /// one or more [`DefKind::Use`] alias definitions and binds those alias definitions in the
    /// scope that contains the `use`.
    ///
    /// For example, `use a::b::C` where `C` is a struct resolves the path to
    /// `(Namespace::Type, DefId(C))`, creates a `DefKind::Use` alias named `C` whose target is the
    /// original struct definition, then inserts that alias into the type namespace of the `use`
    /// scope.
    fn resolve_import(&mut self, import: ImportId) -> ImportResolve {
        let import_data = &self[import];
        match import_data.kind {
            ImportKind::Single | ImportKind::Rename(_) => {
                let local_name = match self.import_local_name(import_data) {
                    ImportLocalName::Name(name) => name,
                    ImportLocalName::NoBinding => {
                        let bindings =
                            self.resolve_import_path(import_data.scope, &import_data.source_path);
                        return match bindings {
                            LookupResolve::Found(_) => ImportResolve::Resolved,
                            LookupResolve::Ambiguous => ImportResolve::Ambiguous,
                            LookupResolve::NotFound => ImportResolve::Pending,
                        };
                    }
                    ImportLocalName::Ambiguous => return ImportResolve::Ambiguous,
                    ImportLocalName::Pending => return ImportResolve::Pending,
                };

                let bindings =
                    match self.resolve_import_path(import_data.scope, &import_data.source_path) {
                        LookupResolve::Found(bindings) => bindings,
                        LookupResolve::Ambiguous => return ImportResolve::Ambiguous,
                        LookupResolve::NotFound => return ImportResolve::Pending,
                    };

                let Some(alias_target) = self.to_import_alias_target(&bindings) else {
                    return ImportResolve::Ambiguous;
                };

                let import_data = import_data.clone();
                let alias = self.get_or_insert_import_def(
                    import_data.scope,
                    Some(local_name),
                    import_data.visibility,
                    import_data.origin,
                    alias_target.target,
                );
                for namespace in alias_target.namespaces {
                    self.insert_unique_binding(import_data.scope, namespace, local_name, alias);
                }
                ImportResolve::Resolved
            }
            ImportKind::Glob => {
                // `use a::b::*;` imports each visible binding from the target's child scope. Each
                // imported child gets its own local `DefKind::Use` alias in the `use` scope.
                let bindings =
                    match self.resolve_import_path(import_data.scope, &import_data.source_path) {
                        LookupResolve::Found(bindings) => bindings,
                        LookupResolve::Ambiguous => return ImportResolve::Ambiguous,
                        LookupResolve::NotFound => return ImportResolve::Pending,
                    };

                let [(_, target)] = bindings.as_slice() else {
                    return ImportResolve::Ambiguous;
                };
                let target = self.follow_aliases(*target);
                let Some(child_scope) = self[target].child_scope else {
                    return ImportResolve::Pending;
                };

                let mut visible = Vec::new();
                for namespace in Namespace::all() {
                    for (&name, binding) in self[child_scope].bindings.map(namespace) {
                        for def in binding.iter() {
                            if self.is_visible_from(def, import_data.scope) {
                                visible.push((namespace, name, def));
                            }
                        }
                    }
                }

                let import_data = import_data.clone();
                for (namespace, name, def) in visible {
                    let target = self.follow_aliases(def);
                    let alias = self.get_or_insert_import_def(
                        import_data.scope,
                        Some(name),
                        import_data.visibility,
                        import_data.origin,
                        target,
                    );
                    self.insert_unique_binding(import_data.scope, namespace, name, alias);
                }

                ImportResolve::Resolved
            }
        }
    }

    /// Returns the local binding name introduced by an import.
    ///
    /// For example,
    /// - `use a::b::C` introduces `C`
    /// - `use a::b::C as D` introduces `D`
    /// - `use crate::a::{self}` introduces `a`
    /// - Glob imports and underscore imports such as `use a::b::*` or `use a::b::C as _` introduce
    ///   no single local name.
    ///
    /// A trailing `self` depends on resolving its parent path. If that parent path is ambiguous or
    /// not found yet, the result preserves that state instead of treating `self` as a local name.
    fn import_local_name(&self, import: &Import<'cx>) -> ImportLocalName<'cx> {
        let name = match import.kind {
            ImportKind::Single => {
                let Some(&terminal) = import.source_path.last() else {
                    return ImportLocalName::Pending;
                };

                if terminal.as_ref() == "self" {
                    let parent = &import.source_path[..import.source_path.len().saturating_sub(1)];

                    let bindings = match self.resolve_import_path(import.scope, parent) {
                        LookupResolve::Found(bindings) => bindings,
                        LookupResolve::Ambiguous => return ImportLocalName::Ambiguous,
                        LookupResolve::NotFound => return ImportLocalName::Pending,
                    };

                    let Some(alias_target) = self.to_import_alias_target(&bindings) else {
                        return ImportLocalName::Ambiguous;
                    };

                    let Some(name) = self[alias_target.target].name else {
                        return ImportLocalName::Pending;
                    };

                    name
                } else {
                    terminal
                }
            }
            ImportKind::Rename(name) => name,
            ImportKind::Glob => return ImportLocalName::NoBinding,
        };

        if name.as_ref() == "_" {
            ImportLocalName::NoBinding
        } else {
            ImportLocalName::Name(name)
        }
    }

    /// Resolves an import path from `scope` into visible candidate definitions.
    ///
    /// For example, resolving `crate::a::C` starts at the root scope, finds module `a`, follows
    /// `a`'s child scope, then resolves `C` in that child scope. If the terminal name exists in
    /// multiple namespaces with the same target, such as an enum variant, the result contains each
    /// namespace-target pair.
    fn resolve_import_path(&self, scope: ScopeId, path: &[Name<'cx>]) -> LookupResolve {
        if path.is_empty() {
            return LookupResolve::NotFound;
        }

        let use_scope = scope;
        let mut current_scope = self.nearest_module_scope(scope);
        let mut current_def = None;
        let mut index = 0;

        while index < path.len() {
            let segment = path[index];
            let is_last = index + 1 == path.len();

            match segment.as_ref() {
                "crate" if index == 0 => {
                    current_scope = self.root_scope();
                    current_def = None;
                    index += 1;
                    continue;
                }
                "self" => {
                    if is_last {
                        return current_def
                            .map(|def| LookupResolve::Found(vec![(Namespace::Type, def)]))
                            .unwrap_or(LookupResolve::NotFound);
                    }
                    index += 1;
                    continue;
                }
                "super" => {
                    let Some(parent) = self.parent_module_scope(current_scope) else {
                        return LookupResolve::NotFound;
                    };
                    current_scope = parent;
                    current_def = None;
                    index += 1;
                    continue;
                }
                _ => {}
            }

            let bindings = if is_last {
                self.resolve_name_all(current_scope, segment, index == 0)
            } else {
                self.resolve_name_in_namespace(current_scope, Namespace::Type, segment, index == 0)
            };

            let bindings = match bindings {
                LookupResolve::Found(bindings) => bindings,
                other => return other,
            };

            let visible = bindings
                .into_iter()
                .filter(|(_, def)| self.is_visible_from(*def, use_scope))
                .collect::<Vec<_>>();

            if visible.is_empty() {
                return LookupResolve::NotFound;
            }

            if is_last {
                return LookupResolve::Found(visible);
            }

            let [(_, def)] = visible.as_slice() else {
                return LookupResolve::Ambiguous;
            };

            let target = self.follow_aliases(*def);
            let Some(child_scope) = self[target].child_scope else {
                return LookupResolve::NotFound;
            };
            current_scope = child_scope;
            current_def = Some(target);
            index += 1;
        }

        LookupResolve::NotFound
    }

    fn resolve_name_all(&self, scope: ScopeId, name: Name<'cx>, lexical: bool) -> LookupResolve {
        let mut defs = Vec::new();
        for namespace in Namespace::all() {
            match self.resolve_name_in_namespace(scope, namespace, name, lexical) {
                LookupResolve::Found(found) => defs.extend(found),
                LookupResolve::Ambiguous => return LookupResolve::Ambiguous,
                LookupResolve::NotFound => {}
            }
        }

        if defs.is_empty() {
            LookupResolve::NotFound
        } else {
            defs.sort_by_key(|(_, def)| def.index());
            defs.dedup();
            LookupResolve::Found(defs)
        }
    }

    fn resolve_name_in_namespace(
        &self,
        scope: ScopeId,
        namespace: Namespace,
        name: Name<'cx>,
        lexical: bool,
    ) -> LookupResolve {
        if lexical {
            match self.resolve_lexical(scope, namespace, name) {
                ResolveResult::Found(def) => LookupResolve::Found(vec![(namespace, def)]),
                ResolveResult::Ambiguous(_) => LookupResolve::Ambiguous,
                ResolveResult::NotFound => LookupResolve::NotFound,
            }
        } else {
            self[scope]
                .bindings
                .get(namespace, name)
                .map(|binding| self.binding_result(namespace, binding))
                .unwrap_or(LookupResolve::NotFound)
        }
    }

    fn binding_result(&self, namespace: Namespace, binding: &Binding) -> LookupResolve {
        let defs = binding.iter().collect::<Vec<_>>();
        match defs.len() {
            0 => LookupResolve::NotFound,
            1 => LookupResolve::Found(vec![(namespace, defs[0])]),
            _ => LookupResolve::Ambiguous,
        }
    }

    fn is_visible_from(&self, def: DefId, scope: ScopeId) -> bool {
        let def = &self[def];
        match def.visibility {
            Visibility::Public => true,
            Visibility::Restricted(ancestor) => {
                self.is_descendant_scope(self.nearest_module_scope(scope), ancestor)
            }
            Visibility::Private => {
                let defining_module = self.nearest_module_scope(def.parent_scope);
                self.is_descendant_scope(self.nearest_module_scope(scope), defining_module)
            }
        }
    }

    fn nearest_module_scope(&self, mut scope: ScopeId) -> ScopeId {
        loop {
            if matches!(self[scope].kind, ScopeKind::CrateRoot | ScopeKind::Module) {
                return scope;
            }
            let Some(parent) = self[scope].parent else {
                return scope;
            };
            scope = parent;
        }
    }

    fn parent_module_scope(&self, scope: ScopeId) -> Option<ScopeId> {
        let mut scope = self[scope].parent?;
        loop {
            if matches!(self[scope].kind, ScopeKind::CrateRoot | ScopeKind::Module) {
                return Some(scope);
            }
            scope = self[scope].parent?;
        }
    }
}

/// Result of resolving a name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolveResult {
    /// Name resolved to one definition.
    Found(DefId),

    /// Name resolved to multiple candidate definitions.
    Ambiguous(Vec<DefId>),

    /// Name was not found.
    NotFound,
}

enum ImportResolve {
    Resolved,
    Ambiguous,
    Pending,
}

enum ImportLocalName<'cx> {
    /// Import introduces this local binding name.
    Name(Name<'cx>),

    /// Import introduces no single local binding, such as a glob or underscore import.
    NoBinding,

    /// Local name computation found an ambiguous path.
    Ambiguous,

    /// Local name computation depends on an import that is not resolved yet.
    Pending,
}

/// Normalized target information for creating one import alias definition.
///
/// For example, `use a::E::V` for an enum variant can produce one alias target for `V` with both
/// the type and value namespaces.
struct ImportAliasTarget {
    target: DefId,
    namespaces: Vec<Namespace>,
}

enum LookupResolve {
    Found(Vec<(Namespace, DefId)>),
    Ambiguous,
    NotFound,
}

impl Default for NameDb<'_> {
    /// Creates a name database with a crate-root scope.
    fn default() -> Self {
        let root_scope = Scope::new(ScopeId::new(0), ScopeKind::CrateRoot, None);

        Self {
            scopes: vec![root_scope],
            defs: Vec::new(),
            imports: Vec::new(),
        }
    }
}

impl<'cx> Index<ScopeId> for NameDb<'cx> {
    type Output = Scope<'cx>;

    fn index(&self, index: ScopeId) -> &Self::Output {
        &self.scopes[index.index()]
    }
}

impl<'cx> IndexMut<ScopeId> for NameDb<'cx> {
    fn index_mut(&mut self, index: ScopeId) -> &mut Self::Output {
        &mut self.scopes[index.index()]
    }
}

impl<'cx> Index<DefId> for NameDb<'cx> {
    type Output = Def<'cx>;

    fn index(&self, index: DefId) -> &Self::Output {
        &self.defs[index.index()]
    }
}

impl<'cx> Index<ImportId> for NameDb<'cx> {
    type Output = Import<'cx>;

    fn index(&self, index: ImportId) -> &Self::Output {
        &self.imports[index.index()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn_sem_common::CommonCx;

    #[test]
    fn lexical_resolution_prefers_inner_scope() {
        let ccx = CommonCx::new();
        let x = ccx.intern("x");

        let mut db = NameDb::default();
        let root = db.root_scope();
        let body = db.add_scope(ScopeKind::FunctionBody, Some(root));

        let outer = db.add_def(
            root,
            DefKind::Local,
            Some(x),
            Visibility::Private,
            Origin::Synthetic,
        );
        let inner = db.add_def(
            body,
            DefKind::Local,
            Some(x),
            Visibility::Private,
            Origin::Synthetic,
        );

        assert_eq!(
            db.resolve_lexical(body, Namespace::Value, x),
            ResolveResult::Found(inner)
        );
        assert_eq!(
            db.resolve_lexical(root, Namespace::Value, x),
            ResolveResult::Found(outer)
        );
    }

    #[test]
    fn namespaces_are_independent() {
        let ccx = CommonCx::new();
        let t = ccx.intern("T");

        let mut db = NameDb::default();
        let root = db.root_scope();
        let type_param = db.add_def(
            root,
            DefKind::TypeParam,
            Some(t),
            Visibility::Private,
            Origin::Synthetic,
        );
        let local = db.add_def(
            root,
            DefKind::Local,
            Some(t),
            Visibility::Private,
            Origin::Synthetic,
        );

        assert_eq!(
            db.resolve_lexical(root, Namespace::Type, t),
            ResolveResult::Found(type_param)
        );
        assert_eq!(
            db.resolve_lexical(root, Namespace::Value, t),
            ResolveResult::Found(local)
        );
    }

    #[test]
    fn generic_scope_is_visible_from_function_body() {
        let ccx = CommonCx::new();
        let t = ccx.intern("T");

        let mut db = NameDb::default();
        let root = db.root_scope();
        let generic_scope = db.add_scope(ScopeKind::GenericParams, Some(root));
        let body = db.add_scope(ScopeKind::FunctionBody, Some(generic_scope));

        let type_param = db.add_def(
            generic_scope,
            DefKind::TypeParam,
            Some(t),
            Visibility::Private,
            Origin::Synthetic,
        );

        assert_eq!(
            db.resolve_lexical(body, Namespace::Type, t),
            ResolveResult::Found(type_param)
        );
    }

    #[test]
    fn local_item_can_be_resolved_in_type_namespace() {
        let ccx = CommonCx::new();
        let local = ccx.intern("Local");

        let mut db = NameDb::default();
        let root = db.root_scope();
        let body = db.add_scope(ScopeKind::FunctionBody, Some(root));
        let block = db.add_scope(ScopeKind::Block, Some(body));

        let local_struct = db.add_def(
            block,
            DefKind::Struct,
            Some(local),
            Visibility::Private,
            Origin::Synthetic,
        );

        assert_eq!(
            db.resolve_lexical(block, Namespace::Type, local),
            ResolveResult::Found(local_struct)
        );
    }

    #[test]
    fn const_generic_lives_in_value_namespace() {
        let ccx = CommonCx::new();
        let n = ccx.intern("N");

        let mut db = NameDb::default();
        let root = db.root_scope();
        let generic_scope = db.add_scope(ScopeKind::GenericParams, Some(root));

        let const_param = db.add_def(
            generic_scope,
            DefKind::ConstParam,
            Some(n),
            Visibility::Private,
            Origin::Synthetic,
        );

        assert_eq!(
            db.resolve_lexical(generic_scope, Namespace::Value, n),
            ResolveResult::Found(const_param)
        );
        assert_eq!(
            db.resolve_lexical(generic_scope, Namespace::Type, n),
            ResolveResult::NotFound
        );
    }

    fn module<'cx>(
        db: &mut NameDb<'cx>,
        parent: ScopeId,
        name: Name<'cx>,
        visibility: Visibility,
    ) -> (DefId, ScopeId) {
        let def = db.add_def(
            parent,
            DefKind::Module,
            Some(name),
            visibility,
            Origin::Synthetic,
        );
        let scope = db.add_scope(ScopeKind::Module, Some(parent));
        db.set_child_scope(def, scope);
        (def, scope)
    }

    fn target_kind(
        db: &NameDb<'_>,
        scope: ScopeId,
        namespace: Namespace,
        name: Name<'_>,
    ) -> DefKind {
        let ResolveResult::Found(def) = db.resolve_lexical(scope, namespace, name) else {
            panic!("expected {name:?} to resolve in {namespace:?}");
        };
        db[db.follow_aliases(def)].kind
    }

    #[test]
    fn resolves_single_rename_self_and_underscore_imports() {
        let ccx = CommonCx::new();
        let mut db = NameDb::default();
        let root = db.root_scope();
        let a = ccx.intern("a");
        let b = ccx.intern("b");
        let s = ccx.intern("S");
        let t = ccx.intern("T");
        let hidden = ccx.intern("_");

        let (_, a_scope) = module(&mut db, root, a, Visibility::Public);
        let (_, b_scope) = module(&mut db, root, b, Visibility::Public);
        db.add_def(
            a_scope,
            DefKind::Struct,
            Some(s),
            Visibility::Public,
            Origin::Synthetic,
        );
        db.add_import(
            b_scope,
            vec![ccx.intern("super"), a, s],
            ImportKind::Single,
            Visibility::Private,
            Origin::Synthetic,
        );
        db.add_import(
            b_scope,
            vec![ccx.intern("super"), a, s],
            ImportKind::Rename(t),
            Visibility::Private,
            Origin::Synthetic,
        );
        db.add_import(
            b_scope,
            vec![ccx.intern("super"), a, ccx.intern("self")],
            ImportKind::Single,
            Visibility::Private,
            Origin::Synthetic,
        );
        db.add_import(
            b_scope,
            vec![ccx.intern("super"), a, s],
            ImportKind::Rename(hidden),
            Visibility::Private,
            Origin::Synthetic,
        );

        db.resolve_imports();

        assert!(db
            .imports()
            .iter()
            .all(|import| import.status == ImportStatus::Resolved));
        assert_eq!(
            target_kind(&db, b_scope, Namespace::Type, s),
            DefKind::Struct
        );
        assert_eq!(
            target_kind(&db, b_scope, Namespace::Type, t),
            DefKind::Struct
        );
        assert_eq!(
            target_kind(&db, b_scope, Namespace::Type, a),
            DefKind::Module
        );
        assert_eq!(
            db.resolve_lexical(b_scope, Namespace::Type, hidden),
            ResolveResult::NotFound
        );
    }

    #[test]
    fn self_import_preserves_parent_path_failure() {
        let ccx = CommonCx::new();
        let mut db = NameDb::default();
        let root = db.root_scope();
        let a = ccx.intern("a");
        let b = ccx.intern("b");
        let missing = ccx.intern("missing");
        let self_name = ccx.intern("self");

        module(&mut db, root, a, Visibility::Public);
        module(&mut db, root, a, Visibility::Public);
        let (_, b_scope) = module(&mut db, root, b, Visibility::Public);

        db.add_import(
            b_scope,
            vec![ccx.intern("super"), a, self_name],
            ImportKind::Single,
            Visibility::Private,
            Origin::Synthetic,
        );
        db.add_import(
            b_scope,
            vec![ccx.intern("super"), missing, self_name],
            ImportKind::Single,
            Visibility::Private,
            Origin::Synthetic,
        );

        db.resolve_imports();

        assert_eq!(db.imports()[0].status, ImportStatus::Ambiguous);
        assert_eq!(db.imports()[1].status, ImportStatus::NotFound);
        assert_eq!(
            db.resolve_lexical(b_scope, Namespace::Type, self_name),
            ResolveResult::NotFound
        );
    }

    #[test]
    fn resolves_chained_reexports_and_globs_with_visibility() {
        let ccx = CommonCx::new();
        let mut db = NameDb::default();
        let root = db.root_scope();
        let a = ccx.intern("a");
        let b = ccx.intern("b");
        let c = ccx.intern("c");
        let d = ccx.intern("d");
        let public = ccx.intern("Public");
        let private = ccx.intern("Private");

        let (_, a_scope) = module(&mut db, root, a, Visibility::Public);
        let (_, b_scope) = module(&mut db, root, b, Visibility::Public);
        let (_, c_scope) = module(&mut db, root, c, Visibility::Public);
        let (_, d_scope) = module(&mut db, root, d, Visibility::Public);
        db.add_def(
            a_scope,
            DefKind::Struct,
            Some(public),
            Visibility::Public,
            Origin::Synthetic,
        );
        db.add_def(
            a_scope,
            DefKind::Struct,
            Some(private),
            Visibility::Private,
            Origin::Synthetic,
        );
        db.add_import(
            b_scope,
            vec![ccx.intern("super"), a, public],
            ImportKind::Single,
            Visibility::Public,
            Origin::Synthetic,
        );
        db.add_import(
            c_scope,
            vec![ccx.intern("super"), b, public],
            ImportKind::Single,
            Visibility::Public,
            Origin::Synthetic,
        );
        db.add_import(
            d_scope,
            vec![ccx.intern("super"), a],
            ImportKind::Glob,
            Visibility::Private,
            Origin::Synthetic,
        );

        db.resolve_imports();

        assert!(db
            .imports()
            .iter()
            .all(|import| import.status == ImportStatus::Resolved));
        assert_eq!(
            target_kind(&db, c_scope, Namespace::Type, public),
            DefKind::Struct
        );
        assert_eq!(
            target_kind(&db, d_scope, Namespace::Type, public),
            DefKind::Struct
        );
        assert_eq!(
            db.resolve_lexical(d_scope, Namespace::Type, private),
            ResolveResult::NotFound
        );
    }

    #[test]
    fn import_resolution_reports_ambiguity_and_not_found() {
        let ccx = CommonCx::new();
        let mut db = NameDb::default();
        let root = db.root_scope();
        let a = ccx.intern("a");
        let b = ccx.intern("b");
        let c = ccx.intern("c");
        let x = ccx.intern("X");
        let missing = ccx.intern("Missing");

        let (_, a_scope) = module(&mut db, root, a, Visibility::Public);
        let (_, b_scope) = module(&mut db, root, b, Visibility::Public);
        let (_, c_scope) = module(&mut db, root, c, Visibility::Public);
        db.add_def(
            a_scope,
            DefKind::Struct,
            Some(x),
            Visibility::Public,
            Origin::Synthetic,
        );
        db.add_def(
            b_scope,
            DefKind::Struct,
            Some(x),
            Visibility::Public,
            Origin::Synthetic,
        );
        db.add_import(
            c_scope,
            vec![ccx.intern("super"), a],
            ImportKind::Glob,
            Visibility::Private,
            Origin::Synthetic,
        );
        db.add_import(
            c_scope,
            vec![ccx.intern("super"), b],
            ImportKind::Glob,
            Visibility::Private,
            Origin::Synthetic,
        );
        db.add_import(
            c_scope,
            vec![ccx.intern("super"), a, missing],
            ImportKind::Single,
            Visibility::Private,
            Origin::Synthetic,
        );

        db.resolve_imports();

        let ResolveResult::Ambiguous(defs) = db.resolve_lexical(c_scope, Namespace::Type, x) else {
            panic!("expected imported globs to make {x:?} ambiguous");
        };
        assert_eq!(defs.len(), 2);
        assert_eq!(db.imports()[2].status, ImportStatus::NotFound);
    }

    #[test]
    fn single_import_is_ambiguous_when_namespaces_resolve_to_distinct_targets() {
        let ccx = CommonCx::new();
        let mut db = NameDb::default();
        let root = db.root_scope();
        let a = ccx.intern("a");
        let b = ccx.intern("b");
        let x = ccx.intern("X");

        let (_, a_scope) = module(&mut db, root, a, Visibility::Public);
        let (_, b_scope) = module(&mut db, root, b, Visibility::Public);
        db.add_def(
            a_scope,
            DefKind::Struct,
            Some(x),
            Visibility::Public,
            Origin::Synthetic,
        );
        db.add_def(
            a_scope,
            DefKind::Const,
            Some(x),
            Visibility::Public,
            Origin::Synthetic,
        );
        db.add_import(
            b_scope,
            vec![ccx.intern("super"), a, x],
            ImportKind::Single,
            Visibility::Private,
            Origin::Synthetic,
        );

        db.resolve_imports();

        assert_eq!(db.imports()[0].status, ImportStatus::Ambiguous);
        assert_eq!(
            db.resolve_lexical(b_scope, Namespace::Type, x),
            ResolveResult::NotFound
        );
        assert_eq!(
            db.resolve_lexical(b_scope, Namespace::Value, x),
            ResolveResult::NotFound
        );
    }

    #[test]
    fn imported_enum_variant_keeps_type_and_value_namespaces() {
        let ccx = CommonCx::new();
        let mut db = NameDb::default();
        let root = db.root_scope();
        let a = ccx.intern("a");
        let b = ccx.intern("b");
        let e = ccx.intern("E");
        let v = ccx.intern("V");

        let (_, a_scope) = module(&mut db, root, a, Visibility::Public);
        let (_, b_scope) = module(&mut db, root, b, Visibility::Public);
        let enum_def = db.add_def(
            a_scope,
            DefKind::Enum,
            Some(e),
            Visibility::Public,
            Origin::Synthetic,
        );
        let enum_scope = db.add_scope(ScopeKind::Item, Some(a_scope));
        db.set_child_scope(enum_def, enum_scope);
        db.add_def(
            enum_scope,
            DefKind::Variant,
            Some(v),
            Visibility::Public,
            Origin::Synthetic,
        );
        db.add_import(
            b_scope,
            vec![ccx.intern("super"), a, e, v],
            ImportKind::Single,
            Visibility::Private,
            Origin::Synthetic,
        );

        db.resolve_imports();

        assert_eq!(
            target_kind(&db, b_scope, Namespace::Type, v),
            DefKind::Variant
        );
        assert_eq!(
            target_kind(&db, b_scope, Namespace::Value, v),
            DefKind::Variant
        );
    }
}
