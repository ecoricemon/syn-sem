use crate::{error, Result};
use proc_macro2::TokenStream as TokenStream2;
use quote::ToTokens;
use std::{hash, marker::PhantomPinned, path::PathBuf, pin::Pin};
use syn_locator::{Locate, LocateEntry, Location};

// === SmFile ===

#[derive(Clone, Debug)]
pub(crate) struct SmFile {
    pub(crate) file: syn::File,
    pub(crate) abs_path: PathBuf,
    _pin: PhantomPinned,
}

impl SmFile {
    pub(crate) fn new(abs_path: PathBuf, code: &str) -> Result<Pin<Box<Self>>> {
        let this = Box::pin(Self {
            file: syn::parse_str(code)?,
            abs_path: abs_path.clone(),
            _pin: PhantomPinned,
        });

        let fpath = abs_path
            .as_os_str()
            .to_str()
            .ok_or(error!("{abs_path:?} contains non UTF-8 character"))?;

        if !syn_locator::is_located(fpath) {
            this.as_ref().locate_as_entry(fpath, code)?;
        }

        Ok(this)
    }
}

impl Locate for SmFile {
    fn find_loc(
        &self,
        locator: &mut syn_locator::Locator,
        file_path: syn_locator::FilePath,
        code: &str,
        offset: usize,
    ) -> Location {
        self.file.locate(locator, file_path, code, offset)
    }
}

impl ToTokens for SmFile {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        self.file.to_tokens(tokens);
    }
}

impl PartialEq for SmFile {
    fn eq(&self, other: &Self) -> bool {
        self.abs_path == other.abs_path
    }
}

impl Eq for SmFile {}

impl hash::Hash for SmFile {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.abs_path.hash(state)
    }
}
