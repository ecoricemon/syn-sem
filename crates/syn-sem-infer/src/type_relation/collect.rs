//! Initial type relation equality fact collection.

use super::{TypeEqualityFact, TypeRelationDb, TypeSubject};
use crate::{
    GenericArg, InferTypes, PathType, PathTypeResolution, PrimitiveType, ProjectionType, QSelf,
    Type, TypeId,
};
use std::collections::hash_map::Entry;
use syn_sem_common::{Map, VecUniqueExt};
use syn_sem_hir as hir;
use syn_sem_name as name;

pub(crate) struct TypeRelationCollector<'a, 'cx> {
    hir: &'a hir::Hir<'cx>,
    names: &'a name::NameDb<'cx>,
    types: &'a mut InferTypes<'cx>,
    equalities: Vec<TypeEqualityFact>,
}

#[derive(Default)]
struct CallTypeSubstitutions<'cx> {
    by_def: Map<name::DefId, TypeId>,
    by_name: Map<name::Name<'cx>, TypeId>,
}

impl<'a, 'cx> TypeRelationCollector<'a, 'cx> {
    pub(crate) fn collect(
        hir: &'a hir::Hir<'cx>,
        names: &'a name::NameDb<'cx>,
        types: &'a mut InferTypes<'cx>,
    ) -> TypeRelationDb {
        Self {
            hir,
            names,
            types,
            equalities: Vec::new(),
        }
        .collect_inner()
    }

    fn collect_inner(mut self) -> TypeRelationDb {
        self.collect_signature_facts();
        self.collect_expr_facts();
        self.collect_lowered_block_facts();

        TypeRelationDb {
            equalities: self.equalities,
            resolved: Vec::new(),
            expr_types: Map::default(),
            def_types: Map::default(),
        }
    }

    fn collect_signature_facts(&mut self) {
        for signature in self.hir.signatures() {
            for param in signature.params.iter().skip(1) {
                if let Some(pat) = param.pat {
                    self.collect_pat_type_facts(pat, param.ty);
                }
            }
        }
    }

    fn collect_expr_facts(&mut self) {
        for expr in self.hir.exprs() {
            match &expr.kind {
                hir::ExprKind::Block { block } | hir::ExprKind::Const { block } => {
                    if let Some(tail_expr) = self.hir.lowered_blocks()[*block].tail_expr {
                        self.intern_type_equality(
                            TypeSubject::Expr(expr.id),
                            TypeSubject::Expr(tail_expr),
                        );
                    }
                }
                hir::ExprKind::Cast { ty, .. } => {
                    self.intern_type_equality(
                        TypeSubject::Expr(expr.id),
                        TypeSubject::Type(
                            self.types
                                .type_for_hir_type(*ty)
                                .expect("HIR types are lowered before type relations"),
                        ),
                    );
                }
                hir::ExprKind::Lit(lit) => {
                    if let Some(ty) = self.lit_type(lit) {
                        self.intern_type_equality(
                            TypeSubject::Expr(expr.id),
                            TypeSubject::Type(ty),
                        );
                    }
                }
                hir::ExprKind::Paren { expr: inner } => {
                    self.intern_type_equality(
                        TypeSubject::Expr(expr.id),
                        TypeSubject::Expr(*inner),
                    );
                }
                hir::ExprKind::Path(path) => {
                    if let Some(def) =
                        Self::resolve_value_path(self.names, expr.scope, &path.segments)
                    {
                        self.intern_type_equality(
                            TypeSubject::Expr(expr.id),
                            TypeSubject::Def(def),
                        );
                    }
                }
                hir::ExprKind::Call { func, args } => {
                    self.collect_call_facts(expr.id, *func, args);
                }
                hir::ExprKind::Array { .. }
                | hir::ExprKind::Assign { .. }
                | hir::ExprKind::Binary { .. }
                | hir::ExprKind::Closure { .. }
                | hir::ExprKind::Field { .. }
                | hir::ExprKind::Index { .. }
                | hir::ExprKind::MethodCall { .. }
                | hir::ExprKind::Reference { .. }
                | hir::ExprKind::Repeat { .. }
                | hir::ExprKind::Return { .. }
                | hir::ExprKind::Struct { .. }
                | hir::ExprKind::Tuple { .. }
                | hir::ExprKind::Unary { .. } => {}
            }
        }

        self.collect_fn_return_facts();
    }

