use std::ops::{Index, IndexMut};
use syn_sem_ast as ast;
use syn_sem_common::FilePath;
use syn_sem_name::{AstNodeId, DefId, Name, NameDb, ScopeId};

/// Builder for [`ProgramRepr`].
pub struct ProgramReprBuilder<'a, 'cx> {
    names: &'a NameDb<'cx>,
    repr: ProgramRepr<'cx>,
}

impl<'a, 'cx> ProgramReprBuilder<'a, 'cx> {
    /// Creates a builder using the currently available name-resolution data.
    pub fn new(names: &'a NameDb<'cx>) -> Self {
        Self {
            names,
            repr: ProgramRepr::default(),
        }
    }

    /// Builds a program representation for one entry file.
    pub fn build(
        mut self,
        file_path: FilePath<'cx>,
        file: &'cx ast::File<'cx>,
    ) -> ProgramRepr<'cx> {
        let root = Some(self.names.root_scope());
        let items = self.collect_items(file.items, root);
        self.repr.add_file(File {
            id: self.repr.next_file_id(),
            file_path,
            items,
        });
        self.repr
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
        let id = self.repr.next_item_id();
        let def = self.def_for_item(item);
        let name = item.ident().map(|ident| ident.inner);
        let source_visibility = item_source_visibility(item);
        let mut module_children = None;
        let kind = match item {
            ast::Item::Const(item) => {
                let ty = self.collect_type(item.ty, TypeSource::ConstType);
                let body = self.collect_body(
                    BodyOwner::Item(id),
                    def.and_then(|def| self.names.def_body_scope(def)),
                    BodyKind::Expr,
                );
                ItemKind::Const { ty, body }
            }
            ast::Item::Enum(item) => {
                let variants = item
                    .variants
                    .iter()
                    .map(|variant| self.collect_variant(variant))
                    .collect();
                ItemKind::Enum { variants }
            }
            ast::Item::Fn(item) => {
                let signature = self.collect_signature(SignatureSource::ItemFn, &item.sig);
                let body = self.collect_body(
                    BodyOwner::Item(id),
                    def.and_then(|def| self.names.def_body_scope(def)),
                    BodyKind::Block,
                );
                ItemKind::Fn { signature, body }
            }
            ast::Item::Impl(item) => {
                let self_ty = self.collect_type(item.self_ty, TypeSource::ImplSelf);
                let items = item
                    .items
                    .iter()
                    .map(|item| self.collect_impl_item(item))
                    .collect();
                ItemKind::Impl {
                    trait_: item.trait_.as_ref().map(Path::from_ast),
                    self_ty,
                    items,
                }
            }
            ast::Item::Mod(item) => {
                let scope = def.and_then(|def| self.names.def_path_scope(def));
                module_children = item.items.map(|items| (items, scope));
                ItemKind::Mod {
                    is_inline: item.is_inline,
                    scope,
                    items: Vec::new(),
                }
            }
            ast::Item::Struct(item) => {
                let fields = item
                    .fields
                    .iter()
                    .map(|field| self.collect_struct_field(field))
                    .collect();
                ItemKind::Struct { fields }
            }
            ast::Item::Trait(item) => {
                let items = item
                    .items
                    .iter()
                    .map(|item| self.collect_trait_item(item))
                    .collect();
                ItemKind::Trait { items }
            }
            ast::Item::Type(item) => {
                let ty = self.collect_type(item.ty, TypeSource::TypeAlias);
                ItemKind::Type { ty }
            }
            ast::Item::Use(_) => ItemKind::Use,
        };

        self.repr.add_item(Item {
            id,
            name,
            source_visibility,
            def,
            parent_scope,
            kind,
        });

        if let Some((children, scope)) = module_children {
            let items = self.collect_items(children, scope);
            if let ItemKind::Mod {
                items: module_items,
                ..
            } = &mut self.repr[id].kind
            {
                *module_items = items;
            }
        }

