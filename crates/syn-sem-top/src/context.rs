use crate::{NameInputBuilder, Semantics};
use std::{fs, path::Path};
use syn_sem_ast::SyntaxCx;
use syn_sem_common::{CommonCx, FilePath, Result, SourceText};
use syn_sem_name::collect::collect_names;
use syn_sem_pr::ProgramReprBuilder;

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
    /// Parses and stores a virtual Rust file, returning its interned file path.
    pub fn insert_virtual_file(
        &'tcx self,
        file_path: FilePath<'tcx>,
        text: SourceText<'tcx>,
    ) -> Result<()> {
        self.syntax.parse_virtual_file(file_path, text)
    }

    /// Analyzes a previously inserted or read entry file.
    pub fn analyze(&'tcx self, entry_path: FilePath<'tcx>) -> Result<Semantics<'tcx>> {
        let name_inputs = NameInputBuilder::new(self).collect(entry_path)?;
        let names = collect_names(name_inputs, entry_path)?;
        let file = self.syntax.lookup_source(entry_path)?.ast();
        let repr = ProgramReprBuilder::new(&names).build(entry_path, file);
        Ok(Semantics::new(self, names, repr))
    }

    pub(crate) fn has_parsed(&'tcx self, file_path: &Path) -> Option<FilePath<'tcx>> {
        let file_path = self.common.intern_path(file_path);
        self.syntax.has_source(file_path).then_some(file_path)
    }

    pub(crate) fn read_physical_file(&'tcx self, file_path: &Path) -> Result<FilePath<'tcx>> {
        let file_path = file_path.canonicalize()?;
        let file_path = self.common.intern_path(&file_path);
        if self.syntax.has_source(file_path) {
            return Ok(file_path);
        }

        let text = fs::read_to_string(&*file_path)?;
        let text = self.common.intern(&text);
        self.syntax
            .parse_physical_file(file_path, text)
            .map(|()| file_path)
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
