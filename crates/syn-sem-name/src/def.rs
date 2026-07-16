use crate::{DefId, Name, Namespace, ScopeId};
use std::marker::PhantomData;
use syn_sem_ast::{AstNode, AstNodeKind};

/// Named definition known to the resolver.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Def<'cx> {
    /// Definition id.
    pub id: DefId,

    /// Optional interned name.
    pub name: Option<Name<'cx>>,

    /// Definition kind.
    pub kind: DefKind,

    /// Scope that owns this definition.
    pub parent_scope: ScopeId,

    /// Scopes owned by or directly attached to this definition.
    pub scopes: DefScopes,

    /// Definition this definition aliases.
    ///
    /// Import definitions use this to point at their resolved target.
    pub target: Option<DefId>,

    /// Visibility of this definition.
    pub visibility: Visibility,

    /// Source origin associated with this definition.
    pub origin: Origin,
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

/// Visibility attached to a named definition or import.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Visibility {
    /// Visible everywhere.
    Public,

    /// Visible within the given scope and its descendants.
    Restricted(ScopeId),

    /// Visible only according to the current module's private visibility rules.
    Private,
}

/// Source origin associated with a definition or import.
///
/// This is the place where future source mapping can attach diagnostics, go-to-definition
/// information, or incremental invalidation data to `Def` and `Import` entries. For now, the name
/// database records untracked origins because no stable AST-node identity is wired through the
/// integration layer yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Origin {
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
