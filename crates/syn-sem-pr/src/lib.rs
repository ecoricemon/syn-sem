//! Program representation for `syn-sem`.
//!
//! This crate owns the current Rust source program representation. It preserves declaration,
//! type, and body entry structure in stable arenas while leaving future desugared body IR as an
//! extension.

mod repr;

pub use repr::*;
