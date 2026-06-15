use crate::hir::{
    item_visibility, ArrayLen, AssocItem, AssocItemKind, Block, ConstArg, ConstParam, Expr,
    ExprKind, ExprStructField, Field, FieldSource, File, GenericArg, GenericParam, Generics, Hir,
    Item, ItemKind, Lit, Local, Pat, PatKind, PatStructField, Path, PathSegment, QSelf, Signature,
    SignatureParam, SignatureSource, Stmt, StmtKind, TraitBound, Type, TypeKind, TypeParam,
    TypeParamBound, TypeSource, Variant, Visibility, WherePredicate,
};
use crate::lower::{self, PredicateSubject};
use crate::{
    AssocItemId, BlockId, ExprId, FieldId, ItemId, LocalId, PatId, SignatureId, StmtId, TypeId,
    VariantId,
};
use syn_sem_ast as ast;
use syn_sem_common::FilePath;
use syn_sem_name::{AstNodeId, DefId, Name, NameDb, ScopeId};

/// Builder for [`Hir`].
pub struct HirBuilder<'a, 'cx> {
    names: &'a NameDb<'cx>,
    hir: Hir<'cx>,
}

impl<'a, 'cx> HirBuilder<'a, 'cx> {
    /// Creates a builder using the currently available name-resolution data.
    pub fn new(names: &'a NameDb<'cx>) -> Self {
        Self {
            names,
            hir: Hir::default(),
        }
    }

    /// Builds HIR for one entry file.
    pub fn build(mut self, file_path: FilePath<'cx>, file: &'cx ast::File<'cx>) -> Hir<'cx> {
        let root = Some(self.names.root_scope());
        let items = self.collect_items(file.items, root);
        self.hir.add_file(File {
            id: self.hir.next_file_id(),
            file_path,
            items,
        });
        self.hir
    }

    fn collect_items(
        &mut self,
        items: &'cx [ast::Item<'cx>],
        parent_scope: Option<ScopeId>,
    ) -> Vec<ItemId> {
        items
            .iter()
            .map(|item| self.collect_item(item, parent_scope))
            .collect()
    }

    fn collect_item(&mut self, item: &'cx ast::Item<'cx>, parent_scope: Option<ScopeId>) -> ItemId {
        let id = self.hir.next_item_id();
        let def = self.def_for_item(item);
        let name = item.ident().map(|ident| ident.inner);
        let visibility = item_visibility(item);
        let kind = self.collect_item_kind(item, parent_scope, def);

        self.hir.add_item(Item {
            id,
            name,
            visibility,
            def,
            parent_scope,
            kind,
        });

        if let ast::Item::Mod(item) = item {
            let scope = def.and_then(|def| self.names.def_path_scope(def));
            let Some(children) = item.items else {
                return id;
            };
            let items = self.collect_items(children, scope);
            if let ItemKind::Mod {
                items: module_items,
                ..
            } = &mut self.hir[id].kind
            {
                *module_items = items;
            }
        }

        id
    }

