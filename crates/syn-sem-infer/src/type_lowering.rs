//! HIR type occurrence lowering for inference.
//!
//! This phase translates each HIR type occurrence into an inference-owned [`TypeId`] while
//! preserving source occurrence identity. Path lowering also classifies name-resolution results,
//! including generic parameters, nominal types, and associated type projections.

use crate::{
    ArrayLen, ConstArg, GenericArg, InferTypes, Path, PathSegment, PathType, PathTypeResolution,
    PrimitiveType, ProjectionType, QSelf, Type, TypeId, TypeParamBound,
};
use syn_sem_hir as hir;
use syn_sem_name::{DefId, DefKind, NameDb, Namespace, ResolveResult, ScopeId};

pub(crate) struct TypeLowering;

impl TypeLowering {
    pub(crate) fn lower_hir_types<'cx>(
        hir: &hir::Hir<'cx>,
        names: &NameDb<'cx>,
        types: &mut InferTypes<'cx>,
    ) {
        let mut ty_lowerer = TypeLowerer::new(hir, names, types);
        for ty in hir.types() {
            ty_lowerer.lower_hir_type(ty.id);
        }
    }
}

pub(crate) struct TypeLowerer<'a, 'cx> {
    hir: &'a hir::Hir<'cx>,
    names: &'a NameDb<'cx>,
    types: &'a mut InferTypes<'cx>,
}

impl<'a, 'cx> TypeLowerer<'a, 'cx> {
    pub(crate) fn new(
        hir: &'a hir::Hir<'cx>,
        names: &'a NameDb<'cx>,
        types: &'a mut InferTypes<'cx>,
    ) -> Self {
        Self { hir, names, types }
    }

    pub(crate) fn lower_hir_type(&mut self, hir_ty_id: hir::TypeId) -> TypeId {
        if let Some(ty) = self.types.type_for_hir_type(hir_ty_id) {
            return ty;
        }

        // HIR types always lower to fresh infer types.
        let ty = self.lower_type(hir_ty_id, &self.hir[hir_ty_id].kind);
        let ty_id = self.types.insert_fresh_type(ty);
        self.types.bind_hir_type(hir_ty_id, ty_id);
        ty_id
    }

