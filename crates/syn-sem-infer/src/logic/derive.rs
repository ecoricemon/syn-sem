//! Logic-backed inference derivation orchestration.

mod projection;
mod subject_type;

use crate::InferDb;
use syn_sem_common::CommonCx;
use syn_sem_name::NameDb;

pub(crate) fn derive<'cx>(ccx: &'cx CommonCx, db: &mut InferDb<'cx>, names: &NameDb<'cx>) {
    projection::ProjectionDeriver::new(ccx, db, names).derive();
    subject_type::SubjectTypeDeriver::new(ccx, db).derive();
}
