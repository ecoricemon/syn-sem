use crate::{FilePath, FrozenMap, InternedStr, RawSourceText, Result, SourceText};
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
    // `files` stores raw handles into `interner`. Keep it declared first so it is dropped before
    // the interner that owns those allocations.
    files: AbstractFiles,
    interner: StringInterner,
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

    /// Returns whether `file_path` has source in this context.
    pub fn has_source(&self, file_path: FilePath<'_>) -> bool {
        self.files.contains(file_path.as_ref())
    }

    /// Stores virtual source text and returns its interned file path.
    pub fn insert_virtual_file(&self, file_path: &str, source_text: &str) -> Result<FilePath<'_>> {
        let source_text = self.intern(source_text);
        let file_path = self
            .files
            .insert_virtual_file(file_path, source_text.raw())?;
        Ok(self.intern_path(&file_path))
    }

    /// Stores virtual source text under an already interned file path.
    pub fn insert_virtual_source(
        &self,
        file_path: FilePath<'_>,
        source_text: SourceText<'_>,
    ) -> Result<()> {
        self.files
            .insert_virtual_file(file_path.as_ref(), source_text.raw())?;
        Ok(())
    }

    /// Stores physical source text and returns its interned file path.
    pub fn insert_physical_file(&self, file_path: &str, source_text: &str) -> Result<FilePath<'_>> {
        let source_text = self.intern(source_text);
        let file_path = self
            .files
            .insert_physical_file(file_path, source_text.raw())?;
        Ok(self.intern_path(&file_path))
    }

    /// Reads a physical source file and returns its interned canonical file path.
    pub fn read_physical_file(&self, file_path: impl AsRef<Path>) -> Result<FilePath<'_>> {
        let file_path = absolute_file_path(file_path.as_ref())?;
        if self.files.contains(&file_path) {
            return Ok(self.intern_path(&file_path));
        }

        let source_text = fs::read_to_string(&file_path)?;
        let source_text = self.intern(&source_text);
        let file_path = self
            .files
            .insert_physical_file(&file_path, source_text.raw())?;
        Ok(self.intern_path(&file_path))
    }

    /// Returns interned source text for `file_path`.
    pub fn source_text(&self, file_path: FilePath<'_>) -> Option<SourceText<'_>> {
        let raw_source_text = self.files.raw_source_text(file_path.as_ref())?;
        Some(self.source_text_from_raw(raw_source_text))
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

    fn source_text_from_raw(&self, raw_source_text: RawSourceText) -> SourceText<'_> {
        // Safety: `AbstractFiles` is private to this module, and every source-text raw handle
        // stored there is created from `self.interner` by `CommonCx` insertion methods. The
        // returned `SourceText` is tied to `&self`, so it cannot outlive the owning interner.
        unsafe { SourceText::from_raw(raw_source_text) }
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

#[derive(Default)]
struct AbstractFiles {
    // Every raw source-text handle stored here is created by the owning `CommonCx`.
    files: FrozenMap<PathBuf, Box<RawSourceText>>,
    known_libraries: FrozenMap<String, Box<Path>>,
}

impl fmt::Debug for AbstractFiles {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AbstractFiles").finish_non_exhaustive()
    }
}

impl AbstractFiles {
    fn contains(&self, file_path: impl AsRef<Path>) -> bool {
        self.files.get(file_path.as_ref()).is_some()
    }

    fn raw_source_text(&self, file_path: impl AsRef<Path>) -> Option<RawSourceText> {
        self.files.get(file_path.as_ref()).copied()
    }

    fn insert_virtual_file(
        &self,
        file_path: impl AsRef<Path>,
        raw_source_text: RawSourceText,
    ) -> Result<PathBuf> {
        self.insert_source(file_path.as_ref().to_path_buf(), raw_source_text)
    }

    fn insert_physical_file(
        &self,
        file_path: impl AsRef<Path>,
        raw_source_text: RawSourceText,
    ) -> Result<PathBuf> {
        validate_absolute_file_path(file_path.as_ref())?;
        self.insert_source(file_path.as_ref().to_path_buf(), raw_source_text)
    }

    fn set_known_library(
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

    fn known_library(&self, name: &str) -> Option<&Path> {
        self.known_libraries.get(name)
    }

    fn insert_source(&self, file_path: PathBuf, raw_source_text: RawSourceText) -> Result<PathBuf> {
        if let Some(existing) = self.raw_source_text(&file_path) {
            if existing != raw_source_text {
                return Err(format!(
                    "source file `{}` already has different text",
                    file_path.display()
                )
                .into());
            }
            return Ok(file_path);
        }

        self.files
            .insert(file_path.clone(), Box::new(raw_source_text));
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

pub fn intern_prefixed_number<'cx>(
    ccx: &'cx CommonCx,
    prefix: &str,
    number: usize,
) -> InternedStr<'cx> {
    struct PrefixedNumber<'a> {
        prefix: &'a str,
        number: usize,
    }

    impl Display for PrefixedNumber<'_> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str(self.prefix)?;
            Display::fmt(&self.number, f)
        }
    }

    let len = prefix.len() + number.checked_ilog10().unwrap_or(0) as usize + 1;
    ccx.intern_display(&PrefixedNumber { prefix, number }, len)
        .unwrap()
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
    fn abstract_files_stores_virtual_source_text() {
        let files = AbstractFiles::default();
        let interner = StringInterner::default();
        let source_text = interner.intern("fn main() {}");

        let file_path = files
            .insert_virtual_file("/virtual/main.rs", source_text.raw())
            .unwrap();

        assert_eq!(file_path, PathBuf::from("/virtual/main.rs"));
        assert!(files.contains(&file_path));
        assert_eq!(files.raw_source_text(&file_path), Some(source_text.raw()));
    }

    #[test]
    fn abstract_files_stores_physical_source_text_without_reading_disk() {
        let files = AbstractFiles::default();
        let interner = StringInterner::default();
        let source_text = interner.intern("fn main() {}");

        let file_path = files
            .insert_physical_file("/virtual/main.rs", source_text.raw())
            .unwrap();

        assert_eq!(file_path, PathBuf::from("/virtual/main.rs"));
        assert_eq!(files.raw_source_text(&file_path), Some(source_text.raw()));
    }

    #[test]
    fn common_context_interns_source_paths_and_text() {
        let ccx = CommonCx::default();
        let file_path = ccx
            .insert_virtual_file("/virtual/main.rs", "fn main() {}")
            .unwrap();

        assert_eq!(file_path.as_ref(), "/virtual/main.rs");
        assert!(ccx.has_source(file_path));
        let source_text = ccx.source_text(file_path).unwrap();
        assert_eq!(source_text.as_ref(), "fn main() {}");
    }

    #[test]
    fn abstract_files_stores_source_handles() {
        let files = AbstractFiles::default();
        let interner = StringInterner::default();
        let virtual_source_text = interner.intern("fn main() {}");
        let physical_source_text = interner.intern("pub fn lib() {}");
        let virtual_path = PathBuf::from("/virtual/main.rs");
        let physical_path = PathBuf::from("/virtual/lib.rs");

        assert_eq!(
            files
                .insert_virtual_file(&virtual_path, virtual_source_text.raw())
                .unwrap(),
            virtual_path
        );
        assert_eq!(
            files
                .insert_physical_file(&physical_path, physical_source_text.raw())
                .unwrap(),
            physical_path
        );

        assert_eq!(
            files.raw_source_text("/virtual/main.rs"),
            Some(virtual_source_text.raw())
        );
        assert_eq!(
            files.raw_source_text("/virtual/lib.rs"),
            Some(physical_source_text.raw())
        );
    }

    #[test]
    fn physical_file_path_must_be_absolute() {
        let files = AbstractFiles::default();
        let interner = StringInterner::default();
        let source_text = interner.intern("");

        let err = files
            .insert_physical_file("relative.rs", source_text.raw())
            .unwrap_err();

        assert_eq!(
            err.to_string(),
            "file path must be absolute: \"relative.rs\""
        );
    }

    #[test]
    fn known_libraries_point_to_file_paths() {
        let files = AbstractFiles::default();
        let interner = StringInterner::default();
        let source_text = interner.intern("mod marker {}");
        let file_path = files
            .insert_virtual_file("/virtual/core.rs", source_text.raw())
            .unwrap();

        assert_eq!(files.set_known_library("core", &file_path).unwrap(), None);
        assert_eq!(files.known_library("core"), Some(file_path.as_path()));
    }
}
