use crate::{DefId, Name, Namespace, ScopeId};
use std::marker::PhantomData;
use syn_sem_ast::{AstNode, AstNodeKind};

/// One declaration or binding known to the name resolver.
///
/// A definition represents the identity introduced by a declaration, not every source occurrence
/// of its name. Items, generic parameters, function parameters, local pattern bindings, and other
/// declaration-like constructs create definitions. A path that refers to one of those constructs
/// does not create another `Def`; resolving the path returns the [`DefId`] of the existing
/// definition. For example:
///
/// ```text
/// const A: usize = 1;
/// const B: usize = A + 1;
/// ```
///
/// Collection creates one `DefKind::Const` definition for `A` and one for `B`. The `A` in the
/// initializer of `B` is a use-site path. Resolving that path from its lexical scope and the value
/// namespace returns the same `DefId` that belongs to the declaration of `A`:
///
/// ```text
/// declaration `A` ──creates──────> DefId(A)
/// path use `A`    ──resolves to──> DefId(A)
/// declaration `B` ──creates──────> DefId(B)
/// ```
///
/// Imports introduce alias definitions rather than duplicating the imported declaration. For
/// example:
///
/// ```text
/// mod m { pub const A: usize = 1; }
/// use m::A as X;
/// const B: usize = X + 1;
/// ```
///
/// The original declaration creates `Def(A)`. Resolving the `use` creates a local `DefKind::Use`
/// definition named `X` whose [`Def::target`] points to `Def(A)`:
///
/// ```text
/// local binding `X` -> Def(X, kind = Use) -> Def(A, kind = Const)
/// path use `X`      -> resolve and follow aliases -> Def(A)
/// ```
///
/// [`crate::Import`] remains the source-level record for the `use` declaration and its resolution
/// status, while the `DefKind::Use` definition is the alias bound in the receiving scope.
/// [`crate::NameDb::follow_aliases`] follows one or more such aliases to the underlying non-`Use`
/// definition. Alias-following name-resolution queries therefore return `Def(A)`, rather than
/// treating `X` as an independent declaration. Renamed imports, public re-exports, and glob imports
/// use the same alias model. Underscore imports such as `use m::A as _;` introduce no local alias
/// binding.
///
/// Identity is therefore distinct from spelling. Two definitions may have the same name in
/// different scopes, and one definition may be referenced by many path occurrences. Resolution
/// selects a definition using the use site's scope, namespace, visibility, and imports.
///
/// A `Def` also records the scopes attached to the declaration, its visibility domain, source
/// origin, and optional alias target. Its [`DefId`] is meaningful only with the [`crate::NameDb`]
/// that created it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Def<'cx> {
    /// Database-local identity of this declaration or binding.
    ///
    /// Name resolution returns this id for every use that selects this definition.
    pub id: DefId,

    /// Interned declared name, when this definition has one.
    ///
    /// Unnamed constructs such as `impl` blocks leave this unset.
    pub name: Option<Name<'cx>>,

    /// Kind of declaration or binding represented by this definition.
    pub kind: DefKind,

    /// Scope in which this definition is declared and bound.
    pub parent_scope: ScopeId,

    /// Scopes owned by or directly attached to this definition.
    pub scopes: DefScopes,

    /// Definition this definition aliases.
    ///
    /// A [`DefKind::Use`] definition uses this to point at the definition imported into its local
    /// scope. Resolution follows this link with [`crate::NameDb::follow_aliases`] so callers can
    /// recover the original declaration through renamed imports and chains of re-exports. Ordinary
    /// declarations leave this unset.
    pub target: Option<DefId>,

    /// Outermost scope from which this definition is visible.
    ///
    /// A use-site must be within this visibility domain in addition to finding the definition
    /// through its applicable namespace and scope chain.
    pub visibility: ScopeId,

    /// Source declaration that created this definition, when tracked.
    pub origin: Origin<'cx>,
}

