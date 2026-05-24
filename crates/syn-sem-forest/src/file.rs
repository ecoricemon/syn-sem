use crate::Result;
use proc_macro2::TokenStream as TokenStream2;
use quote::ToTokens;
use std::{fmt, hash, marker::PhantomPinned, pin::Pin};
use syn_locator::{LocateEntry, Locator};
use syn_sem_common::FilePath;

pub struct File<'cx> {
    pub file: syn::File,
    pub locator: Locator,
    pub file_path: FilePath<'cx>,
    _pin: PhantomPinned,
}

impl fmt::Debug for File<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("File")
            .field("file", &self.file)
            .field("file_path", &self.file_path)
            .finish_non_exhaustive()
    }
}

impl<'cx> File<'cx> {
    pub fn new(file_path: FilePath<'cx>, code: &str) -> Result<Pin<Box<Self>>> {
        let mut this = Box::pin(Self {
            file: syn::parse_str(code)?,
            locator: Locator::new(file_path.as_ref(), code),
            file_path,
            _pin: PhantomPinned,
        });

        unsafe {
            let this = Pin::as_mut(&mut this).get_unchecked_mut();
            this.file.locate_as_entry(&mut this.locator)?;
        }

        Ok(this)
    }
}

impl ToTokens for File<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        self.file.to_tokens(tokens);
    }
}

impl PartialEq for File<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.file_path == other.file_path
    }
}

impl Eq for File<'_> {}

impl hash::Hash for File<'_> {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.file_path.hash(state)
    }
}
