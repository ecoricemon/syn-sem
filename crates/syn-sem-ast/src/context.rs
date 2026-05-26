use crate::{AppendOnlyMap, File as AstFile, FromSyn, InputDesc};
use any_intern::Interned;
use bumpalo::Bump;
use std::{fmt::Display, mem};
use syn_locator::{LocateEntry, Locator};
use syn_sem_common::{CommonCx, FilePath, Result, SourceCode};

/// Allocation and interning context used by the semantic AST.
///
/// For example, parsing a virtual file stores its source text here and allocates AST slices from
/// this context.
pub struct SyntaxCx<'cx> {
    /// Shared common context that owns interned strings.
    pub ccx: &'cx CommonCx,
    /// Arena used for dropless AST allocation.
    pub bump: Bump,
    files: AppendOnlyMap<FilePath<'cx>, Box<Source<'cx>>>,
}

impl<'cx> SyntaxCx<'cx> {
    /// Creates a syntax context borrowing the shared common context.
    pub fn new(ccx: &'cx CommonCx) -> Self {
        Self {
            ccx,
            bump: Bump::new(),
            files: AppendOnlyMap::default(),
        }
    }

    /// Allocates a dropless value in this context.
    pub fn alloc<T>(&'cx self, value: T) -> &'cx T {
        assert!(!mem::needs_drop::<T>());
        self.bump.alloc(value)
    }

    /// Allocates a dropless slice by calling `f` once for each index.
    pub fn alloc_slice<T, F: FnMut(usize) -> T>(&'cx self, len: usize, mut f: F) -> &'cx [T] {
        assert!(!mem::needs_drop::<T>());
        let mut expected_index = 0;
        self.bump.alloc_slice_fill_with(len, |index| {
            // We're expecting that the closure `f` is called with monotonically increasing index.
            assert_eq!(expected_index, index);
            expected_index += 1;
            f(index)
        })
    }

    /// Interns a string through the shared common context.
    pub fn intern(&'cx self, value: &str) -> Interned<'cx, str> {
        self.ccx.intern(value)
    }

    /// Interns a formatted value through the shared common context.
    pub fn intern_display<K: Display + ?Sized>(
        &'cx self,
        value: &K,
        upper_size: usize,
    ) -> Result<Interned<'cx, str>> {
        self.ccx.interner().intern_display(value, upper_size)
    }

    /// Parses and stores a physical source file.
    pub fn parse_physical_file(&'cx self, file_path: &str, text: &str) -> Result<FilePath<'cx>> {
        self.parse_file(file_path, text, SourceKind::Physical)
    }

    /// Parses and stores a virtual source file.
    pub fn parse_virtual_file(&'cx self, file_path: &str, text: &str) -> Result<FilePath<'cx>> {
        self.parse_file(file_path, text, SourceKind::Virtual)
    }

    fn parse_file(
        &'cx self,
        file_path: &str,
        text: &str,
        kind: SourceKind,
    ) -> Result<FilePath<'cx>> {
        let file_path = self.ccx.intern(file_path);
        let text = self.ccx.intern(text);
        let syntax = Box::new(syn::parse_str::<syn::File>(text.as_ref())?);
        let mut locator = Locator::new(file_path.as_ref(), text.as_ref());
        syntax.locate_as_entry(&mut locator)?;

        let ast = AstFile::from_syn(
            self,
            InputDesc {
                file_path,
                source_code: text,
                locator: &locator,
                input: syntax.as_ref(),
            },
        );
        let source = Box::new(Source::new(kind, text, locator, syntax, ast));
        self.files.insert(file_path, source);
        Ok(file_path)
    }

    /// Returns the stored parsed source for `file_path`.
    pub fn get_source(&'cx self, file_path: FilePath<'cx>) -> Option<&'cx Source<'cx>> {
        self.files.get(&file_path)
    }
}

/// Source text and locator state for one parsed file.
///
/// For example, a virtual file added for tests stores its full text and path in this value.
pub struct Source<'cx> {
    /// Whether the source is physical or virtual.
    pub kind: SourceKind,
    /// Interned source text.
    pub text: SourceCode<'cx>,
    locator: Locator,
    // `Locator` requires fixed addresses to the file.
    syntax: Box<syn::File>,
    ast: AstFile<'cx>,
}

impl<'cx> Source<'cx> {
    fn new(
        kind: SourceKind,
        text: SourceCode<'cx>,
        locator: Locator,
        syntax: Box<syn::File>,
        ast: AstFile<'cx>,
    ) -> Self {
        Self {
            kind,
            text,
            locator,
            syntax,
            ast,
        }
    }

    /// Returns the locator populated for this source.
    pub fn locator(&self) -> &Locator {
        &self.locator
    }

    /// Returns the parsed Rust source file.
    pub fn syntax(&self) -> &syn::File {
        &self.syntax
    }

    /// Returns the semantic AST built from this source.
    pub fn ast(&self) -> &AstFile<'cx> {
        &self.ast
    }
}

#[derive(Debug)]
/// Distinguishes whether a source came from disk or was provided in memory.
///
/// For example, tests usually create `Virtual` sources while workspace files are `Physical`.
pub enum SourceKind {
    /// Source associated with a physical file path.
    Physical,
    /// Source supplied in memory by the caller.
    Virtual,
}
