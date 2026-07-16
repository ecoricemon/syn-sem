//! Lifetime-bearing semantic AST built from `syn` syntax trees.
//!
//! This crate stores AST nodes in a [`SyntaxCx`] and ties interned source data to the shared
//! `syn-sem-common` context lifetime.

mod ast_node;
mod common;
mod context;
mod data;
mod expr;
mod file;
mod generics;
mod item;
mod lit;
mod module_tree;
mod pat;
mod path;
mod restriction;
mod stmt;
mod ty;

pub use ast_node::*;
pub use common::*;
pub use context::*;
pub use data::*;
pub use expr::*;
pub use file::*;
pub use generics::*;
pub use item::*;
pub use lit::*;
pub use module_tree::*;
pub use pat::*;
pub use path::*;
pub use restriction::*;
pub use stmt::*;
pub use ty::*;

#[cfg(test)]
pub(crate) mod test_util {
    use crate::{FromSyn, InputDesc, SyntaxCx};
    use std::ops::Deref;
    use std::sync::atomic::{AtomicU32, Ordering::Relaxed};
    use syn::parse::Parse;
    use syn_locator::{LocateEntry, Locator};

    pub(crate) struct Parsed<U, T> {
        pub(crate) value: U,
        _syntax: Box<T>,
    }

    impl<U, T> Deref for Parsed<U, T> {
        type Target = U;

        fn deref(&self) -> &Self::Target {
            &self.value
        }
    }

    pub(crate) fn parse<'cx, T: Parse + LocateEntry + 'static, U: FromSyn<'cx, T>>(
        scx: &'cx SyntaxCx<'cx>,
        source_text: &str,
    ) -> Parsed<U, T> {
        // Creates a unique file path.
        static ID: AtomicU32 = AtomicU32::new(0);
        let id = ID.fetch_add(1, Relaxed);
        let file_path = id.to_string();

        // Parses `T` and generates `U`.
        let file_path = scx.common.intern(&file_path);
        let source_text = scx.common.intern(source_text);
        let syntax = Box::new(syn::parse_str::<T>(source_text.as_ref()).unwrap());
        let mut locator = Locator::new(file_path.as_ref(), source_text.as_ref());
        syntax.locate_as_entry(&mut locator).unwrap();
        let value = U::from_syn(
            scx,
            InputDesc {
                file_path,
                source_text,
                locator: Some(&locator),
                input: syntax.as_ref(),
            },
        );
        Parsed {
            value,
            _syntax: syntax,
        }
    }
}
