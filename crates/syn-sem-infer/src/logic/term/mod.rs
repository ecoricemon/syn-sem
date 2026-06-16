//! Logic term builders grouped by inference relation domain.

mod atom;
mod body_type;
mod projection;

pub(super) use atom::{type_id_from_term, LogicAtom, LogicClause};
pub(super) use body_type::*;
pub(super) use projection::*;
