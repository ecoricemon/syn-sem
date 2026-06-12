use crate::{
    AstNodeId, Binding, Def, DefId, DefKind, Import, ImportId, ImportKind, ImportStatus, Map, Name,
    Namespace, Origin, Scope, ScopeId, ScopeKind, Visibility,
};
use std::ops::{Index, IndexMut};

/// Name-resolution database.
#[derive(Debug, Clone)]
pub struct NameDb<'cx> {
    scopes: Vec<Scope<'cx>>,
    defs: Vec<Def<'cx>>,
    imports: Vec<Import<'cx>>,
    ast_defs: Map<AstNodeId<'cx>, DefId>,
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

    /// Returns the definition created from `node`, if one is tracked.
    pub fn def_for_ast_node(&self, node: AstNodeId<'cx>) -> Option<DefId> {
        self.ast_defs.get(&node).copied()
    }

    /// Records that `def` was created from `node`.
    pub fn set_def_ast_node(&mut self, def: DefId, node: AstNodeId<'cx>) {
        let old = self.ast_defs.insert(node, def);
        assert!(
            old.is_none() || old == Some(def),
            "one AST node cannot create multiple definitions"
        );
    }

    /// Returns the path scope attached to `def`, if any.
    pub fn def_path_scope(&self, def: DefId) -> Option<ScopeId> {
        self[def].scopes.path
    }

    /// Returns the body scope attached to `def`, if any.
    pub fn def_body_scope(&self, def: DefId) -> Option<ScopeId> {
        self[def].scopes.body
    }

    /// Returns the generic scope attached to `def`, if any.
    pub fn def_generic_scope(&self, def: DefId) -> Option<ScopeId> {
        self[def].scopes.generic
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

    /// Resolves a direct member from the path scope attached to `owner`.
    ///
    /// For example, `member(owner = Iterator, namespace = Type, name = Item)` checks the
    /// path-reachable members of `Iterator` for an associated type named `Item`.
    pub fn member(&self, owner: DefId, namespace: Namespace, name: Name<'cx>) -> ResolveResult {
        let owner = self.follow_aliases(owner);
        let Some(path_scope) = self[owner].scopes.path else {
            return ResolveResult::NotFound;
        };
        let Some(binding) = self.binding(path_scope, namespace, name) else {
            return ResolveResult::NotFound;
        };

        let mut defs = binding
            .iter()
            .map(|def| self.follow_aliases(def))
            .collect::<Vec<_>>();
        defs.sort_by_key(|def| def.index());
        defs.dedup();

        match defs.as_slice() {
            [] => ResolveResult::NotFound,
            [def] => ResolveResult::Found(*def),
            _ => ResolveResult::Ambiguous(defs),
        }
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
            kind.namespaces().iter().copied(),
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
        namespaces: impl Iterator<Item = Namespace>,
        visibility: Visibility,
        origin: Origin,
    ) -> DefId {
        let id = DefId::new(self.defs.len());
        self.defs.push(Def {
            id,
            name,
            kind,
            parent_scope,
            scopes: Default::default(),
            target: None,
            visibility,
            origin,
        });

        if let Some(name) = name {
            for namespace in namespaces {
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

    /// Links a definition to a path scope that contains its path-reachable members.
    pub fn set_path_scope(&mut self, def: DefId, path_scope: ScopeId) {
        self.defs[def.index()].scopes.path = Some(path_scope);
    }

    /// Links a definition to the scope containing its generic parameters.
    pub fn set_generic_scope(&mut self, def: DefId, generic_scope: ScopeId) {
        self.defs[def.index()].scopes.generic = Some(generic_scope);
    }

    /// Links a definition to the scope containing its value body.
    pub fn set_body_scope(&mut self, def: DefId, body_scope: ScopeId) {
        self.defs[def.index()].scopes.body = Some(body_scope);
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

    /// Resolves a type path from `scope`.
    ///
    /// Single-segment paths use lexical type-namespace lookup so generic type parameters and local
    /// items can resolve from their use site.
    ///
    /// This follows the same local-crate path traversal rules as import resolution, then narrows
    /// the terminal candidates to the type namespace and follows import aliases.
    pub fn resolve_type_path(
        &self,
        scope: ScopeId,
        mut path: impl ExactSizeIterator<Item = Name<'cx>>,
    ) -> ResolveResult {
        if path.len() == 1 {
            let name = path.next().unwrap();
            return match self.resolve_lexical(scope, Namespace::Type, name) {
                ResolveResult::Found(def) => ResolveResult::Found(self.follow_aliases(def)),
                ResolveResult::Ambiguous(defs) => ResolveResult::Ambiguous(
                    defs.into_iter()
                        .map(|def| self.follow_aliases(def))
                        .collect(),
                ),
                ResolveResult::NotFound => ResolveResult::NotFound,
            };
        }

        let candidates = match self.resolve_import_path(scope, path) {
            CandidateResolution::Found(candidates) => candidates,
            CandidateResolution::Ambiguous => return ResolveResult::Ambiguous(Vec::new()),
            CandidateResolution::NotFound => return ResolveResult::NotFound,
        };

        let mut defs = candidates
            .into_iter()
            .filter(|(namespace, _)| *namespace == Namespace::Type)
            .map(|(_, def)| self.follow_aliases(def))
            .collect::<Vec<_>>();
        defs.sort_by_key(|def| def.index());
        defs.dedup();

        match defs.as_slice() {
            [] => ResolveResult::NotFound,
            [def] => ResolveResult::Found(*def),
            _ => ResolveResult::Ambiguous(defs),
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
            scopes: Default::default(),
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

    /// Validates path lookup candidates for one local import binding.
    ///
    /// Every candidate must resolve through aliases to the same final target. The returned value
    /// keeps that target and the namespaces where the import should be bound. For example, enum
    /// variant `V` from `use a::E::V;` can validate as one target with both type and value
    /// namespaces.
    ///
    /// Empty candidates or candidates with different final targets are caller invariant
    /// violations and panic.
    fn validate_import_binding_candidates(
        &self,
        candidates: &[(Namespace, DefId)],
    ) -> ValidatedImportBinding {
        let (_, first) = *candidates
            .first()
            .expect("import binding validation requires at least one resolved binding");
        let target = self.follow_aliases(first);
        let mut namespaces = Vec::with_capacity(candidates.len());

        for &(namespace, def) in candidates {
            assert_eq!(
                self.follow_aliases(def),
                target,
                "import binding candidates must resolve to one final target"
            );
            namespaces.push(namespace);
        }

        ValidatedImportBinding { target, namespaces }
    }

    /// Resolves one name by walking lexical parent scopes.
    ///
    /// This is a low-level lookup primitive: it checks only the requested namespace and stops at
    /// the first scope with a matching binding. It does not apply visibility, statement order, or
    /// path-specific rules such as `crate`, `self`, or `super`.
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
                        return match self.resolve_import_path(
                            import_data.scope,
                            import_data.source_path.iter().copied(),
                        ) {
                            CandidateResolution::Found(_) => ImportResolve::Resolved,
                            CandidateResolution::Ambiguous => ImportResolve::Ambiguous,
                            CandidateResolution::NotFound => ImportResolve::Pending,
                        };
                    }
                    ImportLocalName::Ambiguous => return ImportResolve::Ambiguous,
                    ImportLocalName::Pending => return ImportResolve::Pending,
                };

                let candidates = match self
                    .resolve_import_path(import_data.scope, import_data.source_path.iter().copied())
                {
                    CandidateResolution::Found(candidates) => candidates,
                    CandidateResolution::Ambiguous => return ImportResolve::Ambiguous,
                    CandidateResolution::NotFound => return ImportResolve::Pending,
                };

                let import_binding = self.validate_import_binding_candidates(&candidates);

                // Creates or reuses the local `DefKind::Use` definition that points at the final
                // target, such as the original `C` definition for `use a::b::C`.
                let import_data = import_data.clone();
                let alias = self.get_or_insert_import_def(
                    import_data.scope,
                    Some(local_name),
                    import_data.visibility,
                    import_data.origin,
                    import_binding.target,
                );
                for namespace in import_binding.namespaces {
                    self.insert_unique_binding(import_data.scope, namespace, local_name, alias);
                }
                ImportResolve::Resolved
            }
            ImportKind::Glob => {
                let candidates = match self
                    .resolve_import_path(import_data.scope, import_data.source_path.iter().copied())
                {
                    CandidateResolution::Found(candidates) => candidates,
                    CandidateResolution::Ambiguous => return ImportResolve::Ambiguous,
                    CandidateResolution::NotFound => return ImportResolve::Pending,
                };

                let [(_, glob_candidate)] = candidates.as_slice() else {
                    return ImportResolve::Ambiguous;
                };

                let glob_target = self.follow_aliases(*glob_candidate);
                let Some(path_scope) = self[glob_target].scopes.path else {
                    return ImportResolve::Pending;
                };

                let mut visible = Vec::new();
                for namespace in Namespace::all() {
                    for (&name, binding) in self[path_scope].bindings.map(namespace) {
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

                    let candidates =
                        match self.resolve_import_path(import.scope, parent.iter().copied()) {
                            CandidateResolution::Found(candidates) => candidates,
                            CandidateResolution::Ambiguous => return ImportLocalName::Ambiguous,
                            CandidateResolution::NotFound => return ImportLocalName::Pending,
                        };

                    let import_binding = self.validate_import_binding_candidates(&candidates);

                    let Some(name) = self[import_binding.target].name else {
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
    fn resolve_import_path(
        &self,
        scope: ScopeId,
        mut path: impl ExactSizeIterator<Item = Name<'cx>>,
    ) -> CandidateResolution {
        if path.len() == 0 {
            return CandidateResolution::NotFound;
        }

        let use_scope = scope;
        let mut current_scope = self.nearest_module_scope(scope);
        let mut current_def = None;
        let mut index = 0;

        while let Some(segment) = path.next() {
            let is_last = path.len() == 0;

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
                            .map(|def| CandidateResolution::Found(vec![(Namespace::Type, def)]))
                            .unwrap_or(CandidateResolution::NotFound);
                    }
                    index += 1;
                    continue;
                }
                "super" => {
                    let Some(parent) = self.parent_module_scope(current_scope) else {
                        return CandidateResolution::NotFound;
                    };
                    current_scope = parent;
                    current_def = None;
                    index += 1;
                    continue;
                }
                _ => {}
            }

            let candidates = if is_last {
                self.resolve_name_in_all_namespaces(current_scope, segment, index == 0)
            } else {
                self.resolve_name_in_namespace(current_scope, Namespace::Type, segment, index == 0)
            };

            let candidates = match candidates {
                CandidateResolution::Found(candidates) => candidates,
                other => return other,
            };

            let visible = candidates
                .into_iter()
                .filter(|(_, def)| self.is_visible_from(*def, use_scope))
                .collect::<Vec<_>>();

            if visible.is_empty() {
                return CandidateResolution::NotFound;
            }

            if is_last {
                return CandidateResolution::Found(visible);
            }

            let [(_, def)] = visible.as_slice() else {
                return CandidateResolution::Ambiguous;
            };

            let target = self.follow_aliases(*def);
            let Some(path_scope) = self[target].scopes.path else {
                return CandidateResolution::NotFound;
            };
            current_scope = path_scope;
            current_def = Some(target);
            index += 1;
        }

        CandidateResolution::NotFound
    }

    /// Resolves one name across all namespaces.
    ///
    /// This is used for terminal import path segments, where a name can legally resolve in more
    /// than one namespace. For example, an enum variant can produce both type and value namespace
    /// candidates that point at the same definition.
    fn resolve_name_in_all_namespaces(
        &self,
        scope: ScopeId,
        name: Name<'cx>,
        is_lexical: bool,
    ) -> CandidateResolution {
        let mut defs = Vec::new();
        for namespace in Namespace::all() {
            match self.resolve_name_in_namespace(scope, namespace, name, is_lexical) {
                CandidateResolution::Found(found) => defs.extend(found),
                CandidateResolution::Ambiguous => return CandidateResolution::Ambiguous,
                CandidateResolution::NotFound => {}
            }
        }

        if defs.is_empty() {
            CandidateResolution::NotFound
        } else {
            defs.sort_by_key(|(_, def)| *def);
            defs.dedup();
            CandidateResolution::Found(defs)
        }
    }

    /// Resolves one name in one namespace for an import path segment.
    ///
    /// When `is_lexical` is true, lookup walks parent scopes with [`Self::resolve_lexical`]. When
    /// false, lookup checks only the current scope's binding map. Import paths use lexical lookup
    /// for their first ordinary segment and current-scope lookup after descending into a child
    /// scope.
    fn resolve_name_in_namespace(
        &self,
        scope: ScopeId,
        namespace: Namespace,
        name: Name<'cx>,
        is_lexical: bool,
    ) -> CandidateResolution {
        if is_lexical {
            match self.resolve_lexical(scope, namespace, name) {
                ResolveResult::Found(def) => CandidateResolution::Found(vec![(namespace, def)]),
                ResolveResult::Ambiguous(_) => CandidateResolution::Ambiguous,
                ResolveResult::NotFound => CandidateResolution::NotFound,
            }
        } else {
            self[scope]
                .bindings
                .get(namespace, name)
                .map(|binding| {
                    let mut defs = binding.iter();
                    match defs.len() {
                        0 => CandidateResolution::NotFound,
                        1 => CandidateResolution::Found(vec![(namespace, defs.next().unwrap())]),
                        _ => CandidateResolution::Ambiguous,
                    }
                })
                .unwrap_or(CandidateResolution::NotFound)
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

impl Default for NameDb<'_> {
    /// Creates a name database with a crate-root scope.
    fn default() -> Self {
        let root_scope = Scope::new(ScopeId::new(0), ScopeKind::CrateRoot, None);

        Self {
            scopes: vec![root_scope],
            defs: Vec::new(),
            imports: Vec::new(),
            ast_defs: Map::default(),
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

/// Validated information for binding one resolved import locally.
///
/// `target` is the final non-`Use` definition after following aliases, such as a module, struct,
/// enum, function, or variant. `namespaces` lists where the import should be bound locally.
///
/// For example, `use a::E::V` for an enum variant can validate to target `V` with both the type
/// and value namespaces.
struct ValidatedImportBinding {
    target: DefId,
    namespaces: Vec<Namespace>,
}

/// Result of resolving namespace-tagged definition candidates.
///
/// For example, resolving `V` in `use a::E::V` can find the same enum variant in both the type
/// and value namespaces, producing `Found([(Type, V), (Value, V)])`.
enum CandidateResolution {
    /// Lookup found one or more namespace-tagged candidates.
    Found(Vec<(Namespace, DefId)>),

    /// Lookup found multiple candidates where a single candidate was required.
    Ambiguous,

    /// Lookup found no candidates.
    NotFound,
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

    mod lexical_resolution {
        use super::*;

        // Covers lexical lookup from this code shape:
        //
        // let x = outer;
        //
        // fn f() {
        //     let x = inner;
        //     x
        // }
        //
        // Lookup from the function body finds the inner `x`; lookup from root finds the outer `x`.
        #[test]
        fn lexical_resolution_prefers_inner_scope() {
            let ccx = CommonCx::default();
            let x = ccx.intern("x");

            let mut db = NameDb::default();
            let root = db.root_scope();
            let body = db.add_scope(ScopeKind::Function, Some(root));

            let outer = db.add_def(
                root,
                DefKind::Local,
                Some(x),
                Visibility::Private,
                Origin::Untracked,
            );
            let inner = db.add_def(
                body,
                DefKind::Local,
                Some(x),
                Visibility::Private,
                Origin::Untracked,
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

        // Covers lexical lookup from this code shape:
        //
        // fn f<T>() {
        //     let _: T;
        // }
        //
        // The function body can see names declared in the generic-parameter scope.
        #[test]
        fn generic_scope_is_visible_from_function_body() {
            let ccx = CommonCx::default();
            let t = ccx.intern("T");

            let mut db = NameDb::default();
            let root = db.root_scope();
            let generic_scope = db.add_scope(ScopeKind::Generic, Some(root));
            let body = db.add_scope(ScopeKind::Function, Some(generic_scope));

            let type_param = db.add_def(
                generic_scope,
                DefKind::GenericType,
                Some(t),
                Visibility::Private,
                Origin::Untracked,
            );

            assert_eq!(
                db.resolve_lexical(body, Namespace::Type, t),
                ResolveResult::Found(type_param)
            );
            assert_eq!(
                db.resolve_type_path(body, [t].into_iter()),
                ResolveResult::Found(type_param)
            );
        }

        // Covers lexical lookup from this code shape:
        //
        // fn f() {
        //     {
        //         struct Local;
        //         let _: Local;
        //     }
        // }
        //
        // The block-local item is visible from the same block in the type namespace.
        #[test]
        fn local_item_can_be_resolved_in_type_namespace() {
            let ccx = CommonCx::default();
            let local = ccx.intern("Local");

            let mut db = NameDb::default();
            let root = db.root_scope();
            let body = db.add_scope(ScopeKind::Function, Some(root));
            let block = db.add_scope(ScopeKind::Block, Some(body));

            let local_struct = db.add_def(
                block,
                DefKind::Struct,
                Some(local),
                Visibility::Private,
                Origin::Untracked,
            );

            assert_eq!(
                db.resolve_lexical(block, Namespace::Type, local),
                ResolveResult::Found(local_struct)
            );
            assert_eq!(
                db.resolve_type_path(block, [local].into_iter()),
                ResolveResult::Found(local_struct)
            );
        }
    }

    mod namespaces {
        use super::*;

        // Covers namespace lookup from this test DB state:
        //
        // type namespace:
        //     T -> type parameter
        //
        // value namespace:
        //     T -> local binding
        //
        // The same spelling can resolve to different definitions in different namespaces.
        #[test]
        fn namespaces_are_independent() {
            let ccx = CommonCx::default();
            let t = ccx.intern("T");

            let mut db = NameDb::default();
            let root = db.root_scope();
            let type_param = db.add_def(
                root,
                DefKind::GenericType,
                Some(t),
                Visibility::Private,
                Origin::Untracked,
            );
            let local = db.add_def(
                root,
                DefKind::Local,
                Some(t),
                Visibility::Private,
                Origin::Untracked,
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

        // Covers namespace lookup from this code shape:
        //
        // fn f<const N: usize>() {
        //     let _ = N;
        // }
        //
        // The const parameter `N` lives in the value namespace, not the type namespace.
        #[test]
        fn const_generic_lives_in_value_namespace() {
            let ccx = CommonCx::default();
            let n = ccx.intern("N");

            let mut db = NameDb::default();
            let root = db.root_scope();
            let generic_scope = db.add_scope(ScopeKind::Generic, Some(root));

            let const_param = db.add_def(
                generic_scope,
                DefKind::GenericConst,
                Some(n),
                Visibility::Private,
                Origin::Untracked,
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
    }

    mod members {
        use super::*;

        // Covers direct member lookup from this code shape:
        //
        // trait Iterator {
        //     type Item;
        // }
        //
        // Upper phases can ask the trait owner for a type-namespace member without inspecting
        // `DefScopes::path` or raw scope bindings directly.
        #[test]
        fn resolves_direct_members_from_definition_path_scope() {
            let ccx = CommonCx::default();
            let iterator = ccx.intern("Iterator");
            let item = ccx.intern("Item");

            let mut db = NameDb::default();
            let root = db.root_scope();
            let iterator_def = db.add_def(
                root,
                DefKind::Trait,
                Some(iterator),
                Visibility::Private,
                Origin::Untracked,
            );
            let iterator_scope = db.add_scope(ScopeKind::Trait, Some(root));
            db.set_path_scope(iterator_def, iterator_scope);
            let item_def = db.add_def(
                iterator_scope,
                DefKind::AssocType,
                Some(item),
                Visibility::Private,
                Origin::Untracked,
            );

            assert_eq!(
                db.member(iterator_def, Namespace::Type, item),
                ResolveResult::Found(item_def)
            );
            assert_eq!(
                db.member(iterator_def, Namespace::Value, item),
                ResolveResult::NotFound
            );
        }

        #[test]
        fn member_lookup_without_path_scope_is_not_found() {
            let ccx = CommonCx::default();
            let unit = ccx.intern("Unit");
            let item = ccx.intern("Item");

            let mut db = NameDb::default();
            let unit_def = db.add_def(
                db.root_scope(),
                DefKind::Struct,
                Some(unit),
                Visibility::Private,
                Origin::Untracked,
            );

            assert_eq!(
                db.member(unit_def, Namespace::Type, item),
                ResolveResult::NotFound
            );
        }
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
            Origin::Untracked,
        );
        let scope = db.add_scope(ScopeKind::Module, Some(parent));
        db.set_path_scope(def, scope);
        (def, scope)
    }

    fn target_kind<'cx>(
        db: &NameDb<'cx>,
        scope: ScopeId,
        namespace: Namespace,
        name: Name<'cx>,
    ) -> DefKind {
        let ResolveResult::Found(def) = db.resolve_lexical(scope, namespace, name) else {
            panic!("expected {name:?} to resolve in {namespace:?}");
        };
        db[db.follow_aliases(def)].kind
    }

    mod import_resolution {
        use super::*;

        // Covers imports from this module shape:
        //
        // mod a {
        //     pub struct S;
        // }
        //
        // mod b {
        //     use super::a::S;
        //     use super::a::S as T;
        //     use super::a::{self};
        //     use super::a::S as _;
        // }
        //
        // The first three introduce local bindings to the resolved target; `_` does not.
        #[test]
        fn resolves_single_rename_self_and_underscore_imports() {
            let ccx = CommonCx::default();
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
                Origin::Untracked,
            );
            db.add_import(
                b_scope,
                vec![ccx.intern("super"), a, s],
                ImportKind::Single,
                Visibility::Private,
                Origin::Untracked,
            );
            db.add_import(
                b_scope,
                vec![ccx.intern("super"), a, s],
                ImportKind::Rename(t),
                Visibility::Private,
                Origin::Untracked,
            );
            db.add_import(
                b_scope,
                vec![ccx.intern("super"), a, ccx.intern("self")],
                ImportKind::Single,
                Visibility::Private,
                Origin::Untracked,
            );
            db.add_import(
                b_scope,
                vec![ccx.intern("super"), a, s],
                ImportKind::Rename(hidden),
                Visibility::Private,
                Origin::Untracked,
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

        // Covers imports from this invalid test DB state:
        //
        // mod a {}
        // mod a {} // invalid test DB state: `a` is ambiguous.
        //
        // mod b {
        //     use super::a::{self};
        //     use super::missing::{self};
        // }
        //
        // The first import preserves the ambiguous parent-path failure; the second is not found.
        #[test]
        fn self_import_preserves_parent_path_failure() {
            let ccx = CommonCx::default();
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
                Origin::Untracked,
            );
            db.add_import(
                b_scope,
                vec![ccx.intern("super"), missing, self_name],
                ImportKind::Single,
                Visibility::Private,
                Origin::Untracked,
            );

            db.resolve_imports();

            assert_eq!(db.imports()[0].status, ImportStatus::Ambiguous);
            assert_eq!(db.imports()[1].status, ImportStatus::NotFound);
            assert_eq!(
                db.resolve_lexical(b_scope, Namespace::Type, self_name),
                ResolveResult::NotFound
            );
        }

        // Covers imports from this module shape:
        //
        // mod a {
        //     pub struct Public;
        //     struct Private;
        // }
        //
        // mod b {
        //     pub use super::a::Public;
        // }
        //
        // mod c {
        //     use super::b::Public;
        // }
        //
        // mod d {
        //     use super::a::*;
        // }
        //
        // Chained re-exports resolve to the original target, and globs skip private children.
        #[test]
        fn resolves_chained_reexports_and_globs_with_visibility() {
            let ccx = CommonCx::default();
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
                Origin::Untracked,
            );
            db.add_def(
                a_scope,
                DefKind::Struct,
                Some(private),
                Visibility::Private,
                Origin::Untracked,
            );
            db.add_import(
                b_scope,
                vec![ccx.intern("super"), a, public],
                ImportKind::Single,
                Visibility::Public,
                Origin::Untracked,
            );
            db.add_import(
                c_scope,
                vec![ccx.intern("super"), b, public],
                ImportKind::Single,
                Visibility::Public,
                Origin::Untracked,
            );
            db.add_import(
                d_scope,
                vec![ccx.intern("super"), a],
                ImportKind::Glob,
                Visibility::Private,
                Origin::Untracked,
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

        // Covers imports from this module shape:
        //
        // mod a {
        //     pub struct X;
        // }
        //
        // mod b {
        //     pub struct X;
        // }
        //
        // mod c {
        //     use super::a::*;
        //     use super::b::*;
        //     use super::a::Missing;
        // }
        //
        // The two globs make `X` ambiguous in `c`; `Missing` reports not found.
        #[test]
        fn import_resolution_reports_ambiguity_and_not_found() {
            let ccx = CommonCx::default();
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
                Origin::Untracked,
            );
            db.add_def(
                b_scope,
                DefKind::Struct,
                Some(x),
                Visibility::Public,
                Origin::Untracked,
            );
            db.add_import(
                c_scope,
                vec![ccx.intern("super"), a],
                ImportKind::Glob,
                Visibility::Private,
                Origin::Untracked,
            );
            db.add_import(
                c_scope,
                vec![ccx.intern("super"), b],
                ImportKind::Glob,
                Visibility::Private,
                Origin::Untracked,
            );
            db.add_import(
                c_scope,
                vec![ccx.intern("super"), a, missing],
                ImportKind::Single,
                Visibility::Private,
                Origin::Untracked,
            );

            db.resolve_imports();

            let ResolveResult::Ambiguous(defs) = db.resolve_lexical(c_scope, Namespace::Type, x)
            else {
                panic!("expected imported globs to make {x:?} ambiguous");
            };
            assert_eq!(defs.len(), 2);
            assert_eq!(db.imports()[2].status, ImportStatus::NotFound);
        }

        // Covers imports from this module shape:
        //
        // mod a {
        //     pub struct X;
        //     pub const X: ();
        // }
        //
        // mod b {
        //     use super::a::X;
        // }
        //
        // This is an invalid DB state for a single import binding: the terminal candidates resolve
        // to different final targets across namespaces, so validation must panic.
        #[test]
        #[should_panic(expected = "import binding candidates must resolve to one final target")]
        fn single_import_panics_when_namespace_candidates_resolve_to_distinct_targets() {
            let ccx = CommonCx::default();
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
                Origin::Untracked,
            );
            db.add_def(
                a_scope,
                DefKind::Const,
                Some(x),
                Visibility::Public,
                Origin::Untracked,
            );
            db.add_import(
                b_scope,
                vec![ccx.intern("super"), a, x],
                ImportKind::Single,
                Visibility::Private,
                Origin::Untracked,
            );

            db.resolve_imports();
        }

        // Covers imports from this module shape:
        //
        // mod a {
        //     pub enum E {
        //         V,
        //     }
        // }
        //
        // mod b {
        //     use super::a::E::V;
        // }
        //
        // The variant `V` is imported into both type and value namespaces while still pointing at
        // one final variant definition.
        #[test]
        fn imported_enum_variant_keeps_type_and_value_namespaces() {
            let ccx = CommonCx::default();
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
                Origin::Untracked,
            );
            let enum_scope = db.add_scope(ScopeKind::Item, Some(a_scope));
            db.set_path_scope(enum_def, enum_scope);
            db.add_def(
                enum_scope,
                DefKind::Variant,
                Some(v),
                Visibility::Public,
                Origin::Untracked,
            );
            db.add_import(
                b_scope,
                vec![ccx.intern("super"), a, e, v],
                ImportKind::Single,
                Visibility::Private,
                Origin::Untracked,
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
}