    fn collect_call_facts(&mut self, call: hir::ExprId, func: hir::ExprId, args: &[hir::ExprId]) {
        let hir::ExprKind::Path(path) = &self.hir[func].kind else {
            return;
        };
        let Some(def) = Self::resolve_value_path(self.names, self.hir[func].scope, &path.segments)
        else {
            return;
        };
        let Some(signature) = self.fn_signature_for_def(def) else {
            return;
        };
        let params = &self.hir[signature].params;
        let Some(return_param) = params.first() else {
            return;
        };
        let input_params = &params[1..];
        if args.len() != input_params.len() {
            return;
        }

        let mut call_substitutions = CallTypeSubstitutions::default();

        let Some(return_ty) = self.types.type_for_hir_type(return_param.ty) else {
            return;
        };
        let return_ty = self.instantiate_call_type(return_ty, &mut call_substitutions);
        self.intern_type_equality(TypeSubject::Expr(call), TypeSubject::Type(return_ty));

        let arg_param_tys = args
            .iter()
            .zip(input_params)
            .filter_map(|(arg, param)| {
                let param_ty = self.types.type_for_hir_type(param.ty)?;
                let param_ty = self.instantiate_call_type(param_ty, &mut call_substitutions);
                Some((*arg, param_ty))
            })
            .collect::<Vec<_>>();
        if arg_param_tys.len() != args.len() {
            return;
        }
        for (arg, param_ty) in arg_param_tys {
            self.intern_type_equality(TypeSubject::Expr(arg), TypeSubject::Type(param_ty));
            if param_ty == return_ty {
                self.intern_type_equality(TypeSubject::Expr(call), TypeSubject::Expr(arg));
            }
        }
    }

    fn instantiate_call_type(
        &mut self,
        ty: TypeId,
        substitutions: &mut CallTypeSubstitutions<'cx>,
    ) -> TypeId {
        if let Some((def, name)) = self.generic_param_key(ty) {
            if let Some(ty) = substitutions.by_def.get(&def).copied() {
                return ty;
            }
            if let Some(name) = name {
                if let Some(ty) = substitutions.by_name.get(&name).copied() {
                    substitutions.by_def.insert(def, ty);
                    return ty;
                }
            }
            let fresh = self.types.insert_fresh_type(Type::Infer);
            substitutions.by_def.insert(def, fresh);
            if let Some(name) = name {
                substitutions.by_name.insert(name, fresh);
            }
            return fresh;
        }

        match self.types[ty].clone() {
            Type::Array { elem, len } => {
                let elem = self.instantiate_call_type(elem, substitutions);
                self.types.intern_type(Type::Array { elem, len })
            }
            Type::Infer => self.types.insert_fresh_type(Type::Infer),
            Type::Primitive(_) => ty,
            Type::Path(path) => self.instantiate_call_path_type(path, substitutions),
            Type::Reference { elem, is_mut } => {
                let elem = self.instantiate_call_type(elem, substitutions);
                self.types.intern_type(Type::Reference { elem, is_mut })
            }
            Type::Slice { elem } => {
                let elem = self.instantiate_call_type(elem, substitutions);
                self.types.intern_type(Type::Slice { elem })
            }
            Type::Tuple { elems } => {
                let elems = elems
                    .into_iter()
                    .map(|elem| self.instantiate_call_type(elem, substitutions))
                    .collect();
                self.types.intern_type(Type::Tuple { elems })
            }
        }
    }

    fn instantiate_call_path_type(
        &mut self,
        path: PathType<'cx>,
        substitutions: &mut CallTypeSubstitutions<'cx>,
    ) -> TypeId {
        let resolution = path.resolution;
        let qself = path
            .qself
            .map(|qself| self.instantiate_call_qself(qself, substitutions));
        let path = crate::Path {
            segments: path
                .path
                .segments
                .into_iter()
                .map(|segment| crate::PathSegment {
                    name: segment.name,
                    args: segment
                        .args
                        .into_iter()
                        .map(|arg| self.instantiate_call_generic_arg(arg, substitutions))
                        .collect(),
                })
                .collect(),
        };
        let resolution = self.instantiate_call_path_resolution(resolution, substitutions);
        self.types.intern_type(Type::Path(PathType {
            qself,
            path,
            resolution,
        }))
    }

    fn instantiate_call_qself(
        &mut self,
        qself: QSelf,
        substitutions: &mut CallTypeSubstitutions<'cx>,
    ) -> QSelf {
        QSelf {
            self_: self.instantiate_call_type(qself.self_, substitutions),
            trait_: qself
                .trait_
                .map(|trait_| self.instantiate_call_type(trait_, substitutions)),
        }
    }

