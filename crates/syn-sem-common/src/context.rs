use crate::{FilePath, InternedStr, LibraryName, Map, Result, SourceCode};
use any_intern::DroplessInterner;
use std::{
    fmt::Display,
    fs, io,
    path::{Path, PathBuf},
};

/// Root context for shared `syn-sem` infrastructure.
///
/// `CommonCx` owns the string interner. Values with the `'ccx` lifetime, such as
/// [`FilePath`] and [`SourceCode`], are valid for the lifetime of this context's interner.
#[derive(Debug, Default)]
pub struct CommonCx {
    interner: StringInterner,
}

impl CommonCx {
    /// Creates an empty common context.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the string interner owned by this context.
    pub fn interner(&self) -> &StringInterner {
        &self.interner
    }

    /// Interns a string in this context.
    pub fn intern(&self, text: &str) -> Result<InternedStr<'_>> {
        self.interner.intern(text)
    }
}

/// String-only wrapper around [`any_intern::DroplessInterner`].
///
/// Use this through [`CommonCx`] in most code so interned strings have an obvious owner.
#[derive(Default)]
pub struct StringInterner {
    inner: DroplessInterner,
}

impl StringInterner {
    /// Creates an empty string interner.
    pub fn new() -> Self {
        Self::default()
    }

    /// Interns `text` and returns a lifetime-bearing interned string.
    pub fn intern(&self, text: &str) -> Result<InternedStr<'_>> {
        self.intern_display(text, text.len())
    }

    /// Interns a display value as a formatted string.
    pub fn intern_display<T: Display + ?Sized>(
        &self,
        value: &T,
        upper_size: usize,
    ) -> Result<InternedStr<'_>> {
        self.inner
            .intern_formatted_str(value, upper_size)
            .map_err(|e| format!("failed to intern formatted string: {e}").into())
    }

    /// Returns the interned string if `text` has already been interned.
    pub fn get(&self, text: &str) -> Option<InternedStr<'_>> {
        self.inner.get(text)
    }

    /// Returns the number of interned strings.
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Returns whether no strings have been interned.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}

impl std::fmt::Debug for StringInterner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StringInterner")
            .field("len", &self.len())
            .finish()
    }
}

/// Source text associated with an interned file path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source<'ccx> {
    /// Source loaded from, or representing, a physical absolute file path.
    Physical { code: SourceCode<'ccx> },

    /// Source supplied by the caller without requiring a real filesystem file.
    Virtual { code: SourceCode<'ccx> },
}

impl<'ccx> Source<'ccx> {
    /// Returns the interned source code for this source.
    pub const fn code(self) -> SourceCode<'ccx> {
        match self {
            Self::Physical { code } | Self::Virtual { code } => code,
        }
    }

    /// Returns whether this source is physical.
    pub const fn is_physical(self) -> bool {
        matches!(self, Self::Physical { .. })
    }

    /// Returns whether this source is virtual.
    pub const fn is_virtual(self) -> bool {
        matches!(self, Self::Virtual { .. })
    }
}

/// Abstract file table keyed by interned file paths.
///
/// This type does not own the interner. Its `'ccx` lifetime ties all stored paths and source text
/// to the [`CommonCx`] / [`StringInterner`] that produced them.
#[derive(Debug, Default)]
pub struct AbstractFiles<'ccx> {
    files: Map<FilePath<'ccx>, Source<'ccx>>,
    known_libraries: Map<LibraryName<'ccx>, FilePath<'ccx>>,
}

impl<'ccx> AbstractFiles<'ccx> {
    /// Creates an empty abstract file table.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns whether `file_path` has source in this table.
    pub fn contains(&self, file_path: FilePath<'ccx>) -> bool {
        self.files.contains_key(&file_path)
    }

    /// Returns source metadata for `file_path`.
    pub fn source(&self, file_path: FilePath<'ccx>) -> Option<Source<'ccx>> {
        self.files.get(&file_path).copied()
    }

    /// Inserts caller-provided source text under `file_path`.
    ///
    /// Virtual paths are still interned file path identifiers, but they are not checked against the
    /// filesystem.
    pub fn insert_virtual_file(
        &mut self,
        interner: &'ccx StringInterner,
        file_path: &str,
        code: &str,
    ) -> Result<FilePath<'ccx>> {
        let file_path = interner.intern(file_path)?;
        let code = interner.intern(code)?;
        self.files.insert(file_path, Source::Virtual { code });
        Ok(file_path)
    }

    /// Inserts caller-provided source text for an absolute physical file path.
    ///
    /// This validates that `file_path` is absolute, but does not check whether it exists.
    pub fn insert_physical_file(
        &mut self,
        interner: &'ccx StringInterner,
        file_path: &str,
        code: &str,
    ) -> Result<FilePath<'ccx>> {
        validate_absolute_file_path(file_path)?;