        id
    }

    fn collect_struct_field(&mut self, field: &'cx ast::Field<'cx>) -> FieldId {
        let id = self.repr.next_field_id();
        let ty = self.collect_type(&field.ty, TypeSource::StructField);
        self.repr.add_field(Field {
            id,
            name: field.ident.inner,
            source_visibility: SourceVisibility::from_ast(&field.vis),
            ty,
            source: FieldSource::Struct,
        });
        id
    }

    fn collect_variant(&mut self, variant: &'cx ast::Variant<'cx>) -> VariantId {
        let id = self.repr.next_variant_id();
        let def = self.def_for_variant(variant);
        let (fields, discriminant) = match &variant.kind {
            ast::VariantKind::Fields(fields) => (
                fields
                    .iter()
                    .map(|field| self.collect_variant_field(field))
                    .collect(),
                None,
            ),
            ast::VariantKind::Discriminant(_) => (
                Vec::new(),
                Some(self.collect_body(BodyOwner::Variant(id), None, BodyKind::Expr)),
            ),
            ast::VariantKind::Unit => (Vec::new(), None),
        };
        self.repr.add_variant(Variant {
            id,
            def,
            name: variant.ident.inner,
            fields,
            discriminant,
        });
        id
    }

    fn collect_variant_field(&mut self, field: &'cx ast::VariantField<'cx>) -> FieldId {
        let id = self.repr.next_field_id();
        let ty = self.collect_type(&field.ty, TypeSource::VariantField);
        self.repr.add_field(Field {
            id,
            name: field.ident.inner,
            source_visibility: SourceVisibility::Private,
            ty,
            source: FieldSource::Variant,
        });
        id
    }

    fn collect_impl_item(&mut self, item: &'cx ast::ImplItem<'cx>) -> AssocItemId {
        let id = self.repr.next_assoc_item_id();
        let def = self.def_for_impl_item(item);
        let kind = match item {
            ast::ImplItem::Const(item) => {
                let ty = self.collect_type(item.ty, TypeSource::AssocConstType);
                let body = self.collect_body(
                    BodyOwner::AssocItem(id),
                    def.and_then(|def| self.names.def_body_scope(def)),
                    BodyKind::Expr,
                );
                AssocItemKind::ImplConst { ty, body }
            }
            ast::ImplItem::Fn(item) => {
                let signature = self.collect_signature(SignatureSource::ImplFn, &item.sig);
                let body = self.collect_body(
                    BodyOwner::AssocItem(id),
                    def.and_then(|def| self.names.def_body_scope(def)),
                    BodyKind::Block,
                );
                AssocItemKind::ImplFn { signature, body }
            }
            ast::ImplItem::Type(item) => {
                let ty = self.collect_type(item.ty, TypeSource::AssocTypeValue);
                AssocItemKind::ImplType { ty }
            }
        };
        self.repr.add_assoc_item(AssocItem {
            id,
            name: item.ident().inner,
            def,
            kind,
        });
        id
    }

    fn collect_trait_item(&mut self, item: &'cx ast::TraitItem<'cx>) -> AssocItemId {
        let id = self.repr.next_assoc_item_id();
        let def = self.def_for_trait_item(item);
        let kind = match item {
            ast::TraitItem::Const(item) => {
                let ty = self.collect_type(item.ty, TypeSource::AssocConstType);
                let default = item.default.map(|_| {
                    self.collect_body(
                        BodyOwner::AssocItem(id),
                        def.and_then(|def| self.names.def_body_scope(def)),
                        BodyKind::Expr,
                    )
                });
                AssocItemKind::TraitConst { ty, default }
            }
            ast::TraitItem::Fn(item) => {
                let signature = self.collect_signature(SignatureSource::TraitFn, &item.sig);
                let default = item.default.as_ref().map(|_| {
                    self.collect_body(
                        BodyOwner::AssocItem(id),
                        def.and_then(|def| self.names.def_body_scope(def)),
                        BodyKind::Block,
                    )
                });
                AssocItemKind::TraitFn { signature, default }
            }
            ast::TraitItem::Type(item) => {
                let default = item
                    .default
                    .map(|ty| self.collect_type(ty, TypeSource::AssocTypeValue));
                AssocItemKind::TraitType { default }
            }
        };
        self.repr.add_assoc_item(AssocItem {
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
    ) -> SignatureId {
        let types = signature
            .params
            .iter()
            .enumerate()
            .map(|(index, param)| {
                self.collect_type(&param.pat.ty, TypeSource::SignatureParam { index })
            })
            .collect();
        let id = self.repr.next_signature_id();
        self.repr.add_signature(Signature { id, source, types });
        id
    }

    fn collect_type(&mut self, ty: &'cx ast::Type<'cx>, source: TypeSource) -> TypeId {
        let id = self.repr.next_type_id();
        self.repr.add_type(Type { id, ty, source });
        id
    }

    fn collect_body(&mut self, owner: BodyOwner, scope: Option<ScopeId>, kind: BodyKind) -> BodyId {
        let id = self.repr.next_body_id();
        self.repr.add_body(Body {
            id,
            owner,
            scope,
            kind,
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

/// Rust source program representation produced for semantic phases.
#[derive(Debug, Default)]
pub struct ProgramRepr<'cx> {
    files: Vec<File<'cx>>,
    items: Vec<Item<'cx>>,
    signatures: Vec<Signature>,
    fields: Vec<Field<'cx>>,
    variants: Vec<Variant<'cx>>,
    assoc_items: Vec<AssocItem<'cx>>,
    bodies: Vec<Body>,
    types: Vec<Type<'cx>>,
}

impl<'cx> ProgramRepr<'cx> {
    /// Returns all represented files.
    pub fn files(&self) -> &[File<'cx>] {
        &self.files
    }

    /// Returns all represented item declarations.
    pub fn items(&self) -> &[Item<'cx>] {
        &self.items
    }

    /// Returns all represented function-like signatures.
    pub fn signatures(&self) -> &[Signature] {
        &self.signatures
    }

    /// Returns all represented fields.
    pub fn fields(&self) -> &[Field<'cx>] {
        &self.fields
    }

    /// Returns all represented enum variants.
    pub fn variants(&self) -> &[Variant<'cx>] {
        &self.variants
    }

    /// Returns all represented associated items.
    pub fn assoc_items(&self) -> &[AssocItem<'cx>] {
        &self.assoc_items
    }

    /// Returns all represented body entries.
    pub fn bodies(&self) -> &[Body] {
        &self.bodies
    }

    /// Returns all represented source types.
    pub fn types(&self) -> &[Type<'cx>] {
        &self.types
    }

    fn next_file_id(&self) -> FileId {
        FileId::new(self.files.len())
    }

    fn add_file(&mut self, file: File<'cx>) {
        let id = file.id;
        assert_eq!(id, self.next_file_id());
        self.files.push(file);
    }

    fn next_item_id(&self) -> ItemId {
        ItemId::new(self.items.len())
    }

    fn add_item(&mut self, item: Item<'cx>) {
        let id = item.id;
        assert_eq!(id, self.next_item_id());
        self.items.push(item);
    }

    fn next_signature_id(&self) -> SignatureId {
        SignatureId::new(self.signatures.len())
    }

    fn add_signature(&mut self, signature: Signature) {
        let id = signature.id;
        assert_eq!(id, self.next_signature_id());
        self.signatures.push(signature);
    }

    fn next_field_id(&self) -> FieldId {
        FieldId::new(self.fields.len())
    }

    fn add_field(&mut self, field: Field<'cx>) {
        let id = field.id;
        assert_eq!(id, self.next_field_id());
        self.fields.push(field);
    }

    fn next_variant_id(&self) -> VariantId {
        VariantId::new(self.variants.len())
    }

    fn add_variant(&mut self, variant: Variant<'cx>) {
        let id = variant.id;
        assert_eq!(id, self.next_variant_id());
        self.variants.push(variant);
    }

    fn next_assoc_item_id(&self) -> AssocItemId {
        AssocItemId::new(self.assoc_items.len())
    }

    fn add_assoc_item(&mut self, item: AssocItem<'cx>) {
        let id = item.id;
        assert_eq!(id, self.next_assoc_item_id());
        self.assoc_items.push(item);
    }

    fn next_body_id(&self) -> BodyId {
        BodyId::new(self.bodies.len())
    }

    fn add_body(&mut self, body: Body) {
        let id = body.id;
        assert_eq!(id, self.next_body_id());
        self.bodies.push(body);
    }

    fn next_type_id(&self) -> TypeId {
        TypeId::new(self.types.len())
    }

    fn add_type(&mut self, ty: Type<'cx>) {
        let id = ty.id;
        assert_eq!(id, self.next_type_id());
        self.types.push(ty);
    }
}

impl<'cx> Index<FileId> for ProgramRepr<'cx> {
    type Output = File<'cx>;

    fn index(&self, id: FileId) -> &Self::Output {
        &self.files[id.index()]
    }
}

impl<'cx> Index<ItemId> for ProgramRepr<'cx> {
    type Output = Item<'cx>;

    fn index(&self, id: ItemId) -> &Self::Output {
        &self.items[id.index()]
    }
}

impl IndexMut<ItemId> for ProgramRepr<'_> {
    fn index_mut(&mut self, id: ItemId) -> &mut Self::Output {
        &mut self.items[id.index()]
    }
}

impl<'cx> Index<SignatureId> for ProgramRepr<'cx> {
    type Output = Signature;

    fn index(&self, id: SignatureId) -> &Self::Output {
        &self.signatures[id.index()]
    }
}

impl<'cx> Index<FieldId> for ProgramRepr<'cx> {
    type Output = Field<'cx>;

    fn index(&self, id: FieldId) -> &Self::Output {
        &self.fields[id.index()]
    }
}

impl<'cx> Index<VariantId> for ProgramRepr<'cx> {
    type Output = Variant<'cx>;

    fn index(&self, id: VariantId) -> &Self::Output {
        &self.variants[id.index()]
    }
}

impl<'cx> Index<AssocItemId> for ProgramRepr<'cx> {
    type Output = AssocItem<'cx>;

    fn index(&self, id: AssocItemId) -> &Self::Output {
        &self.assoc_items[id.index()]
    }
}

impl<'cx> Index<BodyId> for ProgramRepr<'cx> {
    type Output = Body;

    fn index(&self, id: BodyId) -> &Self::Output {
        &self.bodies[id.index()]
    }
}

impl<'cx> Index<TypeId> for ProgramRepr<'cx> {
    type Output = Type<'cx>;

    fn index(&self, id: TypeId) -> &Self::Output {
        &self.types[id.index()]
    }
}

/// One represented source file.
#[derive(Debug)]
pub struct File<'cx> {
    /// File id in the representation.
    pub id: FileId,
    /// Interned file path.
    pub file_path: FilePath<'cx>,
    /// Top-level represented items in source order.
    pub items: Vec<ItemId>,
}

/// One represented item declaration.
#[derive(Debug)]
pub struct Item<'cx> {
    /// Item id in the representation.
    pub id: ItemId,
    /// Item name, when the item has one source-level name.
    pub name: Option<Name<'cx>>,
    /// Source item visibility.
    pub source_visibility: SourceVisibility<'cx>,
    /// Definition linked from the current name-resolution data, if available.
    pub def: Option<DefId>,
    /// Scope containing this item.
    pub parent_scope: Option<ScopeId>,
    /// Source-shaped item payload.
    pub kind: ItemKind<'cx>,
}

/// Source-shaped payload for an item declaration.
#[derive(Debug)]
pub enum ItemKind<'cx> {
    /// Constant item.
    Const {
        /// Constant type.
        ty: TypeId,
        /// Initializer body.
        body: BodyId,
    },
    /// Enum item.
    Enum {
        /// Represented variants.
        variants: Vec<VariantId>,
    },
    /// Function item.
    Fn {
        /// Represented signature.
        signature: SignatureId,
        /// Function body.
        body: BodyId,
    },
    /// Implementation block.
    Impl {
        /// Implemented trait path, if this is a trait impl.
        trait_: Option<Path<'cx>>,
        /// Implementing self type.
        self_ty: TypeId,
        /// Represented associated items.
        items: Vec<AssocItemId>,
    },
    /// Module item.
    Mod {
        /// Whether this module contains its items inline.
        is_inline: bool,
        /// Scope used for module members, if linked from the current name-resolution data.
        scope: Option<ScopeId>,
        /// Inline child items represented under this module.
        items: Vec<ItemId>,
    },
    /// Struct item.
    Struct {
        /// Represented fields.
        fields: Vec<FieldId>,
    },
    /// Trait item.
    Trait {
        /// Represented associated items.
        items: Vec<AssocItemId>,
    },
    /// Type alias item.
    Type {
        /// Aliased type.
        ty: TypeId,
    },
    /// Use item.
    Use,
}