    fn instantiate_call_path_resolution(
        &mut self,
        resolution: PathTypeResolution,
        substitutions: &mut CallTypeSubstitutions<'cx>,
    ) -> PathTypeResolution {
        match resolution {
            PathTypeResolution::Projection(projection) => PathTypeResolution::Projection(
                self.instantiate_call_projection(projection, substitutions),
            ),
            PathTypeResolution::GenericParam(def) => {
                if let Entry::Vacant(entry) = substitutions.by_def.entry(def) {
                    let fresh = self.types.insert_fresh_type(Type::Infer);
                    entry.insert(fresh);
                }
                PathTypeResolution::Unresolved
            }
            PathTypeResolution::Nominal(_)
            | PathTypeResolution::Ambiguous(_)
            | PathTypeResolution::Unresolved => resolution,
        }
    }

    fn instantiate_call_projection(
        &mut self,
        projection: ProjectionType,
        substitutions: &mut CallTypeSubstitutions<'cx>,
    ) -> ProjectionType {
        ProjectionType {
            assoc: projection.assoc,
            self_: projection
                .self_
                .map(|self_| self.instantiate_call_type(self_, substitutions)),
            trait_: projection
                .trait_
                .map(|trait_| self.instantiate_call_type(trait_, substitutions)),
        }
    }

