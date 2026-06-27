//! Type inference for `syn-sem`.
//!
//! This crate consumes the current HIR container from `syn-sem-hir` and name facts from
//! `syn-sem-name`. Its current public surface lowers HIR type occurrences into
//! inference-owned type ids while preserving path syntax and keeping name lookup classification
//! separate from final type solving.
//!
//! Analysis currently has distinct source-shaped phases:
//!
//! * program fact collection records declared trait bounds and associated type impl values;
//! * HIR type occurrence lowering records source type syntax as inference-owned [`TypeId`]s;
//! * Projection normalization resolves associated type projections such as
//!   `<T as Iterator>::Item`;
//! * Type relation resolution follows equality edges between definitions, expressions, and known
//!   type candidates;
//! * Expression result and pattern binding inference are intentionally still narrow; only forms
//!   that feed type relation facts are available through [`InferDb`] query methods.
//!
//! Upper phases should enter through [`InferDb::analyze`], ask for a type occurrence with
//! [`InferDb::type_for_hir_type`] or [`InferDb::normalized_type_for_hir_type`], and inspect the
//! resulting [`Type`] through focused query methods such as [`InferDb::projection`] and
//! [`InferDb::projection_normalization`].

mod db;
mod id;
mod logic;
mod program_fact;
mod projection;
mod type_lowering;
mod type_relation;
mod type_store;
mod types;

pub use db::*;
pub use id::*;
pub use projection::*;
pub use types::*;

pub(crate) use program_fact::{
    ImplAssocType, ImplAssocTypeCollector, TraitBound, TraitBoundCollector,
};
pub(crate) use type_lowering::{TypeLowerer, TypeLowering};
pub(crate) use type_relation::{
    ExprTypeDeriver, PatTypeDeriver, TypeEqualityFact, TypeRelationCollector, TypeRelationDb,
    TypeRelationResolver, TypeSubject,
};
pub(crate) use type_store::InferTypes;
