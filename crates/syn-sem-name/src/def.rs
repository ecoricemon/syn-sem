use crate::{DefId, Name, Namespace, ScopeId};

/// Named definition known to the resolver.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Def<'cx> {
    /// Definition id.
    pub id: DefId,

    /// Optional interned name.
    pub name: Option<Name<'cx>>,

    /// Definition kind.
    pub kind: DefKind,

    /// Scope that owns this definition.
    pub parent_scope: ScopeId,

    /// Scope containing this definition's importable children.
    ///
    /// Modules and item-like definitions such as enums can expose names through a child scope.
    /// Definitions without importable children leave this unset.
    pub child_scope: Option<ScopeId>,

    /// Definition this definition aliases.
    ///
    /// Import definitions use this to point at their resolved target.
    pub target: Option<DefId>,

    /// Visibility of this definition.
    pub visibility: Visibility,

    /// Source origin associated with this definition.
    pub origin: Origin,
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

    /// A function or method.
    Fn,

    /// A constant item or associated constant.
    Const,

    /// A static item.
    Static,

    /// A field.
    Field,

    /// A local variable or pattern binding.
    Local,

    /// A type generic parameter.
    TypeParam,

    /// A const generic parameter.
    ConstParam,

    /// A lifetime generic parameter.
    LifetimeParam,

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
            Self::Module
            | Self::Struct
            | Self::Enum
            | Self::Trait
            | Self::TypeAlias
            | Self::TypeParam => &[Namespace::Type],
            Self::Variant => &[Namespace::Type, Namespace::Value],
            Self::Fn | Self::Const | Self::Static | Self::Local | Self::ConstParam => {
                &[Namespace::Value]
            }
            Self::LifetimeParam => &[Namespace::Lifetime],
            Self::Macro => &[Namespace::Macro],
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
/// The name crate deliberately does not depend on a concrete AST crate. Users can store their own
/// stable node index here and interpret it at the integration boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Origin {
    /// No source node is associated with this entry.
    Synthetic,

    /// Opaque AST node index owned by the caller.
    AstNode(usize),
}
