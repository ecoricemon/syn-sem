//! Logic-backed inference derivation orchestration.

mod projection;
mod subject_type;

pub(crate) use projection::ProjectionDeriver;
pub(crate) use subject_type::SubjectTypeDeriver;