    fn lower_type(&mut self, hir_ty_id: hir::TypeId, kind: &hir::TypeKind<'cx>) -> Type<'cx> {
        match kind {
            hir::TypeKind::Array { elem, len } => Type::Array {
                elem: self.lower_hir_type(*elem),
                len: ArrayLen::from_hir(*len),
            },
            hir::TypeKind::Infer => Type::Infer,
            hir::TypeKind::Path(path) => self.lower_path_type(hir_ty_id, path),
            hir::TypeKind::Reference { elem, is_mut } => Type::Reference {
                elem: self.lower_hir_type(*elem),
                is_mut: *is_mut,
            },
            hir::TypeKind::Slice { elem } => Type::Slice {
                elem: self.lower_hir_type(*elem),
            },
            hir::TypeKind::Tuple { elems } => Type::Tuple {
                elems: elems
                    .iter()
                    .map(|elem| self.lower_hir_type(*elem))
                    .collect(),
            },
        }
    }

    fn lower_path_type(&mut self, hir_ty_id: hir::TypeId, path: &hir::Path<'cx>) -> Type<'cx> {
        if path.qself.is_none() {
            if let Some(primitive) = PrimitiveType::from_hir_path(&path.segments) {
                return Type::Primitive(primitive);
            }
        }

        let scope = self.hir[hir_ty_id].scope;
        let qself = self.lower_qself(path.qself.as_ref(), scope);
        let resolution = self.resolve_path_type(qself.as_ref(), &path.segments, scope);

        Type::Path(PathType {
            qself,
            path: self.lower_plain_path(&path.segments),
            resolution,
        })
    }

    fn lower_qself(
        &mut self,
        qself: Option<&hir::QSelf<'cx>>,
        scope: Option<ScopeId>,
    ) -> Option<QSelf> {
        let qself = qself?;
        let trait_ = (!qself.trait_path.is_empty())
            .then(|| self.lower_plain_path_as_type(&qself.trait_path, scope));
        Some(QSelf {
            self_: self.lower_hir_type(qself.self_),
            trait_,
        })
    }

    pub(crate) fn lower_plain_path_as_type(
        &mut self,
        path: &[hir::PathSegment<'cx>],
        scope: Option<ScopeId>,
    ) -> TypeId {
        let resolution = self.resolve_path_type(None, path, scope);
        let ty = Type::Path(PathType {
            qself: None,
            path: self.lower_plain_path(path),
            resolution,
        });
        self.types.intern_type(ty)
    }

    fn resolve_path_type(
        &self,
        qself: Option<&QSelf>,
        path: &[hir::PathSegment<'cx>],
        scope: Option<ScopeId>,
    ) -> PathTypeResolution {
        if let Some(projection) = self.resolve_qself_trait_member(path, qself) {
            return projection;
        }

        let Some(scope) = scope else {
            return PathTypeResolution::Unresolved;
        };
        match self
            .names
            .resolve_type_path(scope, path.iter().map(|segment| segment.name))
        {
            ResolveResult::Found(def) => self.classify_path_target(def, qself),
            ResolveResult::Ambiguous(defs) => {
                PathTypeResolution::Ambiguous(defs.into_iter().collect())
            }
            ResolveResult::NotFound => PathTypeResolution::Unresolved,
        }
    }

    fn resolve_qself_trait_member(
        &self,
        path: &[hir::PathSegment<'cx>],
        qself: Option<&QSelf>,
    ) -> Option<PathTypeResolution> {
        let qself = qself?;
        let trait_ = qself.trait_?;
        let trait_def = self.trait_def_for_type(trait_)?;
        let assoc_name = path.last()?.name;
        let ResolveResult::Found(assoc) = self.names.member(trait_def, Namespace::Type, assoc_name)
        else {
            return None;
        };
        if self.names[assoc].kind != DefKind::AssocType {
            return None;
        }
        Some(PathTypeResolution::Projection(ProjectionType {
            assoc,
            self_: Some(qself.self_),
            trait_: Some(trait_),
        }))
    }

    pub(crate) fn trait_def_for_type(&self, ty: TypeId) -> Option<DefId> {
        let Type::Path(path) = &self.types[ty.index()] else {
            return None;
        };
        let PathTypeResolution::Nominal(def) = path.resolution else {
            return None;
        };
        if self.names[def].kind != DefKind::Trait {
            return None;
        }
        Some(def)
    }

    fn classify_path_target(&self, def: DefId, qself: Option<&QSelf>) -> PathTypeResolution {
        match self.names[def].kind {
            DefKind::Struct
            | DefKind::Enum
            | DefKind::Variant
            | DefKind::Trait
            | DefKind::TypeAlias => PathTypeResolution::Nominal(def),
            DefKind::GenericType => PathTypeResolution::GenericParam(def),
            DefKind::AssocType => PathTypeResolution::Projection(ProjectionType {
                assoc: def,
                self_: qself.map(|qself| qself.self_),
                trait_: qself.and_then(|qself| qself.trait_),
            }),
            _ => PathTypeResolution::Unresolved,
        }
    }

    fn lower_plain_path(&mut self, path: &[hir::PathSegment<'cx>]) -> Path<'cx> {
        Path {
            segments: path
                .iter()
                .map(|segment| self.lower_path_segment(segment))
                .collect(),
        }
    }

    fn lower_path_segment(&mut self, segment: &hir::PathSegment<'cx>) -> PathSegment<'cx> {
        PathSegment {
            name: segment.name,
            args: segment
                .args
                .iter()
                .map(|arg| self.lower_generic_arg(arg))
                .collect(),
        }
    }

    fn lower_generic_arg(&mut self, arg: &hir::GenericArg<'cx>) -> GenericArg<'cx> {
        match arg {
            hir::GenericArg::Type(ty) => GenericArg::Type(self.lower_hir_type(*ty)),
            hir::GenericArg::Const(value) => GenericArg::Const(self.lower_const_arg(value)),
            hir::GenericArg::AssocType { name, ty } => GenericArg::AssocType {
                name: *name,
                ty: self.lower_hir_type(*ty),
            },
            hir::GenericArg::AssocConst { name, value } => GenericArg::AssocConst {
                name: *name,
                value: self.lower_const_arg(value),
            },
            hir::GenericArg::Constraint { name, bounds } => GenericArg::Constraint {
                name: *name,
                bounds: self.lower_type_bounds(bounds),
            },
            hir::GenericArg::Unsupported => GenericArg::Unsupported,
        }
    }

    fn lower_const_arg(&mut self, arg: &hir::ConstArg<'cx>) -> ConstArg<'cx> {
        match arg {
            hir::ConstArg::Lit(lit) => ConstArg::Lit(crate::Lit::from_hir(lit)),
            hir::ConstArg::Path { path, scope } => ConstArg::Path {
                path: self.lower_plain_path(&path.segments),
                def: self.resolve_const_item(&path.segments, *scope),
            },
            hir::ConstArg::Expr(expr) => ConstArg::Expr(*expr),
        }
    }

    fn resolve_const_item(
        &self,
        path: &[hir::PathSegment<'cx>],
        scope: Option<ScopeId>,
    ) -> Option<DefId> {
        let scope = scope?;
        let ResolveResult::Found(def) = self
            .names
            .resolve_value_path(scope, path.iter().map(|segment| segment.name))
        else {
            return None;
        };
        (self.names[def].kind == DefKind::Const).then_some(def)
    }

    fn lower_type_bounds(
        &mut self,
        bounds: &[hir::TypeParamBound<'cx>],
    ) -> Vec<TypeParamBound<'cx>> {
        bounds
            .iter()
            .map(|bound| self.lower_type_param_bound(bound))
            .collect()
    }

    fn lower_type_param_bound(&mut self, bound: &hir::TypeParamBound<'cx>) -> TypeParamBound<'cx> {
        match bound {
            hir::TypeParamBound::Trait(path) => TypeParamBound::Trait(self.lower_plain_path(path)),
            hir::TypeParamBound::Unsupported => TypeParamBound::Unsupported,
        }
    }
}
