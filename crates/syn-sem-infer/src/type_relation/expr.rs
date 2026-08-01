//! HIR expression result type fact derivation.

use super::{TypeEqualityFact, TypeRelationDb, TypeSubject};
use crate::{
    ArrayLen, GenericArg, InferConstFacts, InferTypes, Path, PathSegment, PathType,
    PathTypeResolution, PrimitiveType, ProjectionDb, ProjectionNormalizationResult,
    ProjectionObligation, ProjectionType, QSelf, Type, TypeId, TypeLowerer,
};
use syn_sem_common::{CommonCx, Str, VecUniqueExt};
use syn_sem_hir as hir;
use syn_sem_name::{DefKind, NameDb, Namespace, ResolveResult};

/// Derives HIR expression result type equalities from resolved operand types.
///
/// This phase extends the subject equality graph with facts that are not visible from source
/// bindings alone. It reads expression shapes from HIR, asks [`TypeRelationDb`] for already resolved
/// operand types, interns any newly constructed result types, and records the result as another
/// subject equality fact.
pub(crate) struct ExprTypeDeriver<'a, 'cx> {
    ccx: &'cx CommonCx,
    hir: &'a hir::Hir<'cx>,
    names: &'a NameDb<'cx>,
    projections: &'a mut ProjectionDb,
    type_relations: &'a mut TypeRelationDb,
    types: &'a mut InferTypes<'cx>,
    const_facts: &'a InferConstFacts,
}

impl<'a, 'cx> ExprTypeDeriver<'a, 'cx> {
    pub(crate) fn new(
        ccx: &'cx CommonCx,
        hir: &'a hir::Hir<'cx>,
        names: &'a NameDb<'cx>,
        projections: &'a mut ProjectionDb,
        type_relations: &'a mut TypeRelationDb,
        types: &'a mut InferTypes<'cx>,
        const_facts: &'a InferConstFacts,
    ) -> Self {
        Self {
            ccx,
            hir,
            names,
            projections,
            type_relations,
            types,
            const_facts,
        }
    }

    /// Runs one expression result type derivation pass.
    ///
    /// For example, given:
    /// ```text
    /// fn f(x: usize) {
    ///     let r = &x;
    /// }
    /// ```
    ///
    /// after type relation resolution has resolved `x -> usize`, this derives:
    /// ```text
    /// operand lookup:      x -> usize
    /// result type:         &x has type &usize
    /// equality fact:       expr(&x) == type(&usize)
    /// next propagation:    r -> &usize
    /// ```
    ///
    /// The return value reports whether new equality facts were added, so callers can repeat
    /// subject propagation and expression derivation until the graph reaches a fixed point.
    pub(crate) fn derive(&mut self) -> bool {
        let mut changed = false;
        for expr in self.hir.exprs() {
            changed |= self.derive_expr(expr);
        }
        changed
    }

    fn derive_expr(&mut self, expr: &hir::Expr<'cx>) -> bool {
        match &expr.kind {
            hir::ExprKind::Array { elems } => self.derive_array(expr.id, elems),
            hir::ExprKind::Binary { op, left, right } => {
                self.derive_binary(expr.id, *op, *left, *right)
            }
            hir::ExprKind::Reference {
                expr: inner,
                is_mut,
            } => self.derive_reference(expr.id, *inner, *is_mut),
            hir::ExprKind::Field { base, member } => self.derive_field(expr.id, *base, *member),
            hir::ExprKind::Repeat { expr: elem, len } => self.derive_repeat(expr.id, *elem, *len),
            hir::ExprKind::Struct { path, fields, rest } => {
                self.derive_struct(expr.id, expr.scope, path, fields, *rest)
            }
            hir::ExprKind::Tuple { elems } => self.derive_tuple(expr.id, elems),
            hir::ExprKind::Unary { op, expr: inner } => self.derive_unary(expr.id, *op, *inner),
            hir::ExprKind::Assign { left, right } => self.derive_assign(expr.id, *left, *right),
            hir::ExprKind::Block { .. }
            | hir::ExprKind::Call { .. }
            | hir::ExprKind::Cast { .. }
            | hir::ExprKind::Closure { .. }
            | hir::ExprKind::Const { .. }
            | hir::ExprKind::Index { .. }
            | hir::ExprKind::Lit(_)
            | hir::ExprKind::MethodCall { .. }
            | hir::ExprKind::Paren { .. }
            | hir::ExprKind::Path(_)
            | hir::ExprKind::Return { .. } => false,
        }
    }