    fn collect_item_kind(
        &mut self,
        item: &'cx ast::Item<'cx>,
        parent_scope: Option<ScopeId>,
        def: Option<DefId>,
    ) -> ItemKind<'cx> {
        let type_scope = self.type_scope_for_def(def, parent_scope);
        match item {
            ast::Item::Const(item) => {
                let ty = self.collect_type(item.ty, type_scope, TypeSource::ConstType);
                let init = self.collect_expr(item.init, type_scope);
                ItemKind::Const { ty, init }
            }
            ast::Item::Enum(item) => {
                let generics = self.collect_generics(&item.generics, type_scope);
                let variants = item
                    .variants
                    .iter()
                    .map(|variant| self.collect_variant(variant, type_scope))
                    .collect();
                ItemKind::Enum { generics, variants }
            }
            ast::Item::Fn(item) => {
                let generics = self.collect_generics(&item.sig.generics, type_scope);
                let signature =
                    self.collect_signature(SignatureSource::ItemFn, &item.sig, type_scope);
                let block = self.collect_block(
                    &item.block,
                    def.and_then(|def| self.names.def_body_scope(def)),
                );
                ItemKind::Fn {
                    generics,
                    signature,
                    block,
                }
            }
            ast::Item::Impl(item) => {
                let generics = self.collect_generics(&item.generics, type_scope);
                let self_ty = self.collect_type(item.self_ty, type_scope, TypeSource::ImplSelf);
                let trait_ = item
                    .trait_
                    .as_ref()
                    .map(|path| self.collect_type_path(path, type_scope));
                let items = item
                    .items
                    .iter()
                    .map(|item| self.collect_impl_item(item, type_scope))
                    .collect();
                ItemKind::Impl {
                    generics,
                    trait_,
                    self_ty,
                    items,
                }
            }
            ast::Item::Mod(item) => {
                let scope = def.and_then(|def| self.names.def_path_scope(def));
                ItemKind::Mod {
                    is_inline: item.is_inline,
                    scope,
                    items: Vec::new(),
                }
            }
            ast::Item::Struct(item) => {
                let generics = self.collect_generics(&item.generics, type_scope);
                let fields = item
                    .fields
                    .iter()
                    .map(|field| self.collect_struct_field(field, type_scope))
                    .collect();
                ItemKind::Struct { generics, fields }
            }
            ast::Item::Trait(item) => {
                let generics = self.collect_generics(&item.generics, type_scope);
                let items = item
                    .items
                    .iter()
                    .map(|item| self.collect_trait_item(item, type_scope))
                    .collect();
                ItemKind::Trait { generics, items }
            }
            ast::Item::Type(item) => {
                let generics = self.collect_generics(&item.generics, type_scope);
                let ty = self.collect_type(item.ty, type_scope, TypeSource::TypeAlias);
                ItemKind::Type { generics, ty }
            }
            ast::Item::Use(_) => ItemKind::Use,
        }
    }

    fn collect_struct_field(
        &mut self,
        field: &'cx ast::Field<'cx>,
        scope: Option<ScopeId>,
    ) -> FieldId {
        let id = self.hir.next_field_id();
        let ty = self.collect_type(&field.ty, scope, TypeSource::StructField);
        self.hir.add_field(Field {
            id,
            name: field.ident.inner,
            visibility: Visibility::from_ast(&field.vis),
            ty,
            source: FieldSource::Struct,
        });
        id
    }

    fn collect_variant(
        &mut self,
        variant: &'cx ast::Variant<'cx>,
        scope: Option<ScopeId>,
    ) -> VariantId {
        let id = self.hir.next_variant_id();
        let def = self.def_for_variant(variant);
        let (fields, discriminant) = match &variant.kind {
            ast::VariantKind::Fields(fields) => (
                fields
                    .iter()
                    .map(|field| self.collect_variant_field(field, scope))
                    .collect(),
                None,
            ),
            ast::VariantKind::Discriminant(expr) => {
                let discriminant = self.collect_expr(expr, scope);
                (Vec::new(), Some(discriminant))
            }
            ast::VariantKind::Unit => (Vec::new(), None),
        };
        self.hir.add_variant(Variant {
            id,
            def,
            name: variant.ident.inner,
            fields,
            discriminant,
        });
        id
    }

    fn collect_variant_field(
        &mut self,
        field: &'cx ast::VariantField<'cx>,
        scope: Option<ScopeId>,
    ) -> FieldId {
        let id = self.hir.next_field_id();
        let ty = self.collect_type(&field.ty, scope, TypeSource::VariantField);
        self.hir.add_field(Field {
            id,
            name: field.ident.inner,
            visibility: Visibility::Private,
            ty,
            source: FieldSource::Variant,
        });
        id
    }

    fn collect_impl_item(
        &mut self,
        item: &'cx ast::ImplItem<'cx>,
        parent_scope: Option<ScopeId>,
    ) -> AssocItemId {
        let id = self.hir.next_assoc_item_id();
        let def = self.def_for_impl_item(item);
        let type_scope = self.type_scope_for_def(def, parent_scope);
        let kind = match item {
            ast::ImplItem::Const(item) => {
                let ty = self.collect_type(item.ty, type_scope, TypeSource::AssocConstType);
                let init = self.collect_expr(item.init, type_scope);
                AssocItemKind::ImplConst { ty, init }
            }
            ast::ImplItem::Fn(item) => {
                let signature =
                    self.collect_signature(SignatureSource::ImplFn, &item.sig, type_scope);
                let block = self.collect_block(
                    &item.block,
                    def.and_then(|def| self.names.def_body_scope(def)),
                );
                AssocItemKind::ImplFn { signature, block }
            }
            ast::ImplItem::Type(item) => {
                let ty = self.collect_type(item.ty, type_scope, TypeSource::AssocTypeValue);
                AssocItemKind::ImplType { ty }
            }
        };
        self.hir.add_assoc_item(AssocItem {
            id,
            name: item.ident().inner,
            def,
            kind,
        });
        id
    }

    fn collect_trait_item(
        &mut self,
        item: &'cx ast::TraitItem<'cx>,
        parent_scope: Option<ScopeId>,
    ) -> AssocItemId {
        let id = self.hir.next_assoc_item_id();
        let def = self.def_for_trait_item(item);
        let type_scope = self.type_scope_for_def(def, parent_scope);
        let kind = match item {
            ast::TraitItem::Const(item) => {
                let ty = self.collect_type(item.ty, type_scope, TypeSource::AssocConstType);
                let default = item.default.map(|expr| self.collect_expr(expr, type_scope));
                AssocItemKind::TraitConst { ty, default }
            }
            ast::TraitItem::Fn(item) => {
                let signature =
                    self.collect_signature(SignatureSource::TraitFn, &item.sig, type_scope);
                let default = item.default.as_ref().map(|block| {
                    self.collect_block(block, def.and_then(|def| self.names.def_body_scope(def)))
                });
                AssocItemKind::TraitFn { signature, default }
            }
            ast::TraitItem::Type(item) => {
                let default = item
                    .default
                    .map(|ty| self.collect_type(ty, type_scope, TypeSource::AssocTypeValue));
                AssocItemKind::TraitType { default }
            }
        };
        self.hir.add_assoc_item(AssocItem {
            id,
            name: item.ident().inner,
            def,
            kind,
        });
        id
    }

    fn collect_signature(
        &mut self,
        source: SignatureSource,
        signature: &'cx ast::Signature<'cx>,
        scope: Option<ScopeId>,
    ) -> SignatureId {
        self.collect_signature_params(source, signature.params, scope)
    }

    fn collect_signature_params(
        &mut self,
        source: SignatureSource,
        params: &'cx [ast::Parameter<'cx>],
        scope: Option<ScopeId>,
    ) -> SignatureId {
        let params: Vec<_> = params
            .iter()
            .enumerate()
            .map(|(index, param)| {
                let ty =
                    self.collect_type(&param.pat.ty, scope, TypeSource::SignatureParam { index });
                SignatureParam {
                    pat: (index != 0).then(|| self.collect_pat(param.pat.pat, scope)),
                    ty,
                }
            })
            .collect();
        assert!(
            !params.is_empty(),
            "semantic AST signatures must include a synthesized return parameter"
        );
        let id = self.hir.next_signature_id();
        self.hir.add_signature(Signature { id, source, params });
        id
    }

    fn collect_generics(
        &mut self,
        generics: &'cx ast::Generics<'cx>,
        scope: Option<ScopeId>,
    ) -> Generics<'cx> {
        let params = generics
            .params
            .iter()
            .map(|param| self.collect_generic_param(param, scope))
            .collect();
        let predicates = lower::generic_predicates(generics)
            .into_iter()
            .map(|predicate| self.collect_lowered_where_predicate(predicate, scope))
            .collect();
        Generics {
            scope,
            params,
            predicates,
        }
    }

    fn collect_generic_param(
        &mut self,
        param: &'cx ast::GenericParam<'cx>,
        scope: Option<ScopeId>,
    ) -> GenericParam<'cx> {
        match param {
            ast::GenericParam::Type(param) => GenericParam::Type(TypeParam {
                name: param.ident.inner,
                default: param
                    .default
                    .map(|ty| self.collect_type(ty, scope, TypeSource::GenericParamDefault)),
            }),
            ast::GenericParam::Const(param) => GenericParam::Const(ConstParam {
                name: param.ident.inner,
                ty: self.collect_type(param.ty, scope, TypeSource::ConstGenericParam),
            }),
            ast::GenericParam::Unsupported(_) => GenericParam::Unsupported,
        }
    }

    fn collect_lowered_where_predicate(
        &mut self,
        predicate: lower::WherePredicate<'cx>,
        scope: Option<ScopeId>,
    ) -> WherePredicate<'cx> {
        match predicate {
            lower::WherePredicate::TypeBound(predicate) => WherePredicate::TypeBound {
                subject: match predicate.subject {
                    PredicateSubject::TypeParam(name) => {
                        self.collect_name_type(name, scope, TypeSource::WherePredicateSubject)
                    }
                    PredicateSubject::Type(ty) => {
                        self.collect_type(ty, scope, TypeSource::WherePredicateSubject)
                    }
                },
                bounds: predicate
                    .bounds
                    .iter()
                    .map(|bound| self.collect_type_param_bound(bound, scope))
                    .collect(),
            },
            lower::WherePredicate::Unsupported => WherePredicate::Unsupported,
        }
    }

    fn collect_type_param_bound(
        &mut self,
        bound: &'cx ast::TypeParamBound<'cx>,
        scope: Option<ScopeId>,
    ) -> TypeParamBound<'cx> {
        match bound {
            ast::TypeParamBound::Trait(bound) => TypeParamBound::Trait(TraitBound {
                path: self.collect_type_path(&bound.path, scope),
            }),
            ast::TypeParamBound::Unsupported(_) => TypeParamBound::Unsupported,
        }
    }

    fn collect_type(
        &mut self,
        ty: &'cx ast::Type<'cx>,
        scope: Option<ScopeId>,
        source: TypeSource,
    ) -> TypeId {
        let kind = self.collect_type_kind(ty, scope);
        let id = self.hir.next_type_id();
        self.hir.add_type(Type {
            id,
            ty: Some(ty),
            kind,
            scope,
            source,
        });
        id
    }

    fn collect_name_type(
        &mut self,
        name: Name<'cx>,
        scope: Option<ScopeId>,
        source: TypeSource,
    ) -> TypeId {
        let id = self.hir.next_type_id();
        self.hir.add_type(Type {
            id,
            ty: None,
            kind: TypeKind::Path(Path {
                qself: None,
                segments: vec![PathSegment {
                    name,
                    args: Vec::new(),
                }],
            }),
            scope,
            source,
        });
        id
    }

    fn collect_type_kind(
        &mut self,
        ty: &'cx ast::Type<'cx>,
        scope: Option<ScopeId>,
    ) -> TypeKind<'cx> {
        match ty {
            ast::Type::Array(ty) => TypeKind::Array {
                elem: self.collect_type(ty.elem, scope, TypeSource::Nested),
                len: ArrayLen::Expr(self.collect_expr(&ty.len, scope)),
            },
            ast::Type::Infer(_) => TypeKind::Infer,
            ast::Type::Path(ty) => TypeKind::Path(Path {
                qself: self.collect_type_qself(ty.qself.as_ref(), &ty.path, scope),
                segments: self.collect_type_path(&ty.path, scope),
            }),
            ast::Type::Reference(ty) => TypeKind::Reference {
                elem: self.collect_type(ty.elem, scope, TypeSource::Nested),
                is_mut: ty.is_mut,
            },
            ast::Type::Slice(ty) => TypeKind::Slice {
                elem: self.collect_type(ty.elem, scope, TypeSource::Nested),
            },
            ast::Type::Tuple(ty) => TypeKind::Tuple {
                elems: ty
                    .elems
                    .iter()
                    .map(|elem| self.collect_type(elem, scope, TypeSource::Nested))
                    .collect(),
            },
        }
    }

    fn collect_type_qself(
        &mut self,
        qself: Option<&ast::QSelf<'cx>>,
        path: &'cx ast::Path<'cx>,
        scope: Option<ScopeId>,
    ) -> Option<QSelf<'cx>> {
        let qself = qself?;
        Some(QSelf {
            self_ty: self.collect_type(qself.ty, scope, TypeSource::Nested),
            trait_path: self.collect_qself_trait_path(qself.position, path, scope),
        })
    }

    fn collect_qself_trait_path(
        &mut self,
        position: usize,
        path: &'cx ast::Path<'cx>,
        scope: Option<ScopeId>,
    ) -> Vec<PathSegment<'cx>> {
        if position == 0 {
            return Vec::new();
        }
        path.segments
            .iter()
            .take(position)
            .map(|segment| self.collect_type_path_segment(segment, scope))
            .collect()
    }

    fn collect_type_path(
        &mut self,
        path: &'cx ast::Path<'cx>,
        scope: Option<ScopeId>,
    ) -> Vec<PathSegment<'cx>> {
        path.segments
            .iter()
            .map(|segment| self.collect_type_path_segment(segment, scope))
            .collect()
    }

    fn collect_type_path_segment(
        &mut self,
        segment: &'cx ast::PathSegment<'cx>,
        scope: Option<ScopeId>,
    ) -> PathSegment<'cx> {
        PathSegment {
            name: segment.ident.inner,
            args: self.collect_generic_args(&segment.args, scope),
        }
    }

    fn collect_generic_args(
        &mut self,
        args: &'cx ast::PathArguments<'cx>,
        scope: Option<ScopeId>,
    ) -> Vec<GenericArg<'cx>> {
        args.args()
            .iter()
            .map(|arg| self.collect_generic_arg(arg, scope))
            .collect()
    }

    fn collect_generic_arg(
        &mut self,
        arg: &'cx ast::GenericArgument<'cx>,
        scope: Option<ScopeId>,
    ) -> GenericArg<'cx> {
        match arg {
            ast::GenericArgument::Type(ty) => {
                GenericArg::Type(self.collect_type(ty, scope, TypeSource::Nested))
            }
            ast::GenericArgument::Const(value) => {
                GenericArg::Const(self.collect_const_arg(value, scope))
            }
            ast::GenericArgument::AssocType(arg) => GenericArg::AssocType {
                name: arg.ident.inner,
                ty: self.collect_type(&arg.ty, scope, TypeSource::Nested),
            },
            ast::GenericArgument::AssocConst(arg) => GenericArg::AssocConst {
                name: arg.ident.inner,
                value: self.collect_const_arg(&arg.value, scope),
            },
            ast::GenericArgument::Constraint(arg) => GenericArg::Constraint {
                name: arg.ident.inner,
                bounds: arg
                    .bounds
                    .iter()
                    .map(|bound| self.collect_type_param_bound(bound, scope))
                    .collect(),
            },
            ast::GenericArgument::Unsupported(_) => GenericArg::Unsupported,
        }
    }

    fn collect_const_arg(
        &mut self,
        arg: &'cx ast::Expr<'cx>,
        scope: Option<ScopeId>,
    ) -> ConstArg<'cx> {
        match arg {
            ast::Expr::Lit(arg) => ConstArg::Lit(Self::collect_lit(&arg.lit)),
            ast::Expr::Path(arg) => ConstArg::Path(Path {
                qself: None,
                segments: self.collect_type_path(&arg.path, scope),
            }),
            _ => ConstArg::Expr(self.collect_expr(arg, scope)),
        }
    }

    fn collect_lit(lit: &ast::Lit<'cx>) -> Lit<'cx> {
        match lit {
            ast::Lit::Int(lit) => Lit::Int(lit.literal),
            ast::Lit::Float(lit) => Lit::Float(lit.literal),
            ast::Lit::Bool(lit) => Lit::Bool(lit.value),
        }
    }

    fn type_scope_for_def(&self, def: Option<DefId>, fallback: Option<ScopeId>) -> Option<ScopeId> {
        def.and_then(|def| self.names.def_generic_scope(def))
            .or(fallback)
    }

    fn collect_block(&mut self, block: &'cx ast::Block<'cx>, scope: Option<ScopeId>) -> BlockId {
        let id = self.hir.next_block_id();
        self.hir.add_block(Block {
            id,
            block,
            stmts: Vec::new(),
            scope,
        });
        let stmts = self.collect_block_contents(block, scope);
        self.hir[id].stmts = stmts;
        id
    }

    fn collect_block_contents(
        &mut self,
        block: &'cx ast::Block<'cx>,
        scope: Option<ScopeId>,
    ) -> Vec<StmtId> {
        block
            .stmts
            .iter()
            .map(|stmt| self.collect_stmt(stmt, scope))
            .collect()
    }

    fn collect_stmt(&mut self, stmt: &'cx ast::Stmt<'cx>, scope: Option<ScopeId>) -> StmtId {
        let kind = match stmt {
            ast::Stmt::Local(local) => StmtKind::Local(self.collect_local(local, scope)),
            ast::Stmt::Item(_) => StmtKind::Item,
            ast::Stmt::Expr(expr) => StmtKind::Expr(self.collect_expr(expr, scope)),
        };
        let id = self.hir.next_stmt_id();
        self.hir.add_stmt(Stmt {
            id,
            stmt,
            kind,
            scope,
        });
        id
    }

    fn collect_local(&mut self, local: &'cx ast::Local<'cx>, scope: Option<ScopeId>) -> LocalId {
        let init = local
            .init
            .as_ref()
            .map(|init| self.collect_expr(init.expr, scope));
        let pat = self.collect_pat(&local.pat, scope);
        let id = self.hir.next_local_id();
        self.hir.add_local(Local {
            id,
            local,
            pat,
            init,
            scope,
        });
        id
    }

    fn collect_pat(&mut self, pat: &'cx ast::Pat<'cx>, scope: Option<ScopeId>) -> PatId {
        let kind = match pat {
            ast::Pat::Ident(pat) => PatKind::Ident {
                name: pat.ident.inner,
                is_ref: pat.is_ref,
                is_mut: pat.is_mut,
            },
            ast::Pat::Reference(pat) => PatKind::Reference {
                pat: self.collect_pat(pat.pat, scope),
                is_mut: pat.is_mut,
            },
            ast::Pat::Path(pat) => PatKind::Path(Path {
                qself: None,
                segments: self.collect_type_path(&pat.path, scope),
            }),
            ast::Pat::Struct(pat) => PatKind::Struct {
                path: self.collect_type_path(&pat.path, scope),
                fields: pat
                    .fields
                    .iter()
                    .map(|field| PatStructField {
                        member: field.member.inner,
                        pat: self.collect_pat(field.pat, scope),
                    })
                    .collect(),
                has_rest: pat.rest.is_some(),
            },
            ast::Pat::Tuple(pat) => PatKind::Tuple {
                elems: pat
                    .elems
                    .iter()
                    .map(|elem| self.collect_pat(elem, scope))
                    .collect(),
            },
            ast::Pat::Type(pat) => PatKind::Type {
                pat: self.collect_pat(pat.pat, scope),
                ty: self.collect_type(&pat.ty, scope, TypeSource::Nested),
            },
            ast::Pat::Lit(_) | ast::Pat::Rest(_) | ast::Pat::Slice(_) => PatKind::Unsupported,
        };
        let id = self.hir.next_pat_id();
        self.hir.add_pat(Pat {
            id,
            pat,
            kind,
            scope,
        });
        id
    }

    fn collect_expr(&mut self, expr: &'cx ast::Expr<'cx>, scope: Option<ScopeId>) -> ExprId {
        let kind = match expr {
            ast::Expr::Array(expr) => ExprKind::Array {
                elems: expr
                    .elems
                    .iter()
                    .map(|elem| self.collect_expr(elem, scope))
                    .collect(),
            },
            ast::Expr::Assign(expr) => ExprKind::Assign {
                left: self.collect_expr(expr.left, scope),
                right: self.collect_expr(expr.right, scope),
            },
            ast::Expr::Binary(expr) => ExprKind::Binary {
                left: self.collect_expr(expr.left, scope),
                right: self.collect_expr(expr.right, scope),
            },
            ast::Expr::Block(expr) => ExprKind::Block {
                block: self.collect_block(&expr.block, scope),
            },
            ast::Expr::Call(expr) => ExprKind::Call {
                func: self.collect_expr(expr.func, scope),
                args: expr
                    .args
                    .iter()
                    .map(|arg| self.collect_expr(arg, scope))
                    .collect(),
            },
            ast::Expr::Cast(expr) => ExprKind::Cast {
                expr: self.collect_expr(expr.expr, scope),
                ty: self.collect_type(expr.ty, scope, TypeSource::Nested),
            },
            ast::Expr::Closure(expr) => ExprKind::Closure {
                signature: self.collect_signature_params(
                    SignatureSource::Closure,
                    expr.params,
                    scope,
                ),
                body: self.collect_expr(expr.body, scope),
            },
            ast::Expr::Const(expr) => ExprKind::Const {
                block: self.collect_block(&expr.block, scope),
            },
            ast::Expr::Field(expr) => ExprKind::Field {
                base: self.collect_expr(expr.base, scope),
                member: expr.member.inner,
            },
            ast::Expr::Index(expr) => ExprKind::Index {
                expr: self.collect_expr(expr.expr, scope),
                index: self.collect_expr(expr.index, scope),
            },
            ast::Expr::Lit(expr) => ExprKind::Lit(Self::collect_lit(&expr.lit)),
            ast::Expr::MethodCall(expr) => ExprKind::MethodCall {
                receiver: self.collect_expr(expr.receiver, scope),
                method: expr.method.inner,
                args: expr
                    .args
                    .iter()
                    .map(|arg| self.collect_expr(arg, scope))
                    .collect(),
            },
            ast::Expr::Paren(expr) => ExprKind::Paren {
                expr: self.collect_expr(expr.expr, scope),
            },
            ast::Expr::Path(expr) => ExprKind::Path(Path {
                qself: None,
                segments: self.collect_type_path(&expr.path, scope),
            }),
            ast::Expr::Reference(expr) => ExprKind::Reference {
                expr: self.collect_expr(expr.expr, scope),
                is_mut: expr.is_mut,
            },
            ast::Expr::Repeat(expr) => ExprKind::Repeat {
                expr: self.collect_expr(expr.expr, scope),
                len: self.collect_expr(expr.len, scope),
            },
            ast::Expr::Return(expr) => ExprKind::Return {
                expr: expr.expr.map(|expr| self.collect_expr(expr, scope)),
            },
            ast::Expr::Struct(expr) => ExprKind::Struct {
                path: self.collect_type_path(&expr.path, scope),
                fields: expr
                    .fields
                    .iter()
                    .map(|field| ExprStructField {
                        member: field.member.inner,
                        expr: self.collect_expr(field.expr, scope),
                    })
                    .collect(),
                rest: expr.rest.map(|rest| self.collect_expr(rest, scope)),
            },
            ast::Expr::Tuple(expr) => ExprKind::Tuple {
                elems: expr
                    .elems
                    .iter()
                    .map(|elem| self.collect_expr(elem, scope))
                    .collect(),
            },
            ast::Expr::Unary(expr) => ExprKind::Unary {
                expr: self.collect_expr(expr.expr, scope),
            },
        };

        let id = self.hir.next_expr_id();
        self.hir.add_expr(Expr {
            id,
            expr,
            kind,
            scope,
        });
        id
    }

    fn def_for_item(&self, item: &'cx ast::Item<'cx>) -> Option<DefId> {
        let def = self.names.def_for_ast_node(AstNodeId::from_ref(item));
        if matches!(item, ast::Item::Use(_)) {
            assert!(
                def.is_none(),
                "use items must not be linked as item definitions"
            );
        }
        def
    }

    fn def_for_variant(&self, variant: &'cx ast::Variant<'cx>) -> Option<DefId> {
        self.names.def_for_ast_node(AstNodeId::from_ref(variant))
    }

    fn def_for_impl_item(&self, item: &'cx ast::ImplItem<'cx>) -> Option<DefId> {
        self.names.def_for_ast_node(AstNodeId::from_ref(item))
    }

    fn def_for_trait_item(&self, item: &'cx ast::TraitItem<'cx>) -> Option<DefId> {
        self.names.def_for_ast_node(AstNodeId::from_ref(item))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Hir, HirBuilder};
    use syn_sem_ast::SyntaxCx;
    use syn_sem_common::CommonCx;
    use syn_sem_name::{AstNodeId, DefKind, NameDb, Origin, Visibility as NameVisibility};

    fn parsed_model<'cx>(ccx: &'cx CommonCx, scx: &'cx SyntaxCx<'cx>, code: &str) -> Hir<'cx> {
        let file_path = ccx.intern("test.rs");
        let text = ccx.intern(code);
        scx.parse_virtual_file(file_path, text).unwrap();
        let file = scx.lookup_source(file_path).unwrap().ast();
        let names = NameDb::default();
        HirBuilder::new(&names).build(file_path, file)
    }

    fn type_sources(model: &Hir<'_>) -> Vec<TypeSource> {
        model.types().iter().map(|ty| ty.source).collect()
    }

    fn item_kind_name(item: &Item<'_>) -> &'static str {
        match item.kind {
            ItemKind::Const { .. } => "const",
            ItemKind::Enum { .. } => "enum",
            ItemKind::Fn { .. } => "fn",
            ItemKind::Impl { .. } => "impl",
            ItemKind::Mod { .. } => "mod",
            ItemKind::Struct { .. } => "struct",
            ItemKind::Trait { .. } => "trait",
            ItemKind::Type { .. } => "type",
            ItemKind::Use => "use",
        }
    }

    fn assoc_item_kind_name(item: &AssocItem<'_>) -> &'static str {
        match item.kind {
            AssocItemKind::ImplConst { .. } => "impl const",
            AssocItemKind::ImplFn { .. } => "impl fn",
            AssocItemKind::ImplType { .. } => "impl type",
            AssocItemKind::TraitConst { .. } => "trait const",
            AssocItemKind::TraitFn { .. } => "trait fn",
            AssocItemKind::TraitType { .. } => "trait type",
        }
    }

    fn named_item<'m, 'cx>(model: &'m Hir<'cx>, name: &str) -> &'m Item<'cx> {
        model
            .items()
            .iter()
            .find(|item| {
                item.name
                    .map_or(false, |item_name| item_name.as_ref() == name)
            })
            .unwrap_or_else(|| panic!("expected item named `{name}`"))
    }

    #[test]
    fn builds_items_signatures_and_block_handles() {
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let model = parsed_model(
            &ccx,
            &scx,
            "fn f<T>(value: T) -> T { value }\nconst N: usize = 1;",
        );

        assert_eq!(model.files().len(), 1);
        assert_eq!(model.items().len(), 2);
        assert_eq!(model.signatures().len(), 1);
        assert_eq!(model.blocks().len(), 1);
        assert!(model.types().len() >= 3);

        let ItemKind::Fn {
            signature, block, ..
        } = model[model.files()[0].items[0]].kind
        else {
            panic!("expected function item");
        };
        assert!(matches!(
            model[model[signature].params[0].ty].kind,
            TypeKind::Path(_)
        ));
        assert!(model[signature].params[0].pat.is_none());
        assert_eq!(model[signature].params.len(), 2);
        assert!(matches!(
            model[model[signature].params[1].ty].kind,
            TypeKind::Path(_)
        ));
        let input_pat = model[signature].params[1]
            .pat
            .expect("input should keep source pattern");
        let PatKind::Ident { name, .. } = &model[input_pat].kind else {
            panic!("expected input ident pattern");
        };
        assert_eq!(name.as_ref(), "value");
        assert_eq!(model[block].block.stmts.len(), 1);
        assert_eq!(model[block].stmts.len(), 1);
        let StmtKind::Expr(expr) = model[model[block].stmts[0]].kind else {
            panic!("expected function body expression statement");
        };
        assert!(matches!(model[expr].kind, ExprKind::Path(_)));
    }

    #[test]
    fn collects_closure_signatures_from_expression_bodies() {
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let model = parsed_model(
            &ccx,
            &scx,
            r#"
            fn f() {
                let first = |x| x;
                let second = |y: i32| -> i32 { y };
            }
            "#,
        );

        let closures = model
            .signatures()
            .iter()
            .filter(|signature| signature.source == SignatureSource::Closure)
            .collect::<Vec<_>>();
        assert_eq!(closures.len(), 2);

        let inferred = closures
            .iter()
            .find(|signature| matches!(model[signature.params[1].ty].kind, TypeKind::Infer))
            .expect("expected closure with inferred input type");
        assert!(matches!(model[inferred.params[0].ty].kind, TypeKind::Infer));
        assert!(matches!(model[inferred.params[1].ty].kind, TypeKind::Infer));

        let typed = closures
            .iter()
            .find(|signature| matches!(model[signature.params[1].ty].kind, TypeKind::Path(_)))
            .expect("expected closure with typed input");
        assert!(matches!(model[typed.params[0].ty].kind, TypeKind::Path(_)));
        assert!(matches!(model[typed.params[1].ty].kind, TypeKind::Path(_)));
    }

    #[test]
    fn represents_block_statements_and_local_initializers() {
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let model = parsed_model(
            &ccx,
            &scx,
            r#"
            fn f() {
                let x = 1;
                struct LocalItem;
                x
            }
            "#,
        );

        let function = named_item(&model, "f");
        let ItemKind::Fn { block, .. } = function.kind else {
            panic!("expected function item");
        };
        assert_eq!(model[block].stmts.len(), 3);

        let StmtKind::Local(local) = model[model[block].stmts[0]].kind else {
            panic!("expected local statement");
        };
        assert!(matches!(
            model[model[local].pat].kind,
            PatKind::Ident { .. }
        ));
        let init = model[local]
            .init
            .expect("local initializer should be represented");
        assert!(matches!(model[init].kind, ExprKind::Lit(_)));

        let StmtKind::Item = model[model[block].stmts[1]].kind else {
            panic!("expected block-local item statement");
        };

        let StmtKind::Expr(expr) = model[model[block].stmts[2]].kind else {
            panic!("expected expression statement");
        };
        assert!(matches!(model[expr].kind, ExprKind::Path(_)));
    }

    #[test]
    fn represents_nested_patterns_and_pattern_type_annotations() {
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let model = parsed_model(
            &ccx,
            &scx,
            r#"
            fn f((left, ref mut right): (usize, usize), value: &usize) {
                let (a, b): (usize, usize) = (left, right);
                let &inner = value;
            }
            "#,
        );

        assert!(model
            .pats()
            .iter()
            .any(|pat| matches!(pat.kind, PatKind::Type { .. })));
        assert!(model
            .pats()
            .iter()
            .any(|pat| matches!(pat.kind, PatKind::Tuple { .. })));
        assert!(model
            .pats()
            .iter()
            .any(|pat| matches!(pat.kind, PatKind::Reference { .. })));
        assert!(model.pats().iter().any(|pat| {
            matches!(
                pat.kind,
                PatKind::Ident {
                    is_ref: true,
                    is_mut: true,
                    ..
                }
            )
        }));
        assert!(model
            .types()
            .iter()
            .any(|ty| ty.source == TypeSource::Nested));
    }

    #[test]
    fn represents_path_and_struct_patterns() {
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let model = parsed_model(
            &ccx,
            &scx,
            r#"
            fn f(value: Point, state: State) {
                let State::Ready = state;
                let Point { x, y: ref y_value, .. } = value;
            }
            "#,
        );

        assert!(model.pats().iter().any(|pat| {
            let PatKind::Path(path) = &pat.kind else {
                return false;
            };
            path.segments[0].name.as_ref() == "State" && path.segments[1].name.as_ref() == "Ready"
        }));

        let struct_pat = model
            .pats()
            .iter()
            .find(|pat| {
                matches!(
                    &pat.kind,
                    PatKind::Struct { path, .. } if path[0].name.as_ref() == "Point"
                )
            })
            .expect("expected struct pattern");
        let PatKind::Struct {
            fields, has_rest, ..
        } = &struct_pat.kind
        else {
            panic!("expected struct pattern");
        };
        assert!(*has_rest);
        assert_eq!(fields.len(), 2);
        assert_eq!(fields[0].member.as_ref(), "x");
        assert!(matches!(model[fields[0].pat].kind, PatKind::Ident { .. }));
        assert_eq!(fields[1].member.as_ref(), "y");
        assert!(matches!(
            model[fields[1].pat].kind,
            PatKind::Ident { is_ref: true, .. }
        ));
    }

    #[test]
    fn builds_struct_enum_trait_impl_type_and_use_shapes() {
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let model = parsed_model(
            &ccx,
            &scx,
            r#"
            use a::B;
            struct S { field: B }
            enum E { A(B), C = 1 }
            trait Tr { type Item; fn get(&self) -> Self::Item; }
            impl Tr for S { type Item = B; fn get(&self) -> B { B } }
            type Alias = S;
            mod inner { struct Inside; }
            "#,
        );

        assert!(model
            .items()
            .iter()
            .any(|item| matches!(item.kind, ItemKind::Use)));
        assert!(model
            .items()
            .iter()
            .any(|item| matches!(item.kind, ItemKind::Type { .. })));
        assert!(!model.fields().is_empty());
        assert!(!model.variants().is_empty());
        assert!(!model.assoc_items().is_empty());

        let module = model
            .items()
            .iter()
            .find(|item| matches!(item.kind, ItemKind::Mod { .. }))
            .unwrap();
        let ItemKind::Mod { items, .. } = &module.kind else {
            panic!("expected module item");
        };
        assert_eq!(items.len(), 1);
    }

    #[test]
    fn covers_all_supported_item_kinds() {
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let model = parsed_model(
            &ccx,
            &scx,
            r#"
            const C: usize = 1;
            enum E { V }
            fn f() {}
            impl S {}
            mod m {}
            struct S;
            trait T {}
            type A = S;
            use m::S;
            "#,
        );

        let kinds: Vec<_> = model.items().iter().map(item_kind_name).collect();
        assert_eq!(
            kinds,
            ["const", "enum", "fn", "impl", "mod", "struct", "trait", "type", "use"]
        );
    }

    #[test]
    fn covers_hir_native_names_visibility_and_paths() {
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let model = parsed_model(
            &ccx,
            &scx,
            r#"
            pub(crate) mod m {}
            pub struct S { pub field: usize, private: usize }
            trait Tr { type Item; }
            impl Tr for S { type Item = usize; }
            "#,
        );

        let module = named_item(&model, "m");
        let Visibility::Restricted(path) = &module.visibility else {
            panic!("expected restricted module source visibility");
        };
        assert_eq!(path.segments.len(), 1);
        assert_eq!(path.segments[0].as_ref(), "crate");

        let struct_item = named_item(&model, "S");
        assert!(matches!(struct_item.visibility, Visibility::Public));

        let impl_item = model
            .items()
            .iter()
            .find(|item| matches!(item.kind, ItemKind::Impl { .. }))
            .expect("expected impl item");
        assert!(impl_item.name.is_none());
        assert!(matches!(impl_item.visibility, Visibility::Private));
        let ItemKind::Impl { trait_, .. } = &impl_item.kind else {
            panic!("expected impl item");
        };
        let trait_ = trait_.as_ref().expect("expected trait path");
        assert_eq!(trait_.len(), 1);
        assert_eq!(trait_[0].name.as_ref(), "Tr");
        assert!(trait_[0].args.is_empty());

        let public_field = model
            .fields()
            .iter()
            .find(|field| field.name.as_ref() == "field")
            .expect("expected public field");
        assert!(matches!(public_field.visibility, Visibility::Public));

        let private_field = model
            .fields()
            .iter()
            .find(|field| field.name.as_ref() == "private")
            .expect("expected private field");
        assert!(matches!(private_field.visibility, Visibility::Private));

        let assoc_names: Vec<_> = model
            .assoc_items()
            .iter()
            .map(|item| item.name.as_ref())
            .collect();
        assert_eq!(assoc_names, ["Item", "Item"]);
    }

    #[test]
    fn represents_impl_trait_path_generic_arguments() {
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let model = parsed_model(
            &ccx,
            &scx,
            r#"
            trait Generic<T> {}
            struct S;
            impl Generic<usize> for S {}
            "#,
        );

        let impl_item = model
            .items()
            .iter()
            .find(|item| matches!(item.kind, ItemKind::Impl { .. }))
            .expect("expected generic impl item");
        let ItemKind::Impl {
            trait_: Some(trait_),
            ..
        } = &impl_item.kind
        else {
            panic!("expected trait impl item");
        };
        assert_eq!(trait_.len(), 1);
        assert_eq!(trait_[0].name.as_ref(), "Generic");
        let [GenericArg::Type(arg)] = trait_[0].args.as_slice() else {
            panic!("expected generic trait argument");
        };
        let TypeKind::Path(path) = &model[*arg].kind else {
            panic!("expected generic trait argument type path");
        };
        assert_eq!(path.segments[0].name.as_ref(), "usize");
    }

    #[test]
    fn covers_all_supported_associated_item_kinds() {
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let model = parsed_model(
            &ccx,
            &scx,
            r#"
            struct S;
            trait Tr {
                const TC: usize = 1;
                fn tf(&self) -> usize { 1 }
                type TT = usize;
            }
            impl Tr for S {
                const IC: usize = 1;
                fn ifn(&self) -> usize { 1 }
                type IT = usize;
            }
            "#,
        );

        let kinds: Vec<_> = model
            .assoc_items()
            .iter()
            .map(assoc_item_kind_name)
            .collect();
        assert_eq!(
            kinds,
            [
                "trait const",
                "trait fn",
                "trait type",
                "impl const",
                "impl fn",
                "impl type"
            ]
        );

        for item in model.assoc_items() {
            match item.kind {
                AssocItemKind::ImplConst { ty, init, .. } => {
                    assert_eq!(model[ty].source, TypeSource::AssocConstType);
                    assert!(matches!(model[init].kind, ExprKind::Lit(_)));
                }
                AssocItemKind::ImplFn {
                    signature, block, ..
                } => {
                    assert!(matches!(model[signature].source, SignatureSource::ImplFn));
                    assert_eq!(model[block].block.stmts.len(), 1);
                }
                AssocItemKind::ImplType { ty, .. } => {
                    assert_eq!(model[ty].source, TypeSource::AssocTypeValue);
                }
                AssocItemKind::TraitConst { ty, default, .. } => {
                    assert_eq!(model[ty].source, TypeSource::AssocConstType);
                    let default = default.expect("trait const should keep default expression");
                    assert!(matches!(model[default].kind, ExprKind::Lit(_)));
                }
                AssocItemKind::TraitFn {
                    signature, default, ..
                } => {
                    assert!(matches!(model[signature].source, SignatureSource::TraitFn));
                    let default = default.expect("trait fn default should create a block");
                    assert_eq!(model[default].block.stmts.len(), 1);
                }
                AssocItemKind::TraitType { default, .. } => {
                    let default = default.expect("trait type default should create a type");
                    assert_eq!(model[default].source, TypeSource::AssocTypeValue);
                }
            }
        }
    }

    #[test]
    fn covers_type_sources_for_declaration_roles() {
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let model = parsed_model(
            &ccx,
            &scx,
            r#"
            const C: usize = 1;
            fn f(arg: &usize) -> (usize, usize) { (arg, arg) }
            struct S { field: [usize; 1] }
            enum E { V([usize]) }
            type A = (usize,);
            trait Tr {
                const TC: usize;
                type TT = usize;
            }
            impl Tr for S {
                const TC: usize = 1;
                type TT = usize;
            }
            "#,
        );

        let sources = type_sources(&model);
        assert!(sources.contains(&TypeSource::ConstType));
        assert!(sources.contains(&TypeSource::SignatureParam { index: 0 }));
        assert!(sources.contains(&TypeSource::SignatureParam { index: 1 }));
        assert!(sources.contains(&TypeSource::ImplSelf));
        assert!(sources.contains(&TypeSource::StructField));
        assert!(sources.contains(&TypeSource::VariantField));
        assert!(sources.contains(&TypeSource::TypeAlias));
        assert!(sources.contains(&TypeSource::AssocConstType));
        assert!(sources.contains(&TypeSource::AssocTypeValue));

        assert!(model
            .types()
            .iter()
            .any(|ty| matches!(ty.kind, TypeKind::Array { .. })));
        assert!(model
            .types()
            .iter()
            .any(|ty| matches!(ty.kind, TypeKind::Reference { .. })));
        assert!(model
            .types()
            .iter()
            .any(|ty| matches!(ty.kind, TypeKind::Slice { .. })));
        assert!(model
            .types()
            .iter()
            .any(|ty| matches!(ty.kind, TypeKind::Tuple { .. })));
        assert!(model
            .types()
            .iter()
            .any(|ty| matches!(ty.kind, TypeKind::Path(_))));
        assert!(sources.contains(&TypeSource::Nested));
    }

    #[test]
    fn represents_type_paths_and_nested_generic_arguments() {
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let model = parsed_model(
            &ccx,
            &scx,
            r#"
            struct S {
                field: std::collections::HashMap<K, Iterator<Item = V>>,
            }
            "#,
        );

        let field_type = model
            .types()
            .iter()
            .find(|ty| ty.source == TypeSource::StructField)
            .expect("expected struct field type");

        let TypeKind::Path(path) = &field_type.kind else {
            panic!("expected path type");
        };
        assert_eq!(path.segments.len(), 3);
        assert_eq!(path.segments[0].name.as_ref(), "std");
        assert_eq!(path.segments[1].name.as_ref(), "collections");

        let last = &path.segments[2];
        assert_eq!(last.name.as_ref(), "HashMap");
        assert_eq!(last.args.len(), 2);
        assert!(matches!(last.args[0], GenericArg::Type(_)));

        let GenericArg::Type(iter_ty) = last.args[1] else {
            panic!("expected type argument");
        };
        let TypeKind::Path(iter_path) = &model[iter_ty].kind else {
            panic!("expected nested iterator path type");
        };
        assert_eq!(iter_path.segments[0].name.as_ref(), "Iterator");
        assert!(matches!(
            iter_path.segments[0].args[0],
            GenericArg::AssocType { .. }
        ));
    }

    #[test]
    fn represents_const_generic_arguments() {
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let model = parsed_model(
            &ccx,
            &scx,
            r#"
            struct Array<const A: usize, const B: usize>;

            struct S<const N: usize> {
                field: Array<3, { N + 1 }>,
            }
            "#,
        );

        let field_type = model
            .types()
            .iter()
            .find(|ty| ty.source == TypeSource::StructField)
            .expect("expected struct field type");

        let TypeKind::Path(path) = &field_type.kind else {
            panic!("expected path type");
        };
        let args = &path.segments[0].args;
        assert_eq!(path.segments[0].name.as_ref(), "Array");
        assert_eq!(args.len(), 2);

        let GenericArg::Const(ConstArg::Lit(Lit::Int(value))) = &args[0] else {
            panic!("expected literal const argument");
        };
        assert_eq!(value.as_ref(), "3");

        assert!(matches!(args[1], GenericArg::Const(ConstArg::Expr(_))));
    }

    #[test]
    fn represents_associated_const_arguments() {
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let model = parsed_model(
            &ccx,
            &scx,
            r#"
            trait Trait {
                const PANIC: bool;
            }

            struct S<T: Trait<PANIC = false>> {
                field: T,
            }
            "#,
        );

        let item = named_item(&model, "S");
        let ItemKind::Struct { generics, .. } = &item.kind else {
            panic!("expected struct item");
        };
        let WherePredicate::TypeBound { bounds, .. } = &generics.predicates[0] else {
            panic!("expected type-bound predicate");
        };
        let TypeParamBound::Trait(bound) = &bounds[0] else {
            panic!("expected trait bound");
        };
        let GenericArg::AssocConst { name, value } = &bound.path[0].args[0] else {
            panic!("expected associated const argument");
        };

        assert_eq!(name.as_ref(), "PANIC");
        assert_eq!(*value, ConstArg::Lit(Lit::Bool(false)));
    }

    #[test]
    fn represents_associated_type_constraint_bounds() {
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let model = parsed_model(
            &ccx,
            &scx,
            r#"
            struct S<T: Iterator<Item: std::fmt::Display>> {
                field: T,
            }
            "#,
        );

        let item = named_item(&model, "S");
        let ItemKind::Struct { generics, .. } = &item.kind else {
            panic!("expected struct item");
        };
        let WherePredicate::TypeBound { bounds, .. } = &generics.predicates[0] else {
            panic!("expected type-bound predicate");
        };
        let TypeParamBound::Trait(bound) = &bounds[0] else {
            panic!("expected trait bound");
        };
        let GenericArg::Constraint { name, bounds } = &bound.path[0].args[0] else {
            panic!("expected associated type constraint");
        };
        assert_eq!(name.as_ref(), "Item");
        assert_eq!(bounds.len(), 1);
        let TypeParamBound::Trait(bound) = &bounds[0] else {
            panic!("expected trait bound");
        };
        assert_eq!(bound.path[0].name.as_ref(), "std");
        assert_eq!(bound.path[1].name.as_ref(), "fmt");
        assert_eq!(bound.path[2].name.as_ref(), "Display");
    }

    #[test]
    fn represents_qualified_type_paths() {
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let model = parsed_model(
            &ccx,
            &scx,
            r#"
            mod a {
                pub mod b {
                    pub trait Trait {
                        type Item;
                    }
                }
            }

            struct S<T: a::b::Trait> {
                field: <T as a::b::Trait>::Item,
            }
            "#,
        );

        let field_type = model
            .types()
            .iter()
            .find(|ty| ty.source == TypeSource::StructField)
            .expect("expected struct field type");

        let TypeKind::Path(path) = &field_type.kind else {
            panic!("expected path type");
        };
        let qself = path.qself.as_ref().expect("expected qualified self type");
        let TypeKind::Path(self_ty) = &model[qself.self_ty].kind else {
            panic!("expected qself self type to be a path");
        };
        let trait_path = &qself.trait_path;

        assert_eq!(self_ty.segments[0].name.as_ref(), "T");
        assert_eq!(trait_path.len(), 3);
        assert_eq!(trait_path[0].name.as_ref(), "a");
        assert_eq!(trait_path[1].name.as_ref(), "b");
        assert_eq!(trait_path[2].name.as_ref(), "Trait");
        assert_eq!(path.segments.len(), 4);
        assert_eq!(path.segments[0].name.as_ref(), "a");
        assert_eq!(path.segments[1].name.as_ref(), "b");
        assert_eq!(path.segments[2].name.as_ref(), "Trait");
        assert_eq!(path.segments[3].name.as_ref(), "Item");
    }

    #[test]
    fn represents_type_param_trait_bounds() {
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let model = parsed_model(
            &ccx,
            &scx,
            "struct S<T: Clone> where T: Iterator { field: <T>::Item }",
        );

        let item = named_item(&model, "S");
        let ItemKind::Struct { generics, .. } = &item.kind else {
            panic!("expected struct item");
        };
        assert_eq!(generics.params.len(), 1);
        let GenericParam::Type(param) = &generics.params[0] else {
            panic!("expected type parameter");
        };
        assert_eq!(param.name.as_ref(), "T");
        assert_eq!(generics.predicates.len(), 2);
        let predicate_bounds = generics
            .predicates
            .iter()
            .map(|predicate| {
                let WherePredicate::TypeBound { subject, bounds } = predicate else {
                    panic!("expected type-bound predicate");
                };
                let TypeKind::Path(subject_path) = &model[*subject].kind else {
                    panic!("expected generic parameter subject type");
                };
                assert_eq!(subject_path.segments[0].name.as_ref(), "T");
                let TypeParamBound::Trait(bound) = &bounds[0] else {
                    panic!("expected trait bound");
                };
                bound.path[0].name.as_ref()
            })
            .collect::<Vec<_>>();
        assert_eq!(predicate_bounds, ["Clone", "Iterator"]);
    }

    #[test]
    fn covers_block_handles_and_source_exprs() {
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let model = parsed_model(
            &ccx,
            &scx,
            r#"
            const C: usize = 1;
            fn f() {}
            enum E { V = 1 }
            trait Tr {
                const TC: usize = 1;
                fn tf(&self) {}
            }
            impl Tr for E {
                const TC: usize = 1;
                fn tf(&self) {}
            }
            "#,
        );

        assert_eq!(model.blocks().len(), 3);
        assert!(model
            .items()
            .iter()
            .any(|item| { matches!(item.kind, ItemKind::Const { .. }) }));
        assert!(model
            .variants()
            .iter()
            .any(|variant| { variant.discriminant.is_some() }));
        assert!(model
            .assoc_items()
            .iter()
            .any(|item| { matches!(item.kind, AssocItemKind::ImplConst { .. }) }));
        assert!(model.assoc_items().iter().any(|item| {
            matches!(
                item.kind,
                AssocItemKind::TraitConst {
                    default: Some(_),
                    ..
                }
            )
        }));
        assert!(model
            .exprs()
            .iter()
            .any(|expr| matches!(expr.kind, ExprKind::Lit(_))));
    }

    #[test]
    fn covers_module_field_and_variant_shapes() {
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let model = parsed_model(
            &ccx,
            &scx,
            r#"
            mod inline { struct Inside; }
            mod external;
            struct Named { a: usize }
            struct Tuple(usize, usize);
            struct Unit;
            enum E {
                Unit,
                Tuple(usize),
                Named { a: usize },
                Discriminant = 1,
            }
            "#,
        );

        let mut inline_module_seen = false;
        let mut external_module_seen = false;
        for item in model.items() {
            if let ItemKind::Mod {
                is_inline, items, ..
            } = &item.kind
            {
                if *is_inline {
                    inline_module_seen = true;
                    assert_eq!(items.len(), 1);
                } else {
                    external_module_seen = true;
                    assert!(items.is_empty());
                }
            }
        }
        assert!(inline_module_seen);
        assert!(external_module_seen);

        assert!(model
            .fields()
            .iter()
            .any(|field| matches!(field.source, FieldSource::Struct)));
        assert!(model
            .fields()
            .iter()
            .any(|field| matches!(field.source, FieldSource::Variant)));

        let variant_field_counts: Vec<_> = model
            .variants()
            .iter()
            .map(|variant| variant.fields.len())
            .collect();
        assert!(variant_field_counts.contains(&0));
        assert!(variant_field_counts.contains(&1));
        assert!(model
            .variants()
            .iter()
            .any(|variant| { variant.discriminant.is_some() }));
    }

    #[test]
    fn links_items_to_current_name_definitions_when_available() {
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let file_path = ccx.intern("test.rs");
        let text = ccx.intern("struct S;");
        scx.parse_virtual_file(file_path, text).unwrap();
        let file = scx.lookup_source(file_path).unwrap().ast();

        let mut names = NameDb::default();
        let def = names.add_def(
            names.root_scope(),
            DefKind::Struct,
            Some(ccx.intern("S")),
            NameVisibility::Private,
            Origin::Untracked,
        );
        let item = &file.items[0];
        let ast::Item::Struct(_) = item else {
            panic!("expected struct item");
        };
        names.set_def_ast_node(def, AstNodeId::from_ref(item));

        let model = HirBuilder::new(&names).build(file_path, file);
        assert_eq!(model.items()[0].def, Some(def));
    }

    #[test]
    fn ast_node_ids_distinguish_wrapper_and_payload_nodes() {
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let file_path = ccx.intern("test.rs");
        let text = ccx.intern("struct S;");
        scx.parse_virtual_file(file_path, text).unwrap();
        let file = scx.lookup_source(file_path).unwrap().ast();

        let item = &file.items[0];
        let ast::Item::Struct(payload) = item else {
            panic!("expected struct item");
        };

        assert_ne!(AstNodeId::from_ref(item), AstNodeId::from_ref(payload));
    }

    #[test]
    fn links_unnamed_impls_and_assoc_items_by_source() {
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let file_path = ccx.intern("test.rs");
        let text = ccx.intern("struct S; impl S { fn a() {} } impl S { fn b() {} }");
        scx.parse_virtual_file(file_path, text).unwrap();
        let file = scx.lookup_source(file_path).unwrap().ast();

        let first_impl_item = &file.items[1];
        let second_impl_item = &file.items[2];
        let ast::Item::Impl(first_impl) = first_impl_item else {
            panic!("expected first impl item");
        };
        let ast::Item::Impl(second_impl) = second_impl_item else {
            panic!("expected second impl item");
        };
        let first_fn_item = &first_impl.items[0];
        let second_fn_item = &second_impl.items[0];
        let ast::ImplItem::Fn(first_fn) = first_fn_item else {
            panic!("expected first impl fn");
        };
        let ast::ImplItem::Fn(second_fn) = second_fn_item else {
            panic!("expected second impl fn");
        };

        let mut names = NameDb::default();
        let root = names.root_scope();
        let first_impl_def = names.add_def(
            root,
            DefKind::Impl,
            None,
            NameVisibility::Private,
            Origin::Untracked,
        );
        names.set_def_ast_node(first_impl_def, AstNodeId::from_ref(first_impl_item));
        let second_impl_def = names.add_def(
            root,
            DefKind::Impl,
            None,
            NameVisibility::Private,
            Origin::Untracked,
        );
        names.set_def_ast_node(second_impl_def, AstNodeId::from_ref(second_impl_item));

        let first_fn_def = names.add_def(
            root,
            DefKind::AssocFn,
            Some(first_fn.sig.ident.inner),
            NameVisibility::Private,
            Origin::Untracked,
        );
        names.set_def_ast_node(first_fn_def, AstNodeId::from_ref(first_fn_item));
        let second_fn_def = names.add_def(
            root,
            DefKind::AssocFn,
            Some(second_fn.sig.ident.inner),
            NameVisibility::Private,
            Origin::Untracked,
        );
        names.set_def_ast_node(second_fn_def, AstNodeId::from_ref(second_fn_item));

        let model = HirBuilder::new(&names).build(file_path, file);
        assert_eq!(model.items()[1].def, Some(first_impl_def));
        assert_eq!(model.items()[2].def, Some(second_impl_def));

        let ItemKind::Impl { items, .. } = &model.items()[1].kind else {
            panic!("expected first HIR impl");
        };
        assert_eq!(model[items[0]].def, Some(first_fn_def));

        let ItemKind::Impl { items, .. } = &model.items()[2].kind else {
            panic!("expected second HIR impl");
        };
        assert_eq!(model[items[0]].def, Some(second_fn_def));
    }
}
