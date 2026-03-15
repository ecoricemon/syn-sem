use any_intern::{Dropless, DroplessInterner, Interned};
use bumpalo::Bump;
use dashmap::{mapref::one::Ref, DashMap};
use std::{
    any::Any,
    fmt::Display,
    mem,
    path::{Path, PathBuf},
    pin::Pin,
    sync::Arc,
};
use syn::parse::Parse;
use syn_locator::LocateEntry;

pub struct SyntaxCx {
    pub bump: Bump,
    pub interner: DroplessInterner,
    pub files: DashMap<PathBuf, Source>,
}

impl SyntaxCx {
    pub fn new(interner: DroplessInterner) -> Self {
        Self {
            bump: Bump::new(),
            interner,
            files: DashMap::default(),
        }
    }

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

    pub fn get_source(&self, file_path: &Path) -> Ref<'_, PathBuf, Source> {
        self.files.get(file_path).unwrap()
    }

    pub fn insert_physical_source<T: Parse + LocateEntry>(
        &self,
        file_path: PathBuf,
        text: Arc<str>,
    ) {
        self.insert_file::<T>(file_path, text, SourceKind::Physical)
    }

    pub fn insert_virtual_source<T: Parse + LocateEntry>(
        &self,
        file_path: PathBuf,
        text: Arc<str>,
    ) {
        self.insert_file::<T>(file_path, text, SourceKind::Virtual)
    }

    fn insert_file<T: Parse + LocateEntry>(
        &self,
        file_path: PathBuf,
        text: Arc<str>,
        kind: SourceKind,
    ) {
        let file: T = syn::parse_str(&text).unwrap();
        let pinned = Box::pin(file);
        pinned
            .as_ref()
            .locate_as_entry(file_path.to_string_lossy().as_ref(), text.clone())
            .unwrap();

        let source = Source {
            kind,
            text,
            syn: pinned,
        };
        self.files.insert(file_path, source);
    }
}

pub struct Source {
    pub kind: SourceKind,
    pub text: Arc<str>,
    pub syn: Pin<Box<dyn Any>>,
}

#[derive(Debug)]
pub enum SourceKind {
    Physical,
    Virtual,
}