    fn derive_array(&mut self, expr: hir::ExprId, elems: &[hir::ExprId]) -> bool {
        if elems.is_empty() {
            return false;
        }
        let Some(elems) = self.resolved_expr_types(elems) else {
            return false;
        };
        let Some(elem) = self.common_elem_type(&elems) else {
            return false;
        };
        let array = self.types.intern_type(Type::Array {
            elem,
            len: ArrayLen::ConstUsize(elems.len()),
        });
        self.insert_expr_type_equality(expr, array)
    }

    fn derive_assign(&mut self, expr: hir::ExprId, left: hir::ExprId, right: hir::ExprId) -> bool {
        let unit = self.types.intern_type(Type::Tuple { elems: Vec::new() });
        self.insert_expr_equality(left, right) | self.insert_expr_type_equality(expr, unit)
    }

    fn derive_binary(
        &mut self,
        expr: hir::ExprId,
        op: hir::BinaryOp,
        left: hir::ExprId,
        right: hir::ExprId,
    ) -> bool {
        match op {
            hir::BinaryOp::Add
            | hir::BinaryOp::Sub
            | hir::BinaryOp::Mul
            | hir::BinaryOp::Div
            | hir::BinaryOp::Rem => self
                .derive_binary_output_operator_projection(expr, op, left, right)
                .unwrap_or_else(|| {
                    self.derive_same_type_binary(expr, left, right, |primitive| {
                        primitive.is_numeric()
                    })
                }),
            hir::BinaryOp::BitXor | hir::BinaryOp::BitAnd | hir::BinaryOp::BitOr => self
                .derive_binary_output_operator_projection(expr, op, left, right)
                .unwrap_or_else(|| {
                    self.derive_same_type_binary(expr, left, right, |primitive| {
                        primitive == PrimitiveType::Bool || primitive.is_integer()
                    })
                }),
            hir::BinaryOp::Shl | hir::BinaryOp::Shr => self.derive_shift_binary(expr, left, right),
            hir::BinaryOp::And | hir::BinaryOp::Or => {
                let bool_ty = self.bool_type();
                self.insert_expr_type_equality(expr, bool_ty)
                    | self.insert_expr_type_equality(left, bool_ty)
                    | self.insert_expr_type_equality(right, bool_ty)
            }
            hir::BinaryOp::Eq
            | hir::BinaryOp::Lt
            | hir::BinaryOp::Le
            | hir::BinaryOp::Ne
            | hir::BinaryOp::Ge
            | hir::BinaryOp::Gt => {
                let bool_ty = self.bool_type();
                self.insert_expr_type_equality(expr, bool_ty)
                    | self.derive_operand_equality(left, right, |primitive| {
                        primitive == PrimitiveType::Bool || primitive.is_numeric()
                    })
            }
        }
    }

    fn derive_same_type_binary(
        &mut self,
        expr: hir::ExprId,
        left: hir::ExprId,
        right: hir::ExprId,
        accepts: impl Fn(PrimitiveType) -> bool,
    ) -> bool {
        if !self.has_concrete_operator_type([expr, left, right], accepts) {
            return false;
        }
        self.insert_expr_equality(expr, left) | self.insert_expr_equality(left, right)
    }

    fn derive_shift_binary(
        &mut self,
        expr: hir::ExprId,
        left: hir::ExprId,
        right: hir::ExprId,
    ) -> bool {
        if !self.has_concrete_operator_type([expr, left], PrimitiveType::is_integer) {
            return false;
        }
        let abstract_int = self.abstract_int_type();
        self.insert_expr_equality(expr, left) | self.insert_expr_type_equality(right, abstract_int)
    }

    fn derive_operand_equality(
        &mut self,
        left: hir::ExprId,
        right: hir::ExprId,
        accepts: impl Fn(PrimitiveType) -> bool,
    ) -> bool {
        if !self.has_concrete_operator_type([left, right], accepts) {
            return false;
        }
        self.insert_expr_equality(left, right)
    }

