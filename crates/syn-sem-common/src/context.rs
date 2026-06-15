use crate::{FilePath, FrozenMap, InternedStr, Result, SourceText};
use any_intern::DroplessInterner;
use std::{
    fmt::{self, Display},
    fs, io,
    path::{Path, PathBuf},
};

/// Root context for shared `syn-sem` infrastructure.
///
/// `CommonCx` owns the string interner. Values with the `'ccx` lifetime, such as
/// [`FilePath`] and [`SourceText`], are valid for the lifetime of this context's interner.
#[derive(Debug, Default)]
pub struct CommonCx {
    interner: StringInterner,
    files: AbstractFiles,
}

impl CommonCx {
    /// Returns the string interner owned by this context.
    pub fn interner(&self) -> &StringInterner {
        &self.interner
    }

    /// Interns a string in this context.
    pub fn intern(&self, text: &str) -> InternedStr<'_> {
        self.interner.intern(text)
    }

    /// Interns a formatted value through the shared common context.
    pub fn intern_display<K: Display + ?Sized>(
        &self,
        value: &K,
        upper_size: usize,
    ) -> Result<InternedStr<'_>> {
        self.interner.intern_display(value, upper_size)
    }

    /// Interns a filesystem path after converting it to UTF-8.
    pub fn intern_path(&self, path: &Path) -> InternedStr<'_> {
        let path = path.to_str().unwrap();
        self.intern(path)
    }

    /// Returns the source-file table owned by this context.
    pub fn files(&self) -> &AbstractFiles {
        &self.files
    }

    /// Stores virtual source text and returns its interned file path.
    pub fn insert_virtual_file(&self, file_path: &str, code: &str) -> Result<FilePath<'_>> {
        let file_path = self.files.insert_virtual_file(file_path, code)?;
        Ok(self.intern_path(&file_path))
    }

    /// Stores virtual source text under an already interned file path.
    pub fn insert_virtual_source(
        &self,
        file_path: FilePath<'_>,
        code: SourceText<'_>,
    ) -> Result<()> {
        self.files
            .insert_virtual_file(file_path.as_ref(), code.as_ref())?;
        Ok(())
    }

    /// Stores physical source text and returns its interned file path.
    pub fn insert_physical_file(&self, file_path: &str, code: &str) -> Result<FilePath<'_>> {
        let file_path = self.files.insert_physical_file(file_path, code)?;
        Ok(self.intern_path(&file_path))
    }

    /// Reads a physical source file and returns its interned canonical file path.
    pub fn read_physical_file(&self, file_path: impl AsRef<Path>) -> Result<FilePath<'_>> {
        let file_path = self.files.read_physical_file(file_path)?;
        Ok(self.intern_path(&file_path))
    }

    /// Returns interned source text for `file_path`.
    pub fn source_text(&self, file_path: FilePath<'_>) -> Option<SourceText<'_>> {
        let code = self.files.code(file_path.as_ref())?;
        Some(self.intern(code))
    }

    /// Associates a known library name with an interned file path.
    pub fn set_known_library(
        &self,
        name: &str,
        file_path: FilePath<'_>,
    ) -> Result<Option<FilePath<'_>>> {
        let old = self.files.set_known_library(name, file_path.as_ref())?;
        Ok(old.map(|path| self.intern_path(&path)))
    }

    /// Returns the interned file path associated with a known library name.
    pub fn known_library(&self, name: &str) -> Option<FilePath<'_>> {
        let path = self.files.known_library(name)?;
        Some(self.intern_path(path))
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
    /// Interns `text` and returns a lifetime-bearing interned string.
    pub fn intern(&self, text: &str) -> InternedStr<'_> {
        self.intern_display(text, text.len()).unwrap()
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

/// Abstract source-file table keyed by owned paths.
///
/// This type is independent from the string interner. [`CommonCx`] owns one table and exposes
/// interned [`FilePath`] and [`SourceText`] values for phase crates that need lifetime-bearing
/// handles.
#[derive(Default)]
pub struct AbstractFiles {
    files: FrozenMap<PathBuf, Box<str>>,
    known_libraries: FrozenMap<String, Box<Path>>,
}

impl fmt::Debug for AbstractFiles {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AbstractFiles").finish_non_exhaustive()
    }
}

impl AbstractFiles {
    /// Returns whether `file_path` has source in this table.
    pub fn contains(&self, file_path: impl AsRef<Path>) -> bool {
        self.files.get(file_path.as_ref()).is_some()
    }

    /// Returns source text for `file_path`.
    pub fn code(&self, file_path: impl AsRef<Path>) -> Option<&str> {
        self.files.get(file_path.as_ref())
    }

    /// Inserts caller-provided virtual source text under `file_path`.
    ///
    /// Virtual paths are source identifiers and are not checked against the filesystem.
    pub fn insert_virtual_file(&self, file_path: impl AsRef<Path>, code: &str) -> Result<PathBuf> {
        self.insert_source(file_path.as_ref().to_path_buf(), code)
    }

    /// Inserts caller-provided source text for an absolute physical file path.
    ///
    /// This validates that `file_path` is absolute, but does not check whether it exists.
    pub fn insert_physical_file(&self, file_path: impl AsRef<Path>, code: &str) -> Result<PathBuf> {
        validate_absolute_file_path(file_path.as_ref())?;
        self.insert_source(file_path.as_ref().to_path_buf(), code)
    }

    /// Reads an absolute physical file path from disk and stores its source text.
    ///
    /// The returned path is canonicalized before it is interned.
    pub fn read_physical_file(&self, file_path: impl AsRef<Path>) -> Result<PathBuf> {
        let file_path = absolute_file_path(file_path.as_ref())?;
        if self.contains(&file_path) {
            return Ok(file_path);
        }

        let code = fs::read_to_string(&file_path)?;
        self.insert_physical_file(&file_path, &code)
    }

