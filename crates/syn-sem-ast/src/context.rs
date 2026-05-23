use crate::AppendOnlyMap;
use any_intern::{Dropless, DroplessInterner, Interned};
use bumpalo::Bump;
use std::{any::Any, fmt::Display, mem};
use syn_locator::{LocateEntry, Locator};

/// Allocation and interning context used by the semantic AST.
///
/// For example, parsing a virtual file stores its source text here and allocates AST slices from
/// this context.
pub struct SyntaxCx {
    pub bump: Bump,
    pub interner: DroplessInterner,
    files: AppendOnlyMap<Box<str>, Box<Source>>,
}

impl SyntaxCx {
    pub fn alloc<T>(&self, value: T) -> &T {
        assert!(!mem::needs_drop::<T>());
        self.bump.alloc(value)
    }

    pub fn alloc_slice<T, F: FnMut(usize) -> T>(&self, len: usize, mut f: F) -> &[T] {
        assert!(!mem::needs_drop::<T>());
        let mut expected_index = 0;
        self.bump.alloc_slice_fill_with(len, |index| {
            // We're expecting that the closure `f` is called with monotonically increasing index.
            assert_eq!(expected_index, index);
            expected_index += 1;
            f(index)
        })
    }

    pub fn intern<K: Dropless + ?Sized>(&self, value: &K) -> Interned<'_, K> {
        self.interner.intern(value)
    }

    pub fn intern_formatted_str<K: Display + ?Sized>(
        &self,
        value: &K,
        upper_size: usize,
    ) -> Interned<'_, str> {
        self.interner
            .intern_formatted_str(value, upper_size)
            .unwrap()
    }

    pub fn parse_physical_file(&self, file_path: Box<str>, text: Box<str>) {
        self.parse_syntax::<syn::File>(file_path, text, SourceKind::Physical)
    }

    pub fn parse_virtual_file(&self, file_path: Box<str>, text: Box<str>) {
        self.parse_syntax::<syn::File>(file_path, text, SourceKind::Virtual)
    }

    #[cfg(test)]
    pub(crate) fn parse_virtual_syntax<T>(&self, file_path: Box<str>, text: Box<str>)
    where
        T: syn::parse::Parse + LocateEntry + 'static,
    {
        self.parse_syntax::<T>(file_path, text, SourceKind::Virtual)
    }

    fn parse_syntax<T>(&self, file_path: Box<str>, text: Box<str>, kind: SourceKind)
    where
        T: syn::parse::Parse + LocateEntry + 'static,
    {
        let syntax = Box::new(syn::parse_str::<T>(&text).unwrap());
        let mut locator = Locator::new(&file_path, text.clone());
        syntax.locate_as_entry(&mut locator).unwrap();
        let source = Box::new(Source::new(kind, text, locator, syntax));
        self.files.insert(file_path, source);
    }

    pub fn get_source(&self, file_path: &str) -> Option<&Source> {
        self.files.get(file_path)
    }
}

impl Default for SyntaxCx {
    fn default() -> Self {
        Self {
            bump: Bump::new(),
            interner: DroplessInterner::new(),
            files: AppendOnlyMap::default(),
        }
    }
}

/// Source text and locator state for one parsed file.
///
/// For example, a virtual file added for tests stores its full text and path in this value.
pub struct Source {
    pub kind: SourceKind,
    pub text: Box<str>,
    locator: Locator,
    syntax: Box<dyn Any>,
}

impl Source {
    fn new<T: Any>(kind: SourceKind, text: Box<str>, locator: Locator, syntax: Box<T>) -> Self {
        Self {
            kind,
            text,
            locator,
            syntax,
        }
    }

    pub fn locator(&self) -> &Locator {
        &self.locator
    }

    pub fn syntax<T: 'static>(&self) -> Option<&T> {
        self.syntax.downcast_ref()
    }
}

#[derive(Debug)]
/// Distinguishes whether a source came from disk or was provided in memory.
///
/// For example, tests usually create `Virtual` sources while workspace files are `Physical`.
pub enum SourceKind {
    Physical,
    Virtual,
}