/// One represented function-like signature.
#[derive(Debug)]
pub struct Signature {
    /// Signature id in the representation.
    pub id: SignatureId,
    /// Source signature.
    pub source: SignatureSource,
    /// Source types used by the return parameter and input parameters.
    pub types: Vec<TypeId>,
}

/// Source role for a represented signature.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignatureSource {
    /// Free function signature.
    ItemFn,
    /// Impl associated function signature.
    ImplFn,
    /// Trait associated function signature.
    TraitFn,
}

/// One represented field declaration.
#[derive(Debug)]
pub struct Field<'cx> {
    /// Field id in the representation.
    pub id: FieldId,
    /// Field name.
    pub name: Name<'cx>,
    /// Source field visibility.
    pub source_visibility: SourceVisibility<'cx>,
    /// Field type.
    pub ty: TypeId,
    /// Source field kind.
    pub source: FieldSource,
}

/// Source for a represented field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldSource {
    /// Struct field.
    Struct,
    /// Enum variant field.
    Variant,
}

/// One represented enum variant declaration.
#[derive(Debug)]
pub struct Variant<'cx> {
    /// Variant id in the representation.
    pub id: VariantId,
    /// Definition linked from the current name-resolution data, if available.
    pub def: Option<DefId>,
    /// Variant name.
    pub name: Name<'cx>,
    /// Represented payload fields.
    pub fields: Vec<FieldId>,
    /// Discriminant body, if present.
    pub discriminant: Option<BodyId>,
}

