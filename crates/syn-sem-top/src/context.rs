use crate::Semantics;
use std::{fmt::Display, io, path::Path};
use syn_sem_ast::{File, SyntaxCx};
use syn_sem_common::{CommonCx, FilePath, InternedStr, Result};

/// Top-level orchestration context for extracted `syn-sem` crates.
///
/// `TopCx` owns shared infrastructure and phase contexts. Phase contexts borrow the owned common
/// context through the top-level root, keeping lower-level contexts from owning one another.
pub struct TopCx<'tcx> {
    syntax: SyntaxCx<'tcx>,

    /// Shared common infrastructure context.
    //
    // `CommonCx` must be dropped last because phase contexts may hold references into it.
    common: Box<CommonCx>,
}

impl<'tcx> TopCx<'tcx> {
    /// Interns a string through the shared common context.
    pub fn intern(&'tcx self, value: &str) -> InternedStr<'tcx> {
        self.common.intern(value)
    }

    /// Interns a formatted value through the shared common context.
    pub fn intern_display<K: Display + ?Sized>(
        &'tcx self,
        value: &K,
        upper_size: usize,
    ) -> Result<InternedStr<'tcx>> {
        self.common.intern_display(value, upper_size)
    }

    /// Parses and stores a virtual Rust file, returning its interned file path.
    pub fn insert_virtual_file(&'tcx self, file_path: &str, text: &str) -> Result<FilePath<'tcx>> {
        self.syntax.parse_virtual_file(file_path, text)
    }

    /// Analyzes a previously inserted or read entry file.
    pub fn analyze(&'tcx self, entry_file: FilePath<'tcx>) -> Result<Semantics<'tcx>> {
        Analyzer::new(self).analyze(entry_file)
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

pub(crate) struct Analyzer<'tcx> {
    tcx: &'tcx TopCx<'tcx>,
}

impl<'tcx> Analyzer<'tcx> {
    pub(crate) fn new(tcx: &'tcx TopCx<'tcx>) -> Self {
        Self { tcx }
    }

    fn analyze(mut self, entry_file: FilePath<'tcx>) -> Result<Semantics<'tcx>> {
        let file = self.build_ast_file(entry_file)?;
        let names = crate::collect_names_in_top(&mut self, entry_file, &file)?;
        Ok(Semantics::new(self.tcx, entry_file, names))
    }

    pub(crate) fn build_ast_file(&self, file_path: FilePath<'tcx>) -> Result<File<'tcx>> {
        let source = self.tcx.syntax.get_source(file_path).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("source file is not parsed: {file_path}"),
            )
        })?;
        Ok(source.ast().clone())
    }

    pub(crate) fn parsed_file_path(&self, file_path: impl AsRef<Path>) -> Option<FilePath<'tcx>> {
        let file_path = file_path.as_ref().to_string_lossy();
        let file_path = self.tcx.common.interner().get(&file_path)?;
        self.tcx.syntax.get_source(file_path).map(|_| file_path)
    }

    pub(crate) fn read_physical_file(&self, file_path: impl AsRef<Path>) -> Result<FilePath<'tcx>> {
        let file_path = file_path.as_ref().canonicalize()?;
        let file_path = file_path.to_string_lossy();
        let interned_file_path = self.tcx.common.intern(&file_path);
        if self.tcx.syntax.get_source(interned_file_path).is_some() {
            return Ok(interned_file_path);
        }

        let text = std::fs::read_to_string(&*file_path)?;
        self.tcx.syntax.parse_physical_file(&file_path, &text)
    }
}
