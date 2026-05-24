//! Lifetime-bearing semantic AST built from `syn` syntax trees.
//!
//! This crate stores AST nodes in a [`SyntaxCx`] and ties interned source data to the shared
//! `syn-sem-common` context lifetime.

/// Shared conversion helpers and source span types.
pub mod common;
/// Syntax allocation and parsed-source context.
pub mod context;
/// Struct and enum data nodes.
pub mod data;
/// Expression AST nodes.
pub mod expr;
/// Source-file AST node.
pub mod file;
/// Generic parameter and where-clause nodes.
pub mod generics;
/// Item, signature, and parameter nodes.
pub mod item;
/// Literal AST nodes.
pub mod lit;
/// Pattern AST nodes.
pub mod pat;
/// Path and generic argument nodes.
pub mod path;
/// Visibility and restriction nodes.
pub mod restriction;
/// Statement and block nodes.
pub mod stmt;
/// Type AST nodes.
pub mod ty;

pub use common::*;
pub use context::*;
pub use data::*;
pub use expr::*;
pub use file::*;
pub use generics::*;
pub use item::*;
pub use lit::*;
pub use pat::*;
pub use path::*;
pub use restriction::*;
pub use stmt::*;
pub use ty::*;

//pub(crate) type Map<K, V> = fxhash::FxHashMap<K, V>;
pub(crate) type AppendOnlyMap<K, V> = elsa::FrozenMap<K, V, fxhash::FxBuildHasher>;

#[cfg(test)]
pub(crate) mod test_util {
    use crate::{FromSyn, InputDesc, SyntaxCx};
    use std::sync::atomic::{AtomicU32, Ordering::Relaxed};
    use syn::parse::Parse;
    use syn_locator::LocateEntry;

    pub(crate) fn parse<'cx, T: Parse + LocateEntry + 'static, U: FromSyn<'cx, T>>(
        cx: &'cx SyntaxCx<'cx>,
        text: &str,
    ) -> U {
        // Creates a unique file path.
        static ID: AtomicU32 = AtomicU32::new(0);
        let id = ID.fetch_add(1, Relaxed);
        let file_path = id.to_string();

        // Parses `T` and generates `U`.
        let file_path = cx.parse_virtual_syntax::<T>(&file_path, text).unwrap();
        let source = cx.get_source(file_path).unwrap();
        let syn = source.syntax::<T>().unwrap();
        U::from_syn(
            cx,
            InputDesc {
                file_path,
                input: syn,
            },
        )
    }
}
