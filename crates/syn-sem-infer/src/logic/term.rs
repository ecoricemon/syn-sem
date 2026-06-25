//! Logic term builders grouped by inference relation domain.

mod atom;
mod equality;
mod projection;
mod subject_type;
pub(in crate::logic) mod symbol;
mod type_shape;

pub(super) use atom::{atom, type_id_from_term, LogicAtom, LogicTerm};
pub(super) use equality::same_type_rules;
pub(super) use projection::*;
pub(super) use subject_type::*;
pub(super) use type_shape::*;
