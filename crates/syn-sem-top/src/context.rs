use crate::Semantics;
use std::path::Path;
use syn_sem_ast::SyntaxCx;
use syn_sem_common::{CommonCx, FilePath, Result, SourceText};

/// Top-level orchestration context for extracted `syn-sem` crates.
///
/// `TopCx` owns shared infrastructure and phase contexts. Phase contexts borrow the owned common
/// context through the top-level root, keeping lower-level contexts from owning one another.
pub struct TopCx<'tcx> {
    pub syntax: SyntaxCx<'tcx>,

    /// Shared common infrastructure context.
    //
    // `CommonCx` must be dropped last because phase contexts may hold references into it.
    pub common: Box<CommonCx>,
}

impl<'tcx> TopCx<'tcx> {
    /// Parses and stores a virtual Rust file, returning its interned file path.
    pub fn insert_virtual_file(
        &'tcx self,
        file_path: FilePath<'tcx>,
        text: SourceText<'tcx>,
    ) -> Result<FilePath<'tcx>> {
        self.syntax.parse_virtual_file(file_path, text)
    }

    /// Analyzes a previously inserted or read entry file.
    pub fn analyze(&'tcx self, entry_path: FilePath<'tcx>) -> Result<Semantics<'tcx>> {
        let file = self.syntax.lookup_source(entry_path)?.ast();
        let names = crate::collect_names_in_top(self, entry_path, file)?;
        Ok(Semantics::new(self, names))
    }

    pub(crate) fn parsed_file_path(&'tcx self, file_path: &Path) -> Option<FilePath<'tcx>> {
        let file_path = file_path.to_string_lossy();
        let file_path = self.common.interner().get(&file_path)?;
        self.syntax.has_source(file_path).then_some(file_path)
    }

    pub(crate) fn read_physical_file(&'tcx self, file_path: &Path) -> Result<FilePath<'tcx>> {
        let file_path = file_path.canonicalize()?;
        let file_path = self.common.intern_path(&file_path);
        if self.syntax.has_source(file_path) {
            return Ok(file_path);
        }

        let text = std::fs::read_to_string(&*file_path)?;
        self.syntax
            .parse_physical_file(file_path, self.common.intern(&text))
    }
}

impl<'tcx> Default for TopCx<'tcx> {
    /// Creates a top-level context with owned shared common infrastructure.
    fn default() -> Self {
        let common = Box::new(CommonCx::new());
        // Safety: `SyntaxCx` borrows the boxed common context. The allocation remains stable when
        // `TopCx` moves.
        let ccx_ref = unsafe { std::mem::transmute::<&CommonCx, &'tcx CommonCx>(common.as_ref()) };
        Self {
            syntax: SyntaxCx::new(ccx_ref),
            common,
        }
    }
}
