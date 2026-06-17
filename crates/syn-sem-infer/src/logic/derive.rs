//! Logic-backed inference derivation orchestration.

mod body_type;
mod projection;

use crate::InferDb;
use syn_sem_common::CommonCx;
use syn_sem_name::NameDb;

pub(crate) fn derive<'cx>(ccx: &'cx CommonCx, db: &mut InferDb<'cx>, names: &NameDb<'cx>) {
    projection::ProjectionDeriver::new(ccx, db, names).derive();
    body_type::BodyTypeDeriver::new(ccx, db).derive();
}
