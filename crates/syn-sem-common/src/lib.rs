//! Shared infrastructure for extracted `syn-sem` crates.
//!
//! This crate owns common facilities such as string interning, shared source identifiers, and
//! abstract source files. It deliberately stays independent from syntax, AST, name-resolution, and
//! semantic crates.

mod arena;
mod ast_node;
mod context;
pub mod known;

pub use arena::*;
pub use ast_node::*;
pub use context::*;

use any_intern::{Interned, RawInterned};
use std::{
    collections::{HashMap, HashSet},
    error::Error as StdError,
};

/// Error type shared by internal `syn-sem` crates.
pub type Error = Box<dyn StdError + Send + Sync>;

/// Result type shared by internal `syn-sem` crates.
pub type Result<T> = std::result::Result<T, Error>;

/// Fallible result whose successful value may be absent.
pub type MaybeResult<T> = Result<Option<T>>;

/// Hash map type used by internal `syn-sem` crates.
pub type Map<K, V> = HashMap<K, V, fxhash::FxBuildHasher>;

/// Append-only map type used by internal `syn-sem` crates.
pub type FrozenMap<K, V> = elsa::FrozenMap<K, V, fxhash::FxBuildHasher>;

/// Hash set type used by internal `syn-sem` crates.
pub type Set<T> = HashSet<T, fxhash::FxBuildHasher>;

/// Extension helpers for order-preserving deduplication on small vectors.
pub trait VecUniqueExt<T> {
    /// Pushes `value` only when the vector does not already contain it.
    ///
    /// Returns whether the value was inserted.
    fn push_unique(&mut self, value: T) -> bool
    where
        T: PartialEq;

    /// Pushes `value` only when no existing item has the same derived key.
    ///
    /// Returns whether the value was inserted.
    fn push_unique_by_key<K>(&mut self, value: T, key: impl Fn(&T) -> K) -> bool
    where
        K: PartialEq;
}

impl<T> VecUniqueExt<T> for Vec<T> {
    fn push_unique(&mut self, value: T) -> bool
    where
        T: PartialEq,
    {
        if self.contains(&value) {
            return false;
        }
        self.push(value);
        true
    }

    fn push_unique_by_key<K>(&mut self, value: T, key: impl Fn(&T) -> K) -> bool
    where
        K: PartialEq,
    {
        let value_key = key(&value);
        if self.iter().any(|existing| key(existing) == value_key) {
            return false;
        }
        self.push(value);
        true
    }
}

/// String interned in [`CommonCx`].
///
/// The `'ccx` lifetime is tied to the [`CommonCx`] / [`StringInterner`] that produced it.
pub type InternedStr<'ccx> = Interned<'ccx, str>;

/// Interned absolute or virtual source file path.
pub type FilePath<'ccx> = InternedStr<'ccx>;

/// Interned source text.
pub type SourceText<'ccx> = InternedStr<'ccx>;

/// Lifetime-erased interned source text.
pub type RawSourceText = RawInterned<str>;

/// Interned known library name, such as `core` or `std`.
pub type LibraryName<'ccx> = InternedStr<'ccx>;
