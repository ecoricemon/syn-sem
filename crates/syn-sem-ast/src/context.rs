use crate::{File as AstFile, FromSyn, InputDesc, ModuleTreeBuilder, SourceInput};
use any_intern::Interned;
use bumpalo::Bump;
use std::{fmt::Display, io, mem, path::Path};
use syn_locator::{LocateEntry, Locator};
use syn_sem_common::{CommonCx, FrozenMap, Result, Str};

/// Allocation and interning context used by the semantic AST.
///
/// For example, parsing a virtual file stores its source text here and allocates AST slices from
/// this context.
pub struct SyntaxCx<'cx> {
    /// Shared common context that owns interned strings.
    pub common: &'cx CommonCx,
    /// Arena used for dropless AST allocation.
    pub bump: Bump,
    files: FrozenMap<Str<'cx>, Box<Source<'cx>>>,
}

impl<'cx> SyntaxCx<'cx> {
    /// Creates a syntax context borrowing the shared common context.
    pub fn new(common: &'cx CommonCx) -> Self {
        Self {
            common,
            bump: Bump::new(),
            files: FrozenMap::default(),
        }
    }

    /// Parses and stores one source file as a semantic AST.
    ///
    /// Known-library sources skip syntax location because they do not need physical source spans.
    pub fn parse_file(
        &'cx self,
        file_path: Str<'cx>,
        source_text: Str<'cx>,
        kind: SourceKind,
    ) -> Result<()> {
        let syntax = Box::new(syn::parse_str::<syn::File>(source_text.as_ref())?);
        let locator = if matches!(kind, SourceKind::Known) {
            None
        } else {
            let mut locator = Locator::new(file_path.as_ref(), source_text.as_ref());
            syntax.locate_as_entry(&mut locator)?;
            Some(locator)
        };
        let ast = AstFile::from_syn(
            self,
            InputDesc {
                file_path,
                source_text,
                locator: locator.as_ref(),
                input: syntax.as_ref(),
            },
        );
        let source = Box::new(Source::new(kind, source_text, locator, syntax, ast));
        self.files.insert(file_path, source);
        Ok(())
    }

    /// Returns whether `file_path` has already been parsed and stored.
    pub fn has_source(&self, file_path: Str<'cx>) -> bool {
        self.files.get(&file_path).is_some()
    }

    /// Returns the stored parsed source for `file_path`.
    pub fn get_source(&'cx self, file_path: Str<'cx>) -> Option<&'cx Source<'cx>> {
        self.files.get(&file_path)
    }

    /// Returns the stored parsed source for `file_path`.
    pub fn lookup_source(&'cx self, file_path: Str<'cx>) -> Result<&'cx Source<'cx>> {
        let source = self.get_source(file_path).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("source file is not parsed: {file_path}"),
            )
        })?;
        Ok(source)
    }

    /// Reads, parses, and returns the physical module tree rooted at `entry_path`.
    pub fn read_physical_module_tree(
        &'cx self,
        entry_path: impl AsRef<Path>,
    ) -> Result<Vec<SourceInput<'cx>>> {
        let entry_path = self.read_physical_file(entry_path.as_ref())?;
        self.collect_module_tree(entry_path)
    }

    /// Returns parsed module-tree inputs rooted at `entry_path`.
    ///
    /// Any missing out-of-line physical module files are read and parsed while walking the tree.
    pub fn collect_module_tree(&'cx self, entry_path: Str<'cx>) -> Result<Vec<SourceInput<'cx>>> {
        ModuleTreeBuilder::new(self).collect(entry_path)
    }

    /// Reads and parses a physical source file if it has not already been parsed.
    pub fn read_physical_file(&'cx self, file_path: impl AsRef<Path>) -> Result<Str<'cx>> {
        let file_path = self.common.read_physical_file(file_path.as_ref())?;
        if self.has_source(file_path) {
            return Ok(file_path);
        }

        let source_text = self
            .common
            .source_text(file_path)
            .ok_or_else(|| format!("source file is not stored: {file_path}"))?;
        self.parse_file(file_path, source_text, SourceKind::Physical)?;
        Ok(file_path)
    }

    /// Allocates a dropless value in this context.
    pub(crate) fn alloc<T>(&'cx self, value: T) -> &'cx T {
        assert!(!mem::needs_drop::<T>());
        self.bump.alloc(value)
    }

    /// Allocates a dropless slice by calling `f` once for each index.
    pub(crate) fn alloc_slice<T, F: FnMut(usize) -> T>(
        &'cx self,
        len: usize,
        mut f: F,
    ) -> &'cx [T] {
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
    pub(crate) fn intern(&'cx self, value: &str) -> Interned<'cx, str> {
        self.common.intern(value)
    }

    /// Interns a formatted value through the shared common context.
    pub(crate) fn intern_display<K: Display + ?Sized>(
        &'cx self,
        value: &K,
        upper_size: usize,
    ) -> Result<Interned<'cx, str>> {
        self.common.interner().intern_display(value, upper_size)
    }
}

/// Source text and locator state for one parsed file.
///
/// For example, a virtual file added for tests stores its full text and path in this value.
pub struct Source<'cx> {
    /// Whether the source is physical or virtual.
    pub kind: SourceKind,
    /// Interned source text.
    pub source_text: Str<'cx>,
    locator: Option<Locator>,
    // `Locator` requires fixed addresses to the file.
    syntax: Box<syn::File>,
    ast: AstFile<'cx>,
}

impl<'cx> Source<'cx> {
    fn new(
        kind: SourceKind,
        source_text: Str<'cx>,
        locator: Option<Locator>,
        syntax: Box<syn::File>,
        ast: AstFile<'cx>,
    ) -> Self {
        Self {
            kind,
            source_text,
            locator,
            syntax,
            ast,
        }
    }

    /// Returns the locator populated for this source.
    pub fn locator(&self) -> Option<&Locator> {
        self.locator.as_ref()
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

/// Distinguishes whether a source came from disk or was provided in memory.
///
/// For example, tests usually create `Virtual` sources while workspace files are `Physical`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceKind {
    /// Source associated with a physical file path.
    Physical,
    /// Source supplied in memory by the caller.
    Virtual,
    /// Built-in semantic model whose source locations were intentionally omitted.
    Known,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Item;

    #[test]
    fn parses_known_source_without_locator_and_preserves_ast_shape() {
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let path = ccx.intern("known.rs");
        let text = ccx.intern("pub mod core {}");

        scx.parse_file(path, text, SourceKind::Known).unwrap();

        let source = scx.lookup_source(path).unwrap();
        assert!(matches!(source.kind, SourceKind::Known));
        assert!(source.locator().is_none());
        let [Item::Mod(module)] = source.ast().items else {
            panic!("expected one module");
        };
        assert_eq!(module.ident.inner.as_ref(), "core");
        assert!(module.is_inline);
        assert!(module.span.source_text().is_empty());
    }
}
