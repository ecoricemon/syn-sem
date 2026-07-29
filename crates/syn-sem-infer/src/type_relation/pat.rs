//! HIR pattern binding type fact derivation.

use super::{TypeEqualityFact, TypeRelationDb, TypeSubject};
use crate::{InferTypes, Type, TypeId};
use syn_sem_common::Str;
use syn_sem_hir as hir;

/// Derives binding type equalities from resolved initializer types and pattern structure.
///
/// This phase handles facts that are only visible after type relation resolution. For example,
/// `let (a, b) = pair;` can bind `a` and `b` only after `pair` resolves to a tuple type.
pub(crate) struct PatTypeDeriver<'a, 'cx> {
    hir: &'a hir::Hir<'cx>,
    type_relations: &'a mut TypeRelationDb,
    types: &'a InferTypes<'cx>,
}

impl<'a, 'cx> PatTypeDeriver<'a, 'cx> {
    pub(crate) fn new(
        hir: &'a hir::Hir<'cx>,
        type_relations: &'a mut TypeRelationDb,
        types: &'a InferTypes<'cx>,
    ) -> Self {
        Self {
            hir,
            type_relations,
            types,
        }
    }

    /// Runs one pattern binding type derivation pass.
    ///
    /// The return value reports whether new equality facts were added, so callers can repeat
    /// subject propagation and pattern derivation until the graph reaches a fixed point.
    pub(crate) fn derive(&mut self) -> bool {
        let mut changed = false;
        for block in self.hir.lowered_blocks().blocks() {
            for stmt in &block.stmts {
                let hir::lower::Stmt::Local(local) = stmt else {
                    continue;
                };
                changed |= self.derive_local(local);
            }
        }
        changed
    }

    fn derive_local(&mut self, local: &hir::lower::Local) -> bool {
        let Some(init) = local.init else {
            return false;
        };
        let Some(ty) = self.type_relations.type_for_hir_expr(init) else {
            return false;
        };
        self.derive_pat_type(local.pat, ty)
    }

    fn derive_pat_type(&mut self, pat: hir::PatId, ty: TypeId) -> bool {
        match &self.hir[pat].kind {
            hir::PatKind::Ident { def: Some(def), .. } => {
                self.type_relations.insert_equality(TypeEqualityFact {
                    left: TypeSubject::Def(*def),
                    right: TypeSubject::Type(ty),
                })
            }
            hir::PatKind::Reference { pat, .. } | hir::PatKind::Type { pat, .. } => {
                self.derive_pat_type(*pat, ty)
            }
            hir::PatKind::Struct { fields, .. } => self.derive_struct_pat_type(fields, ty),
            hir::PatKind::Tuple { elems } => {
                let pat_elems = elems.clone();
                let Type::Tuple { elems: ty_elems } = &self.types[ty] else {
                    return false;
                };
                if pat_elems.len() != ty_elems.len() {
                    return false;
                }
                let ty_elems = ty_elems.clone();
                pat_elems
                    .into_iter()
                    .zip(ty_elems)
                    .fold(false, |changed, (pat, ty)| {
                        self.derive_pat_type(pat, ty) | changed
                    })
            }
            hir::PatKind::Ident { def: None, .. }
            | hir::PatKind::Path(_)
            | hir::PatKind::Unsupported => false,
        }
    }

    fn derive_struct_pat_type(&mut self, fields: &[hir::PatStructField<'cx>], ty: TypeId) -> bool {
        let Some(struct_fields) = self.struct_fields_for_type(ty) else {
            return false;
        };
        let struct_fields = struct_fields.to_vec();
        fields.iter().fold(false, |changed, field| {
            let Some(field_ty) = self.field_type(&struct_fields, field.member) else {
                return changed;
            };
            self.derive_pat_type(field.pat, field_ty) | changed
        })
    }

    fn struct_fields_for_type(&self, ty: TypeId) -> Option<&[hir::FieldId]> {
        let def = self.types.nominal_def(ty)?;
        self.hir.items().iter().find_map(|item| {
            if item.def != Some(def) {
                return None;
            }
            let hir::ItemKind::Struct { fields, .. } = &item.kind else {
                return None;
            };
            Some(fields.as_slice())
        })
    }

    fn field_type(&self, struct_fields: &[hir::FieldId], member: Str<'cx>) -> Option<TypeId> {
        struct_fields.iter().find_map(|field| {
            let field = &self.hir[*field];
            if field.name != member {
                return None;
            }
            self.types.type_for_hir_type(field.ty)
        })
    }
}
