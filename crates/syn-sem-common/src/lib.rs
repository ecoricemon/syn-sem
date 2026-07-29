//! Shared infrastructure for extracted `syn-sem` crates.
//!
//! This crate owns common facilities such as string interning, shared source identifiers, and
//! abstract source files. It deliberately stays independent from syntax, AST, name-resolution, and
//! semantic crates.

mod arena;
mod collections;
mod context;
mod directed_graph;
pub mod known;

pub use arena::*;
pub use collections::*;
pub use context::*;
pub use directed_graph::*;

use any_intern::Interned;
use std::{
    collections::{HashMap, HashSet},
    error::Error as StdError,
    result,
};

/// Error type shared by internal `syn-sem` crates.
pub type Error = Box<dyn StdError + Send + Sync>;

/// Result type shared by internal `syn-sem` crates.
pub type Result<T> = result::Result<T, Error>;

/// Fallible result whose successful value may be absent.
pub type MaybeResult<T> = Result<Option<T>>;

/// Hash map type used by internal `syn-sem` crates.
pub type Map<K, V> = HashMap<K, V, fxhash::FxBuildHasher>;

/// Append-only map type used by internal `syn-sem` crates.
pub type FrozenMap<K, V> = elsa::FrozenMap<K, V, fxhash::FxBuildHasher>;

/// Hash set type used by internal `syn-sem` crates.
pub type Set<T> = HashSet<T, fxhash::FxBuildHasher>;

/// String interned in [`CommonCx`].
pub type Str<'ccx> = Interned<'ccx, str>;
