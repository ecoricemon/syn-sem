//! Body and block inference.
//!
//! This module will own block-level orchestration: function bodies, statements, local bindings,
//! tail expressions, and the body-local type environment used while delegating expression typing
//! to `expr`.