    fn derive_binary_output_operator_projection(
        &mut self,
        expr: hir::ExprId,
        op: hir::BinaryOp,
        left: hir::ExprId,
        right: hir::ExprId,
    ) -> Option<bool> {
        let left_ty = self.type_relations.type_for_hir_expr(left)?;
        let right_ty = self.type_relations.type_for_hir_expr(right)?;
        let scope = self.hir[expr].scope?;
        let trait_name = Self::binary_output_operator_trait_name(op)?;
        let trait_def = self.resolve_core_ops_trait(scope, trait_name)?;
        let output_def = self.trait_assoc(trait_def, "Output")?;
        let rhs_ty = self.defaulted_operator_rhs(left_ty, right_ty);
        let trait_ty = self.core_ops_trait_type(trait_def, trait_name, rhs_ty);
        let projection = self.operator_output_projection(left_ty, trait_ty, output_def);

        let mut changed = self
            .projections
            .obligations
            .push_unique(ProjectionObligation {
                projection,
                assoc: output_def,
                self_: left_ty,
                trait_: Some(trait_ty),
            });
        if rhs_ty != right_ty {
            changed |= self.insert_expr_type_equality(right, rhs_ty);
        }
        if let ProjectionNormalizationResult::Known(value_ty) =
            self.projections.normalization(projection, true)
        {
            changed |= self.insert_expr_type_equality(expr, value_ty);
        }
        Some(changed)
    }