/// Scopes owned by or directly attached to a definition.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DefScopes {
    /// Scope used when path resolution descends through this definition.
    ///
    /// This is for names reachable with paths or imports. For example, `enum E { V }` gives the
    /// `E` definition a path scope containing variant `V`, so `use crate::E::V;` can descend from
    /// `E` into that scope. Definitions without path-reachable children leave this unset.
    pub path: Option<ScopeId>,

    /// Scope containing lexical generic parameters owned by this definition.
    ///
    /// For example, `fn f<T>() {}` gives the `f` definition a generic scope containing `T`.
    /// Generic parameters are lexical names, not path-reachable children, so this is separate from
    /// [`Self::path`]. Definitions without generic parameters leave this unset.
    pub generic: Option<ScopeId>,

    /// Scope containing the value body owned by this definition.
    ///
    /// For example, `fn f(x: i32) { let y = x; }` gives the `f` definition a body scope containing
    /// parameter binding `x`; the block inside the body then gets its own nested block scope for
    /// names such as `y`. Definitions without a collected body leave this unset.
    pub body: Option<ScopeId>,
}

/// Kind of definition stored in the name database.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DefKind {
    /// A Rust module.
    Module,

    /// A struct item.
    Struct,

    /// An enum item.
    Enum,

    /// An enum variant.
    Variant,

    /// A trait item.
    Trait,

    /// A type alias item.
    TypeAlias,

    /// A function item.
    Fn,

    /// A constant item.
    Const,

    /// An associated type item.
    AssocType,

    /// An associated function item.
    AssocFn,

    /// An associated constant item.
    AssocConst,

    /// A static item.
    Static,

    /// A field.
    Field,

    /// A local variable or pattern binding.
    Local,

    /// A generic type parameter.
    GenericType,

    /// A generic const parameter.
    GenericConst,

    /// A generic lifetime parameter.
    GenericLifetime,

    /// A `use` item or imported binding.
    Use,

    /// An impl block.
    Impl,

    /// A macro definition.
    Macro,
}

impl DefKind {
    /// Returns the default namespaces populated by this definition kind.
    pub const fn namespaces(self) -> &'static [Namespace] {
        match self {
            // Type namespace
            Self::Module
            | Self::Struct
            | Self::Enum
            | Self::Trait
            | Self::TypeAlias
            | Self::AssocType
            | Self::GenericType => &[Namespace::Type],

            // Type & Value namespaces
            Self::Variant => &[Namespace::Type, Namespace::Value],

            // Value namespace
            Self::Fn
            | Self::Const
            | Self::AssocFn
            | Self::AssocConst
            | Self::Static
            | Self::Local
            | Self::GenericConst => &[Namespace::Value],

            // Lifetime namespace
            Self::GenericLifetime => &[Namespace::Lifetime],

            // Macro namespace
            Self::Macro => &[Namespace::Macro],

            // None
            Self::Field | Self::Use | Self::Impl => &[],
        }
    }
}

/// Source origin associated with a definition or import.
///
/// Entries collected from semantic AST nodes retain that identity, while synthetic or externally
/// constructed entries can remain untracked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Origin<'cx> {
    /// Entry created from an AST declaration node.
    Ast(AstNodeId<'cx>),

    /// Source origin is not tracked for this entry.
    Untracked,
}

/// AST node identity associated with a definition.
///
/// This key intentionally avoids depending on `syn-sem-ast`. Integration layers can create keys
/// from their own AST arenas and hand them to the name database, which then owns the lookup from an
/// AST declaration node to [`DefId`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AstNodeId<'cx> {
    ptr: usize,
    kind: AstNodeKind,
    _marker: PhantomData<&'cx ()>,
}

impl<'cx> AstNodeId<'cx> {
    /// Creates an AST node identity from an arena-backed AST node.
    pub fn from_ref<T: AstNode>(node: &'cx T) -> Self {
        Self {
            ptr: (node as *const T).cast::<()>() as usize,
            kind: T::KIND,
            _marker: PhantomData,
        }
    }
}