/// One represented associated item declaration.
#[derive(Debug)]
pub struct AssocItem<'cx> {
    /// Associated item id in the representation.
    pub id: AssocItemId,
    /// Associated item name.
    pub name: Name<'cx>,
    /// Definition linked from the current name-resolution data, if available.
    pub def: Option<DefId>,
    /// Source-shaped associated item payload.
    pub kind: AssocItemKind,
}

/// Source-shaped payload for an associated item declaration.
#[derive(Debug)]
pub enum AssocItemKind {
    /// Impl associated const.
    ImplConst {
        /// Associated const type.
        ty: TypeId,
        /// Initializer body.
        body: BodyId,
    },
    /// Impl associated function.
    ImplFn {
        /// Represented signature.
        signature: SignatureId,
        /// Function body.
        body: BodyId,
    },
    /// Impl associated type.
    ImplType {
        /// Assigned type.
        ty: TypeId,
    },
    /// Trait associated const.
    TraitConst {
        /// Associated const type.
        ty: TypeId,
        /// Optional default body.
        default: Option<BodyId>,
    },
    /// Trait associated function.
    TraitFn {
        /// Represented signature.
        signature: SignatureId,
        /// Optional default body.
        default: Option<BodyId>,
    },
    /// Trait associated type.
    TraitType {
        /// Optional default type.
        default: Option<TypeId>,
    },
}

