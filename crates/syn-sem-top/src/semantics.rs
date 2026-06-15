use crate::TopCx;
use std::fmt;
use syn_sem_hir::Hir;
use syn_sem_name::NameDb;

/// Semantic analysis output for one entry file.
///
/// This is the durable product returned by top-level analysis entry points. It currently contains
/// the collected name-resolution database and will grow to include generated IR, diagnostics, and
/// helper queries.
pub struct Semantics<'tcx> {
    tcx: &'tcx TopCx<'tcx>,
    names: NameDb<'tcx>,
    hir: Hir<'tcx>,
}

impl<'tcx> Semantics<'tcx> {
    /// Creates semantic output from its current phase products.
    pub(crate) fn new(tcx: &'tcx TopCx<'tcx>, names: NameDb<'tcx>, hir: Hir<'tcx>) -> Self {
        Self { tcx, names, hir }
    }

    /// Returns the top-level context that owns this semantic output's interned data.
    pub fn tcx(&self) -> &'tcx TopCx<'tcx> {
        self.tcx
    }

    /// Returns the collected name-resolution database.
    pub fn names(&self) -> &NameDb<'tcx> {
        &self.names
    }

    /// Returns the current HIR container.
    pub fn hir(&self) -> &Hir<'tcx> {
        &self.hir
    }

    /// Consumes this semantic output and returns the collected name-resolution database.
    pub fn into_names(self) -> NameDb<'tcx> {
        self.names
    }

    /// Consumes this semantic output and returns the phase products.
    pub fn into_parts(self) -> (NameDb<'tcx>, Hir<'tcx>) {
        (self.names, self.hir)
    }
}

impl fmt::Debug for Semantics<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Semantics")
            .field("names", &self.names)
            .field("hir", &self.hir)
            .finish_non_exhaustive()
    }
}
