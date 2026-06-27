//! Program-declared type facts collected before solver phases run.
//!
//! This module owns facts read directly from Rust declarations rather than facts derived by an
//! inference solver. For example, `where T: Iterator` becomes a [`TraitBound`], and
//! `impl Iterator for MyIter { type Item = u32; }` becomes an [`ImplAssocType`].
//! Projection normalization currently consumes these facts, but they describe the program itself,
//! not the projection phase.

mod impl_assoc_type;
mod trait_bound;

pub(crate) use impl_assoc_type::{ImplAssocType, ImplAssocTypeCollector};
pub(crate) use trait_bound::{TraitBound, TraitBoundCollector};