    fn instantiate_call_generic_arg(
        &mut self,
        arg: GenericArg<'cx>,
        substitutions: &mut CallTypeSubstitutions<'cx>,
    ) -> GenericArg<'cx> {
        match arg {
            GenericArg::Type(ty) => GenericArg::Type(self.instantiate_call_type(ty, substitutions)),
            GenericArg::Const(arg) => GenericArg::Const(arg),
            GenericArg::AssocType { name, ty } => GenericArg::AssocType {
                name,
                ty: self.instantiate_call_type(ty, substitutions),
            },
            GenericArg::AssocConst { name, value } => GenericArg::AssocConst { name, value },
            GenericArg::Constraint { name, bounds } => GenericArg::Constraint {
                name,
                bounds: bounds
                    .into_iter()
                    .map(|bound| self.instantiate_call_bound(bound, substitutions))
                    .collect(),
            },
            GenericArg::Unsupported => GenericArg::Unsupported,
        }
    }

    fn instantiate_call_bound(
        &mut self,
        bound: crate::TypeParamBound<'cx>,
        substitutions: &mut CallTypeSubstitutions<'cx>,
    ) -> crate::TypeParamBound<'cx> {
        match bound {
            crate::TypeParamBound::Trait(path) => crate::TypeParamBound::Trait(crate::Path {
                segments: path
                    .segments
                    .into_iter()
                    .map(|segment| crate::PathSegment {
                        name: segment.name,
                        args: segment
                            .args
                            .into_iter()
                            .map(|arg| self.instantiate_call_generic_arg(arg, substitutions))
                            .collect(),
                    })
                    .collect(),
            }),
            crate::TypeParamBound::Unsupported => crate::TypeParamBound::Unsupported,
        }
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

    fn field_type(
        &self,
        struct_fields: &[hir::FieldId],
        member: name::Name<'cx>,
    ) -> Option<TypeId> {
        struct_fields.iter().find_map(|field| {
            let field = &self.hir[*field];
            if field.name != member {
                return None;
            }
            self.types.type_for_hir_type(field.ty)
        })
    }

    fn generic_param_key(&self, ty: TypeId) -> Option<(name::DefId, Option<name::Name<'cx>>)> {
        let Type::Path(path) = &self.types[ty] else {
            return None;
        };
        let PathTypeResolution::GenericParam(def) = path.resolution else {
            return None;
        };
        let name = match path.path.segments.as_slice() {
            [segment] => Some(segment.name),
            _ => None,
        };
        Some((def, name))
    }

    fn fn_signature_for_def(&self, def: name::DefId) -> Option<hir::SignatureId> {
        self.hir.items().iter().find_map(|item| {
            if item.def != Some(def) {
                return None;
            }
            let hir::ItemKind::Fn { signature, .. } = item.kind else {
                return None;
            };
            Some(signature)
        })
    }

    fn collect_fn_return_facts(&mut self) {
        for item in self.hir.items() {
            let hir::ItemKind::Fn {
                signature, block, ..
            } = item.kind
            else {
                continue;
            };
            let Some(return_param) = self.hir[signature].params.first() else {
                continue;
            };
            let Some(return_ty) = self.types.type_for_hir_type(return_param.ty) else {
                continue;
            };
            self.collect_tail_return_fact(block, return_ty);
            self.collect_return_expr_facts_in_block(block, return_ty);
        }
    }

    fn collect_tail_return_fact(&mut self, block: hir::BlockId, return_ty: TypeId) {
        let Some(tail_expr) = self.hir.lowered_blocks()[block].tail_expr else {
            return;
        };
        self.intern_type_equality(TypeSubject::Expr(tail_expr), TypeSubject::Type(return_ty));
    }

    fn collect_return_expr_facts_in_block(&mut self, block: hir::BlockId, return_ty: TypeId) {
        let exprs = self.hir.lowered_blocks()[block]
            .stmts
            .iter()
            .filter_map(|stmt| match stmt {
                hir::lower::Stmt::Expr(expr) => Some(*expr),
                hir::lower::Stmt::Local(local) => local.init,
                hir::lower::Stmt::Item(_) => None,
            })
            .collect::<Vec<_>>();
        for expr in exprs {
            self.collect_return_expr_facts_in_expr(expr, return_ty);
        }
    }

    fn collect_return_expr_facts_in_expr(&mut self, expr: hir::ExprId, return_ty: TypeId) {
        match &self.hir[expr].kind {
            hir::ExprKind::Return { expr: Some(value) } => {
                let value = *value;
                self.intern_type_equality(TypeSubject::Expr(value), TypeSubject::Type(return_ty));
                self.collect_return_expr_facts_in_expr(value, return_ty);
            }
            hir::ExprKind::Return { expr: None }
            | hir::ExprKind::Closure { .. }
            | hir::ExprKind::Const { .. }
            | hir::ExprKind::Lit(_)
            | hir::ExprKind::Path(_) => {}
            hir::ExprKind::Array { elems } | hir::ExprKind::Tuple { elems } => {
                let elems = elems.clone();
                for elem in elems {
                    self.collect_return_expr_facts_in_expr(elem, return_ty);
                }
            }
            hir::ExprKind::Assign { left, right } | hir::ExprKind::Binary { left, right, .. } => {
                let (left, right) = (*left, *right);
                self.collect_return_expr_facts_in_expr(left, return_ty);
                self.collect_return_expr_facts_in_expr(right, return_ty);
            }
            hir::ExprKind::Block { block } => {
                self.collect_return_expr_facts_in_block(*block, return_ty);
            }
            hir::ExprKind::Call { func, args } => {
                let func = *func;
                let args = args.clone();
                self.collect_return_expr_facts_in_expr(func, return_ty);
                for arg in args {
                    self.collect_return_expr_facts_in_expr(arg, return_ty);
                }
            }
            hir::ExprKind::Cast { expr, .. }
            | hir::ExprKind::Field { base: expr, .. }
            | hir::ExprKind::Paren { expr }
            | hir::ExprKind::Reference { expr, .. }
            | hir::ExprKind::Repeat { expr, .. }
            | hir::ExprKind::Unary { expr, .. } => {
                self.collect_return_expr_facts_in_expr(*expr, return_ty);
            }
            hir::ExprKind::Index { expr, index } => {
                let (expr, index) = (*expr, *index);
                self.collect_return_expr_facts_in_expr(expr, return_ty);
                self.collect_return_expr_facts_in_expr(index, return_ty);
            }
            hir::ExprKind::MethodCall { receiver, args, .. } => {
                let receiver = *receiver;
                let args = args.clone();
                self.collect_return_expr_facts_in_expr(receiver, return_ty);
                for arg in args {
                    self.collect_return_expr_facts_in_expr(arg, return_ty);
                }
            }
            hir::ExprKind::Struct { fields, rest, .. } => {
                let fields = fields.clone();
                let rest = *rest;
                for field in fields {
                    self.collect_return_expr_facts_in_expr(field.expr, return_ty);
                }
                if let Some(rest) = rest {
                    self.collect_return_expr_facts_in_expr(rest, return_ty);
                }
            }
        }
    }

    fn collect_lowered_block_facts(&mut self) {
        for block in self.hir.lowered_blocks().blocks() {
            for stmt in &block.stmts {
                let hir::lower::Stmt::Local(local) = stmt else {
                    continue;
                };
                self.collect_local_facts(local);
            }
        }
    }

    fn collect_local_facts(&mut self, local: &hir::lower::Local) {
        if let Some(init) = local.init {
            self.bind_pat_to_expr(local.pat, init);
        }
    }

    fn collect_pat_type_facts(&mut self, pat: hir::PatId, hir_ty_id: hir::TypeId) {
        if let Some(ty) = self.types.type_for_hir_type(hir_ty_id) {
            self.bind_pat_to_type(pat, ty);
        }
    }

    fn bind_pat_to_type(&mut self, pat: hir::PatId, ty: TypeId) {
        match &self.hir[pat].kind {
            hir::PatKind::Ident { def: Some(def), .. } => {
                self.intern_type_equality(TypeSubject::Def(*def), TypeSubject::Type(ty));
            }
            hir::PatKind::Reference { pat, .. } | hir::PatKind::Type { pat, .. } => {
                self.bind_pat_to_type(*pat, ty);
            }
            hir::PatKind::Struct { fields, .. } => {
                self.bind_struct_pat_to_type(fields, ty);
            }
            hir::PatKind::Tuple { elems } => {
                let pat_elems = elems.clone();
                let Type::Tuple { elems: ty_elems } = &self.types[ty] else {
                    return;
                };
                if pat_elems.len() != ty_elems.len() {
                    return;
                }
                let ty_elems = ty_elems.clone();
                for (pat, ty) in pat_elems.into_iter().zip(ty_elems) {
                    self.bind_pat_to_type(pat, ty);
                }
            }
            hir::PatKind::Ident { def: None, .. }
            | hir::PatKind::Path(_)
            | hir::PatKind::Unsupported => {}
        }
    }

    fn bind_struct_pat_to_type(&mut self, fields: &[hir::PatStructField<'cx>], ty: TypeId) {
        let Some(struct_fields) = self.struct_fields_for_type(ty) else {
            return;
        };
        let struct_fields = struct_fields.to_vec();
        for field in fields {
            let Some(field_ty) = self.field_type(&struct_fields, field.member) else {
                continue;
            };
            self.bind_pat_to_type(field.pat, field_ty);
        }
    }

    fn bind_pat_to_expr(&mut self, pat: hir::PatId, expr: hir::ExprId) {
        match &self.hir[pat].kind {
            hir::PatKind::Ident { def: Some(def), .. } => {
                self.intern_type_equality(TypeSubject::Def(*def), TypeSubject::Expr(expr));
            }
            hir::PatKind::Reference { pat, .. } => self.bind_pat_to_expr(*pat, expr),
            hir::PatKind::Type { pat, ty } => {
                let Some(ty) = self.types.type_for_hir_type(*ty) else {
                    return;
                };
                self.bind_pat_to_type(*pat, ty);
                self.intern_type_equality(TypeSubject::Expr(expr), TypeSubject::Type(ty));
            }
            hir::PatKind::Ident { def: None, .. }
            | hir::PatKind::Path(_)
            | hir::PatKind::Struct { .. }
            | hir::PatKind::Tuple { .. }
            | hir::PatKind::Unsupported => {}
        }
    }

    fn lit_type(&mut self, lit: &hir::Lit<'cx>) -> Option<TypeId> {
        match lit {
            hir::Lit::Bool(_) => Some(self.types.intern_type(Type::Primitive(PrimitiveType::Bool))),
            hir::Lit::Int(_) => Some(
                self.types
                    .insert_fresh_type(Type::Primitive(PrimitiveType::AbstractInt)),
            ),
            hir::Lit::Float(_) => Some(
                self.types
                    .insert_fresh_type(Type::Primitive(PrimitiveType::AbstractFloat)),
            ),
        }
    }

    fn intern_type_equality(&mut self, left: TypeSubject, right: TypeSubject) {
        let fact = TypeEqualityFact { left, right };
        self.equalities.push_unique(fact);
    }

    fn resolve_value_path(
        names: &name::NameDb<'cx>,
        scope: Option<syn_sem_name::ScopeId>,
        path: &[hir::PathSegment<'cx>],
    ) -> Option<syn_sem_name::DefId> {
        let scope = scope?;
        match names.resolve_value_path(scope, path.iter().map(|segment| segment.name)) {
            name::ResolveResult::Found(def) => Some(def),
            name::ResolveResult::Ambiguous(_) | name::ResolveResult::NotFound => None,
        }
    }
}
