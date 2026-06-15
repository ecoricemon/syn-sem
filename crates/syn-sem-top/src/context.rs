use crate::Semantics;
use std::path::Path;
use syn_sem_ast::SyntaxCx;
use syn_sem_common::{CommonCx, FilePath, Result};
use syn_sem_hir::HirBuilder;
use syn_sem_name::collect::NameCollector;

/// Top-level orchestration context for extracted `syn-sem` crates.
///
/// `TopCx` owns shared infrastructure and phase contexts. Phase contexts borrow the owned common
/// context through the top-level root, keeping lower-level contexts from owning one another.
pub struct TopCx<'tcx> {
    /// Syntax parsing and source storage context.
    pub syntax: SyntaxCx<'tcx>,

    /// Shared common infrastructure context.
    //
    // `CommonCx` must be dropped last because phase contexts may hold references into it.
    pub common: Box<CommonCx>,
}

impl<'tcx> TopCx<'tcx> {
    /// Reads a physical entry file and analyzes its connected module tree.
    pub fn analyze_file(&'tcx self, entry_path: impl AsRef<Path>) -> Result<Semantics<'tcx>> {
        let entry_path = self.syntax.read_physical_file(entry_path)?;
        self.analyze_entry(entry_path)
    }

    /// Parses a virtual entry file and analyzes its connected module tree.
    pub fn analyze_virtual_file(
        &'tcx self,
        entry_path: &str,
        source_text: &str,
    ) -> Result<Semantics<'tcx>> {
        let entry_path = self.common.insert_virtual_file(entry_path, source_text)?;
        let source_text = self
            .common
            .source_text(entry_path)
            .ok_or_else(|| format!("source file is not stored: {entry_path}"))?;
        self.syntax.parse_virtual_file(entry_path, source_text)?;
        self.analyze_entry(entry_path)
    }

    fn analyze_entry(&'tcx self, entry_path: FilePath<'tcx>) -> Result<Semantics<'tcx>> {
        let name_inputs = self.syntax.collect_module_tree(entry_path)?;
        let names = NameCollector::new(name_inputs).collect(entry_path)?;
        let file = self.syntax.lookup_source(entry_path)?.ast();
        let hir = HirBuilder::new(&names).build(entry_path, file);
        Ok(Semantics::new(self, names, hir))
    }
}

impl<'tcx> Default for TopCx<'tcx> {
    /// Creates a top-level context with owned shared common infrastructure.
    fn default() -> Self {
        let common = Box::new(CommonCx::default());
        // Safety: `SyntaxCx` borrows the boxed common context. The allocation remains stable when
        // `TopCx` moves.
        let ccx_ref = unsafe { std::mem::transmute::<&CommonCx, &'tcx CommonCx>(common.as_ref()) };
        Self {
            syntax: SyntaxCx::new(ccx_ref),
            common,
        }
    }
}
