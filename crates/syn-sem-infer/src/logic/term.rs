//! Logic term builders grouped by inference relation domain.

mod atom;
mod equality;
mod projection;
mod subject_type;

pub(super) use atom::{type_id_from_term, LogicAtom, LogicClause};
pub(super) use equality::same_type_rules;
pub(super) use projection::*;
pub(super) use subject_type::*;
