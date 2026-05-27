use crate::{
    Def, DefId, DefKind, Import, ImportId, ImportKind, ImportStatus, Name, Namespace, Origin,
    Scope, ScopeId, ScopeKind, Visibility,
};
use std::ops::{Index, IndexMut};

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

    /// Resolves a single-segment name lexically in one namespace.
    pub fn resolve_lexical(
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

    /// Returns whether `descendant` is equal to or nested inside `ancestor`.
    pub fn is_descendant_scope(&self, mut descendant: ScopeId, ancestor: ScopeId) -> bool {
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
}

impl Default for NameDb<'_> {
    /// Creates a name database with a crate-root scope.
    fn default() -> Self {
        Self {
            scopes: vec![Scope::new(ScopeId::new(0), ScopeKind::CrateRoot, None)],
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
}
