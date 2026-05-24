use crate::{Name, Origin, ScopeId, Visibility};

/// Import declaration collected during name resolution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Import<'cx> {
    /// Import id.
    pub id: crate::ImportId,

    /// Scope that receives the imported binding.
    pub scope: ScopeId,

    /// Path segments naming the import target.
    pub source_path: Vec<Name<'cx>>,

    /// Import kind.
    pub kind: ImportKind<'cx>,

    /// Visibility of the imported binding.
    pub visibility: Visibility,

    /// Current import status.
    pub status: ImportStatus,

    /// Source origin associated with this import.
    pub origin: Origin,
}

/// Kind of import declaration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ImportKind<'cx> {
    /// Imports one named target.
    Single,

    /// Imports one named target under a different local name.
    ///
    /// For example, `use foo::Bar as Baz;` stores `Baz` here.
    Rename(Name<'cx>),

    /// Imports all public names from the target.
    Glob,
}

/// Resolution status for an import.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ImportStatus {
    /// Import has not been resolved yet.
    Unresolved,

    /// Import resolved successfully.
    Resolved,

    /// Import resolution produced an ambiguity.
    Ambiguous,

    /// Import target was not found.
    NotFound,
}