        let file_path = interner.intern(file_path)?;
        let code = interner.intern(code)?;
        self.files.insert(file_path, Source::Physical { code });
        Ok(file_path)
    }

    /// Reads an absolute physical file path from disk and stores its source text.
    ///
    /// The returned path is canonicalized before it is interned.
    pub fn read_physical_file(
        &mut self,
        interner: &'ccx StringInterner,
        file_path: &str,
    ) -> Result<FilePath<'ccx>> {
        let file_path = absolute_file_path(file_path)?;

        if let Some(interned) = interner.get(&file_path) {
            if self.files.contains_key(&interned) {
                return Ok(interned);
            }
        }

        let code = fs::read_to_string(&*file_path)?;
        self.insert_physical_file(interner, &file_path, &code)
    }

    /// Returns source text for `file_path`.
    pub fn code(&self, file_path: FilePath<'ccx>) -> Option<&'ccx str> {
        let code = self.source(file_path)?.code();
        Some(code.0)
    }

    /// Associates a known library name with a file path.
    ///
    /// Names are library identifiers such as `core` or `std`, not paths.
    pub fn set_known_library(
        &mut self,
        interner: &'ccx StringInterner,
        name: &str,
        file_path: FilePath<'ccx>,
    ) -> Result<Option<FilePath<'ccx>>> {
        debug_assert!(
            !name.ends_with(".rs"),
            "expected library name, but received file path-like name `{name}`"
        );

        let name = interner.intern(name)?;
        Ok(self.known_libraries.insert(name, file_path))
    }

    /// Returns the file path associated with a known library name.
    pub fn known_library(
        &self,
        interner: &'ccx StringInterner,
        name: &str,
    ) -> Option<FilePath<'ccx>> {
        let name = interner.get(name)?;
        self.known_libraries.get(&name).copied()
    }

    /// Iterates known library mappings.
    pub fn known_libraries(
        &self,
    ) -> impl ExactSizeIterator<Item = (LibraryName<'ccx>, FilePath<'ccx>)> + '_ {
        self.known_libraries
            .iter()
            .map(|(&name, &file_path)| (name, file_path))
    }
}

/// Validates that `file_path` is a non-empty absolute path.
///
/// This only validates path shape. It does not check filesystem existence.
pub fn validate_absolute_file_path(file_path: &str) -> Result<()> {
    if file_path.is_empty() {
        return Err("file path must not be empty".into());
    }

    let path = Path::new(file_path);
    if !path.is_absolute() {
        return Err(format!("file path must be absolute: {file_path:?}").into());
    }

    Ok(())
}

/// Returns the canonical absolute path for an existing physical file.
pub fn absolute_file_path(file_path: &str) -> Result<Box<str>> {
    validate_absolute_file_path(file_path)?;

    let canonical = PathBuf::from(file_path).canonicalize().map_err(|e| {
        let path = Path::new(file_path).to_string_lossy();
        match e.kind() {
            io::ErrorKind::NotFound => format!("couldn't find `{path}`: {e}"),
            _ => format!("`{path}`: {e}"),
        }
    })?;

    let canonical = canonical
        .to_str()
        .ok_or_else(|| format!("{canonical:?} contains non UTF-8 characters"))?;

    Ok(canonical.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interner_deduplicates_strings() {
        let interner = StringInterner::new();

        let a = interner.intern("hello").unwrap();
        let b = interner.intern("hello").unwrap();
        let c = interner.intern("world").unwrap();

        assert_eq!(a, b);
        assert_ne!(a, c);
        assert_eq!(interner.len(), 2);
        assert_eq!(a.as_ref(), "hello");
    }

    #[test]
    fn abstract_files_stores_virtual_file_code() {
        let ccx = CommonCx::new();
        let mut files = AbstractFiles::new();

        let file_path = files
            .insert_virtual_file(ccx.interner(), "/virtual/main.rs", "fn main() {}")
            .unwrap();

        assert!(files.contains(file_path));
        assert_eq!(files.code(file_path), Some("fn main() {}"));
        assert!(files.source(file_path).unwrap().is_virtual());
    }

    #[test]
    fn abstract_files_stores_physical_file_code_without_reading_disk() {
        let ccx = CommonCx::new();
        let mut files = AbstractFiles::new();

        let file_path = files
            .insert_physical_file(ccx.interner(), "/virtual/main.rs", "fn main() {}")
            .unwrap();

        assert_eq!(file_path.as_ref(), "/virtual/main.rs");
        assert_eq!(files.code(file_path), Some("fn main() {}"));
        assert!(files.source(file_path).unwrap().is_physical());
    }

    #[test]
    fn physical_file_path_must_be_absolute() {
        let ccx = CommonCx::new();
        let mut files = AbstractFiles::new();

        let err = files
            .insert_physical_file(ccx.interner(), "relative.rs", "")
            .unwrap_err();

        assert_eq!(
            err.to_string(),
            "file path must be absolute: \"relative.rs\""
        );
    }

    #[test]
    fn known_libraries_point_to_file_paths() {
        let ccx = CommonCx::new();
        let mut files = AbstractFiles::new();
        let file_path = files
            .insert_virtual_file(ccx.interner(), "/virtual/core.rs", "mod marker {}")
            .unwrap();

        assert_eq!(
            files
                .set_known_library(ccx.interner(), "core", file_path)
                .unwrap(),
            None
        );
        assert_eq!(files.known_library(ccx.interner(), "core"), Some(file_path));
    }
}