    fn binary_output_operator_trait_name(op: hir::BinaryOp) -> Option<&'static str> {
        match op {
            hir::BinaryOp::Add => Some("Add"),
            hir::BinaryOp::Sub => Some("Sub"),
            hir::BinaryOp::Mul => Some("Mul"),
            hir::BinaryOp::Div => Some("Div"),
            hir::BinaryOp::Rem => Some("Rem"),
            hir::BinaryOp::BitXor => Some("BitXor"),
            hir::BinaryOp::BitAnd => Some("BitAnd"),
            hir::BinaryOp::BitOr => Some("BitOr"),
            hir::BinaryOp::And
            | hir::BinaryOp::Or
            | hir::BinaryOp::Shl
            | hir::BinaryOp::Shr
            | hir::BinaryOp::Eq
            | hir::BinaryOp::Lt
            | hir::BinaryOp::Le
            | hir::BinaryOp::Ne
            | hir::BinaryOp::Ge
            | hir::BinaryOp::Gt => None,
        }
    }

    fn resolve_core_ops_trait(
        &self,
        scope: syn_sem_name::ScopeId,
        trait_name: &str,
    ) -> Option<syn_sem_name::DefId> {
        let core = self.ccx.intern("core");
        let ops = self.ccx.intern("ops");
        let trait_name = self.ccx.intern(trait_name);
        let ResolveResult::Found(def) = self
            .names
            .resolve_type_path(scope, [core, ops, trait_name].into_iter())
        else {
            return None;
        };
        (self.names[def].kind == DefKind::Trait).then_some(def)
    }

    fn trait_assoc(
        &self,
        trait_def: syn_sem_name::DefId,
        assoc_name: &str,
    ) -> Option<syn_sem_name::DefId> {
        let assoc_name = self.ccx.intern(assoc_name);
        let ResolveResult::Found(def) = self.names.member(trait_def, Namespace::Type, assoc_name)
        else {
            return None;
        };
        (self.names[def].kind == DefKind::AssocType).then_some(def)
    }

    fn defaulted_operator_rhs(&self, left_ty: TypeId, right_ty: TypeId) -> TypeId {
        let Some(right_primitive) = self.primitive(right_ty) else {
            return right_ty;
        };
        if !matches!(
            right_primitive,
            PrimitiveType::AbstractInt | PrimitiveType::AbstractFloat
        ) {
            return right_ty;
        }
        let Some(left_primitive) = self.primitive(left_ty) else {
            return right_ty;
        };
        if right_primitive.is_abstract_of(left_primitive) {
            left_ty
        } else {
            right_ty
        }
    }

    fn core_ops_trait_type(
        &mut self,
        trait_def: syn_sem_name::DefId,
        trait_name: &str,
        rhs_ty: TypeId,
    ) -> TypeId {
        let trait_name = self.ccx.intern(trait_name);
        self.types.intern_type(Type::Path(PathType {
            qself: None,
            path: Path {
                segments: vec![PathSegment {
                    name: trait_name,
                    args: vec![GenericArg::Type(rhs_ty)],
                }],
            },
            resolution: PathTypeResolution::Nominal(trait_def),
        }))
    }

    fn operator_output_projection(
        &mut self,
        self_: TypeId,
        trait_: TypeId,
        assoc: syn_sem_name::DefId,
    ) -> TypeId {
        let output = self.ccx.intern("Output");
        self.types.intern_type(Type::Path(PathType {
            qself: Some(QSelf {
                self_,
                trait_: Some(trait_),
            }),
            path: Path {
                segments: vec![PathSegment {
                    name: output,
                    args: Vec::new(),
                }],
            },
            resolution: PathTypeResolution::Projection(ProjectionType {
                assoc,
                self_: Some(self_),
                trait_: Some(trait_),
            }),
        }))
    }

    fn derive_reference(&mut self, expr: hir::ExprId, inner: hir::ExprId, is_mut: bool) -> bool {
        let Some(elem) = self.type_relations.type_for_hir_expr(inner) else {
            return false;
        };
        let reference = self.types.intern_type(Type::Reference { elem, is_mut });
        self.insert_expr_type_equality(expr, reference)
    }

    fn derive_field(&mut self, expr: hir::ExprId, base: hir::ExprId, member: Str<'cx>) -> bool {
        let Some(base_ty) = self.type_relations.type_for_hir_expr(base) else {
            return false;
        };
        let Some(field_ty) = self.field_type_for_base(base_ty, member) else {
            return false;
        };
        self.insert_expr_type_equality(expr, field_ty)
    }

    fn derive_struct(
        &mut self,
        expr: hir::ExprId,
        scope: Option<syn_sem_name::ScopeId>,
        path: &hir::Path<'cx>,
        fields: &[hir::ExprStructField<'cx>],
        rest: Option<hir::ExprId>,
    ) -> bool {
        let ty = TypeLowerer::new(self.hir, self.names, self.types)
            .lower_plain_path_as_type(&path.segments, scope);
        let Some(struct_fields) = self.non_generic_struct_fields_for_type(ty) else {
            return false;
        };
        let struct_fields = struct_fields.to_vec();

        let mut changed = self.insert_expr_type_equality(expr, ty);
        for field in fields {
            let Some(field_ty) = self.field_type_from_fields(&struct_fields, field.member) else {
                continue;
            };
            changed |= self.insert_expr_type_equality(field.expr, field_ty);
        }
        if let Some(rest) = rest {
            changed |= self.insert_expr_type_equality(rest, ty);
        }
        changed
    }

    fn derive_repeat(&mut self, expr: hir::ExprId, elem: hir::ExprId, len: hir::ExprId) -> bool {
        let Some(elem) = self.type_relations.type_for_hir_expr(elem) else {
            return false;
        };
        let len = self
            .const_facts
            .expect_integer(len)
            .map(ArrayLen::ConstUsize)
            .unwrap_or(ArrayLen::Expr(len));
        let array = self.types.intern_type(Type::Array { elem, len });
        self.insert_expr_type_equality(expr, array)
    }

    fn derive_tuple(&mut self, expr: hir::ExprId, elems: &[hir::ExprId]) -> bool {
        let Some(elems) = self.resolved_expr_types(elems) else {
            return false;
        };
        let tuple = self.types.intern_type(Type::Tuple { elems });
        self.insert_expr_type_equality(expr, tuple)
    }

    fn derive_unary(&mut self, expr: hir::ExprId, op: hir::UnaryOp, inner: hir::ExprId) -> bool {
        let Some(inner_ty) = self.type_relations.type_for_hir_expr(inner) else {
            return false;
        };
        if op == hir::UnaryOp::Deref {
            return self.derive_deref(expr, inner_ty);
        }

        let Some(primitive) = self.primitive(inner_ty) else {
            return false;
        };

        match op {
            hir::UnaryOp::Not if primitive == PrimitiveType::Bool || primitive.is_integer() => {
                self.insert_expr_type_equality(expr, inner_ty)
            }
            hir::UnaryOp::Neg if primitive.is_numeric() => {
                self.insert_expr_type_equality(expr, inner_ty)
            }
            hir::UnaryOp::Deref | hir::UnaryOp::Not | hir::UnaryOp::Neg => false,
        }
    }

    fn derive_deref(&mut self, expr: hir::ExprId, inner_ty: TypeId) -> bool {
        let Type::Reference { elem, .. } = self.types[inner_ty] else {
            return false;
        };
        self.insert_expr_type_equality(expr, elem)
    }

    fn resolved_expr_types(&self, exprs: &[hir::ExprId]) -> Option<Vec<TypeId>> {
        exprs
            .iter()
            .map(|expr| self.type_relations.type_for_hir_expr(*expr))
            .collect()
    }

    fn common_elem_type(&self, elems: &[TypeId]) -> Option<TypeId> {
        let (&first, rest) = elems.split_first()?;
        rest.iter()
            .all(|elem| first == *elem || self.types[first] == self.types[*elem])
            .then_some(first)
    }

    fn primitive(&self, ty: TypeId) -> Option<PrimitiveType> {
        match self.types[ty] {
            Type::Primitive(primitive) => Some(primitive),
            _ => None,
        }
    }

    fn field_type_for_base(&self, base_ty: TypeId, member: Str<'cx>) -> Option<TypeId> {
        let struct_fields = self.struct_fields_for_type(base_ty)?;
        self.field_type_from_fields(struct_fields, member)
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

    fn non_generic_struct_fields_for_type(&self, ty: TypeId) -> Option<&[hir::FieldId]> {
        let def = self.types.nominal_def(ty)?;
        self.hir.items().iter().find_map(|item| {
            if item.def != Some(def) {
                return None;
            }
            let hir::ItemKind::Struct { generics, fields } = &item.kind else {
                return None;
            };
            generics.params.is_empty().then_some(fields.as_slice())
        })
    }

    fn field_type_from_fields(&self, fields: &[hir::FieldId], member: Str<'cx>) -> Option<TypeId> {
        fields.iter().find_map(|field| {
            let field = &self.hir[*field];
            if field.name != member {
                return None;
            }
            self.types.type_for_hir_type(field.ty)
        })
    }

    fn has_concrete_operator_type(
        &self,
        exprs: impl IntoIterator<Item = hir::ExprId>,
        accepts: impl Fn(PrimitiveType) -> bool,
    ) -> bool {
        exprs.into_iter().any(|expr| {
            let Some(primitive) = self
                .type_relations
                .type_for_hir_expr(expr)
                .and_then(|ty| self.primitive(ty))
            else {
                return false;
            };
            accepts(primitive)
                && !matches!(
                    primitive,
                    PrimitiveType::AbstractInt | PrimitiveType::AbstractFloat
                )
        })
    }

    fn abstract_int_type(&mut self) -> TypeId {
        self.types
            .intern_type(Type::Primitive(PrimitiveType::AbstractInt))
    }

    fn bool_type(&mut self) -> TypeId {
        self.types.intern_type(Type::Primitive(PrimitiveType::Bool))
    }

    fn insert_expr_equality(&mut self, left: hir::ExprId, right: hir::ExprId) -> bool {
        self.type_relations.insert_equality(TypeEqualityFact {
            left: TypeSubject::Expr(left),
            right: TypeSubject::Expr(right),
        })
    }

    fn insert_expr_type_equality(&mut self, expr: hir::ExprId, ty: TypeId) -> bool {
        self.type_relations.insert_equality(TypeEqualityFact {
            left: TypeSubject::Expr(expr),
            right: TypeSubject::Type(ty),
        })
    }
}

impl PrimitiveType {
    fn is_integer(self) -> bool {
        matches!(
            self,
            Self::AbstractInt
                | Self::I8
                | Self::I16
                | Self::I32
                | Self::I64
                | Self::I128
                | Self::Isize
                | Self::U8
                | Self::U16
                | Self::U32
                | Self::U64
                | Self::U128
                | Self::Usize
        )
    }

    fn is_numeric(self) -> bool {
        self.is_integer() || matches!(self, Self::AbstractFloat | Self::F32 | Self::F64)
    }
}
