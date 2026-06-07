//! Rust-shaped semantic program model for `syn-sem`.
//!
//! This crate will own the model built after AST collection and name resolution. The model should
//! preserve source-level item and body structure while giving later semantic phases a stable input.

/// Semantic program model produced from AST and name-resolution data.
#[derive(Debug, Default)]
pub struct Model;
