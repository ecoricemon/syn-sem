use std::{fmt::Display, path::Path};
use syn_sem_ast::{File, FromSyn, InputDesc, SyntaxCx};
use syn_sem_common::{CommonCx, FilePath, InternedStr, Result};
use syn_sem_name::NameDb;

/// Top-level orchestration context for extracted `syn-sem` crates.
///
/// `TopCx` owns shared infrastructure and phase contexts. Phase contexts borrow the owned common
/// context through the top-level root, keeping lower-level contexts from owning one another.
pub struct TopCx<'tcx> {
    syntax: SyntaxCx<'tcx>,

    /// Shared common infrastructure context.
    //
    // `CommonCx` must be dropped last because phase contexts may hold references into it.
    pub ccx: Box<CommonCx>,
}

impl<'tcx> TopCx<'tcx> {
    /// Creates a top-level context from existing shared common infrastructure.
    pub fn with_common(ccx: CommonCx) -> Self {
        let ccx = Box::new(ccx);
        // Safety: `SyntaxCx` borrows the boxed common context. The allocation remains stable when
        // `TopCx` moves.
        let ccx_ref = unsafe { std::mem::transmute::<&CommonCx, &'tcx CommonCx>(ccx.as_ref()) };
        Self {
            syntax: SyntaxCx::new(ccx_ref),
            ccx,
        }
    }

    /// Returns the semantic syntax context.
    pub fn syntax(&self) -> &SyntaxCx<'tcx> {
        &self.syntax
    }

    /// Interns a string through the shared common context.
    pub fn intern(&'tcx self, value: &str) -> InternedStr<'tcx> {
        self.ccx.intern(value)
    }

    /// Interns a formatted value through the shared common context.
    pub fn intern_display<K: Display + ?Sized>(
        &'tcx self,
        value: &K,
        upper_size: usize,
    ) -> Result<InternedStr<'tcx>> {
        self.syntax.intern_display(value, upper_size)
    }

    /// Parses and stores a virtual Rust file, returning its interned file path.
    pub fn parse_virtual_file(&'tcx self, file_path: &str, text: &str) -> Result<FilePath<'tcx>> {
        self.syntax.parse_virtual_file(file_path, text)
    }

    /// Parses and stores a physical Rust file, returning its interned file path.
    pub fn parse_physical_file(&'tcx self, file_path: &str, text: &str) -> Result<FilePath<'tcx>> {
        self.syntax.parse_physical_file(file_path, text)
    }

    /// Reads, parses, and stores a physical Rust file, returning its interned file path.
    pub fn read_physical_file(&'tcx self, file_path: impl AsRef<Path>) -> Result<FilePath<'tcx>> {
        let file_path = file_path.as_ref().canonicalize()?;
        let file_path = file_path.to_string_lossy();
        let interned_file_path = self.ccx.intern(&file_path);
        if self.syntax.get_source(interned_file_path).is_some() {
            return Ok(interned_file_path);
        }

        let text = std::fs::read_to_string(&*file_path)?;
        self.parse_physical_file(&file_path, &text)
    }

    /// Builds the semantic AST for a previously parsed file.
    pub fn ast_file(&'tcx self, file_path: FilePath<'tcx>) -> Result<File<'tcx>> {
        let source = self.syntax.get_source(file_path).ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("source file is not parsed: {file_path}"),
            )
        })?;
        let input = source.syntax::<syn::File>().ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("source file does not contain a syn::File: {file_path}"),
            )
        })?;

        Ok(File::from_syn(&self.syntax, InputDesc { file_path, input }))
    }

    /// Collects the name-resolution database for a previously parsed file.
    pub fn names_for_file(&'tcx self, file_path: FilePath<'tcx>) -> Result<NameDb<'tcx>> {
        let file = self.ast_file(file_path)?;
        crate::collect_names_in_top(self, file_path, &file)
    }

    /// Parses a virtual Rust file and collects its name-resolution database.
    pub fn parse_virtual_names(&'tcx self, file_path: &str, text: &str) -> Result<NameDb<'tcx>> {
        let file_path = self.parse_virtual_file(file_path, text)?;
        self.names_for_file(file_path)
    }

    /// Reads a physical Rust file and collects its name-resolution database.
    pub fn read_physical_names(&'tcx self, file_path: impl AsRef<Path>) -> Result<NameDb<'tcx>> {
        let file_path = self.read_physical_file(file_path)?;
        self.names_for_file(file_path)
    }
}

impl Default for TopCx<'_> {
    /// Creates a top-level context with owned shared common infrastructure.
    fn default() -> Self {
        Self::with_common(CommonCx::new())
    }
}
