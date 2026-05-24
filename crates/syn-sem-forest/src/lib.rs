//! Raw `syn` syntax forest, identity, parent lookup, and source-location support.
//!
//! The forest stores pinned parsed files and cloned syntax fragments keyed by shared interned
//! file paths from `syn-sem-common`.

mod attr;
mod common;
mod file;
mod find_child;
mod find_parent;
mod forest;
mod identify;

pub use attr::*;
pub use file::*;
pub use find_child::*;
pub use find_parent::*;
pub use forest::*;
pub use identify::*;

pub(crate) type Result<T> = syn_sem_common::Result<T>;
pub(crate) type Map<K, V> = syn_sem_common::Map<K, V>;