/// Source-level visibility for represented declarations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceVisibility<'cx> {
    /// Public visibility.
    Public,
    /// Restricted visibility path, such as `crate` or `foo::bar`.
    Restricted(Path<'cx>),
    /// Inherited private visibility.
    Private,
}

/// Source path represented without depending on the AST crate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Path<'cx> {
    /// Path segment names in source order.
    pub segments: Vec<Name<'cx>>,
}

impl<'cx> Path<'cx> {
    fn from_ast(path: &ast::Path<'cx>) -> Self {
        Self {
            segments: path
                .segments
                .iter()
                .map(|segment| segment.ident.inner)
                .collect(),
        }
    }
}

impl<'cx> SourceVisibility<'cx> {
    fn from_ast(visibility: &ast::Visibility<'cx>) -> Self {
        match visibility {
            ast::Visibility::Public(_) => Self::Public,
            ast::Visibility::Restricted(path) => Self::Restricted(Path::from_ast(path)),
            ast::Visibility::Private => Self::Private,
        }
    }
}

fn item_source_visibility<'cx>(item: &'cx ast::Item<'cx>) -> SourceVisibility<'cx> {
    match item {
        ast::Item::Const(item) => SourceVisibility::from_ast(&item.vis),
        ast::Item::Enum(item) => SourceVisibility::from_ast(&item.vis),
        ast::Item::Fn(item) => SourceVisibility::from_ast(&item.vis),
        ast::Item::Impl(_) => SourceVisibility::Private,
        ast::Item::Mod(item) => SourceVisibility::from_ast(&item.vis),
        ast::Item::Struct(item) => SourceVisibility::from_ast(&item.vis),
        ast::Item::Trait(item) => SourceVisibility::from_ast(&item.vis),
        ast::Item::Type(item) => SourceVisibility::from_ast(&item.vis),
        ast::Item::Use(item) => SourceVisibility::from_ast(&item.vis),
    }
}

