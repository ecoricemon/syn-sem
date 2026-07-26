use crate::TopCx;
use std::fmt;
use syn_sem_eval::EvalDb;
use syn_sem_hir::Hir;
use syn_sem_infer::InferDb;
use syn_sem_name::NameDb;

/// Semantic analysis output for one entry file.
///
/// This is the durable product returned by top-level analysis entry points. It keeps the current
/// extracted phase products together so callers can query source-shaped HIR, name facts,
/// inference facts, and constant values from one analysis result.
pub struct Semantics<'tcx> {
    tcx: &'tcx TopCx<'tcx>,
    names: NameDb<'tcx>,
    hir: Hir<'tcx>,
    infer: InferDb<'tcx>,
    eval: EvalDb,
}

impl<'tcx> Semantics<'tcx> {
    /// Creates semantic output from its current phase products.
    pub(crate) fn new(
        tcx: &'tcx TopCx<'tcx>,
        names: NameDb<'tcx>,
        hir: Hir<'tcx>,
        infer: InferDb<'tcx>,
        eval: EvalDb,
    ) -> Self {
        Self {
            tcx,
            names,
            hir,
            infer,
            eval,
        }
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

    /// Returns the collected type-inference database.
    pub fn infer(&self) -> &InferDb<'tcx> {
        &self.infer
    }

    /// Returns the collected constant-evaluation database.
    pub fn eval(&self) -> &EvalDb {
        &self.eval
    }
}

impl fmt::Debug for Semantics<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Semantics")
            .field("names", &self.names)
            .field("hir", &self.hir)
            .field("infer", &self.infer)
            .field("eval", &self.eval)
            .finish_non_exhaustive()
    }
}