    /// Associates a known library name with a file path.
    ///
    /// Names are library identifiers such as `core` or `std`, not paths.
    pub fn set_known_library(
        &self,
        name: &str,
        file_path: impl AsRef<Path>,
    ) -> Result<Option<PathBuf>> {
        debug_assert!(
            !name.ends_with(".rs"),
            "expected library name, but received file path-like name `{name}`"
        );

        let file_path = file_path.as_ref();
        if let Some(existing) = self.known_library(name) {
            if existing != file_path {
                return Err(format!(
                    "known library `{name}` already points to `{}`",
                    existing.display()
                )
                .into());
            }
            return Ok(Some(existing.to_path_buf()));
        }

        self.known_libraries
            .insert(name.to_owned(), file_path.to_path_buf().into_boxed_path());
        Ok(None)
    }

    /// Returns the file path associated with a known library name.
    pub fn known_library(&self, name: &str) -> Option<&Path> {
        self.known_libraries.get(name)
    }

    fn insert_source(&self, file_path: PathBuf, code: &str) -> Result<PathBuf> {
        if let Some(existing) = self.code(&file_path) {
            if existing != code {
                return Err(format!(
                    "source file `{}` already has different text",
                    file_path.display()
                )
                .into());
            }
            return Ok(file_path);
        }

        self.files.insert(file_path.clone(), code.into());
        Ok(file_path)
    }
}

/// Validates that `file_path` is a non-empty absolute path.
///
/// This only validates path shape. It does not check filesystem existence.
pub fn validate_absolute_file_path(file_path: &Path) -> Result<()> {
    if file_path.as_os_str().is_empty() {
        return Err("file path must not be empty".into());
    }

    if !file_path.is_absolute() {
        return Err(format!("file path must be absolute: {file_path:?}").into());
    }

    Ok(())
}

/// Returns the canonical absolute path for an existing physical file.
pub fn absolute_file_path(file_path: &Path) -> Result<PathBuf> {
    validate_absolute_file_path(file_path)?;

    let canonical = file_path.canonicalize().map_err(|e| {
        let path = file_path.to_string_lossy();
        match e.kind() {
            io::ErrorKind::NotFound => format!("couldn't find `{path}`: {e}"),
            _ => format!("`{path}`: {e}"),
        }
    })?;

    if canonical.to_str().is_none() {
        return Err(format!("{canonical:?} contains non UTF-8 characters").into());
    }

    Ok(canonical)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interner_deduplicates_strings() {
        let interner = StringInterner::default();

        let a = interner.intern("hello");
        let b = interner.intern("hello");
        let c = interner.intern("world");

        assert_eq!(a, b);
        assert_ne!(a, c);
        assert_eq!(interner.len(), 2);
        assert_eq!(a.as_ref(), "hello");
    }

    #[test]
    fn abstract_files_stores_virtual_file_code() {
        let files = AbstractFiles::default();

        let file_path = files
            .insert_virtual_file("/virtual/main.rs", "fn main() {}")
            .unwrap();

        assert_eq!(file_path, PathBuf::from("/virtual/main.rs"));
        assert!(files.contains(&file_path));
        assert_eq!(files.code(&file_path), Some("fn main() {}"));
    }

    #[test]
    fn abstract_files_stores_physical_file_code_without_reading_disk() {
        let files = AbstractFiles::default();

        let file_path = files
            .insert_physical_file("/virtual/main.rs", "fn main() {}")
            .unwrap();

        assert_eq!(file_path, PathBuf::from("/virtual/main.rs"));
        assert_eq!(files.code(&file_path), Some("fn main() {}"));
    }

    #[test]
    fn common_context_interns_source_paths_and_text() {
        let ccx = CommonCx::default();
        let file_path = ccx
            .insert_virtual_file("/virtual/main.rs", "fn main() {}")
            .unwrap();

        assert_eq!(file_path.as_ref(), "/virtual/main.rs");
        assert!(ccx.files().contains(file_path.as_ref()));
        let source_text = ccx.source_text(file_path).unwrap();
        assert_eq!(source_text.as_ref(), "fn main() {}");
    }

    #[test]
    fn abstract_files_stores_sources_without_interner() {
        let files = AbstractFiles::default();
        let virtual_path = PathBuf::from("/virtual/main.rs");
        let physical_path = PathBuf::from("/virtual/lib.rs");

        assert_eq!(
            files
                .insert_virtual_file(&virtual_path, "fn main() {}")
                .unwrap(),
            virtual_path
        );
        assert_eq!(
            files
                .insert_physical_file(&physical_path, "pub fn lib() {}")
                .unwrap(),
            physical_path
        );

        assert_eq!(files.code("/virtual/main.rs"), Some("fn main() {}"));
        assert_eq!(files.code("/virtual/lib.rs"), Some("pub fn lib() {}"));
    }

    #[test]
    fn physical_file_path_must_be_absolute() {
        let files = AbstractFiles::default();

        let err = files.insert_physical_file("relative.rs", "").unwrap_err();

        assert_eq!(
            err.to_string(),
            "file path must be absolute: \"relative.rs\""
        );
    }

    #[test]
    fn known_libraries_point_to_file_paths() {
        let files = AbstractFiles::default();
        let file_path = files
            .insert_virtual_file("/virtual/core.rs", "mod marker {}")
            .unwrap();

        assert_eq!(files.set_known_library("core", &file_path).unwrap(), None);
        assert_eq!(files.known_library("core"), Some(file_path.as_path()));
    }
}