/// One source body entry.
#[derive(Debug)]
pub struct Body {
    /// Body id in the representation.
    pub id: BodyId,
    /// Owner of this body.
    pub owner: BodyOwner,
    /// Scope containing body-local bindings, if linked from the current name-resolution data.
    pub scope: Option<ScopeId>,
    /// Source body kind.
    pub kind: BodyKind,
}

/// Owner of one source body.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BodyOwner {
    /// Body owned by an item.
    Item(ItemId),
    /// Body owned by an associated item.
    AssocItem(AssocItemId),
    /// Body owned by an enum variant discriminant.
    Variant(VariantId),
}

/// Source kind for a body entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BodyKind {
    /// Block body.
    Block,
    /// Expression body.
    Expr,
}

/// One source type occurrence.
#[derive(Debug)]
pub struct Type<'cx> {
    /// Type id in the representation.
    pub id: TypeId,
    /// Original semantic AST type.
    pub ty: &'cx ast::Type<'cx>,
    /// Source role for this type occurrence.
    pub source: TypeSource,
}

/// Source role for a represented type occurrence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeSource {
    /// Constant item type.
    ConstType,
    /// Function signature parameter or return type.
    SignatureParam {
        /// Parameter index in the source signature, with return type at index `0`.
        index: usize,
    },
    /// Impl self type.
    ImplSelf,
    /// Struct field type.
    StructField,
    /// Enum variant field type.
    VariantField,
    /// Type alias target.
    TypeAlias,
    /// Associated const type.
    AssocConstType,
    /// Associated type value.
    AssocTypeValue,
}

