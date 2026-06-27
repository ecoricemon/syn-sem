//! Associated type projection obligation collection.

use super::{ProjectionDb, ProjectionObligation};
use crate::{InferTypes, PathTypeResolution, Type};

pub(crate) struct ProjectionCollector;

impl ProjectionCollector {
    pub(crate) fn collect(types: &InferTypes<'_>) -> ProjectionDb {
        let obligations = types
            .iter()
            .filter_map(|(ty_id, ty)| {
                let Type::Path(path_ty) = ty else {
                    return None;
                };
                let PathTypeResolution::Projection(projection) = &path_ty.resolution else {
                    return None;
                };
                let self_ = projection.self_?;

                Some(ProjectionObligation {
                    projection: ty_id,
                    assoc: projection.assoc,
                    self_,
                    trait_: projection.trait_,
                })
            })
            .collect();

        ProjectionDb {
            obligations,
            projection_matches: Vec::new(),
            impl_self_matches: Vec::new(),
            impl_self_generic_bindings: Vec::new(),
            type_substitutions: Vec::new(),
            normalizations: Vec::new(),
        }
    }
}
