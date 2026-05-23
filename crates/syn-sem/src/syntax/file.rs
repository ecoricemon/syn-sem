use crate::{error, Result};
use proc_macro2::TokenStream as TokenStream2;
use quote::ToTokens;
use std::{fmt, hash, marker::PhantomPinned, path::PathBuf, pin::Pin};
use syn_locator::{Locate, LocateEntry, Location, Locator};

pub(crate) struct File {
    pub(crate) file: syn::File,
    pub(crate) locator: Locator,
    pub(crate) abs_path: PathBuf,
    _pin: PhantomPinned,
}

impl fmt::Debug for File {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("File")
            .field("file", &self.file)
            .field("abs_path", &self.abs_path)
            .finish_non_exhaustive()
    }
}

impl File {
    pub(crate) fn new(abs_path: PathBuf, code: &str) -> Result<Pin<Box<Self>>> {
        let fpath = abs_path
            .as_os_str()
            .to_str()
            .ok_or(error!("{abs_path:?} contains non UTF-8 character"))?;

        let mut this = Box::pin(Self {
            file: syn::parse_str(code)?,
            locator: Locator::new(fpath, code),
            abs_path: abs_path.clone(),
            _pin: PhantomPinned,
        });

        unsafe {
            let this = Pin::as_mut(&mut this).get_unchecked_mut();
            this.file.locate_as_entry(&mut this.locator)?;
        }

        Ok(this)
    }
}

impl Locate for File {
    fn find_loc(&self, locator: &mut syn_locator::Locator, code: &str, offset: usize) -> Location {
        self.file.locate(locator, code, offset)
    }
}

impl ToTokens for File {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        self.file.to_tokens(tokens);
    }
}

impl PartialEq for File {
    fn eq(&self, other: &Self) -> bool {
        self.abs_path == other.abs_path
    }
}

impl Eq for File {}

impl hash::Hash for File {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.abs_path.hash(state)
    }
}
