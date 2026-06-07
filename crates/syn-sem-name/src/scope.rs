use crate::{DefId, Map, Name, Namespace, ScopeId};

/// Lexical scope with namespace-partitioned bindings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scope<'cx> {
    /// Scope id.
    pub id: ScopeId,

    /// Parent scope used for lexical lookup.
    pub parent: Option<ScopeId>,

    /// Scope kind.
    pub kind: ScopeKind,

    /// Bindings declared in this scope.
    pub bindings: Bindings<'cx>,
}

impl<'cx> Scope<'cx> {
    /// Creates an empty scope.
    pub(crate) fn new(id: ScopeId, kind: ScopeKind, parent: Option<ScopeId>) -> Self {
        Self {
            id,
            parent,
            kind,
            bindings: Bindings::new(),
        }
    }
}

/// Kind of lexical or item scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScopeKind {
    /// Crate root scope.
    CrateRoot,

    /// Module scope.
    Module,

    /// Item body or item member scope.
    Item,

    /// Scope containing generic type, const, and lifetime parameter bindings.
    Generic,

    /// Function scope containing parameter bindings and enclosing the function block scope.
    Function,

    /// Block expression or statement block scope.
    Block,

    /// Impl block scope.
    Impl,

    /// Trait item scope.
    Trait,
}

/// Namespace-partitioned bindings for a scope.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Bindings<'cx> {
    types: Map<Name<'cx>, Binding>,
    values: Map<Name<'cx>, Binding>,
    macros: Map<Name<'cx>, Binding>,
    lifetimes: Map<Name<'cx>, Binding>,
}

impl<'cx> Bindings<'cx> {
    /// Returns the binding for `name` in `namespace`.
    pub fn get(&self, namespace: Namespace, name: Name<'cx>) -> Option<&Binding> {
        self.map(namespace).get(&name)
    }

    /// Returns the map for `namespace`.
    pub fn map(&self, namespace: Namespace) -> &Map<Name<'cx>, Binding> {
        match namespace {
            Namespace::Type => &self.types,
            Namespace::Value => &self.values,
            Namespace::Macro => &self.macros,
            Namespace::Lifetime => &self.lifetimes,
        }
    }

    /// Creates empty bindings.
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Inserts a definition under `name` in `namespace`.
    pub(crate) fn insert(&mut self, namespace: Namespace, name: Name<'cx>, def: DefId) {
        self.map_mut(namespace).entry(name).or_default().push(def);
    }

    /// Inserts a definition unless the same definition is already bound there, then returns true
    /// if the given definition is successfully inserted.
    pub(crate) fn insert_unique(
        &mut self,
        namespace: Namespace,
        name: Name<'cx>,
        def: DefId,
    ) -> bool {
        let binding = self.map_mut(namespace).entry(name).or_default();
        if binding.contains(def) {
            false
        } else {
            binding.push(def);
            true
        }
    }

    /// Returns the mutable map for `namespace`.
    pub(crate) fn map_mut(&mut self, namespace: Namespace) -> &mut Map<Name<'cx>, Binding> {
        match namespace {
            Namespace::Type => &mut self.types,
            Namespace::Value => &mut self.values,
            Namespace::Macro => &mut self.macros,
            Namespace::Lifetime => &mut self.lifetimes,
        }
    }
}

/// Binding for one name in one namespace.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Binding {
    defs: Vec<DefId>,
}

impl Binding {
    /// Iterates definitions attached to this binding.
    pub fn iter(&self) -> impl ExactSizeIterator<Item = DefId> + '_ {
        self.defs.iter().copied()
    }

    /// Returns the only definition when this binding is unambiguous.
    pub fn single(&self) -> Option<DefId> {
        match self.defs.as_slice() {
            [def] => Some(*def),
            _ => None,
        }
    }

    /// Returns whether the binding has more than one candidate.
    pub fn is_ambiguous(&self) -> bool {
        self.defs.len() > 1
    }

    /// Returns whether the binding has no definitions.
    pub fn is_empty(&self) -> bool {
        self.defs.is_empty()
    }

    /// Appends a definition to this binding.
    pub(crate) fn push(&mut self, def: DefId) {
        self.defs.push(def);
    }

    /// Returns whether this binding already contains `def`.
    pub(crate) fn contains(&self, def: DefId) -> bool {
        self.defs.contains(&def)
    }
}
