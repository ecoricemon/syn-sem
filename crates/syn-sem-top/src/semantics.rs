use crate::TopCx;
use std::fmt;
use syn_sem_name::NameDb;
use syn_sem_pr::ProgramRepr;

/// Semantic analysis output for one entry file.
///
/// This is the durable product returned by top-level analysis entry points. It currently contains
/// the collected name-resolution database and will grow to include generated IR, diagnostics, and
/// helper queries.
pub struct Semantics<'tcx> {
    tcx: &'tcx TopCx<'tcx>,
    names: NameDb<'tcx>,
    repr: ProgramRepr<'tcx>,
}

impl<'tcx> Semantics<'tcx> {
    /// Creates semantic output from its current phase products.
    pub(crate) fn new(
        tcx: &'tcx TopCx<'tcx>,
        names: NameDb<'tcx>,
        repr: ProgramRepr<'tcx>,
    ) -> Self {
        Self { tcx, names, repr }
    }

    /// Returns the top-level context that owns this semantic output's interned data.
    pub fn tcx(&self) -> &'tcx TopCx<'tcx> {
        self.tcx
    }

    /// Returns the collected name-resolution database.
    pub fn names(&self) -> &NameDb<'tcx> {
        &self.names
    }

    /// Returns the Rust source program representation.
    pub fn repr(&self) -> &ProgramRepr<'tcx> {
        &self.repr
    }

    /// Consumes this semantic output and returns the collected name-resolution database.
    pub fn into_names(self) -> NameDb<'tcx> {
        self.names
    }

    /// Consumes this semantic output and returns the phase products.
    pub fn into_parts(self) -> (NameDb<'tcx>, ProgramRepr<'tcx>) {
        (self.names, self.repr)
    }
}

impl fmt::Debug for Semantics<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Semantics")
            .field("names", &self.names)
            .field("repr", &self.repr)
            .finish_non_exhaustive()
    }
}
