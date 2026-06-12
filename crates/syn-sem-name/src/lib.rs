//! Name-resolution model for `syn-sem`.
//!
//! This crate owns the reusable vocabulary for resolving names: definitions, scopes, namespaces,
//! bindings, imports, and simple lexical lookup. Its core database remains syntax-agnostic, while
//! [`collect`] provides AST-aware collection from already parsed source inputs.

pub mod collect;

mod db;
mod def;
mod id;
mod import;
mod namespace;
mod scope;

pub use db::*;
pub use def::*;
pub use id::*;
pub use import::*;
pub use namespace::*;
pub use scope::*;

/// Interned name used by the name-resolution database.
pub type Name<'cx> = syn_sem_common::InternedStr<'cx>;

pub(crate) type Map<K, V> = syn_sem_common::Map<K, V>;