macro_rules! id_type {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(usize);

        impl $name {
            /// Creates an id from a raw index.
            pub const fn new(index: usize) -> Self {
                Self(index)
            }

            /// Returns the raw index represented by this id.
            pub const fn index(self) -> usize {
                self.0
            }
        }
    };
}

id_type! {
    /// Stable identity for one represented source file.
    FileId
}

id_type! {
    /// Stable identity for one represented item declaration.
    ItemId
}

id_type! {
    /// Stable identity for one represented function-like signature.
    SignatureId
}

id_type! {
    /// Stable identity for one represented field declaration.
    FieldId
}

id_type! {
    /// Stable identity for one represented enum variant declaration.
    VariantId
}

id_type! {
    /// Stable identity for one represented associated item declaration.
    AssocItemId
}

id_type! {
    /// Stable identity for one source body entry.
    BodyId
}

id_type! {
    /// Stable identity for one source type occurrence.
    TypeId
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn_sem_ast::SyntaxCx;
    use syn_sem_common::CommonCx;
    use syn_sem_name::{DefKind, NameDb, Origin, Visibility as NameVisibility};

    fn parsed_model<'cx>(
        ccx: &'cx CommonCx,
        scx: &'cx SyntaxCx<'cx>,
        code: &str,
    ) -> ProgramRepr<'cx> {
        let file_path = ccx.intern("test.rs");
        let text = ccx.intern(code);
        scx.parse_virtual_file(file_path, text).unwrap();
        let file = scx.lookup_source(file_path).unwrap().ast();
        let names = NameDb::default();
        ProgramReprBuilder::new(&names).build(file_path, file)
    }

    fn type_sources(model: &ProgramRepr<'_>) -> Vec<TypeSource> {
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

    fn named_item<'m, 'cx>(model: &'m ProgramRepr<'cx>, name: &str) -> &'m Item<'cx> {
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
    fn builds_items_signatures_and_body_handles() {
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
        assert_eq!(model.bodies().len(), 2);
        assert!(model.types().len() >= 3);

        let ItemKind::Fn {
            signature, body, ..
        } = model[model.files()[0].items[0]].kind
        else {
            panic!("expected function item");
        };
        assert_eq!(model[signature].types.len(), 2);
        assert!(matches!(model[body].kind, BodyKind::Block));
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
    fn covers_repr_native_names_visibility_and_paths() {
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
        let SourceVisibility::Restricted(path) = &module.source_visibility else {
            panic!("expected restricted module source visibility");
        };
        assert_eq!(path.segments.len(), 1);
        assert_eq!(path.segments[0].as_ref(), "crate");

        let struct_item = named_item(&model, "S");
        assert!(matches!(
            struct_item.source_visibility,
            SourceVisibility::Public
        ));

        let impl_item = model
            .items()
            .iter()
            .find(|item| matches!(item.kind, ItemKind::Impl { .. }))
            .expect("expected impl item");
        assert!(impl_item.name.is_none());
        assert!(matches!(
            impl_item.source_visibility,
            SourceVisibility::Private
        ));
        let ItemKind::Impl { trait_, .. } = &impl_item.kind else {
            panic!("expected impl item");
        };
        let trait_ = trait_.as_ref().expect("expected trait path");
        assert_eq!(trait_.segments.len(), 1);
        assert_eq!(trait_.segments[0].as_ref(), "Tr");

        let public_field = model
            .fields()
            .iter()
            .find(|field| field.name.as_ref() == "field")
            .expect("expected public field");
        assert!(matches!(
            public_field.source_visibility,
            SourceVisibility::Public
        ));

        let private_field = model
            .fields()
            .iter()
            .find(|field| field.name.as_ref() == "private")
            .expect("expected private field");
        assert!(matches!(
            private_field.source_visibility,
            SourceVisibility::Private
        ));

        let assoc_names: Vec<_> = model
            .assoc_items()
            .iter()
            .map(|item| item.name.as_ref())
            .collect();
        assert_eq!(assoc_names, ["Item", "Item"]);
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
                AssocItemKind::ImplConst { ty, body, .. } => {
                    assert_eq!(model[ty].source, TypeSource::AssocConstType);
                    assert_eq!(model[body].owner, BodyOwner::AssocItem(item.id));
                }
                AssocItemKind::ImplFn {
                    signature, body, ..
                } => {
                    assert!(matches!(model[signature].source, SignatureSource::ImplFn));
                    assert_eq!(model[body].owner, BodyOwner::AssocItem(item.id));
                    assert!(matches!(model[body].kind, BodyKind::Block));
                }
                AssocItemKind::ImplType { ty, .. } => {
                    assert_eq!(model[ty].source, TypeSource::AssocTypeValue);
                }
                AssocItemKind::TraitConst { ty, default, .. } => {
                    assert_eq!(model[ty].source, TypeSource::AssocConstType);
                    let default = default.expect("trait const default should create a body");
                    assert_eq!(model[default].owner, BodyOwner::AssocItem(item.id));
                    assert!(matches!(model[default].kind, BodyKind::Expr));
                }
                AssocItemKind::TraitFn {
                    signature, default, ..
                } => {
                    assert!(matches!(model[signature].source, SignatureSource::TraitFn));
                    let default = default.expect("trait fn default should create a body");
                    assert_eq!(model[default].owner, BodyOwner::AssocItem(item.id));
                    assert!(matches!(model[default].kind, BodyKind::Block));
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
            .any(|ty| matches!(ty.ty, ast::Type::Array(_))));
        assert!(model
            .types()
            .iter()
            .any(|ty| matches!(ty.ty, ast::Type::Reference(_))));
        assert!(model
            .types()
            .iter()
            .any(|ty| matches!(ty.ty, ast::Type::Slice(_))));
        assert!(model
            .types()
            .iter()
            .any(|ty| matches!(ty.ty, ast::Type::Tuple(_))));
        assert!(model
            .types()
            .iter()
            .any(|ty| matches!(ty.ty, ast::Type::Path(_))));
    }

    #[test]
    fn covers_body_owners_and_kinds() {
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

        assert!(model.bodies().iter().any(|body| {
            matches!(body.owner, BodyOwner::Item(_)) && matches!(body.kind, BodyKind::Expr)
        }));
        assert!(model.bodies().iter().any(|body| {
            matches!(body.owner, BodyOwner::Item(_)) && matches!(body.kind, BodyKind::Block)
        }));
        assert!(model.bodies().iter().any(|body| {
            matches!(body.owner, BodyOwner::Variant(_)) && matches!(body.kind, BodyKind::Expr)
        }));
        assert!(model.bodies().iter().any(|body| {
            matches!(body.owner, BodyOwner::AssocItem(_)) && matches!(body.kind, BodyKind::Expr)
        }));
        assert!(model.bodies().iter().any(|body| {
            matches!(body.owner, BodyOwner::AssocItem(_)) && matches!(body.kind, BodyKind::Block)
        }));
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
        assert!(model.variants().iter().any(|variant| {
            matches!(
                variant.discriminant,
                Some(body) if model[body].owner == BodyOwner::Variant(variant.id)
            )
        }));
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

        let model = ProgramReprBuilder::new(&names).build(file_path, file);
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

        let model = ProgramReprBuilder::new(&names).build(file_path, file);
        assert_eq!(model.items()[1].def, Some(first_impl_def));
        assert_eq!(model.items()[2].def, Some(second_impl_def));

        let ItemKind::Impl { items, .. } = &model.items()[1].kind else {
            panic!("expected first repr impl");
        };
        assert_eq!(model[items[0]].def, Some(first_fn_def));

        let ItemKind::Impl { items, .. } = &model.items()[2].kind else {
            panic!("expected second repr impl");
        };
        assert_eq!(model[items[0]].def, Some(second_fn_def));
    }
}
